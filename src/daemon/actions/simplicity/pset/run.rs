use super::PsetError;
use crate::daemon::actions::types::{JetCall, PsetRunRequest, PsetRunResponse};
use crate::hal_simplicity::Program;
use crate::simplicity::bit_machine::{BitMachine, ExecTracker};
use crate::simplicity::jet;
use crate::simplicity::{Cmr, Ihr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PsetRunError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("Failed to decode PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("Failed to parse program: {0}")]
	ProgramParse(simplicity::ParseError),

	#[error("Program does not have a redeem node")]
	NoRedeemNode,

	#[error("Failed to construct bit machine: {0}")]
	BitMachineConstruction(simplicity::bit_machine::LimitError),
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
			"eq_1" => None,
			"eq_2" => None,
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

pub fn run(req: PsetRunRequest) -> Result<PsetRunResponse, PsetRunError> {
	let pset: elements::pset::PartiallySignedTransaction =
		req.pset.parse().map_err(PsetRunError::PsetDecode)?;

	let input_idx_usize = req.input_index as usize;

	let program = Program::<jet::Elements>::from_str(&req.program, Some(&req.witness))
		.map_err(PsetRunError::ProgramParse)?;

	let (tx_env, _control_block, _tap_leaf) = super::execution_environment(
		&pset,
		input_idx_usize,
		program.cmr(),
		req.genesis_hash.as_deref(),
	)?;

	let redeem_node = program.redeem_node().ok_or_else(|| PsetRunError::NoRedeemNode)?;

	let mut mac =
		BitMachine::for_program(redeem_node).map_err(PsetRunError::BitMachineConstruction)?;

	let mut tracker = JetTracker(vec![]);
	let success = mac.exec_with_tracker(redeem_node, &tx_env, &mut tracker).is_ok();

	Ok(PsetRunResponse {
		success,
		jets: tracker.0,
	})
}
