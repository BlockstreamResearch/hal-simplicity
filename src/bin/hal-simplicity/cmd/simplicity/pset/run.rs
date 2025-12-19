// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use hal_simplicity::hal_simplicity::Program;
use hal_simplicity::simplicity::bit_machine::{BitMachine, ExecTracker, FrameIter, NodeOutput};
use hal_simplicity::simplicity::Value;
use hal_simplicity::simplicity::{jet, node};

use super::super::{Error, ErrorExt as _};

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
		Err(e) => cmd::print_output(matches, &e),
	}
}

#[derive(serde::Serialize)]
struct JetCall {
	jet: String,
	source_ty: String,
	target_ty: String,
	success: bool,
	input_value: String,
	output_value: String,
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
) -> Result<Response, Error> {
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

	// 1. Parse everything.
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().result_context("decoding PSET")?;
	let input_idx: u32 = input_idx.parse().result_context("parsing input-idx")?;
	let input_idx_usize = input_idx as usize; // 32->usize cast ok on almost all systems

	let program = Program::<jet::Elements>::from_str(program, Some(witness))
		.result_context("parsing program")?;

	// 2. Extract transaction environment.
	let (tx_env, _control_block, _tap_leaf) =
		super::execution_environment(&pset, input_idx_usize, program.cmr(), genesis_hash)?;

	// 3. Prune program.
	let redeem_node = program.redeem_node().expect("populated");

	let mut mac =
		BitMachine::for_program(redeem_node).result_context("constructing bit machine")?;
	let mut tracker = JetTracker(vec![]);
	// Eat success/failure. FIXME should probably report this to the user.
	let success = mac.exec_with_tracker(redeem_node, &tx_env, &mut tracker).is_ok();
	Ok(Response {
		success,
		jets: tracker.0,
	})
}
