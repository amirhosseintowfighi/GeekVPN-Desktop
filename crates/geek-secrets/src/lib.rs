//! What identifies this computer to GeekVPN, and where its session is kept.

mod device;
mod vault;

pub use device::{device_id_from, device_info, platform};
pub use vault::KeychainStore;
