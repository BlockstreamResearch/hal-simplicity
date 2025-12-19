use std::io::Write;

use crate::cmd;
use crate::Network;
use hal_simplicity::hal_simplicity_client::HalSimplicity;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("tx", "manipulate transactions")
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
	cmd::subcommand("create", "create a raw transaction from JSON").args(&[
		cmd::arg("tx-info", "the transaction info in JSON").required(false),
		cmd::opt("raw-stdout", "output the raw bytes of the result to stdout")
			.short("r")
			.required(false),
	])
}

fn exec_create<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	let tx_info = cmd::arg_or_stdin(matches, "tx-info").to_string();
	let raw_stdout = matches.is_present("raw-stdout");

	let result = client.tx_create(tx_info, Some(raw_stdout)).expect("failed to create transaction");

	if raw_stdout {
		if let Some(raw_hex) = result.as_str() {
			let raw_bytes = hex::decode(raw_hex).expect("invalid hex in response");
			::std::io::stdout().write_all(&raw_bytes).unwrap();
		}
	} else {
		cmd::print_output(matches, &result);
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw transaction to JSON")
		.args(&cmd::opts_networks())
		.args(&[cmd::opt_yaml(), cmd::arg("raw-tx", "the raw transaction in hex").required(false)])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	let hex_tx = cmd::arg_or_stdin(matches, "raw-tx").to_string();
	let network = cmd::network(matches);

	let network_str = match network {
		Network::ElementsRegtest => Some("elementsregtest".to_string()),
		Network::Liquid => Some("liquid".to_string()),
		Network::LiquidTestnet => Some("liquidtestnet".to_string()),
	};

	let result = client.tx_decode(hex_tx, network_str).expect("failed to decode transaction");

	cmd::print_output(matches, &result);
}
