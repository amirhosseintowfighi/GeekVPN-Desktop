//! The Iran lists out of Xray's `geoip.dat` and `geosite.dat`, for sing-box.
//!
//! In TUN mode sing-box does the routing, and sing-box 1.12+ no longer reads
//! Xray's geo files. Rather than ship a second copy of the same lists in
//! sing-box's format, the app pulls `geoip:ir`, `geosite:category-ir` (and
//! `domain:ir`) out of the files it already has, so both engines route by
//! exactly the same data.
//!
//! The files are protobuf (`GeoIPList`, `GeoSiteList` in Xray's
//! `app/router/routercommon`). Only the few fields used here are read.

use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GeoError {
    #[error("geo file is truncated or not protobuf")]
    Malformed,
    #[error("no `{0}` entry in the geo file")]
    Missing(String),
}

/// Domain and address rules, in sing-box's headless-rule vocabulary.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleLists {
    pub domain: Vec<String>,
    pub domain_suffix: Vec<String>,
    pub domain_keyword: Vec<String>,
    pub domain_regex: Vec<String>,
    pub ip_cidr: Vec<String>,
}

impl RuleLists {
    pub fn is_empty(&self) -> bool {
        self.domain.is_empty()
            && self.domain_suffix.is_empty()
            && self.domain_keyword.is_empty()
            && self.domain_regex.is_empty()
            && self.ip_cidr.is_empty()
    }
}

/// What Smart routing sends direct: v2rayNG's `WHITE_IRAN`, the same as
/// the Xray rules in `xray.rs`.
pub fn iran_rules(geoip: &[u8], geosite: &[u8]) -> Result<RuleLists, GeoError> {
    let mut r = RuleLists { domain_suffix: vec!["ir".into()], ..Default::default() };
    add_sites(geosite, "CATEGORY-IR", &mut r)?;
    r.ip_cidr = cidrs(geoip, "IR")?;
    Ok(r)
}

struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    fn varint(&mut self) -> Result<u64, GeoError> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let (&b, rest) = self.buf.split_first().ok_or(GeoError::Malformed)?;
            self.buf = rest;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(GeoError::Malformed)
    }

    /// The next field as (number, value); length-delimited values come back
    /// as their bytes, varints as an empty slice with the number in `n`.
    fn field(&mut self) -> Result<Option<(u64, Field<'a>)>, GeoError> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        let key = self.varint()?;
        let value = match key & 7 {
            0 => Field::Varint(self.varint()?),
            2 => {
                let len = usize::try_from(self.varint()?).map_err(|_| GeoError::Malformed)?;
                if len > self.buf.len() {
                    return Err(GeoError::Malformed);
                }
                let (bytes, rest) = self.buf.split_at(len);
                self.buf = rest;
                Field::Bytes(bytes)
            }
            1 => {
                self.skip(8)?;
                Field::Varint(0)
            }
            5 => {
                self.skip(4)?;
                Field::Varint(0)
            }
            _ => return Err(GeoError::Malformed),
        };
        Ok(Some((key >> 3, value)))
    }

    fn skip(&mut self, n: usize) -> Result<(), GeoError> {
        if n > self.buf.len() {
            return Err(GeoError::Malformed);
        }
        self.buf = &self.buf[n..];
        Ok(())
    }
}

enum Field<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
}

/// The entry whose `country_code` (field 1) is `code`, out of a list whose
/// entries are field 1 of the top-level message.
fn entry<'a>(file: &'a [u8], code: &str) -> Result<&'a [u8], GeoError> {
    let mut top = Reader { buf: file };
    while let Some((n, f)) = top.field()? {
        let (1, Field::Bytes(entry)) = (n, f) else { continue };
        let mut e = Reader { buf: entry };
        while let Some((n, f)) = e.field()? {
            if let (1, Field::Bytes(c)) = (n, f) {
                if c.eq_ignore_ascii_case(code.as_bytes()) {
                    return Ok(entry);
                }
                break;
            }
        }
    }
    Err(GeoError::Missing(code.to_ascii_lowercase()))
}

fn cidrs(geoip: &[u8], code: &str) -> Result<Vec<String>, GeoError> {
    let mut out = Vec::new();
    let mut e = Reader { buf: entry(geoip, code)? };
    while let Some((n, f)) = e.field()? {
        let (2, Field::Bytes(cidr)) = (n, f) else { continue };
        let (mut ip, mut prefix) = (&[][..], 0u64);
        let mut c = Reader { buf: cidr };
        while let Some((n, f)) = c.field()? {
            match (n, f) {
                (1, Field::Bytes(b)) => ip = b,
                (2, Field::Varint(p)) => prefix = p,
                _ => {}
            }
        }
        match ip.len() {
            4 => out.push(format!("{}/{prefix}", Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]))),
            16 => {
                let b: [u8; 16] = ip.try_into().map_err(|_| GeoError::Malformed)?;
                out.push(format!("{}/{prefix}", Ipv6Addr::from(b)));
            }
            _ => return Err(GeoError::Malformed),
        }
    }
    Ok(out)
}

fn add_sites(geosite: &[u8], code: &str, r: &mut RuleLists) -> Result<(), GeoError> {
    let mut e = Reader { buf: entry(geosite, code)? };
    while let Some((n, f)) = e.field()? {
        let (2, Field::Bytes(domain)) = (n, f) else { continue };
        let (mut kind, mut value) = (0u64, "");
        let mut d = Reader { buf: domain };
        while let Some((n, f)) = d.field()? {
            match (n, f) {
                (1, Field::Varint(k)) => kind = k,
                (2, Field::Bytes(v)) => value = std::str::from_utf8(v).map_err(|_| GeoError::Malformed)?,
                _ => {}
            }
        }
        let value = value.to_string();
        // Domain.Type: Plain = 0 (substring), Regex = 1, Domain = 2 (the
        // domain and its subdomains), Full = 3.
        match kind {
            0 => r.domain_keyword.push(value),
            1 => r.domain_regex.push(value),
            2 => r.domain_suffix.push(value),
            3 => r.domain.push(value),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut v: u64, out: &mut Vec<u8>) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return;
            }
            out.push(b | 0x80);
        }
    }
    fn bytes(n: u64, b: &[u8], out: &mut Vec<u8>) {
        varint(n << 3 | 2, out);
        varint(b.len() as u64, out);
        out.extend_from_slice(b);
    }
    fn num(n: u64, v: u64, out: &mut Vec<u8>) {
        varint(n << 3, out);
        varint(v, out);
    }

    fn geoip() -> Vec<u8> {
        let cidr = |ip: &[u8], p: u64| {
            let mut c = Vec::new();
            bytes(1, ip, &mut c);
            num(2, p, &mut c);
            c
        };
        let country = |code: &str, cidrs: &[Vec<u8>]| {
            let mut g = Vec::new();
            bytes(1, code.as_bytes(), &mut g);
            for c in cidrs {
                bytes(2, c, &mut g);
            }
            g
        };
        let mut file = Vec::new();
        bytes(1, &country("CN", &[cidr(&[1, 0, 1, 0], 24)]), &mut file);
        let v6 = [0x2a, 0x01, 0x05, 0xec, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        bytes(1, &country("IR", &[cidr(&[2, 144, 0, 0], 14), cidr(&v6, 32)]), &mut file);
        file
    }

    fn geosite() -> Vec<u8> {
        let domain = |kind: u64, v: &str| {
            let mut d = Vec::new();
            num(1, kind, &mut d);
            bytes(2, v.as_bytes(), &mut d);
            d
        };
        let mut site = Vec::new();
        bytes(1, b"CATEGORY-IR", &mut site);
        for d in [domain(2, "digikala.com"), domain(3, "www.shaparak.ir"), domain(0, "snapp"), domain(1, r"^bank\d+\.ir$")] {
            bytes(2, &d, &mut site);
        }
        let mut other = Vec::new();
        bytes(1, b"GOOGLE", &mut other);
        bytes(2, &domain(2, "google.com"), &mut other);
        let mut file = Vec::new();
        bytes(1, &other, &mut file);
        bytes(1, &site, &mut file);
        file
    }

    #[test]
    fn pulls_out_irans_lists_and_nothing_else() {
        let r = iran_rules(&geoip(), &geosite()).unwrap();
        assert_eq!(r.ip_cidr, ["2.144.0.0/14", "2a01:5ec::/32"]);
        assert_eq!(r.domain_suffix, ["ir", "digikala.com"]);
        assert_eq!(r.domain, ["www.shaparak.ir"]);
        assert_eq!(r.domain_keyword, ["snapp"]);
        assert_eq!(r.domain_regex, [r"^bank\d+\.ir$"]);
    }

    #[test]
    fn a_missing_country_or_a_broken_file_is_an_error() {
        assert_eq!(cidrs(&geoip(), "DE"), Err(GeoError::Missing("de".into())));
        let mut cut = geoip();
        cut.truncate(cut.len() - 3);
        assert_eq!(cidrs(&cut, "IR"), Err(GeoError::Malformed));
    }

    /// The real files, when `scripts/fetch-geo.sh` has put them in place.
    #[test]
    fn the_shipped_files_have_iran() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/resources/geo");
        let (Ok(ip), Ok(site)) = (std::fs::read(dir.join("geoip.dat")), std::fs::read(dir.join("geosite.dat"))) else {
            eprintln!("geo files not fetched; run scripts/fetch-geo.sh");
            return;
        };
        let r = iran_rules(&ip, &site).unwrap();
        assert!(r.ip_cidr.len() > 100, "{} Iranian ranges", r.ip_cidr.len());
        assert!(r.domain_suffix.len() > 100, "{} Iranian domains", r.domain_suffix.len());
    }
}
