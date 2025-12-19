pub extern crate elements;
pub extern crate simplicity;

pub mod jsonrpc;
pub mod types;
pub mod utils;

pub use types::Network;

#[cfg(feature = "daemon")]
pub mod daemon;

#[cfg(feature = "daemon")]
pub use daemon::HalSimplicityDaemon;
