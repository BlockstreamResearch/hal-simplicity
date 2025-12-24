// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;
use crate::cmd::simplicity::pset::PsetError;

use hal_simplicity::hal_simplicity::Program;
use hal_simplicity::simplicity::jet;

use super::super::Error;
use super::UpdatedPset;

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
}

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("finalize", "Attach a Simplicity program and witness to a PSET input")
		.args(&cmd::opts_networks())
		.args(&[
			cmd::arg("pset", "PSET to update (base64)").takes_value(true).required(true),
			cmd::arg("input-index", "the index of the input to sign (decimal)")
				.takes_value(true)
				.required(true),
			cmd::arg("program", "Simplicity program (base64)").takes_value(true).required(true),
			cmd::arg("witness", "Simplicity program witness (hex)")
				.takes_value(true)
				.required(true),
			cmd::opt(
				"genesis-hash",
				"genesis hash of the blockchain the transaction belongs to (hex)",
			)
			.short("g")
			.required(false),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory");
	let input_idx = matches.value_of("input-index").expect("input-idx is mandatory");
	let program = matches.value_of("program").expect("program is mandatory");
	let witness = matches.value_of("witness").expect("witness is mandatory");
	let genesis_hash = matches.value_of("genesis-hash");

	match exec_inner(pset_b64, input_idx, program, witness, genesis_hash) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}

#[allow(clippy::too_many_arguments)]
fn exec_inner(
	pset_b64: &str,
	input_idx: &str,
	program: &str,
	witness: &str,
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
		super::execution_environment(&pset, input_idx_usize, program.cmr(), genesis_hash)?;
	let cb_serialized = control_block.serialize();

	// 3. Prune program.
	let redeem_node = program.redeem_node().ok_or(PsetFinalizeError::NoRedeemNode)?;
	let pruned = redeem_node.prune(&tx_env).map_err(PsetFinalizeError::ProgramPrune)?;

	let (prog, witness) = pruned.to_vec_with_witness();
	// If `execution_environment` above succeeded we are guaranteed that this index is in bounds.
	let input = &mut pset.inputs_mut()[input_idx_usize];
	input.final_script_witness = Some(vec![witness, prog, tap_leaf.into_bytes(), cb_serialized]);

	let updated_values = vec!["final_script_witness"];

	Ok(UpdatedPset {
		pset: pset.to_string(),
		updated_values,
	})
}
