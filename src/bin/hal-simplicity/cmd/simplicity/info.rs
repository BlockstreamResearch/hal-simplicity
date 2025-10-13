// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use super::{Error, ErrorExt as _};

use hal_simplicity::hal_simplicity::{elements_address, Program};
use hal_simplicity::simplicity::{jet, Amr, Cmr, Ihr};

use serde::Serialize;

#[derive(Serialize)]
struct RedeemInfo {
	redeem_base64: String,
	witness_hex: String,
	amr: Amr,
	ihr: Ihr,
}

#[derive(Serialize)]
struct ProgramInfo {
	jets: &'static str,
	commit_base64: String,
	commit_decode: String,
	type_arrow: String,
	cmr: Cmr,
	liquid_address_unconf: String,
	liquid_testnet_address_unconf: String,
	is_redeem: bool,
	#[serde(flatten)]
	#[serde(skip_serializing_if = "Option::is_none")]
	redeem_info: Option<RedeemInfo>,
}

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("info", "Parse a base64-encoded Simplicity program and decode it")
		.args(&cmd::opts_networks())
		.args(&[
			cmd::opt_yaml(),
			cmd::arg("program", "a Simplicity program in base64").takes_value(true).required(true),
			cmd::arg("witness", "a hex encoding of all the witness data for the program")
				.takes_value(true)
				.required(false),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let program = matches.value_of("program").expect("program is mandatory");
	let witness = matches.value_of("witness");

	match exec_inner(program, witness) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(matches, &e),
	}
}

fn exec_inner(program: &str, witness: Option<&str>) -> Result<ProgramInfo, Error> {
	// In the future we should attempt to parse as a Bitcoin program if parsing as
	// Elements fails. May be tricky/annoying in Rust since Program<Elements> is a
	// different type from Program<Bitcoin>.
	let program =
		Program::<jet::Elements>::from_str(program, witness).result_context("parsing program")?;

	let redeem_info = program.redeem_node().map(|node| {
		let disp = node.display();
		let x = RedeemInfo {
			redeem_base64: disp.program().to_string(),
			witness_hex: disp.witness().to_string(),
			amr: node.amr(),
			ihr: node.ihr(),
		};
		x // binding needed for truly stupid borrowck reasons
	});

	Ok(ProgramInfo {
		jets: "core",
		commit_base64: program.commit_prog().to_string(),
		// FIXME this is, in general, exponential in size. Need to limit it somehow; probably need upstream support
		commit_decode: program.commit_prog().display_expr().to_string(),
		type_arrow: program.commit_prog().arrow().to_string(),
		cmr: program.cmr(),
		liquid_address_unconf: elements_address(program.cmr(), &elements::AddressParams::LIQUID)
			.to_string(),
		liquid_testnet_address_unconf: elements_address(
			program.cmr(),
			&elements::AddressParams::LIQUID_TESTNET,
		)
		.to_string(),
		is_redeem: redeem_info.is_some(),
		redeem_info,
	})
}
