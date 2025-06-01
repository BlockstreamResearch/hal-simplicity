extern crate elements;
extern crate hal;
extern crate hex;
extern crate serde;

pub mod address;
pub mod block;
pub mod tx;

pub mod confidential;

pub use hal::HexBytes;
pub use elements::bitcoin;

use elements::AddressParams;
use serde::{Deserialize, Serialize};

/// Known Elements networks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
	ElementsRegtest,
	Liquid,
}

impl Network {
	pub fn from_params(params: &'static AddressParams) -> Option<Network> {
		if *params == AddressParams::ELEMENTS {
			Some(Network::ElementsRegtest)
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
		}
	}
}

/// Get JSON-able objects that describe the type.
pub trait GetInfo<T: ::serde::Serialize> {
	/// Get a description of this object given the network of interest.
	fn get_info(&self, network: Network) -> T;
}
