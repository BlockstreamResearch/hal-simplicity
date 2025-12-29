// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use core::str::FromStr;
use std::collections::BTreeMap;

use elements::bitcoin::secp256k1;
use elements::schnorr::XOnlyPublicKey;
use simplicity::hex::parse::FromHex as _;

use crate::hal_simplicity::taproot_spend_info;

use super::{PsetError, UpdatedPset};

use crate::actions::simplicity::ParseElementsUtxoError;

#[derive(Debug, thiserror::Error)]
pub enum PsetUpdateInputError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("invalid PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("invalid input index: {0}")]
	InputIndexParse(std::num::ParseIntError),

	#[error("input index {index} out-of-range for PSET with {total} inputs")]
	InputIndexOutOfRange {
		index: usize,
		total: usize,
	},

	#[error("invalid CMR: {0}")]
	CmrParse(elements::hashes::hex::HexToArrayError),

	#[error("invalid internal key: {0}")]
	InternalKeyParse(secp256k1::Error),

	#[error("internal key must be present if CMR is; PSET requires a control block for each CMR, which in turn requires the internal key. If you don't know the internal key, good chance it is the BIP-0341 'unspendable key' 50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0 or the web IDE's 'unspendable key' (highly discouraged for use in production) of f5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2")]
	MissingInternalKey,

	#[error("input UTXO does not appear to be a Taproot output")]
	NotTaprootOutput,

	#[error("invalid state commitment: {0}")]
	StateParse(elements::hashes::hex::HexToArrayError),

	#[error("CMR and internal key imply output key {output_key}, which does not match input scriptPubKey {script_pubkey}")]
	OutputKeyMismatch {
		output_key: String,
		script_pubkey: String,
	},

	#[error("invalid elements UTXO: {0}")]
	ElementsUtxoParse(ParseElementsUtxoError),
}

/// Attach UTXO data to a PSET input
pub fn pset_update_input(
	pset_b64: &str,
	input_idx: &str,
	input_utxo: &str,
	internal_key: Option<&str>,
	cmr: Option<&str>,
	state: Option<&str>,
) -> Result<UpdatedPset, PsetUpdateInputError> {
	let mut pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetUpdateInputError::PsetDecode)?;
	let input_idx: usize = input_idx.parse().map_err(PsetUpdateInputError::InputIndexParse)?;
	let input_utxo = super::super::parse_elements_utxo(input_utxo)
		.map_err(PsetUpdateInputError::ElementsUtxoParse)?;

	let n_inputs = pset.n_inputs();
	let input = pset.inputs_mut().get_mut(input_idx).ok_or_else(|| {
		PsetUpdateInputError::InputIndexOutOfRange {
			index: input_idx,
			total: n_inputs,
		}
	})?;

	let cmr =
		cmr.map(simplicity::Cmr::from_str).transpose().map_err(PsetUpdateInputError::CmrParse)?;
	let internal_key = internal_key
		.map(XOnlyPublicKey::from_str)
		.transpose()
		.map_err(PsetUpdateInputError::InternalKeyParse)?;
	if cmr.is_some() && internal_key.is_none() {
		return Err(PsetUpdateInputError::MissingInternalKey);
	}

	if !input_utxo.script_pubkey.is_v1_p2tr() {
		return Err(PsetUpdateInputError::NotTaprootOutput);
	}

	// FIXME state is meaningless without CMR; should we warn here
	// FIXME also should we warn if you don't provide a CMR? seems like if you're calling `simplicity pset update-input`
	//   you probably have a simplicity program right? maybe we should even provide a --no-cmr flag
	let state =
		state.map(<[u8; 32]>::from_hex).transpose().map_err(PsetUpdateInputError::StateParse)?;

	let mut updated_values = vec![];
	if let Some(internal_key) = internal_key {
		updated_values.push("tap_internal_key");
		input.tap_internal_key = Some(internal_key);
		// FIXME should we check whether we're using the "bad" internal key
		//  from the web IDE, and warn or something?
		if let Some(cmr) = cmr {
			// Guess that the given program is the only Tapleaf. This is the case for addresses
			// generated from the web IDE, and from `hal-simplicity simplicity info`, and for
			// most "test" scenarios. We need to design an API to handle more general cases.
			let spend_info = taproot_spend_info(internal_key, state, cmr);
			if spend_info.output_key().as_inner().serialize() != input_utxo.script_pubkey[2..] {
				// If our guess was wrong, at least error out..
				return Err(PsetUpdateInputError::OutputKeyMismatch {
					output_key: format!("{}", spend_info.output_key().as_inner()),
					script_pubkey: format!("{}", input_utxo.script_pubkey),
				});
			}

			// FIXME these unwraps and clones should be fixed by a new rust-bitcoin taproot API
			let script_ver = spend_info.as_script_map().keys().next().unwrap();
			let cb = spend_info.control_block(script_ver).unwrap();
			input.tap_merkle_root = spend_info.merkle_root();
			input.tap_scripts = BTreeMap::new();
			input.tap_scripts.insert(cb, script_ver.clone());
			updated_values.push("tap_merkle_root");
			updated_values.push("tap_scripts");
		}
	}

	// FIXME should we bother erroring or warning if we clobber this or other fields?
	input.witness_utxo = Some(elements::TxOut {
		asset: input_utxo.asset,
		value: input_utxo.value,
		nonce: elements::confidential::Nonce::Null, // not in UTXO set, irrelevant to PSET
		script_pubkey: input_utxo.script_pubkey,
		witness: elements::TxOutWitness::empty(), // not in UTXO set, irrelevant to PSET
	});
	updated_values.push("witness_utxo");

	Ok(UpdatedPset {
		pset: pset.to_string(),
		updated_values,
	})
}
