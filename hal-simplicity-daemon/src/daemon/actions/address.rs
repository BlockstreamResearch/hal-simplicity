use elements::bitcoin::{secp256k1, PublicKey};
use elements::hashes::Hash;
use elements::{Address, WPubkeyHash, WScriptHash};
use thiserror::Error;

use crate::utils::{
	address::{AddressInfo, Addresses},
	Network,
};

use super::types::AddressCreateRequest;

#[derive(Debug, Error)]
pub enum AddressError {
	#[error("Failed to parse network: {0}")]
	NetworkParse(String),

	#[error("Failed to parse blinder hex: {0}")]
	BlinderHex(hex::FromHexError),

	#[error("Invalid blinder: {0}")]
	BlinderInvalid(secp256k1::Error),

	#[error("Invalid pubkey: {0}")]
	PubkeyInvalid(elements::bitcoin::key::ParsePublicKeyError),

	#[error("Invalid script hex: {0}")]
	ScriptHex(hex::FromHexError),

	#[error("Either pubkey or script must be provided")]
	MissingInput,

	#[error("Invalid address format: {0}")]
	AddressParse(elements::address::AddressError),
}

pub fn create(req: AddressCreateRequest) -> Result<Addresses, AddressError> {
	let network =
		req.network.as_deref().map(parse_network).transpose()?.unwrap_or(Network::ElementsRegtest);

	let blinder = req
		.blinder
		.map(|b| {
			let bytes = hex::decode(&b).map_err(AddressError::BlinderHex)?;
			secp256k1::PublicKey::from_slice(&bytes).map_err(AddressError::BlinderInvalid)
		})
		.transpose()?;

	if let Some(pubkey_hex) = req.pubkey {
		let pubkey: PublicKey = pubkey_hex.parse().map_err(AddressError::PubkeyInvalid)?;
		Ok(Addresses::from_pubkey(&pubkey, blinder, network))
	} else if let Some(script_hex) = req.script {
		let script_bytes = hex::decode(&script_hex).map_err(AddressError::ScriptHex)?;
		let script = script_bytes.into();
		Ok(Addresses::from_script(&script, blinder, network))
	} else {
		Err(AddressError::MissingInput)
	}
}

pub fn inspect(address_str: &str) -> Result<AddressInfo, AddressError> {
	let address: Address = address_str.parse().map_err(AddressError::AddressParse)?;

	let script_pk = address.script_pubkey();

	let mut info = AddressInfo {
		network: Network::from_params(address.params).expect("addresses always have params"),
		script_pub_key: hal::tx::OutputScriptInfo {
			hex: Some(script_pk.to_bytes().into()),
			asm: Some(script_pk.asm()),
			address: None,
			type_: None,
		},
		type_: None,
		pubkey_hash: None,
		script_hash: None,
		witness_pubkey_hash: None,
		witness_script_hash: None,
		witness_program_version: None,
		blinding_pubkey: address.blinding_pubkey,
		unconfidential: if address.blinding_pubkey.is_some() {
			Some(Address {
				params: address.params,
				payload: address.payload.clone(),
				blinding_pubkey: None,
			})
		} else {
			None
		},
	};

	use elements::address::Payload;
	match address.payload {
		Payload::PubkeyHash(pkh) => {
			info.type_ = Some("p2pkh".to_owned());
			info.pubkey_hash = Some(pkh);
		}
		Payload::ScriptHash(sh) => {
			info.type_ = Some("p2sh".to_owned());
			info.script_hash = Some(sh);
		}
		Payload::WitnessProgram {
			version,
			program,
		} => {
			let version = version.to_u8() as usize;
			info.witness_program_version = Some(version);

			if version == 0 {
				if program.len() == 20 {
					info.type_ = Some("p2wpkh".to_owned());
					info.witness_pubkey_hash =
						Some(WPubkeyHash::from_slice(&program).expect("size 20"));
				} else if program.len() == 32 {
					info.type_ = Some("p2wsh".to_owned());
					info.witness_script_hash =
						Some(WScriptHash::from_slice(&program).expect("size 32"));
				} else {
					info.type_ = Some("invalid-witness-program".to_owned());
				}
			} else {
				info.type_ = Some("unknown-witness-program-version".to_owned());
			}
		}
	}

	Ok(info)
}

fn parse_network(s: &str) -> Result<Network, AddressError> {
	match s.to_lowercase().as_str() {
		"liquid" => Ok(Network::Liquid),
		"liquid-testnet" | "liquidtestnet" => Ok(Network::LiquidTestnet),
		"elementsregtest" | "elements-regtest" | "regtest" => Ok(Network::ElementsRegtest),
		_ => Err(AddressError::NetworkParse(format!("unknown network: {}", s))),
	}
}
