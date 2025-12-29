use std::io::Write;

use elements::encode::serialize;

use crate::cmd;
use hal_simplicity::block::BlockInfo;

use log::warn;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("block", "manipulate blocks")
		.subcommand(cmd_create())
		.subcommand(cmd_decode())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("create", Some(m)) => exec_create(m),
		("decode", Some(m)) => exec_decode(m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_create<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("create", "create a raw block from JSON").args(&[
		cmd::arg("block-info", "the block info in JSON").required(false),
		cmd::opt("raw-stdout", "output the raw bytes of the result to stdout")
			.short("r")
			.required(false),
	])
}

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let info = serde_json::from_str::<BlockInfo>(&cmd::arg_or_stdin(matches, "block-info"))
		.unwrap_or_else(|e| panic!("invalid json JSON input: {}", e));

	if info.txids.is_some() {
		warn!("Field \"txids\" is ignored.");
	}

	let block =
		hal_simplicity::actions::block::block_create(info).unwrap_or_else(|e| panic!("{}", e));

	let block_bytes = serialize(&block);
	if matches.is_present("raw-stdout") {
		::std::io::stdout().write_all(&block_bytes).unwrap();
	} else {
		print!("{}", hex::encode(&block_bytes));
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw block to JSON").args(&cmd::opts_networks()).args(&[
		cmd::opt_yaml(),
		cmd::arg("raw-block", "the raw block in hex").required(false),
		cmd::opt("txids", "provide transactions IDs instead of full transactions"),
	])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>) {
	let hex_block = cmd::arg_or_stdin(matches, "raw-block");
	let network = cmd::network(matches);
	let txids_only = matches.is_present("txids");

	let info =
		hal_simplicity::actions::block::block_decode(hex_block.as_ref(), network, txids_only)
			.unwrap_or_else(|e| panic!("{}", e));

	cmd::print_output(matches, &info)
}
