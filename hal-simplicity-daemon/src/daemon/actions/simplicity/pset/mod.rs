mod create;
mod extract;
mod finalize;
mod run;
mod update_input;

pub use create::create;
pub use extract::extract;
pub use finalize::finalize;
pub use run::run;
pub use update_input::update_input;

use crate::simplicity::elements::Transaction;
use crate::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use crate::simplicity::Cmr;
use elements::hashes::Hash as _;
use elements::pset::PartiallySignedTransaction;
use elements::taproot::ControlBlock;
use elements::Script;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PsetError {
	#[error("Input index {index} out-of-range for PSET with {total} inputs")]
	InputIndexOutOfRange {
		index: usize,
		total: usize,
	},

	#[error("Failed to parse genesis hash: {0}")]
	GenesisHashParse(String),

	#[error("Could not find Simplicity leaf in PSET taptree with CMR {cmr}; did you forget to run 'simplicity pset update-input'?")]
	MissingSimplicityLeaf {
		cmr: String,
	},

	#[error("Failed to extract transaction from PSET: {0}")]
	PsetExtract(String),

	#[error("witness_utxo field not populated for input {0}")]
	MissingWitnessUtxo(usize),
}

fn execution_environment(
	pset: &PartiallySignedTransaction,
	input_idx: usize,
	cmr: Cmr,
	genesis_hash: Option<&str>,
) -> Result<(ElementsEnv<Arc<Transaction>>, ControlBlock, Script), PsetError> {
	let n_inputs = pset.n_inputs();
	let input = pset.inputs().get(input_idx).ok_or(PsetError::InputIndexOutOfRange {
		index: input_idx,
		total: n_inputs,
	})?;

	// Default to Liquid Testnet genesis block
	let genesis_hash = match genesis_hash {
		Some(s) => s.parse().map_err(|e| PsetError::GenesisHashParse(format!("{}", e)))?,
		None => elements::BlockHash::from_byte_array([
			0xc1, 0xb1, 0x6a, 0xe2, 0x4f, 0x24, 0x23, 0xae, 0xa2, 0xea, 0x34, 0x55, 0x22, 0x92,
			0x79, 0x3b, 0x5b, 0x5e, 0x82, 0x99, 0x9a, 0x1e, 0xed, 0x81, 0xd5, 0x6a, 0xee, 0x52,
			0x8e, 0xda, 0x71, 0xa7,
		]),
	};

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
			})
		}
	};

	let tx = pset.extract_tx().map_err(|e| PsetError::PsetExtract(format!("{}", e)))?;
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
		input_idx as u32,
		cmr,
		control_block.clone(),
		None,
		genesis_hash,
	);

	Ok((tx_env, control_block, tap_leaf))
}
