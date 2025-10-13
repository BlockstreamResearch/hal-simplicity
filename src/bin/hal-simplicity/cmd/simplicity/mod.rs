// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod info;
mod sighash;

use crate::cmd;

use serde::Serialize;

#[derive(Serialize)]
struct Error {
	context: &'static str,
	error: String,
}

trait ErrorExt<T> {
	fn result_context(self, context: &'static str) -> Result<T, Error>;
}

impl<T, E: core::fmt::Display> ErrorExt<T> for Result<T, E> {
	fn result_context(self, context: &'static str) -> Result<T, Error> {
		self.map_err(|e| Error {
			context,
			error: e.to_string(),
		})
	}
}

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("simplicity", "manipulate Simplicity programs")
		.subcommand(self::info::cmd())
		.subcommand(self::sighash::cmd())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("info", Some(m)) => self::info::exec(m),
		("sighash", Some(m)) => self::sighash::exec(m),
		(_, _) => unreachable!("clap prints help"),
	};
}
