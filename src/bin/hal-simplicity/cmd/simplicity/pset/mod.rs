// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod extract;
mod finalize;
mod update_input;

use crate::cmd;
use serde::Serialize;

#[derive(Serialize)]
struct UpdatedPset {
	pset: String,
	updated_values: Vec<&'static str>,
}

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("pset", "manipulate PSETs for spending from Simplicity programs")
		.subcommand(self::extract::cmd())
		.subcommand(self::finalize::cmd())
		.subcommand(self::update_input::cmd())
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("extract", Some(m)) => self::extract::exec(m),
		("finalize", Some(m)) => self::finalize::exec(m),
		("update-input", Some(m)) => self::update_input::exec(m),
		(_, _) => unreachable!("clap prints help"),
	};
}
