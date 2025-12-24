// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;
use crate::cmd::simplicity::pset::PsetError;

use hal_simplicity::hal_simplicity::Program;
use hal_simplicity::simplicity::bit_machine::{BitMachine, ExecTracker};
use hal_simplicity::simplicity::jet;
use hal_simplicity::simplicity::{Cmr, Ihr};

use super::super::Error;

#[derive(Debug, thiserror::Error)]
pub enum PsetRunError {
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

	#[error("failed to construct bit machine: {0}")]
	BitMachineConstruction(simplicity::bit_machine::LimitError),
}

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("run", "Run a Simplicity program in the context of a PSET input.")
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

#[derive(serde::Serialize)]
struct JetCall {
	jet: String,
	source_ty: String,
	target_ty: String,
	success: bool,
	input_hex: String,
	output_hex: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	equality_check: Option<(String, String)>,
}

#[derive(serde::Serialize)]
struct Response {
	success: bool,
	jets: Vec<JetCall>,
}

#[allow(clippy::too_many_arguments)]
fn exec_inner(
	pset_b64: &str,
	input_idx: &str,
	program: &str,
	witness: &str,
	genesis_hash: Option<&str>,
) -> Result<Response, PsetRunError> {
	struct JetTracker(Vec<JetCall>);
	impl<J: jet::Jet> ExecTracker<J> for JetTracker {
		fn track_left(&mut self, _: Ihr) {}
		fn track_right(&mut self, _: Ihr) {}
		fn track_jet_call(
			&mut self,
			jet: &J,
			input_buffer: &[simplicity::ffi::ffi::UWORD],
			output_buffer: &[simplicity::ffi::ffi::UWORD],
			success: bool,
		) {
			// The word slices are in reverse order for some reason.
			// FIXME maybe we should attempt to parse out Simplicity values here which
			//    can often be displayed in a better way, esp for e.g. option types.
			let mut input_hex = String::new();
			for word in input_buffer.iter().rev() {
				for byte in word.to_be_bytes() {
					input_hex.push_str(&format!("{:02x}", byte));
				}
			}

			let mut output_hex = String::new();
			for word in output_buffer.iter().rev() {
				for byte in word.to_be_bytes() {
					output_hex.push_str(&format!("{:02x}", byte));
				}
			}

			let jet_name = jet.to_string();
			let equality_check = match jet_name.as_str() {
				"eq_1" => None, // FIXME parse bits out of input
				"eq_2" => None, // FIXME parse bits out of input
				x if x.strip_prefix("eq_").is_some() => {
					let split = input_hex.split_at(input_hex.len() / 2);
					Some((split.0.to_owned(), split.1.to_owned()))
				}
				_ => None,
			};
			self.0.push(JetCall {
				jet: jet_name,
				source_ty: jet.source_ty().to_final().to_string(),
				target_ty: jet.target_ty().to_final().to_string(),
				success,
				input_hex,
				output_hex,
				equality_check,
			});
		}

		fn track_dbg_call(&mut self, _: &Cmr, _: simplicity::Value) {}
		fn is_track_debug_enabled(&self) -> bool {
			false
		}
	}

	// 1. Parse everything.
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetRunError::PsetDecode)?;
	let input_idx: u32 = input_idx.parse().map_err(PsetRunError::InputIndexParse)?;
	let input_idx_usize = input_idx as usize; // 32->usize cast ok on almost all systems

	let program = Program::<jet::Elements>::from_str(program, Some(witness))
		.map_err(PsetRunError::ProgramParse)?;

	// 2. Extract transaction environment.
	let (tx_env, _control_block, _tap_leaf) =
		super::execution_environment(&pset, input_idx_usize, program.cmr(), genesis_hash)?;

	// 3. Prune program.
	let redeem_node = program.redeem_node().ok_or(PsetRunError::NoRedeemNode)?;

	let mut mac =
		BitMachine::for_program(redeem_node).map_err(PsetRunError::BitMachineConstruction)?;
	let mut tracker = JetTracker(vec![]);
	// Eat success/failure. FIXME should probably report this to the user.
	let success = mac.exec_with_tracker(redeem_node, &tx_env, &mut tracker).is_ok();
	Ok(Response {
		success,
		jets: tracker.0,
	})
}
