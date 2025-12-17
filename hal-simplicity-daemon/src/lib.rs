pub extern crate simplicity;

pub mod jsonrpc;
pub mod types;

#[cfg(feature = "daemon")]
pub mod daemon;
#[cfg(feature = "daemon")]
pub mod utils;

#[cfg(feature = "daemon")]
pub use daemon::HalSimplicityDaemon;
