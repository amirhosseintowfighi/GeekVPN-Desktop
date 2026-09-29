//! Share links (`vless://`, `vmess://`, `trojan://`, `ss://`) to a `Server`.
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
        };
        let protocol = match self.protocol {
            Protocol::Shadowsocks => "shadowsocks",
            Protocol::Vless => "vless",
            Protocol::Vmess => "vmess",
            Protocol::Trojan => "trojan",
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

/// A subscription body: share links one per line, the whole thing usually
/// base64. Lines this client cannot read are skipped, not fatal: one exotic
/// server must not hide the other twenty.
pub fn parse_subscription(body: &str) -> Vec<Server> {
    let text = if body.contains("://") {
        body.to_string()
    } else {
        b64(&body.split_whitespace().collect::<String>())
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default()
    };
    text.lines().filter_map(|l| Server::parse(l).ok()).collect()
}
