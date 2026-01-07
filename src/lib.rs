pub extern crate simplicity;

pub mod actions;

pub mod address;
pub mod block;
pub mod hal_simplicity;
pub mod tx;

pub mod confidential;

pub use elements::bitcoin;
pub use hal::HexBytes;

#[cfg(feature = "daemon")]
pub mod daemon;

use elements::AddressParams;
use serde::{Deserialize, Serialize};

/// Known Elements networks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
	ElementsRegtest,
	Liquid,
	LiquidTestnet,
}

impl Network {
	pub fn from_params(params: &'static AddressParams) -> Option<Network> {
		if *params == AddressParams::ELEMENTS {
			Some(Network::ElementsRegtest)
		} else if *params == AddressParams::LIQUID_TESTNET {
			Some(Network::LiquidTestnet)
		} else if *params == AddressParams::LIQUID {
			Some(Network::Liquid)
		} else {
			None
		}
	}

	pub fn address_params(self) -> &'static AddressParams {
		match self {
			Network::ElementsRegtest => &AddressParams::ELEMENTS,
			Network::Liquid => &AddressParams::LIQUID,
			Network::LiquidTestnet => &AddressParams::LIQUID_TESTNET,
		}
	}
}

/// Get JSON-able objects that describe the type.
pub trait GetInfo<T: ::serde::Serialize> {
	/// Get a description of this object given the network of interest.
	fn get_info(&self, network: Network) -> T;
}

/// Parse a string which may be base64 or hex-encoded.
///
/// An even-length string with exclusively lowercase hex characters will be parsed as hex;
/// failing that, it will be parsed as base64 and return an error accordingly.
pub fn hex_or_base64(s: &str) -> Result<Vec<u8>, simplicity::base64::DecodeError> {
	if s.len() % 2 == 0 && s.bytes().all(|b| b.is_ascii_hexdigit() && b.is_ascii_lowercase()) {
		use simplicity::hex::FromHex as _;
		Ok(Vec::from_hex(s).expect("charset checked above"))
	} else {
		use simplicity::base64::prelude::Engine as _;
		simplicity::base64::prelude::BASE64_STANDARD.decode(s)
	}
}
