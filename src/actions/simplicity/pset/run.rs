// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use serde::Serialize;

use crate::hal_simplicity::Program;
use crate::simplicity::bit_machine::{BitMachine, ExecTracker};
use crate::simplicity::jet;
use crate::simplicity::{Cmr, Ihr};

use super::{execution_environment, PsetError};

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

#[derive(Serialize)]
pub struct JetCall {
	pub jet: String,
	pub source_ty: String,
	pub target_ty: String,
	pub success: bool,
	pub input_hex: String,
	pub output_hex: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub equality_check: Option<(String, String)>,
}

#[derive(Serialize)]
pub struct RunResponse {
	pub success: bool,
	pub jets: Vec<JetCall>,
}

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

/// Run a Simplicity program in the context of a PSET input
pub fn pset_run(
	pset_b64: &str,
	input_idx: &str,
	program: &str,
	witness: &str,
	genesis_hash: Option<&str>,
) -> Result<RunResponse, PsetRunError> {
	// 1. Parse everything.
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetRunError::PsetDecode)?;
	let input_idx: u32 = input_idx.parse().map_err(PsetRunError::InputIndexParse)?;
	let input_idx_usize = input_idx as usize; // 32->usize cast ok on almost all systems

	let program = Program::<jet::Elements>::from_str(program, Some(witness))
		.map_err(PsetRunError::ProgramParse)?;

	// 2. Extract transaction environment.
	let (tx_env, _control_block, _tap_leaf) =
		execution_environment(&pset, input_idx_usize, program.cmr(), genesis_hash)?;

	// 3. Prune program.
	let redeem_node = program.redeem_node().ok_or(PsetRunError::NoRedeemNode)?;

	let mut mac =
		BitMachine::for_program(redeem_node).map_err(PsetRunError::BitMachineConstruction)?;
	let mut tracker = JetTracker(vec![]);
	// Eat success/failure. FIXME should probably report this to the user.
	let success = mac.exec_with_tracker(redeem_node, &tx_env, &mut tracker).is_ok();
	Ok(RunResponse {
		success,
		jets: tracker.0,
	})
}
