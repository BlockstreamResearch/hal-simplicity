// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use serde::Serialize;

use crate::hal_simplicity::Program;
use crate::simplicity::bit_machine::{BitMachine, ExecTracker, FrameIter, NodeOutput};
use crate::simplicity::Value;
use crate::simplicity::{jet, node};

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
	pub input_value: String,
	pub output_value: String,
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
	fn visit_node(
		&mut self,
		node: &simplicity::RedeemNode<J>,
		mut input: FrameIter,
		output: NodeOutput,
	) {
		if let node::Inner::Jet(jet) = node.inner() {
			let input_value = Value::from_padded_bits(&mut input, &node.arrow().source)
				.expect("valid value from bit machine");

			let (success, output_value) = match output {
				NodeOutput::NonTerminal => unreachable!(),
				NodeOutput::JetFailed => (false, Value::unit()),
				NodeOutput::Success(mut iter) => (
					true,
					Value::from_padded_bits(&mut iter, &node.arrow().target)
						.expect("valid value from bit machine"),
				),
			};

			let jet_name = jet.to_string();
			let equality_check = if jet_name.strip_prefix("eq_").is_some() {
				let (left, right) = input_value.as_product().unwrap();
				Some((left.to_value().to_string(), right.to_value().to_string()))
			} else {
				None
			};

			self.0.push(JetCall {
				jet: jet_name,
				source_ty: jet.source_ty().to_final().to_string(),
				target_ty: jet.target_ty().to_final().to_string(),
				success,
				input_value: input_value.to_string(),
				output_value: output_value.to_string(),
				equality_check,
			});
		}
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
