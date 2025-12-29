use std::io::Write;

use clap;
use elements::encode::serialize;

use crate::cmd;
use hal_simplicity::tx::TransactionInfo;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("tx", "manipulate transactions")
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
	cmd::subcommand("create", "create a raw transaction from JSON").args(&[
		cmd::arg("tx-info", "the transaction info in JSON").required(false),
		cmd::opt("raw-stdout", "output the raw bytes of the result to stdout")
			.short("r")
			.required(false),
	])
}

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let info = serde_json::from_str::<TransactionInfo>(&cmd::arg_or_stdin(matches, "tx-info"))
		.unwrap_or_else(|e| panic!("invalid JSON provided: {}", e));

	let tx = hal_simplicity::actions::tx::tx_create(info).unwrap_or_else(|e| panic!("{}", e));

	let tx_bytes = serialize(&tx);
	if matches.is_present("raw-stdout") {
		::std::io::stdout().write_all(&tx_bytes).unwrap();
	} else {
		print!("{}", hex::encode(&tx_bytes));
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw transaction to JSON")
		.args(&cmd::opts_networks())
		.args(&[cmd::opt_yaml(), cmd::arg("raw-tx", "the raw transaction in hex").required(false)])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>) {
	let hex_tx = cmd::arg_or_stdin(matches, "raw-tx");
	let network = cmd::network(matches);

	let info = hal_simplicity::actions::tx::tx_decode(hex_tx.as_ref(), network)
		.unwrap_or_else(|e| panic!("{}", e));

	cmd::print_output(matches, &info)
}
