// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use elements::bitcoin::taproot::TAPROOT_ANNEX_PREFIX;

use crate::hal_simplicity::Program;
use crate::simplicity::jet;

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

	#[error("cost of transaction exceeds budget. Suggested annex padding bytes to add (including prefix): {padding_required}")]
	CostExceedsBudget {
		padding_required: usize,
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

	let program = Program::<jet::Elements>::from_str(program, Some(witness))
		.map_err(PsetFinalizeError::ProgramParse)?;

	// 2. Extract transaction environment.
	let (tx_env, control_block, tap_leaf) =
		execution_environment(&pset, input_idx_usize, program.cmr(), genesis_hash)?;
	let cb_serialized = control_block.serialize();

	// 3. Prune program.
	let redeem_node = program.redeem_node().ok_or(PsetFinalizeError::NoRedeemNode)?;
	let pruned = redeem_node.prune(&tx_env).map_err(PsetFinalizeError::ProgramPrune)?;

	let (prog, witness) = pruned.to_vec_with_witness();
	// If `execution_environment` above succeeded we are guaranteed that this index is in bounds.
	let input = &mut pset.inputs_mut()[input_idx_usize];

	let final_script_witness = if annex_padding_size == 0 {
		vec![witness, prog, tap_leaf.into_bytes(), cb_serialized]
	} else {
		let mut annex = vec![0u8; annex_padding_size];
		annex[0] = TAPROOT_ANNEX_PREFIX;

		vec![witness, prog, tap_leaf.into_bytes(), cb_serialized, annex]
	};

	// Calculate the budget and warn the user if it is not enough.
	let cost = pruned.bounds().cost;
	if !cost.is_budget_valid(&final_script_witness) {
		let padding_required = cost.get_padding(&final_script_witness).map(|bytes| bytes.len());
		return Err(PsetFinalizeError::CostExceedsBudget {
			padding_required: padding_required.unwrap_or_default(),
		});
	}

	input.final_script_witness = Some(final_script_witness);

	let updated_values = vec!["final_script_witness"];

	Ok(UpdatedPset {
		pset: pset.to_string(),
		updated_values,
	})
}
