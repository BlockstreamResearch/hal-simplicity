// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use std::sync::Arc;

use elements::hashes::Hash as _;
use hal_simplicity::hal_simplicity::Program;
use hal_simplicity::simplicity::jet;
use hal_simplicity::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};

use super::super::{Error, ErrorExt as _};
use super::UpdatedPset;

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
		Err(e) => cmd::print_output(matches, &e),
	}
}

#[allow(clippy::too_many_arguments)]
fn exec_inner(
	pset_b64: &str,
	input_idx: &str,
	program: &str,
	witness: &str,
	genesis_hash: Option<&str>,
) -> Result<UpdatedPset, Error> {
	// 1. Parse everything.
	let mut pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().result_context("decoding PSET")?;
	let input_idx: u32 = input_idx.parse().result_context("parsing input-idx")?;
	let input_idx_usize = input_idx as usize; // 32->usize cast ok on almost all systems

	let n_inputs = pset.n_inputs();
	let input = pset
		.inputs_mut()
		.get_mut(input_idx_usize)
		.ok_or_else(|| {
			format!("index {} out-of-range for PSET with {} inputs", input_idx, n_inputs)
		})
		.result_context("parsing input index")?;

	let program = Program::<jet::Elements>::from_str(program, Some(witness))
		.result_context("parsing program")?;

	// 2. Build transaction environment.
	// Default to Liquid Testnet genesis block
	let genesis_hash = match genesis_hash {
		Some(s) => s.parse().result_context("parsing genesis hash")?,
		None => elements::BlockHash::from_byte_array([
			// copied out of simplicity-webide source
			0xc1, 0xb1, 0x6a, 0xe2, 0x4f, 0x24, 0x23, 0xae, 0xa2, 0xea, 0x34, 0x55, 0x22, 0x92,
			0x79, 0x3b, 0x5b, 0x5e, 0x82, 0x99, 0x9a, 0x1e, 0xed, 0x81, 0xd5, 0x6a, 0xee, 0x52,
			0x8e, 0xda, 0x71, 0xa7,
		]),
	};

	let cmr = program.cmr();
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
        	return Err(format!("could not find Simplicity leaf in PSET taptree with CMR {}; did you forget to run 'simplicity pset update-input'?", cmr))
        	    .result_context("PSET tap_scripts field")
        }
	};

	let tx = pset.extract_tx().result_context("extracting transaction from PSET")?;
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
			None => Err(format!("witness_utxo field not populated for input {n}")),
		})
		.collect::<Result<Vec<_>, _>>()
		.result_context("extracting input UTXO information")?;

	let cb_serialized = control_block.serialize();
	let tx_env = ElementsEnv::new(
		tx,
		input_utxos,
		input_idx,
		cmr,
		control_block,
		None, // FIXME populate this; needs https://github.com/BlockstreamResearch/rust-simplicity/issues/315 first
		genesis_hash,
	);

	// 3. Prune program.
	let redeem_node = program.redeem_node().expect("populated");
	let pruned = redeem_node.prune(&tx_env).result_context("pruning program")?;

	let (prog, witness) = pruned.to_vec_with_witness();
	// Rust makes us re-borrow 'input' mutably since we used 'pset' immutably since we
	// last borrowed it. We can unwrap() this time since we know it'll succeed.
	let input = &mut pset.inputs_mut()[input_idx_usize];
	input.final_script_witness = Some(vec![witness, prog, tap_leaf.into_bytes(), cb_serialized]);

	let updated_values = vec!["final_script_witness"];

	Ok(UpdatedPset {
		pset: pset.to_string(),
		updated_values,
	})
}
