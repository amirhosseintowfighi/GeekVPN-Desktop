//! Servers from share links and subscriptions, the Xray configs the desktop
//! core runs, and the sing-box config of TUN mode. Pure: no I/O, so every
//! rule here is unit-tested.

mod cdn;
mod geo;
mod link;
mod singbox;
mod xray;

pub use cdn::{
    is_domain, profile_key, rank, scan_config, verdict, CdnTarget, CfRanges, CleanIp, FailoverPolicy, FailoverThreshold, KEEP, STALE_AFTER_MS,
};
pub use geo::{iran_rules, GeoError, RuleLists};
pub use link::{parse_subscription, LinkError, Protocol, Server};
pub use singbox::{tun_config, AppMode, AppRouting, TunHost, TunSpec, Upstream, LINUX_MARK};
pub use xray::{client_config, delay_config, tun_upstream_config, LocalPorts, Route};

#[cfg(test)]
mod tests;
