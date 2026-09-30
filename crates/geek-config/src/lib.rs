//! Servers from share links and subscriptions, and the Xray configs the
//! desktop core runs. Pure: no I/O, so every rule here is unit-tested.

mod link;
mod xray;

pub use link::{parse_subscription, LinkError, Protocol, Server};
pub use xray::{client_config, delay_config, LocalPorts, Route};

#[cfg(test)]
mod tests;
