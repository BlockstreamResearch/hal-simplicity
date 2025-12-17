use super::PsetError;
use crate::daemon::actions::types::{PsetUpdateInputRequest, PsetUpdateInputResponse};
use crate::utils::hal_simplicity::taproot_spend_info;
use core::str::FromStr;
use elements::bitcoin::secp256k1;
use elements::schnorr::XOnlyPublicKey;
use simplicity::hex::parse::FromHex as _;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PsetUpdateInputError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("Failed to decode PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("Input index {index} out-of-range for PSET with {total} inputs")]
	InputIndexOutOfRange {
		index: usize,
		total: usize,
	},

	#[error("Failed to parse CMR: {0}")]
	CmrParse(elements::hashes::hex::HexToArrayError),

	#[error("Failed to parse internal key: {0}")]
	InternalKeyParse(secp256k1::Error),

	#[error("Internal key must be present if CMR is; PSET requires a control block for each CMR, which in turn requires the internal key. If you don't know the internal key, good chance it is the BIP-0341 'unspendable key' 50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0 or the web IDE's 'unspendable key' (highly discouraged for use in production) of f5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2")]
	MissingInternalKey,

	#[error("Input UTXO does not appear to be a Taproot output")]
	NotTaprootOutput,

	#[error("Failed to parse 32-byte state commitment as hex: {0}")]
	StateParse(elements::hashes::hex::HexToArrayError),

	#[error("CMR and internal key imply output key {output_key}, which does not match input scriptPubKey {script_pubkey}")]
	OutputKeyMismatch {
		output_key: String,
		script_pubkey: String,
	},

	#[error("Failed to parse input UTXO: expected format <scriptPubKey>:<asset>:<value>")]
	UtxoFormatInvalid,

	#[error("Failed to parse scriptPubKey hex: {0}")]
	ScriptPubKeyParse(elements::hex::Error),

	#[error("Failed to parse asset hex: {0}")]
	AssetParse(elements::hashes::hex::HexToArrayError),

	#[error("Failed to parse asset commitment hex: {0}")]
	AssetCommitmentParse(elements::hashes::hex::HexToBytesError),

	#[error("Failed to decode asset commitment: {0}")]
	AssetCommitmentDecode(elements::encode::Error),

	#[error("Failed to parse value commitment hex: {0}")]
	ValueCommitmentParse(elements::hashes::hex::HexToBytesError),

	#[error("Failed to decode value commitment: {0}")]
	ValueCommitmentDecode(elements::encode::Error),
}

pub fn update_input(
	req: PsetUpdateInputRequest,
) -> Result<PsetUpdateInputResponse, PsetUpdateInputError> {
	let mut pset: elements::pset::PartiallySignedTransaction =
		req.pset.parse().map_err(PsetUpdateInputError::PsetDecode)?;

	let input_idx = req.input_index as usize;
	let input_utxo = parse_elements_utxo(&req.input_utxo)?;

	let n_inputs = pset.n_inputs();
	let input = pset.inputs_mut().get_mut(input_idx).ok_or_else(|| {
		PsetUpdateInputError::InputIndexOutOfRange {
			index: input_idx,
			total: n_inputs,
		}
	})?;

	let cmr = req
		.cmr
		.as_deref()
		.map(simplicity::Cmr::from_str)
		.transpose()
		.map_err(PsetUpdateInputError::CmrParse)?;

	let internal_key = req
		.internal_key
		.as_deref()
		.map(XOnlyPublicKey::from_str)
		.transpose()
		.map_err(PsetUpdateInputError::InternalKeyParse)?;

	if cmr.is_some() && internal_key.is_none() {
		return Err(PsetUpdateInputError::MissingInternalKey);
	}

	if !input_utxo.script_pubkey.is_v1_p2tr() {
		return Err(PsetUpdateInputError::NotTaprootOutput);
	}

	let state = req
		.state
		.as_deref()
		.map(<[u8; 32]>::from_hex)
		.transpose()
		.map_err(PsetUpdateInputError::StateParse)?;

	let mut updated_values = vec![];
	if let Some(internal_key) = internal_key {
		updated_values.push("tap_internal_key".to_string());
		input.tap_internal_key = Some(internal_key);

		if let Some(cmr) = cmr {
			let spend_info = taproot_spend_info(internal_key, state, cmr);
			if spend_info.output_key().as_inner().serialize() != input_utxo.script_pubkey[2..] {
				return Err(PsetUpdateInputError::OutputKeyMismatch {
					output_key: spend_info.output_key().as_inner().to_string(),
					script_pubkey: input_utxo.script_pubkey.to_string(),
				});
			}

			let script_ver = spend_info.as_script_map().keys().next().unwrap();
			let cb = spend_info.control_block(script_ver).unwrap();
			input.tap_merkle_root = spend_info.merkle_root();
			input.tap_scripts = BTreeMap::new();
			input.tap_scripts.insert(cb, script_ver.clone());
			updated_values.push("tap_merkle_root".to_string());
			updated_values.push("tap_scripts".to_string());
		}
	}

	input.witness_utxo = Some(elements::TxOut {
		asset: input_utxo.asset,
		value: input_utxo.value,
		nonce: elements::confidential::Nonce::Null,
		script_pubkey: input_utxo.script_pubkey,
		witness: elements::TxOutWitness::empty(),
	});
	updated_values.push("witness_utxo".to_string());

	Ok(PsetUpdateInputResponse {
		pset: pset.to_string(),
		updated_values,
	})
}

fn parse_elements_utxo(
	s: &str,
) -> Result<crate::simplicity::jet::elements::ElementsUtxo, PsetUpdateInputError> {
	use crate::simplicity::bitcoin::{Amount, Denomination};

	let parts: Vec<&str> = s.split(':').collect();
	if parts.len() != 3 {
		return Err(PsetUpdateInputError::UtxoFormatInvalid);
	}

	let script_pubkey: elements::Script =
		parts[0].parse().map_err(PsetUpdateInputError::ScriptPubKeyParse)?;

	let asset = if parts[1].len() == 64 {
		let asset_id: elements::AssetId =
			parts[1].parse().map_err(PsetUpdateInputError::AssetParse)?;
		elements::confidential::Asset::Explicit(asset_id)
	} else {
		let commitment_bytes =
			Vec::from_hex(parts[1]).map_err(PsetUpdateInputError::AssetCommitmentParse)?;
		elements::confidential::Asset::from_commitment(&commitment_bytes)
			.map_err(PsetUpdateInputError::AssetCommitmentDecode)?
	};

	let value = if let Ok(btc_amount) = Amount::from_str_in(parts[2], Denomination::Bitcoin) {
		elements::confidential::Value::Explicit(btc_amount.to_sat())
	} else {
		let commitment_bytes =
			Vec::from_hex(parts[2]).map_err(PsetUpdateInputError::ValueCommitmentParse)?;
		elements::confidential::Value::from_commitment(&commitment_bytes)
			.map_err(PsetUpdateInputError::ValueCommitmentDecode)?
	};

	Ok(crate::simplicity::jet::elements::ElementsUtxo {
		script_pubkey,
		asset,
		value,
	})
}
