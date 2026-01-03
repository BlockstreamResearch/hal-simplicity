pub mod info;
pub mod pset;
pub mod sighash;

pub use info::*;
pub use sighash::*;

use crate::simplicity::bitcoin::{Amount, Denomination};
use crate::simplicity::elements::confidential;
use crate::simplicity::elements::hex::FromHex as _;
use crate::simplicity::jet::elements::ElementsUtxo;

#[derive(Debug, thiserror::Error)]
pub enum ParseElementsUtxoError {
	#[error("invalid format: expected <scriptPubKey>:<asset>:<value>")]
	InvalidFormat,

	#[error("invalid scriptPubKey hex: {0}")]
	ScriptPubKeyParsing(elements::hex::Error),

	#[error("invalid asset hex: {0}")]
	AssetHexParsing(elements::hashes::hex::HexToArrayError),

	#[error("invalid asset commitment hex: {0}")]
	AssetCommitmentHexParsing(elements::hex::Error),

	#[error("invalid asset commitment: {0}")]
	AssetCommitmentDecoding(elements::encode::Error),

	#[error("invalid value commitment hex: {0}")]
	ValueCommitmentHexParsing(elements::hex::Error),

	#[error("invalid value commitment: {0}")]
	ValueCommitmentDecoding(elements::encode::Error),
}

pub fn parse_elements_utxo(s: &str) -> Result<ElementsUtxo, ParseElementsUtxoError> {
	let parts: Vec<&str> = s.split(':').collect();
	if parts.len() != 3 {
		return Err(ParseElementsUtxoError::InvalidFormat);
	}
	// Parse scriptPubKey
	let script_pubkey: elements::Script =
		parts[0].parse().map_err(ParseElementsUtxoError::ScriptPubKeyParsing)?;

	// Parse asset - try as explicit AssetId first, then as confidential commitment
	let asset = if parts[1].len() == 64 {
		// 32 bytes = explicit AssetId
		let asset_id: elements::AssetId =
			parts[1].parse().map_err(ParseElementsUtxoError::AssetHexParsing)?;
		confidential::Asset::Explicit(asset_id)
	} else {
		// Parse anything except 32 bytes as a confidential commitment (which must be 33 bytes)
		let commitment_bytes =
			Vec::from_hex(parts[1]).map_err(ParseElementsUtxoError::AssetCommitmentHexParsing)?;
		elements::confidential::Asset::from_commitment(&commitment_bytes)
			.map_err(ParseElementsUtxoError::AssetCommitmentDecoding)?
	};

	// Parse value - try as BTC decimal first, then as confidential commitment
	let value = if let Ok(btc_amount) = Amount::from_str_in(parts[2], Denomination::Bitcoin) {
		// Explicit value in BTC
		elements::confidential::Value::Explicit(btc_amount.to_sat())
	} else {
		// 33 bytes = confidential commitment
		let commitment_bytes =
			Vec::from_hex(parts[2]).map_err(ParseElementsUtxoError::ValueCommitmentHexParsing)?;
		elements::confidential::Value::from_commitment(&commitment_bytes)
			.map_err(ParseElementsUtxoError::ValueCommitmentDecoding)?
	};

	Ok(ElementsUtxo {
		script_pubkey,
		asset,
		value,
	})
}
