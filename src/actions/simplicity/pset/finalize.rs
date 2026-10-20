// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use elements::bitcoin::taproot::TAPROOT_ANNEX_PREFIX;

use crate::hal_simplicity::Program;

use super::{execution_environment, PsetError, UpdatedPset};

#[derive(Debug, thiserror::Error)]
pub enum PsetFinalizeError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("invalid PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("invalid input index: {0}")]
	InputIndexParse(std::num::ParseIntError),

	#[error("invalid program: {0}")]
	ProgramParse(simplicity::ParseError),

	#[error("program does not have a redeem node")]
	NoRedeemNode,

	#[error("failed to prune program: {0}")]
	ProgramPrune(simplicity::bit_machine::ExecutionError),

	#[error("program cost exceeds the witness budget: needs an annex of at least {required} bytes (including the 0x50 prefix), got {provided}")]
	CostExceedsBudget {
		required: usize,
		provided: usize,
	},
}

/// Attach a Simplicity program and witness to a PSET input
pub fn pset_finalize(
	pset_b64: &str,
	input_idx: &str,
	program: &str,
	witness: &str,
	annex_padding_size: usize,
	genesis_hash: Option<&str>,
) -> Result<UpdatedPset, PsetFinalizeError> {
	// 1. Parse everything.
	let mut pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetFinalizeError::PsetDecode)?;
	let input_idx: u32 = input_idx.parse().map_err(PsetFinalizeError::InputIndexParse)?;
	let input_idx_usize = input_idx as usize; // 32->usize cast ok on almost all systems

	let program =
		Program::from_str(program, Some(witness)).map_err(PsetFinalizeError::ProgramParse)?;

	let annex = (annex_padding_size > 0).then(|| {
		let mut annex = vec![0u8; annex_padding_size];
		annex[0] = TAPROOT_ANNEX_PREFIX;
		annex
	});

	// The sighash commits to the annex, so the program has to run against a transaction
	// that has it. rust-simplicity reads the annex from the input's witness, so stage it
	// there. This also discards the witness from any earlier finalization.
	if let Some(input) = pset.inputs_mut().get_mut(input_idx_usize) {
		input.final_script_witness = annex.clone().map(|annex| vec![annex]);
	}

	// 2. Extract transaction environment.
	let (tx_env, control_block, tap_leaf) =
		execution_environment(&pset, input_idx_usize, program.cmr(), genesis_hash)?;
	let cb_serialized = control_block.serialize();

	// 3. Prune program.
	let redeem_node = program.redeem_node().ok_or(PsetFinalizeError::NoRedeemNode)?;
	let pruned = redeem_node.prune(&tx_env).map_err(PsetFinalizeError::ProgramPrune)?;

	let (prog, witness) = pruned.to_vec_with_witness();
	let mut final_script_witness = vec![witness, prog, tap_leaf.into_bytes(), cb_serialized];

	// 4. Check that the witness pays for the program's cost. The required padding is
	// computed before adding the annex so that it is the total annex size needed.
	let cost = pruned.bounds().cost;
	let required = cost.get_padding(&final_script_witness).map_or(0, |annex| annex.len());
	final_script_witness.extend(annex);
	if !cost.is_budget_valid(&final_script_witness) {
		return Err(PsetFinalizeError::CostExceedsBudget {
			required,
			provided: annex_padding_size,
		});
	}

	// If `execution_environment` above succeeded we are guaranteed that this index is in bounds.
	pset.inputs_mut()[input_idx_usize].final_script_witness = Some(final_script_witness);

	let updated_values = vec!["final_script_witness"];

	Ok(UpdatedPset {
		pset: pset.to_string(),
		updated_values,
	})
}
