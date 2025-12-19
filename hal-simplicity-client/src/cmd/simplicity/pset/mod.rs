// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod create;
mod extract;
mod finalize;
mod run;
mod update_input;

use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("pset", "manipulate PSETs for spending from Simplicity programs")
		.subcommand(self::create::cmd())
		.subcommand(self::extract::cmd())
		.subcommand(self::finalize::cmd())
		.subcommand(self::run::cmd())
		.subcommand(self::update_input::cmd())
}

pub fn exec(
	matches: &clap::ArgMatches<'_>,
	client: &hal_simplicity::hal_simplicity_client::HalSimplicity,
) {
	match matches.subcommand() {
		("create", Some(m)) => self::create::exec(m, client),
		("extract", Some(m)) => self::extract::exec(m, client),
		("finalize", Some(m)) => self::finalize::exec(m, client),
		("run", Some(m)) => self::run::exec(m, client),
		("update-input", Some(m)) => self::update_input::exec(m, client),
		(_, _) => unreachable!("clap prints help"),
	};
}
