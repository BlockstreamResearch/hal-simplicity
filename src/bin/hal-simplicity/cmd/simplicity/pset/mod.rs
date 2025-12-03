// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod create;
mod extract;
mod finalize;
mod update_input;

use std::sync::Arc;

use super::{Error, ErrorExt as _};
use crate::cmd;

use elements::hashes::Hash as _;
use elements::pset::PartiallySignedTransaction;
use elements::taproot::ControlBlock;
use elements::Script;
use hal_simplicity::simplicity::elements::Transaction;
use hal_simplicity::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use hal_simplicity::simplicity::Cmr;
use serde::Serialize;

#[derive(Serialize)]
struct UpdatedPset {
	pset: String,
	updated_values: Vec<&'static str>,
}

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("pset", "manipulate PSETs for spending from Simplicity programs")
		.subcommand(self::create::cmd())
		.subcommand(self::extract::cmd())
		.subcommand(self::finalize::cmd())
		.subcommand(self::update_input::cmd())
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("create", Some(m)) => self::create::exec(m),
		("extract", Some(m)) => self::extract::exec(m),
		("finalize", Some(m)) => self::finalize::exec(m),
		("update-input", Some(m)) => self::update_input::exec(m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn execution_environment(
	pset: &PartiallySignedTransaction,
	input_idx: usize,
	cmr: Cmr,
	genesis_hash: Option<&str>,
) -> Result<(ElementsEnv<Arc<Transaction>>, ControlBlock, Script), Error> {
	let n_inputs = pset.n_inputs();
	let input = pset
		.inputs()
		.get(input_idx)
		.ok_or_else(|| {
			format!("index {} out-of-range for PSET with {} inputs", input_idx, n_inputs)
		})
		.result_context("parsing input index")?;

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

	let tx_env = ElementsEnv::new(
		tx,
		input_utxos,
		input_idx as u32, // cast fine, input indices are always small
		cmr,
		control_block.clone(),
		None, // FIXME populate this; needs https://github.com/BlockstreamResearch/rust-simplicity/issues/315 first
		genesis_hash,
	);

	// 3. Prune program.
	Ok((tx_env, control_block, tap_leaf))
}
