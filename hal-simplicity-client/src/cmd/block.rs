use std::io::Write;

use crate::cmd;
use hal_simplicity::hal_simplicity_client::HalSimplicity;

use hal_simplicity_daemon::Network;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("block", "manipulate blocks")
		.subcommand(cmd_create())
		.subcommand(cmd_decode())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	match matches.subcommand() {
		("create", Some(m)) => exec_create(m, client),
		("decode", Some(m)) => exec_decode(m, client),
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

fn exec_create<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	let block_info = cmd::arg_or_stdin(matches, "block-info").to_string();
	let raw_stdout = matches.is_present("raw-stdout");

	let result = client.block_create(block_info, Some(raw_stdout)).expect("failed to create block");

	if raw_stdout {
		let raw_bytes = hex::decode(&result.raw_block).expect("invalid hex in response");
		::std::io::stdout().write_all(&raw_bytes).unwrap();
	} else {
		cmd::print_output(matches, &result);
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw block to JSON").args(&cmd::opts_networks()).args(&[
		cmd::opt_yaml(),
		cmd::arg("raw-block", "the raw block in hex").required(false),
		cmd::opt("txids", "provide transactions IDs instead of full transactions"),
	])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	let hex_block = cmd::arg_or_stdin(matches, "raw-block").to_string();
	let network = cmd::network(matches);

	let network_str = match network {
		Network::ElementsRegtest => Some("elementsregtest".to_string()),
		Network::Liquid => Some("liquid".to_string()),
		Network::LiquidTestnet => Some("liquidtestnet".to_string()),
	};

	let txids = matches.is_present("txids");

	let result =
		client.block_decode(hex_block, network_str, Some(txids)).expect("failed to decode block");

	cmd::print_output(matches, &result);
}
