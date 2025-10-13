// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod info;

use crate::cmd;

use serde::Serialize;

#[derive(Serialize)]
struct Error {
	error: String,
}

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("simplicity", "manipulate Simplicity programs")
		.subcommand(self::info::cmd())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("info", Some(m)) => self::info::exec(m),
		(_, _) => unreachable!("clap prints help"),
	};
}
