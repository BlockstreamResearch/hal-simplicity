// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod update_input;

use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("pset", "manipulate PSETs for spending from Simplicity programs")
		.subcommand(self::update_input::cmd())
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("update-input", Some(m)) => self::update_input::exec(m),
		(_, _) => unreachable!("clap prints help"),
	};
}
