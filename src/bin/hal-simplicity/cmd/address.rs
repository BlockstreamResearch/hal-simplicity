use clap;
use elements::bitcoin::{secp256k1, PublicKey};
use elements::hashes::Hash;
use elements::{Address, WPubkeyHash, WScriptHash};

use crate::cmd;

use crate::Network;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("address", "work with addresses")
		.subcommand(cmd_create())
		.subcommand(cmd_inspect())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("create", Some(m)) => exec_create(m),
		("inspect", Some(m)) => exec_inspect(m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_create<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("create", "create addresses").args(&cmd::opts_networks()).args(&[
		cmd::opt_yaml(),
		cmd::opt("pubkey", "a public key in hex").takes_value(true).required(false),
		cmd::opt("script", "a script in hex").takes_value(true).required(false),
		cmd::opt("blinder", "a blinding pubkey in hex").takes_value(true).required(false),
	])
}

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let network = cmd::network(matches);

	let blinder = matches.value_of("blinder").map(|b| {
		let bytes = hex::decode(b).expect("invaid blinder hex");
		secp256k1::PublicKey::from_slice(&bytes).expect("invalid blinder")
	});

	let created = if let Some(pubkey_hex) = matches.value_of("pubkey") {
		let pubkey: PublicKey = pubkey_hex.parse().expect("invalid pubkey");
		hal_simplicity::address::Addresses::from_pubkey(&pubkey, blinder, network)
	} else if let Some(script_hex) = matches.value_of("script") {
		let script_bytes = hex::decode(script_hex).expect("invalid script hex");
		let script = script_bytes.into();

		hal_simplicity::address::Addresses::from_script(&script, blinder, network)
	} else {
		panic!("Can't create addresses without a pubkey");
	};

	cmd::print_output(matches, &created)
}

fn cmd_inspect<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("inspect", "inspect addresses")
		.args(&[cmd::opt_yaml(), cmd::arg("address", "the address").required(true)])
}

fn exec_inspect<'a>(matches: &clap::ArgMatches<'a>) {
	let address_str = matches.value_of("address").expect("no address provided");
	let address: Address = address_str.parse().expect("invalid address format");
	let script_pk = address.script_pubkey();

	let mut info = hal_simplicity::address::AddressInfo {
		network: Network::from_params(address.params).expect("addresses always have params"),
		script_pub_key: hal::tx::OutputScriptInfo {
			hex: Some(script_pk.to_bytes().into()),
			asm: Some(script_pk.asm()),
			address: None,
			type_: None,
		},
		type_: None,
		pubkey_hash: None,
		script_hash: None,
		witness_pubkey_hash: None,
		witness_script_hash: None,
		witness_program_version: None,
		blinding_pubkey: address.blinding_pubkey,
		unconfidential: if address.blinding_pubkey.is_some() {
			Some(Address {
				params: address.params,
				payload: address.payload.clone(),
				blinding_pubkey: None,
			})
		} else {
			None
		},
	};

	use elements::address::Payload;
	match address.payload {
		Payload::PubkeyHash(pkh) => {
			info.type_ = Some("p2pkh".to_owned());
			info.pubkey_hash = Some(pkh);
		}
		Payload::ScriptHash(sh) => {
			info.type_ = Some("p2sh".to_owned());
			info.script_hash = Some(sh);
		}
		Payload::WitnessProgram {
			version,
			program,
		} => {
			let version = version.to_u8() as usize;
			info.witness_program_version = Some(version);

			if version == 0 {
				if program.len() == 20 {
					info.type_ = Some("p2wpkh".to_owned());
					info.witness_pubkey_hash =
						Some(WPubkeyHash::from_slice(&program).expect("size 20"));
				} else if program.len() == 32 {
					info.type_ = Some("p2wsh".to_owned());
					info.witness_script_hash =
						Some(WScriptHash::from_slice(&program).expect("size 32"));
				} else {
					info.type_ = Some("invalid-witness-program".to_owned());
				}
			} else {
				info.type_ = Some("unknown-witness-program-version".to_owned());
			}
		}
	}

	cmd::print_output(matches, &info)
}
