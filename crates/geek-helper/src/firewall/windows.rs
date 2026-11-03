//! Windows Filtering Platform: a sublayer of our own at the ALE connect
//! layers (every outbound TCP connection and UDP flow, v4 and v6), a block
//! for everything at the lowest weight, and permits above it.
//!
//! Normal mode uses a dynamic WFP session: if the helper dies, Windows
//! deletes the filters with it, so a crash never leaves the user offline.
//! Strict mode adds them persistent instead, and they hold across crashes
//! and reboots (the BFE enforces them at boot, before any service runs)
//! until the helper releases them.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::ptr::{null, null_mut};

use geek_ipc::KillSwitch;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::NetworkManagement::IpHelper::ConvertInterfaceAliasToLuid;
use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;
use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::*;
use windows_sys::Win32::System::Rpc::RPC_C_AUTHN_WINNT;

use super::{LAN_V4, LAN_V6};

const SUBLAYER: GUID = GUID::from_u128(0x5f1c2b8e_6a0d_4c77_9e3a_1b7c0d2e4f00);
/// Filter keys are fixed (base + index) so a later run, or release after a
/// reboot, can delete strict mode's persistent filters by key.
const FILTER_BASE: u128 = 0x5f1c2b8e_6a0d_4c77_9e3a_1b7c0d2e5000;
const MAX_FILTERS: u128 = 64;

#[derive(Default)]
pub struct Firewall {
    /// The engine handle (as an integer, so the struct is Send) and whether
    /// its session is dynamic.
    engine: Option<(usize, bool)>,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn check(what: &str, code: u32) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!("{what}: 0x{code:08x}"))
    }
}

fn open(dynamic: bool) -> Result<HANDLE, String> {
    let mut name = wide("GeekVPN kill switch");
    // SAFETY: zeroed is a valid FWPM_SESSION0 (all pointers null).
    let mut session: FWPM_SESSION0 = unsafe { std::mem::zeroed() };
    session.displayData.name = name.as_mut_ptr();
    session.flags = if dynamic { FWPM_SESSION_FLAG_DYNAMIC } else { 0 };
    let mut h: HANDLE = null_mut();
    // SAFETY: `session` and `name` outlive the call; `h` receives the handle.
    check("FwpmEngineOpen0", unsafe { FwpmEngineOpen0(null(), RPC_C_AUTHN_WINNT, null(), &session, &mut h) })?;
    Ok(h)
}

/// One filter's conditions, with the values they point at kept alive.
enum Cond {
    Loopback,
    App(*mut FWP_BYTE_BLOB),
    Interface(u64),
    UdpPort(u16),
    V4(FWP_V4_ADDR_AND_MASK),
    V6(FWP_V6_ADDR_AND_MASK),
}

struct Rule {
    permit: bool,
    v6: bool,
    conds: Vec<Cond>,
}

fn add_filter(h: HANDLE, index: u128, rule: &mut Rule, persistent: bool) -> Result<(), String> {
    let mut name = wide(if rule.permit { "GeekVPN kill switch: allow" } else { "GeekVPN kill switch: block" });
    let mut conds: Vec<FWPM_FILTER_CONDITION0> = Vec::new();
    for c in rule.conds.iter_mut() {
        // SAFETY: zeroed is a valid condition (FWP_EMPTY, null pointers);
        // every field set below points into `rule`, which outlives the call.
        let mut fc: FWPM_FILTER_CONDITION0 = unsafe { std::mem::zeroed() };
        fc.matchType = FWP_MATCH_EQUAL;
        match c {
            Cond::Loopback => {
                fc.fieldKey = FWPM_CONDITION_FLAGS;
                fc.matchType = FWP_MATCH_FLAGS_ALL_SET;
                fc.conditionValue.r#type = FWP_UINT32;
                fc.conditionValue.Anonymous.uint32 = FWP_CONDITION_FLAG_IS_LOOPBACK;
            }
            Cond::App(blob) => {
                fc.fieldKey = FWPM_CONDITION_ALE_APP_ID;
                fc.conditionValue.r#type = FWP_BYTE_BLOB_TYPE;
                fc.conditionValue.Anonymous.byteBlob = *blob;
            }
            Cond::Interface(luid) => {
                fc.fieldKey = FWPM_CONDITION_IP_LOCAL_INTERFACE;
                fc.conditionValue.r#type = FWP_UINT64;
                fc.conditionValue.Anonymous.uint64 = luid as *mut u64;
            }
            Cond::UdpPort(port) => {
                // Two conditions: UDP, and the port.
                let mut proto = fc;
                proto.fieldKey = FWPM_CONDITION_IP_PROTOCOL;
                proto.conditionValue.r#type = FWP_UINT8;
                proto.conditionValue.Anonymous.uint8 = 17;
                conds.push(proto);
                fc.fieldKey = FWPM_CONDITION_IP_REMOTE_PORT;
                fc.conditionValue.r#type = FWP_UINT16;
                fc.conditionValue.Anonymous.uint16 = *port;
            }
            Cond::V4(m) => {
                fc.fieldKey = FWPM_CONDITION_IP_REMOTE_ADDRESS;
                fc.conditionValue.r#type = FWP_V4_ADDR_MASK;
                fc.conditionValue.Anonymous.v4AddrMask = m as *mut _;
            }
            Cond::V6(m) => {
                fc.fieldKey = FWPM_CONDITION_IP_REMOTE_ADDRESS;
                fc.conditionValue.r#type = FWP_V6_ADDR_MASK;
                fc.conditionValue.Anonymous.v6AddrMask = m as *mut _;
            }
        }
        conds.push(fc);
    }
    // SAFETY: zeroed is a valid FWPM_FILTER0; the pointers set below live
    // until FwpmFilterAdd0 returns.
    let mut f: FWPM_FILTER0 = unsafe { std::mem::zeroed() };
    f.filterKey = GUID::from_u128(FILTER_BASE + index);
    f.displayData.name = name.as_mut_ptr();
    f.flags = if persistent { FWPM_FILTER_FLAG_PERSISTENT } else { 0 };
    f.layerKey = if rule.v6 { FWPM_LAYER_ALE_AUTH_CONNECT_V6 } else { FWPM_LAYER_ALE_AUTH_CONNECT_V4 };
    f.subLayerKey = SUBLAYER;
    f.weight.r#type = FWP_UINT8;
    // Permits outrank the catch-all block inside our sublayer.
    f.weight.Anonymous.uint8 = if rule.permit { 10 } else { 0 };
    f.numFilterConditions = conds.len() as u32;
    f.filterCondition = if conds.is_empty() { null_mut() } else { conds.as_mut_ptr() };
    f.action.r#type = if rule.permit { FWP_ACTION_PERMIT } else { FWP_ACTION_BLOCK };
    // SAFETY: see above.
    check("FwpmFilterAdd0", unsafe { FwpmFilterAdd0(h, &f, null_mut(), null_mut()) })
}

fn v4_mask(cidr: &str) -> Option<FWP_V4_ADDR_AND_MASK> {
    let (ip, bits) = cidr.split_once('/')?;
    let ip: Ipv4Addr = ip.parse().ok()?;
    let bits: u32 = bits.parse().ok()?;
    let mask = if bits == 0 { 0 } else { u32::MAX << (32 - bits) };
    Some(FWP_V4_ADDR_AND_MASK { addr: u32::from(ip), mask })
}

fn v6_mask(cidr: &str) -> Option<FWP_V6_ADDR_AND_MASK> {
    let (ip, bits) = cidr.split_once('/')?;
    let ip: Ipv6Addr = ip.parse().ok()?;
    Some(FWP_V6_ADDR_AND_MASK { addr: ip.octets(), prefixLength: bits.parse().ok()? })
}

fn tun_luid(alias: &str) -> Option<u64> {
    let name = wide(alias);
    // SAFETY: zeroed is a valid NET_LUID_LH; `name` is NUL-terminated.
    let mut luid: NET_LUID_LH = unsafe { std::mem::zeroed() };
    let rc = unsafe { ConvertInterfaceAliasToLuid(name.as_ptr(), &mut luid) };
    // SAFETY: `Value` is the plain 64-bit view of the union.
    (rc == 0).then_some(unsafe { luid.Value })
}

/// Deletes our filters and sublayer; missing ones are fine.
fn delete_all(h: HANDLE) {
    for i in 0..MAX_FILTERS {
        let key = GUID::from_u128(FILTER_BASE + i);
        // SAFETY: `key` lives for the call.
        unsafe { FwpmFilterDeleteByKey0(h, &key) };
    }
    // SAFETY: as above.
    unsafe { FwpmSubLayerDeleteByKey0(h, &SUBLAYER) };
}

impl Firewall {
    fn handle(&self) -> Option<HANDLE> {
        self.engine.map(|(h, _)| h as HANDLE)
    }

    #[allow(dead_code)]
    pub fn engage(&mut self, ks: &KillSwitch, tun: Option<&str>, engine: &Path) -> Result<(), String> {
        self.engage_with_apps(ks, tun, engine, &[])
    }

    pub fn engage_with_apps(&mut self, ks: &KillSwitch, tun: Option<&str>, engine: &Path, extra_apps: &[String]) -> Result<(), String> {
        let dynamic = !ks.strict;
        if self.engine.is_some_and(|(_, d)| d != dynamic) {
            self.release()?;
        }
        if self.engine.is_none() {
            self.engine = Some((open(dynamic)? as usize, dynamic));
        }
        let h = self.handle().ok_or("no WFP engine")?;

        let path = wide(&engine.to_string_lossy());
        let mut app: *mut FWP_BYTE_BLOB = null_mut();
        // SAFETY: `path` is NUL-terminated; `app` is freed below.
        check("FwpmGetAppIdFromFileName0", unsafe { FwpmGetAppIdFromFileName0(path.as_ptr(), &mut app) })?;
        // Extra apps (geekcore) that also need to bypass the kill switch on Windows:
        // hev's traffic goes via geekcore's SOCKS, so geekcore's own outbound must not be blocked.
        let mut extra_blobs: Vec<(*mut FWP_BYTE_BLOB, Vec<u16>)> = Vec::new();
        for p in extra_apps {
            let w = wide(p);
            let mut blob: *mut FWP_BYTE_BLOB = null_mut();
            if unsafe { FwpmGetAppIdFromFileName0(w.as_ptr(), &mut blob) } == 0 {
                extra_blobs.push((blob, w));
            }
        }

        let mut rules: Vec<Rule> = Vec::new();
        for v6 in [false, true] {
            rules.push(Rule { permit: false, v6, conds: vec![] });
            rules.push(Rule { permit: true, v6, conds: vec![Cond::Loopback] });
            rules.push(Rule { permit: true, v6, conds: vec![Cond::App(app)] });
            for (blob, _) in &extra_blobs {
                rules.push(Rule { permit: true, v6, conds: vec![Cond::App(*blob)] });
            }
            if let Some(luid) = tun.and_then(tun_luid) {
                rules.push(Rule { permit: true, v6, conds: vec![Cond::Interface(luid)] });
            }
            rules.push(Rule { permit: true, v6, conds: vec![Cond::UdpPort(if v6 { 547 } else { 67 })] });
        }
        if ks.allow_lan {
            rules.extend(LAN_V4.iter().filter_map(|c| v4_mask(c)).map(|m| Rule { permit: true, v6: false, conds: vec![Cond::V4(m)] }));
            rules.extend(LAN_V6.iter().filter_map(|c| v6_mask(c)).map(|m| Rule { permit: true, v6: true, conds: vec![Cond::V6(m)] }));
        }

        let result = (|| {
            // SAFETY: `h` is an open engine handle.
            check("FwpmTransactionBegin0", unsafe { FwpmTransactionBegin0(h, 0) })?;
            delete_all(h);
            let mut name = wide("GeekVPN kill switch");
            // SAFETY: zeroed is a valid FWPM_SUBLAYER0; `name` outlives the call.
            let mut sub: FWPM_SUBLAYER0 = unsafe { std::mem::zeroed() };
            sub.subLayerKey = SUBLAYER;
            sub.displayData.name = name.as_mut_ptr();
            sub.flags = if dynamic { 0 } else { FWPM_SUBLAYER_FLAG_PERSISTENT };
            sub.weight = 0xFFFF;
            let added = (|| {
                check("FwpmSubLayerAdd0", unsafe { FwpmSubLayerAdd0(h, &sub, null_mut()) })?;
                for (i, r) in rules.iter_mut().enumerate() {
                    add_filter(h, i as u128, r, !dynamic)?;
                }
                Ok::<(), String>(())
            })();
            match added {
                // SAFETY: a transaction is open on `h`.
                Ok(()) => check("FwpmTransactionCommit0", unsafe { FwpmTransactionCommit0(h) }),
                Err(e) => {
                    unsafe { FwpmTransactionAbort0(h) };
                    Err(e)
                }
            }
        })();
        // SAFETY: `app` came from FwpmGetAppIdFromFileName0.
        unsafe { FwpmFreeMemory0(&mut app as *mut _ as *mut *mut core::ffi::c_void) };
        for (mut blob, _) in extra_blobs {
            unsafe { FwpmFreeMemory0(&mut blob as *mut _ as *mut *mut core::ffi::c_void) };
        }
        result
    }

    pub fn release(&mut self) -> Result<(), String> {
        // After a restart there is no session; a fresh, non-dynamic one can
        // delete strict mode's persistent filters.
        let (h, opened_here) = match self.handle() {
            Some(h) => (h, false),
            None => (open(false)?, true),
        };
        // SAFETY: `h` is an open engine handle for all calls below.
        let r = check("FwpmTransactionBegin0", unsafe { FwpmTransactionBegin0(h, 0) }).and_then(|()| {
            delete_all(h);
            check("FwpmTransactionCommit0", unsafe { FwpmTransactionCommit0(h) })
        });
        unsafe { FwpmEngineClose0(h) };
        let _ = opened_here;
        self.engine = None;
        r
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn masks() {
        let m = super::v4_mask("192.168.0.0/16").unwrap();
        assert_eq!((m.addr, m.mask), (0xC0A8_0000, 0xFFFF_0000));
        assert_eq!(super::v6_mask("fe80::/10").unwrap().prefixLength, 10);
    }
}
