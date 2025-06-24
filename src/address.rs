use elements::bitcoin::{secp256k1, PublicKey};
use elements::{Address, PubkeyHash, Script, ScriptHash, WPubkeyHash, WScriptHash};
use serde::{Deserialize, Serialize};

use crate::Network;

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct AddressInfo {
	pub network: Network,
	#[serde(rename = "type")]
	pub type_: Option<String>,
	pub script_pub_key: ::hal::tx::OutputScriptInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub witness_program_version: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pubkey_hash: Option<PubkeyHash>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub script_hash: Option<ScriptHash>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub witness_pubkey_hash: Option<WPubkeyHash>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub witness_script_hash: Option<WScriptHash>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub blinding_pubkey: Option<secp256k1::PublicKey>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unconfidential: Option<Address>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct Addresses {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub p2pkh: Option<Address>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub p2wpkh: Option<Address>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub p2shwpkh: Option<Address>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub p2sh: Option<Address>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub p2wsh: Option<Address>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub p2shwsh: Option<Address>,
}

impl Addresses {
	pub fn from_pubkey(
		pubkey: &PublicKey,
		blinder: Option<secp256k1::PublicKey>,
		network: Network,
	) -> Addresses {
		let params = network.address_params();
		Addresses {
			p2pkh: Some(Address::p2pkh(pubkey, blinder, params)),
			p2wpkh: Some(Address::p2wpkh(pubkey, blinder, params)),
			p2shwpkh: Some(Address::p2shwpkh(pubkey, blinder, params)),
			..Default::default()
		}
	}

	pub fn from_script(
		script: &Script,
		blinder: Option<secp256k1::PublicKey>,
		network: Network,
	) -> Addresses {
		let params = network.address_params();
		Addresses {
			p2sh: Some(Address::p2sh(script, blinder, params)),
			p2wsh: Some(Address::p2wsh(script, blinder, params)),
			p2shwsh: Some(Address::p2shwsh(script, blinder, params)),
			..Default::default()
		}
	}
}
