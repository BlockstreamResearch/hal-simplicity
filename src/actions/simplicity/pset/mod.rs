// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod create;
mod extract;
mod finalize;
mod run;
mod update_input;

pub use create::*;
pub use extract::*;
pub use finalize::*;
pub use run::*;
pub use update_input::*;

use std::sync::Arc;

use elements::hashes::Hash as _;
use elements::pset::PartiallySignedTransaction;
use elements::taproot::ControlBlock;
use elements::Script;
use serde::Serialize;

use crate::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use crate::simplicity::Cmr;

#[derive(Debug, thiserror::Error)]
pub enum PsetError {
	#[error("input index {index} out-of-range for PSET with {total} inputs")]
	InputIndexOutOfRange {
		index: usize,
		total: usize,
	},

	#[error("failed to parse genesis hash: {0}")]
	GenesisHashParse(elements::hashes::hex::HexToArrayError),

	#[error("could not find Simplicity leaf in PSET taptree with CMR {cmr})")]
	MissingSimplicityLeaf {
		cmr: String,
	},

	#[error("failed to extract transaction from PSET: {0}")]
	PsetExtract(elements::pset::Error),

	#[error("witness_utxo field not populated for input {0}")]
	MissingWitnessUtxo(usize),
}

#[derive(Serialize)]
pub struct UpdatedPset {
	pub pset: String,
	pub updated_values: Vec<&'static str>,
}

/// Helper function to create execution environment for PSET operations
pub fn execution_environment(
	pset: &PartiallySignedTransaction,
	input_idx: usize,
	cmr: Cmr,
	genesis_hash: Option<&str>,
) -> Result<(ElementsEnv<Arc<elements::Transaction>>, ControlBlock, Script), PsetError> {
	let n_inputs = pset.n_inputs();
	let input = pset.inputs().get(input_idx).ok_or(PsetError::InputIndexOutOfRange {
		index: input_idx,
		total: n_inputs,
	})?;

	// Default to Liquid Testnet genesis block
	let genesis_hash = match genesis_hash {
		Some(s) => s.parse().map_err(PsetError::GenesisHashParse)?,
		None => elements::BlockHash::from_byte_array([
			// copied out of simplicity-webide source
			0xc1, 0xb1, 0x6a, 0xe2, 0x4f, 0x24, 0x23, 0xae, 0xa2, 0xea, 0x34, 0x55, 0x22, 0x92,
			0x79, 0x3b, 0x5b, 0x5e, 0x82, 0x99, 0x9a, 0x1e, 0xed, 0x81, 0xd5, 0x6a, 0xee, 0x52,
			0x8e, 0xda, 0x71, 0xa7,
		]),
	};

	// Unlike in the 'update-input' case we don't insist on any particular form of
	// the Taptree. We just look for the CMR in the list.
	let mut control_block_leaf = None;
	for (cb, script_ver) in &input.tap_scripts {
		if script_ver.1 == simplicity::leaf_version() && &script_ver.0[..] == cmr.as_ref() {
			control_block_leaf = Some((cb.clone(), script_ver.0.clone()));
		}
	}
	let (control_block, tap_leaf) = match control_block_leaf {
		Some((cb, leaf)) => (cb, leaf),
		None => {
			return Err(PsetError::MissingSimplicityLeaf {
				cmr: cmr.to_string(),
			});
		}
	};

	let tx = pset.extract_tx().map_err(PsetError::PsetExtract)?;
	let tx = Arc::new(tx);

	let input_utxos = pset
		.inputs()
		.iter()
		.enumerate()
		.map(|(n, input)| match input.witness_utxo {
			Some(ref utxo) => Ok(ElementsUtxo {
				script_pubkey: utxo.script_pubkey.clone(),
				asset: utxo.asset,
				value: utxo.value,
			}),
			None => Err(PsetError::MissingWitnessUtxo(n)),
		})
		.collect::<Result<Vec<_>, _>>()?;

	let tx_env = ElementsEnv::new(
		tx,
		input_utxos,
		input_idx as u32, // cast fine, input indices are always small
		cmr,
		control_block.clone(),
		None, // FIXME populate this; needs https://github.com/BlockstreamResearch/rust-simplicity/issues/315 first
		genesis_hash,
	);

	Ok((tx_env, control_block, tap_leaf))
}
