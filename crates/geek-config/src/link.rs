//! Share links (`vless://`, `vmess://`, `trojan://`, `ss://`, `wireguard://`)
//! to a `Server`, and WireGuard `.conf` files to a `wireguard://` link.
//!
//! The field names follow the de-facto standard v2rayN and v2rayNG write and
//! read, so a link copied between any of them means the same server.

use std::collections::HashMap;

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Vless,
    Vmess,
    Trojan,
    Shadowsocks,
    WireGuard,
}

/// One server, parsed from a share link. The link itself is kept: it is what
/// the user sees in «لینک و QR» and what survives a format change here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    /// Stable across subscription refreshes as long as the link is unchanged.
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub address: String,
    pub port: u16,
    /// tcp | ws | grpc | xhttp | httpupgrade | kcp | quic
    pub network: String,
    /// none | tls | reality
    pub security: String,
    pub sni: String,
    /// The HTTP Host header for ws / xhttp / httpupgrade.
    pub host: String,
    pub path: String,
    pub link: String,
    #[serde(skip)]
    params: Params,
}

/// Everything else a link carried, for building the outbound.
#[derive(Debug, Clone, PartialEq, Default)]
struct Params {
    id: String,
    alter_id: u32,
    cipher: String,
    flow: String,
    encryption: String,
    fingerprint: String,
    alpn: Vec<String>,
    public_key: String,
    short_id: String,
    spider_x: String,
    service_name: String,
    header_type: String,
    mode: String,
    allow_insecure: bool,
    /// WireGuard: the interface's addresses, MTU, keepalive, pre-shared key
    /// and reserved bytes (the private key is `id`, the peer's key `public_key`).
    wg_address: Vec<String>,
    mtu: u32,
    keepalive: u32,
    preshared: String,
    reserved: Vec<u8>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LinkError {
    #[error("unsupported link type")]
    Unsupported,
    #[error("malformed link: {0}")]
    Malformed(&'static str),
}

impl Server {
    /// Parses one share link. Surrounding whitespace is ignored.
    pub fn parse(link: &str) -> Result<Self, LinkError> {
        let link = link.trim();
        let scheme = link.split("://").next().unwrap_or_default().to_ascii_lowercase();
        let mut server = match scheme.as_str() {
            "vless" => parse_url(link, Protocol::Vless)?,
            "trojan" => parse_url(link, Protocol::Trojan)?,
            "vmess" => parse_vmess(link)?,
            "ss" => parse_ss(link)?,
            "wireguard" | "wg" => parse_wireguard(link)?,
            _ => return Err(LinkError::Unsupported),
        };
        if server.address.is_empty() || server.port == 0 {
            return Err(LinkError::Malformed("no address or port"));
        }
        server.id = stable_id(link);
        if server.name.trim().is_empty() {
            server.name = format!("{}:{}", server.address, server.port);
        }
        Ok(server)
    }

    /// Whether the scanner can replace this server's address with a clean
    /// Cloudflare IP: a CDN transport over TLS, fronted by a domain in both
    /// SNI and Host (the Android app's `CdnTarget`).
    pub fn cdn_fronted(&self) -> bool {
        matches!(self.network.as_str(), "ws" | "grpc" | "xhttp" | "httpupgrade")
            && self.security == "tls"
            && !self.sni.is_empty()
            && self.sni.parse::<std::net::IpAddr>().is_err()
    }

    /// The Xray outbound, tagged `tag`. `address_override` swaps the address
    /// (a clean IP from the scanner) while SNI and Host keep the domain.
    pub fn outbound(&self, tag: &str, address_override: Option<&str>) -> Value {
        let address = address_override.unwrap_or(&self.address);
        let p = &self.params;
        if self.protocol == Protocol::WireGuard {
            // Xray's own userspace WireGuard: no kernel device, so no root.
            let endpoint = if address.contains(':') { format!("[{address}]:{}", self.port) } else { format!("{address}:{}", self.port) };
            let mut peer = serde_json::json!({ "publicKey": p.public_key, "endpoint": endpoint });
            if !p.preshared.is_empty() {
                peer["preSharedKey"] = Value::from(p.preshared.as_str());
            }
            if p.keepalive > 0 {
                peer["keepAlive"] = Value::from(p.keepalive);
            }
            let mut settings = serde_json::json!({
                "secretKey": p.id,
                "address": p.wg_address,
                "peers": [peer],
                "mtu": if p.mtu > 0 { p.mtu } else { 1420 },
                "noKernelTun": true,
            });
            if !p.reserved.is_empty() {
                settings["reserved"] = Value::from(p.reserved.clone());
            }
            return serde_json::json!({ "tag": tag, "protocol": "wireguard", "settings": settings });
        }
        let settings = match self.protocol {
            Protocol::Vless => serde_json::json!({ "vnext": [{
                "address": address, "port": self.port,
                "users": [{ "id": p.id, "encryption": if p.encryption.is_empty() { "none" } else { &p.encryption }, "flow": p.flow }]
            }]}),
            Protocol::Vmess => serde_json::json!({ "vnext": [{
                "address": address, "port": self.port,
                "users": [{ "id": p.id, "alterId": p.alter_id, "security": if p.cipher.is_empty() { "auto" } else { &p.cipher } }]
            }]}),
            Protocol::Trojan => serde_json::json!({ "servers": [{
                "address": address, "port": self.port, "password": p.id
            }]}),
            Protocol::Shadowsocks => serde_json::json!({ "servers": [{
                "address": address, "port": self.port, "method": p.cipher, "password": p.id
            }]}),
            Protocol::WireGuard => unreachable!("handled above"),
        };
        let protocol = match self.protocol {
            Protocol::Shadowsocks => "shadowsocks",
            Protocol::Vless => "vless",
            Protocol::Vmess => "vmess",
            Protocol::Trojan => "trojan",
            Protocol::WireGuard => "wireguard",
        };
        serde_json::json!({
            "tag": tag,
            "protocol": protocol,
            "settings": settings,
            "streamSettings": self.stream_settings(),
        })
    }

    fn stream_settings(&self) -> Value {
        let p = &self.params;
        let mut s = serde_json::Map::new();
        s.insert("network".into(), Value::from(self.network.as_str()));
        s.insert("security".into(), Value::from(self.security.as_str()));
        let host = if self.host.is_empty() { self.sni.clone() } else { self.host.clone() };
        match self.network.as_str() {
            "ws" => {
                s.insert("wsSettings".into(), serde_json::json!({ "path": or_slash(&self.path), "host": host }));
            }
            "httpupgrade" => {
                s.insert("httpupgradeSettings".into(), serde_json::json!({ "path": or_slash(&self.path), "host": host }));
            }
            "xhttp" => {
                let mut x = serde_json::json!({ "path": or_slash(&self.path), "host": host });
                if !p.mode.is_empty() {
                    x["mode"] = Value::from(p.mode.as_str());
                }
                s.insert("xhttpSettings".into(), x);
            }
            "grpc" => {
                s.insert(
                    "grpcSettings".into(),
                    serde_json::json!({ "serviceName": p.service_name, "multiMode": p.mode == "multi" }),
                );
            }
            "tcp" if p.header_type == "http" => {
                s.insert(
                    "tcpSettings".into(),
                    serde_json::json!({ "header": { "type": "http", "request": {
                        "path": [or_slash(&self.path)],
                        "headers": { "Host": host.split(',').map(str::trim).collect::<Vec<_>>() }
                    }}}),
                );
            }
            _ => {}
        }
        match self.security.as_str() {
            "tls" => {
                let mut t = serde_json::json!({ "serverName": self.sni, "allowInsecure": p.allow_insecure });
                if !p.fingerprint.is_empty() {
                    t["fingerprint"] = Value::from(p.fingerprint.as_str());
                }
                if !p.alpn.is_empty() {
                    t["alpn"] = Value::from(p.alpn.clone());
                }
                s.insert("tlsSettings".into(), t);
            }
            "reality" => {
                s.insert(
                    "realitySettings".into(),
                    serde_json::json!({
                        "serverName": self.sni,
                        "fingerprint": if p.fingerprint.is_empty() { "chrome" } else { &p.fingerprint },
                        "publicKey": p.public_key,
                        "shortId": p.short_id,
                        "spiderX": p.spider_x,
                    }),
                );
            }
            _ => {}
        }
        Value::Object(s)
    }
}

fn or_slash(path: &str) -> &str {
    if path.is_empty() {
        "/"
    } else {
        path
    }
}

fn stable_id(link: &str) -> String {
    hex::encode(&Sha256::digest(link.as_bytes())[..8])
}

fn decode_name(fragment: Option<&str>) -> String {
    fragment.map(|f| percent_decode_str(f).decode_utf8_lossy().trim().to_string()).unwrap_or_default()
}

fn b64(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD]
        .iter()
        .find_map(|e| e.decode(s).ok())
}

/// vless:// and trojan:// share the same URL shape.
fn parse_url(link: &str, protocol: Protocol) -> Result<Server, LinkError> {
    let url = Url::parse(link).map_err(|_| LinkError::Malformed("not a URL"))?;
    let q: HashMap<String, String> = url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let get = |k: &str| q.get(k).cloned().unwrap_or_default();
    let id = percent_decode_str(url.username()).decode_utf8_lossy().to_string();
    if id.is_empty() {
        return Err(LinkError::Malformed("no id"));
    }
    let address = url.host_str().unwrap_or_default().trim_matches(['[', ']']).to_string();
    let mut network = get("type");
    if network.is_empty() {
        network = "tcp".into();
    }
    let mut security = get("security");
    if security.is_empty() {
        // Trojan is TLS unless the link says otherwise.
        security = if protocol == Protocol::Trojan { "tls".into() } else { "none".into() };
    }
    let host = get("host");
    let sni = match get("sni") {
        s if !s.is_empty() => s,
        _ if security != "none" && !host.is_empty() => host.clone(),
        _ if security != "none" => address.clone(),
        _ => String::new(),
    };
    Ok(Server {
        id: String::new(),
        name: decode_name(url.fragment()),
        protocol,
        address,
        port: url.port().unwrap_or(443),
        network,
        security,
        sni,
        host,
        path: get("path"),
        link: link.to_string(),
        params: Params {
            id,
            flow: get("flow"),
            encryption: get("encryption"),
            fingerprint: get("fp"),
            alpn: split_list(&get("alpn")),
            public_key: get("pbk"),
            short_id: get("sid"),
            spider_x: get("spx"),
            service_name: get("serviceName"),
            header_type: get("headerType"),
            mode: get("mode"),
            allow_insecure: matches!(get("allowInsecure").as_str(), "1" | "true"),
            ..Params::default()
        },
    })
}

fn split_list(s: &str) -> Vec<String> {
    s.split(',').map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect()
}

/// vmess://base64(JSON), the v2rayN "v2" format.
fn parse_vmess(link: &str) -> Result<Server, LinkError> {
    let body = link.split_once("://").map(|(_, b)| b).unwrap_or_default();
    let raw = b64(body).ok_or(LinkError::Malformed("vmess is not base64"))?;
    let v: Value = serde_json::from_slice(&raw).map_err(|_| LinkError::Malformed("vmess is not JSON"))?;
    // Numbers arrive as numbers or as strings, depending on the writer.
    let s = |k: &str| match &v[k] {
        Value::String(x) => x.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    };
    let network = match s("net") {
        n if n.is_empty() => "tcp".to_string(),
        n => n,
    };
    let security = if s("tls") == "tls" { "tls".to_string() } else { "none".to_string() };
    let host = s("host");
    let sni = match s("sni") {
        x if !x.is_empty() => x,
        _ if security == "tls" && !host.is_empty() => host.clone(),
        _ if security == "tls" => s("add"),
        _ => String::new(),
    };
    let (path, service_name) = if network == "grpc" { (String::new(), s("path")) } else { (s("path"), String::new()) };
    Ok(Server {
        id: String::new(),
        name: s("ps"),
        protocol: Protocol::Vmess,
        address: s("add"),
        port: s("port").parse().map_err(|_| LinkError::Malformed("vmess port"))?,
        network,
        security,
        sni,
        host,
        path,
        link: link.to_string(),
        params: Params {
            id: s("id"),
            alter_id: s("aid").parse().unwrap_or(0),
            cipher: s("scy"),
            fingerprint: s("fp"),
            alpn: split_list(&s("alpn")),
            service_name,
            header_type: s("type"),
            ..Params::default()
        },
    })
}

/// ss://, both SIP002 (`ss://base64(method:pass)@host:port#name`) and the
/// legacy whole-body base64 (`ss://base64(method:pass@host:port)#name`).
fn parse_ss(link: &str) -> Result<Server, LinkError> {
    let body = link.split_once("://").map(|(_, b)| b).unwrap_or_default();
    let (body, name) = match body.split_once('#') {
        Some((b, n)) => (b, decode_name(Some(n))),
        None => (body, String::new()),
    };
    let body = body.split(['?', '/']).next().unwrap_or_default();
    let (userinfo, hostport) = match body.rsplit_once('@') {
        Some((u, h)) => {
            let u = percent_decode_str(u).decode_utf8_lossy().to_string();
            let decoded = b64(&u).and_then(|b| String::from_utf8(b).ok()).filter(|d| d.contains(':'));
            (decoded.unwrap_or(u), h.to_string())
        }
        None => {
            let all = b64(body).and_then(|b| String::from_utf8(b).ok()).ok_or(LinkError::Malformed("ss body"))?;
            let (u, h) = all.rsplit_once('@').ok_or(LinkError::Malformed("ss body"))?;
            (u.to_string(), h.to_string())
        }
    };
    let (cipher, password) = userinfo.split_once(':').ok_or(LinkError::Malformed("ss method"))?;
    let (host, port) = hostport.rsplit_once(':').ok_or(LinkError::Malformed("ss port"))?;
    Ok(Server {
        id: String::new(),
        name,
        protocol: Protocol::Shadowsocks,
        address: host.trim_matches(['[', ']']).to_string(),
        port: port.parse().map_err(|_| LinkError::Malformed("ss port"))?,
        network: "tcp".into(),
        security: "none".into(),
        sni: String::new(),
        host: String::new(),
        path: String::new(),
        link: link.to_string(),
        params: Params { id: password.to_string(), cipher: cipher.to_string(), ..Params::default() },
    })
}

/// `wireguard://<private key>@host:port?publickey=…&address=…&mtu=…#name`,
/// the form v2rayN and NekoBox share.
fn parse_wireguard(link: &str) -> Result<Server, LinkError> {
    let url = Url::parse(link).map_err(|_| LinkError::Malformed("not a URL"))?;
    let q: HashMap<String, String> = url.query_pairs().map(|(k, v)| (k.to_ascii_lowercase(), v.into_owned())).collect();
    let get = |keys: &[&str]| keys.iter().find_map(|k| q.get(*k).cloned()).unwrap_or_default();
    let private = percent_decode_str(url.username()).decode_utf8_lossy().to_string();
    let public = get(&["publickey", "public_key", "peer_public_key", "pbk"]);
    if private.is_empty() || public.is_empty() {
        return Err(LinkError::Malformed("wireguard keys"));
    }
    let wg_address = split_list(&get(&["address", "ip", "local_address"]));
    if wg_address.is_empty() {
        return Err(LinkError::Malformed("wireguard address"));
    }
    let reserved = split_list(&get(&["reserved"])).iter().filter_map(|b| b.parse::<u8>().ok()).collect();
    Ok(Server {
        id: String::new(),
        name: decode_name(url.fragment()),
        protocol: Protocol::WireGuard,
        address: url.host_str().unwrap_or_default().trim_matches(['[', ']']).to_string(),
        port: url.port().unwrap_or(51820),
        network: "udp".into(),
        security: "none".into(),
        sni: String::new(),
        host: String::new(),
        path: String::new(),
        link: link.to_string(),
        params: Params {
            id: private,
            public_key: public,
            preshared: get(&["presharedkey", "preshared_key", "psk"]),
            wg_address,
            mtu: get(&["mtu"]).parse().unwrap_or(0),
            keepalive: get(&["keepalive", "persistentkeepalive"]).parse().unwrap_or(0),
            reserved,
            ..Params::default()
        },
    })
}

/// A WireGuard `.conf` (what WGDashboard's share page and most panels hand
/// out) as one `wireguard://` link, so the store keeps links only.
/// AmneziaWG files (Jc, S1, H1…) need a different engine and are refused.
pub fn wireguard_conf_to_link(conf: &str, name: &str) -> Result<String, LinkError> {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    let mut section = String::new();
    let mut iface: HashMap<String, String> = HashMap::new();
    let mut peer: HashMap<String, String> = HashMap::new();
    for line in conf.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_string());
        match section.as_str() {
            "interface" => iface.insert(k, v),
            // Only the first peer: a client config has one.
            "peer" if !peer.contains_key(&k) => peer.insert(k, v),
            _ => None,
        };
    }
    if ["jc", "jmin", "jmax", "s1", "s2", "h1", "h2", "h3", "h4"].iter().any(|k| iface.contains_key(*k)) {
        return Err(LinkError::Unsupported);
    }
    let private = iface.get("privatekey").ok_or(LinkError::Malformed("no PrivateKey"))?;
    let public = peer.get("publickey").ok_or(LinkError::Malformed("no PublicKey"))?;
    let address = iface.get("address").ok_or(LinkError::Malformed("no Address"))?;
    let endpoint = peer.get("endpoint").ok_or(LinkError::Malformed("no Endpoint"))?;
    let (host, port) = endpoint.rsplit_once(':').ok_or(LinkError::Malformed("Endpoint port"))?;
    let port: u16 = port.trim().parse().map_err(|_| LinkError::Malformed("Endpoint port"))?;
    let host = host.trim().trim_matches(['[', ']']);
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    let enc = |s: &str| utf8_percent_encode(s, NON_ALPHANUMERIC).to_string();
    let address: Vec<String> = split_list(address);
    let mut link = format!("wireguard://{}@{host}:{port}?publickey={}&address={}", enc(private), enc(public), enc(&address.join(",")));
    if let Some(psk) = peer.get("presharedkey") {
        link += &format!("&presharedkey={}", enc(psk));
    }
    if let Some(mtu) = iface.get("mtu").and_then(|m| m.parse::<u32>().ok()) {
        link += &format!("&mtu={mtu}");
    }
    if let Some(ka) = peer.get("persistentkeepalive").and_then(|m| m.parse::<u32>().ok()) {
        link += &format!("&keepalive={ka}");
    }
    let name = if name.trim().is_empty() { "WireGuard" } else { name.trim() };
    link += &format!("#{}", enc(name));
    Server::parse(&link)?;
    Ok(link)
}

/// WGDashboard's share page (`https://panel/#/share?ShareID=…`) has no
/// config in it; its public API (`/api/sharePeer/get`) does. The API URL for
/// a share link, or `None` for any other URL.
pub fn wgdashboard_share_api(url: &str) -> Option<String> {
    let u = Url::parse(url.trim()).ok()?;
    let frag = u.fragment()?;
    let (_, query) = frag.split_once("share?")?;
    let id = url::form_urlencoded::parse(query.as_bytes()).find(|(k, _)| k == "ShareID").map(|(_, v)| v.into_owned())?;
    if id.is_empty() {
        return None;
    }
    let mut base = u.clone();
    base.set_fragment(None);
    base.set_query(None);
    let mut path = base.path().to_string();
    if !path.ends_with('/') {
        path.push('/');
    }
    base.set_path(&format!("{path}api/sharePeer/get"));
    base.query_pairs_mut().append_pair("ShareID", &id);
    Some(base.to_string())
}

/// The `.conf` text from WGDashboard's `sharePeer/get` answer.
pub fn wgdashboard_conf(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    if v["status"] != Value::Bool(true) {
        return None;
    }
    v["data"]["file"].as_str().filter(|f| f.contains("[Interface]")).map(String::from)
}

/// A subscription body: share links one per line, the whole thing usually
/// base64. Lines this client cannot read are skipped, not fatal: one exotic
/// server must not hide the other twenty. A WireGuard `.conf` is one server.
pub fn parse_subscription(body: &str) -> Vec<Server> {
    if body.contains("[Interface]") {
        return wireguard_conf_to_link(body, "WireGuard").ok().and_then(|l| Server::parse(&l).ok()).into_iter().collect();
    }
    let text = if body.contains("://") {
        body.to_string()
    } else {
        b64(&body.split_whitespace().collect::<String>())
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default()
    };
    text.lines().filter_map(|l| Server::parse(l).ok()).collect()
}

/// Whether a subscription entry is the panel's status line, not a server
/// the customer would pick: Marzban, 3x-ui, Hiddify and the like put the
/// account's traffic, days left and state in the names of extra configs
/// («📊 24.85GB / 200.0GB», «⏳ days left ∞», «amir ✅»). They connect, but
/// they belong above the list, as information.
pub fn is_info_entry(name: &str) -> bool {
    let n = name.trim().to_lowercase();
    if n.is_empty() {
        return false;
    }
    const MARKS: [char; 8] = ['📊', '⏳', '⌛', '📅', '🗓', '🔋', '♾', '∞'];
    const WORDS: [&str; 12] = [
        "days left", "day left", "remaining", "expire", "expiry", "traffic", "انقضا", "باقی‌مانده", "باقیمانده", "روز مانده", "حجم", "مصرف",
    ];
    if n.chars().any(|c| MARKS.contains(&c)) || WORDS.iter().any(|w| n.contains(w)) {
        return true;
    }
    // A used/total pair: "24.85GB / 200GB", "1.2 GB/50 GB".
    let has_size = ["gb", "mb", "tb", "gib", "mib", "گیگ"].iter().any(|u| n.contains(u)) && n.chars().any(|c| c.is_ascii_digit());
    if has_size && n.contains('/') {
        return true;
    }
    // The account's own name with a state mark: «amir ✅», «user ❌».
    let marked = n.ends_with('✅') || n.ends_with('❌') || n.ends_with('🟢') || n.ends_with('🔴');
    marked && n.split_whitespace().count() <= 2
}

#[cfg(test)]
mod info_entry_tests {
    use super::is_info_entry;

    #[test]
    fn status_lines_are_not_servers() {
        for info in ["📊 24.85GB / 200.0GB", "⏳ days left ∞", "amir ✅", "gv1405000079 ✅", "1.2 GB/50 GB", "انقضا: ۱۴۰۵/۰۲/۰۱", "Traffic: 10GB"] {
            assert!(is_info_entry(info), "{info}");
        }
        for server in ["🇩🇪 server1 | amir - ", "Germany 01", "🇹🇷 Turkey ✅ fast server", "DE-2 vless ws", "📶 Irancell"] {
            assert!(!is_info_entry(server), "{server}");
        }
    }
}

#[cfg(test)]
mod wireguard_tests {
    use super::*;

    const CONF: &str = "[Interface]\nPrivateKey = cOFA+x5UvHF+a3xJ6enLkxUyASaA4MbOoTQmB2gzb2s=\nAddress = 10.0.0.2/32, fd00::2/128\nMTU = 1380\nDNS = 1.1.1.1\n\n[Peer]\nPublicKey = 9G7IqmZkzNhbfjXWAqzd9B1AE6G1MoaMOP+x5ou6BTk=\nPresharedKey = FpCyhws9cxwWoV4xELtfJvjJN+zQVRPISllRWgeopVE=\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = wg.example.com:51820\nPersistentKeepalive = 25\n";

    #[test]
    fn a_conf_becomes_one_link_and_an_xray_outbound() {
        let link = wireguard_conf_to_link(CONF, "آلمان WG").unwrap();
        assert!(link.starts_with("wireguard://"));
        let s = Server::parse(&link).unwrap();
        assert_eq!((s.protocol, s.address.as_str(), s.port, s.name.as_str()), (Protocol::WireGuard, "wg.example.com", 51820, "آلمان WG"));
        let o = s.outbound("proxy", None);
        assert_eq!(o["protocol"], "wireguard");
        let st = &o["settings"];
        assert_eq!(st["secretKey"], "cOFA+x5UvHF+a3xJ6enLkxUyASaA4MbOoTQmB2gzb2s=");
        assert_eq!(st["address"], serde_json::json!(["10.0.0.2/32", "fd00::2/128"]));
        assert_eq!(st["mtu"], 1380);
        assert_eq!(st["noKernelTun"], true);
        assert_eq!(st["peers"][0]["publicKey"], "9G7IqmZkzNhbfjXWAqzd9B1AE6G1MoaMOP+x5ou6BTk=");
        assert_eq!(st["peers"][0]["preSharedKey"], "FpCyhws9cxwWoV4xELtfJvjJN+zQVRPISllRWgeopVE=");
        assert_eq!(st["peers"][0]["endpoint"], "wg.example.com:51820");
        assert_eq!(st["peers"][0]["keepAlive"], 25);
        // A subscription body that is the conf itself.
        assert_eq!(parse_subscription(CONF).len(), 1);
    }

    #[test]
    fn ipv6_endpoints_and_v2rayn_links() {
        let conf = CONF.replace("wg.example.com:51820", "[2001:db8::1]:443");
        let s = Server::parse(&wireguard_conf_to_link(&conf, "").unwrap()).unwrap();
        assert_eq!(s.address, "2001:db8::1");
        assert_eq!(s.outbound("p", None)["settings"]["peers"][0]["endpoint"], "[2001:db8::1]:443");
        let v = Server::parse("wireguard://cOFA%2Bx5UvHF%2Ba3xJ6enLkxUyASaA4MbOoTQmB2gzb2s%3D@1.2.3.4:2408?publickey=bmXOC%2BF1FxEMF9dyiK2H5%2F1SUtzH0JuVo51h2wPfgyo%3D&reserved=1,2,3&address=172.16.0.2%2F32&mtu=1280#warp").unwrap();
        assert_eq!(v.outbound("p", None)["settings"]["reserved"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn amneziawg_and_broken_confs_are_refused() {
        let awg = CONF.replace("MTU = 1380", "MTU = 1380\nJc = 4\nS1 = 20");
        assert_eq!(wireguard_conf_to_link(&awg, ""), Err(LinkError::Unsupported));
        assert!(wireguard_conf_to_link("[Interface]\nAddress = 10.0.0.2/32\n", "").is_err());
    }

    #[test]
    fn wgdashboard_share_pages() {
        assert_eq!(wgdashboard_share_api("https://wg.example.com/#/share?ShareID=ab-12").as_deref(), Some("https://wg.example.com/api/sharePeer/get?ShareID=ab-12"));
        assert_eq!(wgdashboard_share_api("https://p.example/dash/#/share?ShareID=x").as_deref(), Some("https://p.example/dash/api/sharePeer/get?ShareID=x"));
        assert!(wgdashboard_share_api("https://sub.example/sub/abc").is_none());
        let body = serde_json::json!({ "status": true, "message": null, "data": { "fileName": "amir", "file": CONF } }).to_string();
        assert_eq!(wgdashboard_conf(&body).as_deref(), Some(CONF));
        assert!(wgdashboard_conf(r#"{"status":false,"message":"This link is either expired to invalid","data":null}"#).is_none());
    }
}
