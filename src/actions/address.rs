use elements::bitcoin::{secp256k1, PublicKey};
use elements::{Address, Script};

use crate::address::{AddressInfo, Addresses};
use crate::Network;

#[derive(Debug, thiserror::Error)]
pub enum AddressError {
	#[error("invalid blinder hex: {0}")]
	BlinderHex(hex::FromHexError),

	#[error("invalid blinder: {0}")]
	BlinderInvalid(secp256k1::Error),

	#[error("invalid pubkey: {0}")]
	PubkeyInvalid(elements::bitcoin::key::ParsePublicKeyError),

	#[error("invalid script hex: {0}")]
	ScriptHex(hex::FromHexError),

	#[error("can't create addresses without a pubkey")]
	MissingInput,

	#[error("invalid address format: {0}")]
	AddressParse(elements::address::AddressError),

	#[error("no address provided")]
	NoAddressProvided,

	#[error("addresses always have params")]
	AddressesAlwaysHaveParams,
}

/// Create addresses from a public key or script.
pub fn address_create(
	pubkey_hex: Option<&str>,
	script_hex: Option<&str>,
	blinder_hex: Option<&str>,
	network: Network,
) -> Result<Addresses, AddressError> {
	let blinder = blinder_hex
		.map(|b| {
			let bytes = hex::decode(b).map_err(AddressError::BlinderHex)?;
			secp256k1::PublicKey::from_slice(&bytes).map_err(AddressError::BlinderInvalid)
		})
		.transpose()?;

	let created = if let Some(pubkey_hex) = pubkey_hex {
		let pubkey: PublicKey = pubkey_hex.parse().map_err(AddressError::PubkeyInvalid)?;
		Addresses::from_pubkey(&pubkey, blinder, network)
	} else if let Some(script_hex) = script_hex {
		let script_bytes = hex::decode(script_hex).map_err(AddressError::ScriptHex)?;
		let script: Script = script_bytes.into();
		Addresses::from_script(&script, blinder, network)
	} else {
		return Err(AddressError::MissingInput);
	};

	Ok(created)
}

/// Inspect an address and return detailed information.
pub fn address_inspect(address_str: &str) -> Result<AddressInfo, AddressError> {
	let address: Address = address_str.parse().map_err(AddressError::AddressParse)?;
	let script_pk = address.script_pubkey();

	let mut info = AddressInfo {
		network: Network::from_params(address.params)
			.ok_or(AddressError::AddressesAlwaysHaveParams)?,
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
	use elements::hashes::Hash;
	use elements::{WPubkeyHash, WScriptHash};

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
