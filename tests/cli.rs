#![cfg(test)]

use std::process::Command;

use elements::hashes::hex::DisplayHex;

fn self_command_str() -> &'static str {
	env!("CARGO_BIN_EXE_hal-simplicity")
}

fn self_command() -> Command {
	use std::path::Path;
	Command::new(Path::new(self_command_str()))
}

/// Asserts that the stderr of a command is empty, and that its stdout can be parsed by the
/// given [`deserialize_fn`].
///
/// Typical values of `deserialize_fn` are `serde_json::from_slice` and `serde_yaml::from_slice`.
#[track_caller]
fn assert_deserialize_cmd<T, E>(args: &[&str], deserialize_fn: fn(&[u8]) -> Result<T, E>) -> T
where
	T: for<'de> serde::de::Deserialize<'de>,
	E: core::fmt::Display,
{
	let args_string = || {
		let v =
			args.iter().map(|s| s.replace("\\", "\\\\").replace("\"", "\\\"")).collect::<Vec<_>>();
		v.join(" ")
	};

	let output = self_command().args(args.iter()).output().unwrap();
	if !output.stderr.is_empty() {
		eprintln!("Command: {} {}", self_command_str(), args_string());
		eprintln!(
			"Stderr:\n-----\n{}\n-----\n(stderr should have been empty.)",
			String::from_utf8_lossy(&output.stderr),
		);
	}

	match deserialize_fn(&output.stdout) {
		Ok(decode) => decode,
		Err(e) => {
			eprintln!("Stdout:\n-----\n{}\n-----", String::from_utf8_lossy(&output.stdout),);
			panic!("Attempted to parse stdout, but got error: {}", e);
		}
	}
}

#[track_caller]
fn assert_cmd(args: &[&str], expected_stdout: impl AsRef<str>, expected_stderr: impl AsRef<str>) {
	let expected_stdout = expected_stdout.as_ref();
	let expected_stderr = expected_stderr.as_ref();

	let args_string = || {
		let v =
			args.iter().map(|s| s.replace("\\", "\\\\").replace("\"", "\\\"")).collect::<Vec<_>>();
		v.join(" ")
	};

	let output = self_command().args(args.iter()).output().unwrap();
	let stdout = String::from_utf8(output.stdout).expect("stdout valid utf-8");
	let stderr = String::from_utf8(output.stderr).expect("stdout valid utf-8");
	if stdout != expected_stdout {
		eprintln!("Command: {} {}", self_command_str(), args_string());
		eprintln!(
			"Stdout:\n-----\n{}\n-----\nExpected stdout:\n-----\n{}\n-----",
			stdout, expected_stdout
		);
		panic!("stdout mismatch");
	}
	if stderr != expected_stderr {
		eprintln!("Command: {} {}", self_command_str(), args_string());
		eprintln!(
			"Stderr:\n-----\n{}\n-----\nExpected stderr:\n-----\n{}\n-----",
			stderr, expected_stderr
		);
		panic!("stderr mismatch");
	}
}

#[test]
fn cli_help() {
	let expected_help = "\
hal-simplicity 0.1.0
hal-simplicity -- a Simplicity-enabled fork of hal

USAGE:
    hal-simplicity [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    print verbose logging output to stderr

SUBCOMMANDS:
    address       work with addresses
    block         manipulate blocks
    help          Prints this message or the help of the given subcommand(s)
    keypair       manipulate private and public keys
    simplicity    manipulate Simplicity programs
    tx            manipulate transactions
";
	assert_cmd(&[], "", expected_help); // note on stdout, not stderr
	assert_cmd(&["help"], expected_help, "");
	assert_cmd(&["--help"], expected_help, "");
	assert_cmd(&["-h"], expected_help, "");
}

#[test]
fn cli_bad_flag() {
	assert_cmd(
		&["-?"],
		"",
		"\
error: Found argument '-?' which wasn't expected, or isn't valid in this context

USAGE:
    hal-simplicity [FLAGS] <SUBCOMMAND>

For more information try --help
",
	);
}

#[test]
fn cli_address() {
	let expected_help = "\
hal-simplicity-address 0.1.0
work with addresses

USAGE:
    hal-simplicity address [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr

SUBCOMMANDS:
    create     create addresses
    inspect    inspect addresses
";
	assert_cmd(&["address"], "", expected_help);
	assert_cmd(&["address", "-h"], expected_help, "");
	assert_cmd(&["address", "--help"], expected_help, "");
	assert_cmd(&["address", "--help", "xyz"], expected_help, "");
}

#[test]
fn cli_address_create() {
	let expected_help = "\
hal-simplicity-address-create 0.1.0
create addresses

USAGE:
    hal-simplicity address create [FLAGS] [OPTIONS]

FLAGS:
    -r, --elementsregtest    run in elementsregtest mode
    -h, --help               Prints help information
        --liquid             run in liquid mode
    -v, --verbose            print verbose logging output to stderr
    -y, --yaml               print output in YAML instead of JSON

OPTIONS:
        --blinder <blinder>    a blinding pubkey in hex
        --pubkey <pubkey>      a public key in hex
        --script <script>      a script in hex
";
	// newline not escaped v
	// FIXME yes, you can, with a script rather than pubkey. Also the script is not
	// length-prefixed, which is a little surprising and should be documented
	assert_cmd(
		&["address", "create"],
		"Execution failed: Can't create addresses without a pubkey\n",
		"",
	);
	assert_cmd(&["address", "create", "-h"], expected_help, "");
	assert_cmd(&["address", "create", "--help"], expected_help, "");
	assert_cmd(&["address", "create", "--help", "xyz"], expected_help, "");
	// Bad public key
	assert_cmd(
		&["address", "create", ""],
		"",
		"\
error: Found argument '' which wasn't expected, or isn't valid in this context

USAGE:
    hal-simplicity address create [FLAGS] [OPTIONS]

For more information try --help
",
	);
	// FIXME stdout instead of stderr
	assert_cmd(
		&["address", "create", "--pubkey", ""],
		"Execution failed: invalid pubkey: InvalidHexLength(0)\n",
		"",
	);
	// x-only keys not supported
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"abababababababababababababababababababababababababababababababab",
		],
		"Execution failed: invalid pubkey: InvalidHexLength(64)\n",
		"",
	);
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"020000000000000000000000000000000000000000000000000000000000000000",
		],
		"Execution failed: invalid pubkey: Encoding(Secp256k1(InvalidPublicKey))\n",
		"",
	);
	// uncompressed keys ok (though FIXME we should not produce p2wpkh or p2shwpkh addresses which are unspendable!!)
	assert_cmd(
		&["address", "create", "--pubkey", "0400000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c633f3979bf72ae8202983dc989aec7f2ff2ed91bdd69ce02fc0700ca100e59ddf3"],
		r#"{
  "p2pkh": "2dfGL9NZh5ZHpQjJNiwu6pDe3R6du5GCNgY",
  "p2wpkh": "ert1qgqyvtapw3hp7p9anwf580rz4z0p4v9dy203prh",
  "p2shwpkh": "XQgqPjiN7DgRqPv66V8YLJ3a6u4RYeFAhH"
}"#,
		"",
	);
	// hybrid keys are not
	assert_cmd(
		&["address", "create", "--pubkey", "0700000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c633f3979bf72ae8202983dc989aec7f2ff2ed91bdd69ce02fc0700ca100e59ddf3"],
		"Execution failed: invalid pubkey: Encoding(InvalidKeyPrefix(7))\n",
		"",
	);
	// compressed keys are ok, and the output is NOT the same as for uncompressed keys
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
		],
		r#"{
  "p2pkh": "2dcJQ2ctSXJirCQH3BEwqCDaVUBtoVCf2Pg",
  "p2wpkh": "ert1qr7z8s0phhs4v4v968cmhu2jcemkyllt0hcpm6d",
  "p2shwpkh": "XUBf77ZpEZsLLMGfVeRxpGcWGuMuS72DcY"
}"#,
		"",
	);

	// Valid blinder, no pubkey
	assert_cmd(
		&[
			"address",
			"create",
			"--blinder",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
		],
		"Execution failed: Can't create addresses without a pubkey\n",
		"",
	);
	// Invalid blinders all get the same generic message, and we don't even check for a pubkey
	assert_cmd(
		&["address", "create", "--blinder", ""],
		"Execution failed: invalid blinder: InvalidPublicKey\n",
		"",
	);
	assert_cmd(
		&["address", "create", "--blinder", "02abcd"],
		"Execution failed: invalid blinder: InvalidPublicKey\n",
		"",
	);
	assert_cmd(
		&[
			"address",
			"create",
			"--blinder",
			"abababababababababababababababababababababababababababababababab",
		],
		"Execution failed: invalid blinder: InvalidPublicKey\n",
		"",
	);
	assert_cmd(
		&[
			"address",
			"create",
			"--blinder",
			"020000000000000000000000000000000000000000000000000000000000000000",
		],
		"Execution failed: invalid blinder: InvalidPublicKey\n",
		"",
	);
	// good pubkey, blinder
	let good_key_output = r#"{
  "p2pkh": "CTErcmNXWAsDa1cYJT5uvKzn41nwDiYVjEYRfJdKa3P4657XGZtVWenzawNtFGiYs4oXKtGiou9XoH49",
  "p2wpkh": "el1qqvqqqqqqqqqqqqqqqqqrk7xw2clcng8djs20t23g45xed4net7wxx8uy0q7r00p2e2ct503h0c493nhvfl7k7sa2ka87ya3j6",
  "p2shwpkh": "AzpquMY1JJesARTG3nBzUpP9Bhpj8vFAoygZFf6R9Su9BDyLDS4SRZ1NCsHDZrAjVXdwh6ULKnKj5P27"
}"#;
	assert_cmd(
		&[
			"-v", // -v can go anywhere, and does nothing
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--blinder",
			"0300000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
		],
		good_key_output,
		"",
	);
	// FIXME we accept hybrid and uncompressed keys for blinders, which is probably wrong. But
	//  observe that they all produce the same address, since internally they're just converted
	//  to compressed keys.
	assert_cmd(
		&[
			"address", "create",
			"--pubkey", "0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--blinder", "0400000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c633f3979bf72ae8202983dc989aec7f2ff2ed91bdd69ce02fc0700ca100e59ddf3"
		],
		good_key_output,
		"",
	);
	assert_cmd(
		&[
			"address", "create",
			"--pubkey", "0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--blinder", "0700000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c633f3979bf72ae8202983dc989aec7f2ff2ed91bdd69ce02fc0700ca100e59ddf3"
		],
		good_key_output,
		"",
	);
	// FIXME if you provide a script as well as a pubkey then the script is ignored
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--blinder",
			"0300000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--script",
			"abcd",
		],
		good_key_output,
		"",
	);
	// Empty script is OK, even though it's unspendable. Same with various invalid/unparseable scripts.
	assert_cmd(
		&["address", "create", "--script", ""],
		r#"{
  "p2sh": "XToMocNywBYNSiXUe5xvoa2naAps9Ek1hq",
  "p2wsh": "ert1quwcvgs5clswpfxhm7nyfjmaeysn6us0yvjdexn9yjkv3k7zjhp2szaqlpq",
  "p2shwsh": "XLJnepfKgZPGu95CJFxBnjF9TGi6urS48V"
}"#,
		"",
	);
	// Verbose does nothing
	assert_cmd(
		&["address", "create", "-v", "--script", ""],
		r#"{
  "p2sh": "XToMocNywBYNSiXUe5xvoa2naAps9Ek1hq",
  "p2wsh": "ert1quwcvgs5clswpfxhm7nyfjmaeysn6us0yvjdexn9yjkv3k7zjhp2szaqlpq",
  "p2shwsh": "XLJnepfKgZPGu95CJFxBnjF9TGi6urS48V"
}"#,
		"",
	);
	assert_cmd(
		&[
			"address",
			"create",
			"--blinder",
			"0300000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--script",
			"",
		],
		r#"{
  "p2sh": "AzpquMY1JJesARTG3nBzUpP9Bhpj8vFAoygZFf6R9Su9BDyKq8kwEihysuPapfKB2VdF7Nmbnk3B54Uu",
  "p2wsh": "el1qqvqqqqqqqqqqqqqqqqqrk7xw2clcng8djs20t23g45xed4net7wx8casc3pf3lquzjd0haxgn9hmjfp84eq7geymjdx2f9verdu99wz4h4jgt4lr3g6h",
  "p2shwsh": "AzpquMY1JJesARTG3nBzUpP9Bhpj8vFAoygZFf6R9Su9BDyCLZc9X4TMior1NNyM1kcQKjehfyn1zfFA"
}"#,
		"",
	);
	// This script is invalid (is a 64-byte push followed by nothing) but still can be parsed.
	assert_cmd(
		&["address", "create", "--script", "40"],
		r#"{
  "p2sh": "XKLW7rD7tEnddSzwsHfg8rZa3a8wLTuEts",
  "p2wsh": "ert1qcdjplp2y6lqz7dvqkp7qlxy87rr2yll44vw5503fetce0n7znxhqtj2wee",
  "p2shwsh": "XVvMsyprXnwTCUxQZ4RQpkQYg5bBqYSofS"
}"#,
		"",
	);
	// Check that all three things are allowed only once
	assert_cmd(
		&[
			"address", "create",
			"--pubkey", "40",
			"--pubkey", "20"
		],
		"",
		"\
error: The argument '--pubkey <pubkey>' was provided more than once, but cannot be used multiple times

USAGE:
    hal-simplicity address create --pubkey <pubkey>

For more information try --help
",
	);
	assert_cmd(
		&[
			"address", "create",
			"--blinder", "40",
			"--blinder", "20"
		],
		"",
		"\
error: The argument '--blinder <blinder>' was provided more than once, but cannot be used multiple times

USAGE:
    hal-simplicity address create --blinder <blinder>

For more information try --help
",
	);
	assert_cmd(
		&[
			"address", "create",
			"--script", "40",
			"--script", "20"
		],
		"",
		"\
error: The argument '--script <script>' was provided more than once, but cannot be used multiple times

USAGE:
    hal-simplicity address create --script <script>

For more information try --help
",
	);

	// Test --yaml flag changes output format
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--yaml",
		],
		"---\np2pkh: 2dcJQ2ctSXJirCQH3BEwqCDaVUBtoVCf2Pg\np2wpkh: ert1qr7z8s0phhs4v4v968cmhu2jcemkyllt0hcpm6d\np2shwpkh: XUBf77ZpEZsLLMGfVeRxpGcWGuMuS72DcY",
		"",
	);

	// Test -y flag (short form of --yaml)
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"-y",
		],
		"---\np2pkh: 2dcJQ2ctSXJirCQH3BEwqCDaVUBtoVCf2Pg\np2wpkh: ert1qr7z8s0phhs4v4v968cmhu2jcemkyllt0hcpm6d\np2shwpkh: XUBf77ZpEZsLLMGfVeRxpGcWGuMuS72DcY",
		"",
	);

	// Test --liquid flag changes address format
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--liquid",
		],
		r#"{
  "p2pkh": "Pz92mHqA9CEtdFTcpZf6su8TSQ2tysQMCb",
  "p2wpkh": "ex1qr7z8s0phhs4v4v968cmhu2jcemkyllt0d2tr9h",
  "p2shwpkh": "Gz1wfCqSg5BntkFYcYSVMkpBck5wu6ZcEK"
}"#,
		"",
	);

	// Test --elementsregtest flag (should be same as default)
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--elementsregtest",
		],
		r#"{
  "p2pkh": "2dcJQ2ctSXJirCQH3BEwqCDaVUBtoVCf2Pg",
  "p2wpkh": "ert1qr7z8s0phhs4v4v968cmhu2jcemkyllt0hcpm6d",
  "p2shwpkh": "XUBf77ZpEZsLLMGfVeRxpGcWGuMuS72DcY"
}"#,
		"",
	);

	// Test -r flag (short form of --elementsregtest)
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"-r",
		],
		r#"{
  "p2pkh": "2dcJQ2ctSXJirCQH3BEwqCDaVUBtoVCf2Pg",
  "p2wpkh": "ert1qr7z8s0phhs4v4v968cmhu2jcemkyllt0hcpm6d",
  "p2shwpkh": "XUBf77ZpEZsLLMGfVeRxpGcWGuMuS72DcY"
}"#,
		"",
	);

	// Test combining flags: --yaml with --liquid
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--liquid",
			"--yaml",
		],
		"---\np2pkh: Pz92mHqA9CEtdFTcpZf6su8TSQ2tysQMCb\np2wpkh: ex1qr7z8s0phhs4v4v968cmhu2jcemkyllt0d2tr9h\np2shwpkh: Gz1wfCqSg5BntkFYcYSVMkpBck5wu6ZcEK",
		"",
	);

	// Test combining flags: -y with --liquid (short form)
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--liquid",
			"-y",
		],
		"---\np2pkh: Pz92mHqA9CEtdFTcpZf6su8TSQ2tysQMCb\np2wpkh: ex1qr7z8s0phhs4v4v968cmhu2jcemkyllt0d2tr9h\np2shwpkh: Gz1wfCqSg5BntkFYcYSVMkpBck5wu6ZcEK",
		"",
	);

	// Test combining flags: -r with -y (both short forms)
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"-r",
			"-y",
		],
		"---\np2pkh: 2dcJQ2ctSXJirCQH3BEwqCDaVUBtoVCf2Pg\np2wpkh: ert1qr7z8s0phhs4v4v968cmhu2jcemkyllt0hcpm6d\np2shwpkh: XUBf77ZpEZsLLMGfVeRxpGcWGuMuS72DcY",
		"",
	);

	// Test with blinder and different network flags
	assert_cmd(
		&[
			"address",
			"create",
			"--pubkey",
			"0200000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--blinder",
			"0300000000000000000000003b78ce563f89a0ed9414f5aa28ad0d96d6795f9c63",
			"--liquid",
		],
		r#"{
  "p2pkh": "VTpzxkqVGbraaCz18fRVd7EtpG4FBoAFDAbGgBR8mzP2cUVwPWcTBKe75cwYH2rYjYoKFog3Hs1nVKPN",
  "p2wpkh": "lq1qqvqqqqqqqqqqqqqqqqqrk7xw2clcng8djs20t23g45xed4net7wxx8uy0q7r00p2e2ct503h0c493nhvfl7k7m4297fq56rwq",
  "p2shwpkh": "VJLCUu2hpcjPaTGMnAQ18s2uk3tJBFrM4Gtrt27tBKhdz5aJgkgQQjEFyot72jzycwzSPckzCXXVzwG5"
}"#,
		"",
	);
}

// TODO address inspect

#[test]
fn cli_address_inspect() {
	let expected_help = "\
hal-simplicity-address-inspect 0.1.0
inspect addresses

USAGE:
    hal-simplicity address inspect [FLAGS] <address>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr
    -y, --yaml       print output in YAML instead of JSON

ARGS:
    <address>    the address
";
	// newline not escaped v
	// FIXME yes, you can, with a script rather than pubkey. Also the script is not
	// length-prefixed, which is a little surprising and should be documented
	assert_cmd(
		&["address", "inspect"],
		"",
		"error: The following required arguments were not provided:
    <address>

USAGE:
    hal-simplicity address inspect [FLAGS] <address>

For more information try --help
",
	);
	assert_cmd(&["address", "inspect", "-h"], expected_help, "");
	assert_cmd(&["address", "inspect", "--help"], expected_help, "");
	assert_cmd(&["address", "inspect", "--help", "xyz"], expected_help, "");

	// FIXME stdout instead of stderr
	assert_cmd(
		&["address", "inspect", ""],
		"Execution failed: invalid address format: Base58(TooShort(TooShortError { length: 0 }))\n",
		"",
	);
	// FIXME this error is absolutely terrible
	assert_cmd(
		&["address", "inspect", "bc1q7z3dshje7e4tftag5c3w7e85pr00r6cq34khh8"],
		"Execution failed: invalid address format: Base58(Decode(InvalidCharacterError { invalid: 48 }))\n",
		"",
	);
	// FIXME this one is possibly even worse
	assert_cmd(
		&["address", "inspect", "1Au8w4fejHaJBbrZCMrfg6v2hwJNr3go1N"],
		"Execution failed: invalid address format: InvalidAddress(\"1Au8w4fejHaJBbrZCMrfg6v2hwJNr3go1N\")\n",
		"",
	);
	// liquid addresses ok
	assert_cmd(
		&["address", "inspect", "ex1q7z3dshje7e4tftag5c3w7e85pr00r6cqmut068"],
		r#"{
  "network": "liquid",
  "type": "p2wpkh",
  "script_pub_key": {
    "hex": "0014f0a2d85e59f66ab4afa8a622ef64f408def1eb00",
    "asm": "OP_0 OP_PUSHBYTES_20 f0a2d85e59f66ab4afa8a622ef64f408def1eb00"
  },
  "witness_program_version": 0,
  "witness_pubkey_hash": "f0a2d85e59f66ab4afa8a622ef64f408def1eb00"
}"#,
		"",
	);
	assert_cmd(
		&["address", "inspect", "ert1q7z3dshje7e4tftag5c3w7e85pr00r6cqpwph9a"],
		r#"{
  "network": "elementsregtest",
  "type": "p2wpkh",
  "script_pub_key": {
    "hex": "0014f0a2d85e59f66ab4afa8a622ef64f408def1eb00",
    "asm": "OP_0 OP_PUSHBYTES_20 f0a2d85e59f66ab4afa8a622ef64f408def1eb00"
  },
  "witness_program_version": 0,
  "witness_pubkey_hash": "f0a2d85e59f66ab4afa8a622ef64f408def1eb00"
}"#,
		"",
	);
	assert_cmd(
		&["address", "inspect", "Q7AX4Ff5CZzEoJoVbGqqKFRsagz9Q3bS1v"],
		r#"{
  "network": "liquid",
  "type": "p2pkh",
  "script_pub_key": {
    "hex": "76a9146c95622b280be97792ec1b3505700f9e674cf50988ac",
    "asm": "OP_DUP OP_HASH160 OP_PUSHBYTES_20 6c95622b280be97792ec1b3505700f9e674cf509 OP_EQUALVERIFY OP_CHECKSIG"
  },
  "pubkey_hash": "6c95622b280be97792ec1b3505700f9e674cf509"
}"#,
		"",
	);
	assert_cmd(
		&["address", "inspect", "2djKtKaiMagUCNTcuwx8ZdZsucUr3tt4WQu"],
		r#"{
  "network": "elementsregtest",
  "type": "p2pkh",
  "script_pub_key": {
    "hex": "76a9146c95622b280be97792ec1b3505700f9e674cf50988ac",
    "asm": "OP_DUP OP_HASH160 OP_PUSHBYTES_20 6c95622b280be97792ec1b3505700f9e674cf509 OP_EQUALVERIFY OP_CHECKSIG"
  },
  "pubkey_hash": "6c95622b280be97792ec1b3505700f9e674cf509"
}"#,
		"",
	);
	// -v does nothing
	assert_cmd(
		&["-v", "address", "inspect", "2djKtKaiMagUCNTcuwx8ZdZsucUr3tt4WQu"],
		r#"{
  "network": "elementsregtest",
  "type": "p2pkh",
  "script_pub_key": {
    "hex": "76a9146c95622b280be97792ec1b3505700f9e674cf50988ac",
    "asm": "OP_DUP OP_HASH160 OP_PUSHBYTES_20 6c95622b280be97792ec1b3505700f9e674cf509 OP_EQUALVERIFY OP_CHECKSIG"
  },
  "pubkey_hash": "6c95622b280be97792ec1b3505700f9e674cf509"
}"#,
		"",
	);
	// -y outputs yaml
	assert_cmd(
		&["address", "inspect", "-y", "2djKtKaiMagUCNTcuwx8ZdZsucUr3tt4WQu"],
		r#"---
network: elementsregtest
type: p2pkh
script_pub_key:
  hex: 76a9146c95622b280be97792ec1b3505700f9e674cf50988ac
  asm: OP_DUP OP_HASH160 OP_PUSHBYTES_20 6c95622b280be97792ec1b3505700f9e674cf509 OP_EQUALVERIFY OP_CHECKSIG
pubkey_hash: 6c95622b280be97792ec1b3505700f9e674cf509"#,
		"",
	);
	assert_cmd(
		&["address", "inspect", "2djKtKaiMagUCNTcuwx8ZdZsucUr3tt4WQu", ""],
		"",
		"\
error: Found argument '' which wasn't expected, or isn't valid in this context

USAGE:
    hal-simplicity address inspect [FLAGS] <address>

For more information try --help
",
	);
}

#[test]
fn cli_block() {
	let expected_help = "\
hal-simplicity-block 0.1.0
manipulate blocks

USAGE:
    hal-simplicity block [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr

SUBCOMMANDS:
    create    create a raw block from JSON
    decode    decode a raw block to JSON
";
	assert_cmd(&["block"], "", expected_help);
	assert_cmd(&["block", "-h"], expected_help, "");
	assert_cmd(&["block", "--help"], expected_help, "");
	assert_cmd(&["block", "--help", "xyz"], expected_help, "");
}

#[test]
fn cli_block_create() {
	let expected_help = "\
hal-simplicity-block-create 0.1.0
create a raw block from JSON

USAGE:
    hal-simplicity block create [FLAGS] [block-info]

FLAGS:
    -h, --help          Prints help information
    -r, --raw-stdout    output the raw bytes of the result to stdout
    -v, --verbose       print verbose logging output to stderr

ARGS:
    <block-info>    the block info in JSON
";
	// FIXME stdout not stderr
	assert_cmd(&["block", "create"], "Execution failed: no 'block-info' argument given\n", "");
	assert_cmd(&["block", "create", "-h"], expected_help, "");
	assert_cmd(&["block", "create", "--help"], expected_help, "");
	assert_cmd(&["block", "create", "--help", "xyz"], expected_help, "");

	// TODO this was as far as I got trying to find a valid input
	assert_cmd(&["block", "create", ""], "Execution failed: invaid json JSON input: Error(\"EOF while parsing a value\", line: 1, column: 0)\n", "");
	assert_cmd(&["block", "create", "{}"], "Execution failed: invaid json JSON input: Error(\"missing field `header`\", line: 1, column: 2)\n", "");
	assert_cmd(
		&[
			"block",
			"create",
			r#"{
			"header": {
			    "version": 1,
			    "previous_block_hash": "046cf11845388f39eeb83b73dee09c25e9db08a19b3ab2612c80c5f20d605084",
			    "merkle_root": "046cf11845388f39eeb83b73dee09c25e9db08a19b3ab2612c80c5f20d605084",
			    "dynafed": false,
			    "time": 100,
				"height": 10
			}
		 }"#,
		],
		"Execution failed: missing challenge\n",
		"",
	);
	assert_cmd(&["block", "create", "{}"], "Execution failed: invaid json JSON input: Error(\"missing field `header`\", line: 1, column: 2)\n", "");
	// FIXME this error is awful; the actual field it wants is called `dynafed_current`
	assert_cmd(
		&[
			"block",
			"create",
			r#"{
			"header": {
			    "version": 1,
			    "previous_block_hash": "046cf11845388f39eeb83b73dee09c25e9db08a19b3ab2612c80c5f20d605084",
			    "merkle_root": "046cf11845388f39eeb83b73dee09c25e9db08a19b3ab2612c80c5f20d605084",
			    "dynafed": true,
			    "time": 100,
				"height": 10
			}
		 }"#,
		],
		"Execution failed: missing current params\n",
		"",
	);

	let header_json = r#"{
		"header": {
		    "version": 1,
		    "previous_block_hash": "046cf11845388f39eeb83b73dee09c25e9db08a19b3ab2612c80c5f20d605084",
		    "merkle_root": "046cf11845388f39eeb83b73dee09c25e9db08a19b3ab2612c80c5f20d605084",
		    "dynafed": true,
		    "time": 100,
			"height": 10,
		  "dynafed_current": {
		    "params_type": "compact",
		    "signblockscript": "0020e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c",
		    "signblock_witness_limit": 1416,
		    "elided_root": "ff0f60e85234ad045ac9a8f174b41ac9e3461ad2f6b05d0fccbd964eed5d757e"
		  },
		  "dynafed_proposed": {
		    "params_type": "null",
		    "signblockscript": null,
		    "signblock_witness_limit": null
		  },
		  "dynafed_witness": []
		}
		%TRANSACTIONS%
	}"#;
	// FIXME this error is pretty bad. Incosistent format and also no indication of how to specify transactions.
	//  Note that `decode` on a valid block does not show transactions. In fact, there are two possibilities:
	//  the `transactions` array which takes a poorly specified json array and the `raw_transactions` array
	//  which takes hex. Also you are not allowed to provide both. Also you can provide an empty array, which
	//  will satisfy the "no transactions provided" error.
	//
	// Also, as always, these errors show up on stdout instead of stderr..
	assert_cmd(
		&["block", "create", &header_json.replace("%TRANSACTIONS%", "")],
		"Execution failed: No transactions provided.\n",
		"",
	);
	assert_cmd(
		&[
			"block",
			"create",
			&header_json.replace("%TRANSACTIONS%", ", \"transactions\": []"),
		],
		"010000808450600df2c5802c61b23a9ba108dbe9259ce0de733bb8ee398f384518f16c048450600df2c5802c61b23a9ba108dbe9259ce0de733bb8ee398f384518f16c04640000000a00000001220020e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c880500007e755ded4e96bdcc0f5db0f6d21a46e3c91ab474f1a8c95a04ad3452e8600fff000000",
		"",
	);
	assert_cmd(
		&[
			"block",
			"create",
			&header_json.replace("%TRANSACTIONS%", ", \"raw_transactions\": []"),
		],
		"010000808450600df2c5802c61b23a9ba108dbe9259ce0de733bb8ee398f384518f16c048450600df2c5802c61b23a9ba108dbe9259ce0de733bb8ee398f384518f16c04640000000a00000001220020e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c880500007e755ded4e96bdcc0f5db0f6d21a46e3c91ab474f1a8c95a04ad3452e8600fff000000",
		"",
	);
	assert_cmd(
		&[
			"block",
			"create",
			&header_json
				.replace("%TRANSACTIONS%", ", \"transactions\": [], \"raw_transactions\": []"),
		],
		"Execution failed: Can't provide transactions both in JSON and raw.\n",
		"",
	);

	// To test -r we can't use `assert_cmd` since it assumes that stdout
	// is valid utf-8, which a raw block will not be.
	let args = &[
		"block",
		"create",
		"-r",
		&header_json.replace("%TRANSACTIONS%", ", \"raw_transactions\": []"),
	];
	let output = self_command().args(args.iter()).output().unwrap();
	assert_eq!(output.stdout.as_hex().to_string(),
		"010000808450600df2c5802c61b23a9ba108dbe9259ce0de733bb8ee398f384518f16c048450600df2c5802c61b23a9ba108dbe9259ce0de733bb8ee398f384518f16c04640000000a00000001220020e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c880500007e755ded4e96bdcc0f5db0f6d21a46e3c91ab474f1a8c95a04ad3452e8600fff000000"
			);
	assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn cli_block_decode() {
	let expected_help = "\
hal-simplicity-block-decode 0.1.0
decode a raw block to JSON

USAGE:
    hal-simplicity block decode [FLAGS] [raw-block]

FLAGS:
    -r, --elementsregtest    run in elementsregtest mode
    -h, --help               Prints help information
        --liquid             run in liquid mode
        --txids              provide transactions IDs instead of full transactions
    -v, --verbose            print verbose logging output to stderr
    -y, --yaml               print output in YAML instead of JSON

ARGS:
    <raw-block>    the raw block in hex
";
	// FIXME stdout not stderr
	assert_cmd(&["block", "decode"], "Execution failed: no 'raw-block' argument given\n", "");
	assert_cmd(&["block", "decode", "-h"], expected_help, "");
	assert_cmd(&["block", "decode", "--help"], expected_help, "");
	assert_cmd(&["block", "decode", "--help", "xyz"], expected_help, "");

	// FIXME this error message is awful, and it's on stdout
	assert_cmd(&["block", "decode", ""], "Execution failed: invalid block format: Io(Error { kind: UnexpectedEof, message: \"failed to fill whole buffer\" })\n", "");
	// This is a hex-encoded block header, not a full block
	assert_cmd(&["block", "decode", BLOCK_HEADER_1585319], HEADER_DECODE_1585319, "");
	// This is the same hex-encoded block header, with --txids. FIXME this is awful.
	assert_cmd(&["block", "decode", "--txids", BLOCK_HEADER_1585319],
		"Execution failed: invalid block format: Io(Error { kind: UnexpectedEof, message: \"failed to fill whole buffer\" })\n",
"");
	// Here is the header plus some arbitrary junk
	assert_cmd(&["block", "decode", &(BLOCK_HEADER_1585319.to_owned() + "0000")],
		"Execution failed: invalid block format: ParseFailed(\"data not consumed entirely when explicitly deserializing\")\n",
"");
	// Here is the whole block.
	assert_cmd(&["block", "decode", FULL_BLOCK_1585319], HEADER_DECODE_1585319, "");
	assert_cmd(&["block", "decode", "--liquid", FULL_BLOCK_1585319], HEADER_DECODE_1585319, "");
	assert_cmd(
		&["block", "decode", "--elementsregtest", FULL_BLOCK_1585319],
		HEADER_DECODE_1585319,
		"",
	);
	assert_cmd(&["block", "decode", "-r", FULL_BLOCK_1585319], HEADER_DECODE_1585319, "");
	// FIXME you can pass -r and --liquid at the same time, but these are incompatible. (Though they appear
	//  to do nothing so maybe this is fine..)
	assert_cmd(
		&["block", "decode", "-r", "--liquid", FULL_BLOCK_1585319],
		HEADER_DECODE_1585319,
		"",
	);
	// Here is the whole block. FIXME if you provide --txids it gives you the txids, but if you don't, it gives you nothing
	assert_cmd(
		&["block", "decode", "--txids", FULL_BLOCK_1585319],
		format!(
			r#"{{
  "header": {},
  "txids": [
    "9523d75b48b3411a3f4ebd31b6005898deebbe748875aa6ee084b94aa8422ba6",
    "ae9d4031fbbb21950837012fe1dbbf53501cca0cf0796e7b53bc7a38c91c463c"
  ]
}}"#,
			HEADER_DECODE_1585319.replace("\n  ", "\n    ").replace("\n}", "\n  }")
		),
		"",
	);
}

#[test]
fn cli_keypair() {
	let expected_help = "\
hal-simplicity-keypair 0.1.0
manipulate private and public keys

USAGE:
    hal-simplicity keypair [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr

SUBCOMMANDS:
    generate    generate a random private/public keypair
";
	assert_cmd(&["keypair"], "", expected_help);
	// -h does NOT mean --help. It is just ignored entirely.
	//assert_cmd(&["keypair", "-h"], expected_help, "");
	assert_cmd(&["keypair", "--help"], expected_help, "");
	assert_cmd(&["keypair", "--help", "xyz"], expected_help, "");
}

#[test]
fn cli_keypair_generate() {
	let expected_help = "\
hal-simplicity-keypair-generate 0.1.0
generate a random private/public keypair

USAGE:
    hal-simplicity keypair generate [FLAGS]

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr
    -y, --yaml       print output in YAML instead of JSON
";
	assert_cmd(&["keypair", "generate", "-h"], expected_help, "");
	assert_cmd(&["keypair", "generate", "--help"], expected_help, "");
	assert_cmd(&["keypair", "generate", "--help", "xyz"], expected_help, "");

	// New block to avoid warnings about `struct`s being defined not at the beginning of block
	{
		use elements::bitcoin::secp256k1;

		#[allow(dead_code)]
		#[derive(serde::Deserialize)]
		struct Object {
			secret: secp256k1::SecretKey,
			x_only: secp256k1::XOnlyPublicKey,
			parity: usize, // secp256k1::Parity does not seem to round-trip through serde_json
		}

		// Closure needed for borrowck reasons
		assert_deserialize_cmd(&["keypair", "generate"], |s| serde_json::from_slice::<Object>(s));
		assert_deserialize_cmd(&["keypair", "generate"], serde_yaml::from_slice::<Object>);
	}
}

#[test]
fn cli_simplicity() {
	let expected_help = "\
hal-simplicity-simplicity 0.1.0
manipulate Simplicity programs

USAGE:
    hal-simplicity simplicity [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr

SUBCOMMANDS:
    info       Parse a base64-encoded Simplicity program and decode it
    sighash    Compute signature hashes or signatures for use with Simplicity
";
	assert_cmd(&["simplicity"], "", expected_help);
	assert_cmd(&["simplicity", "-h"], expected_help, "");
	assert_cmd(&["simplicity", "--help"], expected_help, "");
	assert_cmd(&["simplicity", "--help", "xyz"], expected_help, "");
}

#[test]
fn cli_simplicity_info() {
	let expected_help = "\
hal-simplicity-simplicity-info 0.1.0
Parse a base64-encoded Simplicity program and decode it

USAGE:
    hal-simplicity simplicity info [FLAGS] <program> [witness]

FLAGS:
    -r, --elementsregtest    run in elementsregtest mode
    -h, --help               Prints help information
        --liquid             run in liquid mode
    -v, --verbose            print verbose logging output to stderr
    -y, --yaml               print output in YAML instead of JSON

ARGS:
    <program>    a Simplicity program in base64
    <witness>    a hex encoding of all the witness data for the program
";
	// For the transaction/block create / decode functions we can take input by
	// stdin as an undocumented JSON blob. FIXME we probably want to do this
	// here (and in the other simplicity commands) to allow for very large
	// programs and witnesses. But I'd rather do it properly (i.e. with some
	// docs and help) so not gonna do it now.
	assert_cmd(
		&["simplicity", "info"],
		"",
		"\
error: The following required arguments were not provided:
    <program>

USAGE:
    hal-simplicity simplicity info [FLAGS] <program> [witness]

For more information try --help
",
	);
	assert_cmd(&["simplicity", "info", "-h"], expected_help, "");
	assert_cmd(&["simplicity", "info", "--help"], expected_help, "");
	assert_cmd(&["simplicity", "info", "--help", "xyz"], expected_help, "");
}

#[test]
fn cli_tx() {
	let expected_help = "\
hal-simplicity-tx 0.1.0
manipulate transactions

USAGE:
    hal-simplicity tx [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    print verbose logging output to stderr

SUBCOMMANDS:
    create    create a raw transaction from JSON
    decode    decode a raw transaction to JSON
";
	assert_cmd(&["tx"], "", expected_help);
	assert_cmd(&["tx", "-h"], expected_help, "");
	assert_cmd(&["tx", "--help"], expected_help, "");
	assert_cmd(&["tx", "--help", "xyz"], expected_help, "");
}

#[test]
fn cli_tx_create() {
	let expected_help = "\
hal-simplicity-tx-create 0.1.0
create a raw transaction from JSON

USAGE:
    hal-simplicity tx create [FLAGS] [tx-info]

FLAGS:
    -h, --help          Prints help information
    -r, --raw-stdout    output the raw bytes of the result to stdout
    -v, --verbose       print verbose logging output to stderr

ARGS:
    <tx-info>    the transaction info in JSON
";
	assert_cmd(&["tx", "create"], "Execution failed: no 'tx-info' argument given\n", "");
	assert_cmd(&["tx", "create", "-h"], expected_help, "");
	assert_cmd(&["tx", "create", "--help"], expected_help, "");
	assert_cmd(&["tx", "create", "--help", "xyz"], expected_help, "");

	assert_cmd(&["tx", "create", ""], "Execution failed: invalid JSON provided: Error(\"EOF while parsing a value\", line: 1, column: 0)\n", "");
	assert_cmd(&["tx", "create", "{ }"], "Execution failed: Field \"version\" is required.\n", "");
	// FIXME I have no idea what is wrong here. But putting a test in to track fixing
	//  whatever is causing this nonsense error.
	assert_cmd(
		&["tx", "create", "{ \"version\": 10, \"locktime\": 10 }"],
		"Execution failed: invalid JSON provided: Error(\"expected value\", line: 1, column: 30)\n",
		"",
	);
	// FIXME: lol, replace this locktime format with something sane
	assert_cmd(
		&["tx", "create", "{ \"version\": 10, \"locktime\": { \"Blocks\": 10 }, \"inputs\": [], \"outputs\": [] }"],
		"0a0000000000000a000000",
		"",
	);
	// -v does nothing
	assert_cmd(
		&["tx", "create", "-v", "{ \"version\": 10, \"locktime\": { \"Blocks\": 10 }, \"inputs\": [], \"outputs\": [] }"],
		"0a0000000000000a000000",
		"",
	);

	// To test -r we can't use `assert_cmd` since it assumes that stdout
	// is valid utf-8, which a raw block will not be.
	let args = &[
		"tx",
		"create",
		"-r",
		"{ \"version\": 10, \"locktime\": { \"Blocks\": 10 }, \"inputs\": [], \"outputs\": [] }",
	];
	let output = self_command().args(args.iter()).output().unwrap();
	assert_eq!(output.stdout.as_hex().to_string(), "0a0000000000000a000000",);
	assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn cli_tx_decode() {
	let expected_help = "\
hal-simplicity-tx-decode 0.1.0
decode a raw transaction to JSON

USAGE:
    hal-simplicity tx decode [FLAGS] [raw-tx]

FLAGS:
    -r, --elementsregtest    run in elementsregtest mode
    -h, --help               Prints help information
        --liquid             run in liquid mode
    -v, --verbose            print verbose logging output to stderr
    -y, --yaml               print output in YAML instead of JSON

ARGS:
    <raw-tx>    the raw transaction in hex
";
	assert_cmd(&["tx", "decode"], "Execution failed: no 'raw-tx' argument given\n", "");
	assert_cmd(&["tx", "decode", "-h"], expected_help, "");
	assert_cmd(&["tx", "decode", "--help"], expected_help, "");
	assert_cmd(&["tx", "decode", "--help", "xyz"], expected_help, "");

	assert_cmd(&["tx", "decode", ""], "Execution failed: invalid tx format: Io(Error { kind: UnexpectedEof, message: \"failed to fill whole buffer\" })\n", "");
	// A bitcoin transaction
	assert_cmd(&["tx", "decode", "02000000000101cd5d8addc8ed0d91d9338a1e524a87185b8bb3c1760e0a19c4ad576b217fd7ca0100000000fdffffff02f50100000000000016001468647ece9c25ab162c72dbedfe7de63db1913e39e50d00000000000016001413aac2fc1cef3dacc656bfe8fe342a03a5feac6302473044022059e6f5ccc1d89bf31a3847a464cce1fcf0e56e43633787d03ebb2ebc1899e28c02207f3f05a16a87f07fe82bfa35c509e7d969243c6215080a6775877bef113c9e7b012103b303769299ca63c9076fc8f91d6e27152a81fc884f9fe95f47fd2a262c987256b7c50d00"], "Execution failed: invalid tx format: NonMinimalVarInt\n", "");
	// A Liquid transaction
	let tx_decode = r#"{
  "txid": "9523d75b48b3411a3f4ebd31b6005898deebbe748875aa6ee084b94aa8422ba6",
  "wtxid": "c1107130eaa29002ceac7c7fc9a93cd46a15a030a8f21ad579a4a06a3deff008",
  "hash": "c1107130eaa29002ceac7c7fc9a93cd46a15a030a8f21ad579a4a06a3deff008",
  "size": 334,
  "weight": 1207,
  "vsize": 301,
  "version": 2,
  "locktime": {
    "Blocks": 0
  },
  "inputs": [
    {
      "prevout": "0000000000000000000000000000000000000000000000000000000000000000:4294967295",
      "txid": "0000000000000000000000000000000000000000000000000000000000000000",
      "vout": 4294967295,
      "script_sig": {
        "hex": "03a730180101",
        "asm": "OP_PUSHBYTES_3 a73018 OP_PUSHBYTES_1 01"
      },
      "sequence": 4294967295,
      "is_pegin": false,
      "has_issuance": false,
      "witness": {
        "amount_rangeproof": null,
        "inflation_keys_rangeproof": null,
        "script_witness": [
          "0000000000000000000000000000000000000000000000000000000000000000"
        ]
      }
    }
  ],
  "outputs": [
    {
      "script_pub_key": {
        "hex": "6a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000",
        "asm": "OP_RETURN OP_PUSHBYTES_36 0a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000",
        "type": "opreturn"
      },
      "asset": {
        "type": "explicit",
        "asset": "6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d",
        "label": "liquid_bitcoin"
      },
      "value": {
        "type": "explicit",
        "value": 0
      },
      "nonce": {
        "type": "null"
      },
      "witness": {
        "surjection_proof": null,
        "rangeproof": null
      },
      "is_fee": false
    },
    {
      "script_pub_key": {
        "hex": "76a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac",
        "asm": "OP_DUP OP_HASH160 OP_PUSHBYTES_20 fc26751a5025129a2fd006c6fbfa598ddd67f7e1 OP_EQUALVERIFY OP_CHECKSIG",
        "type": "p2pkh",
        "address": "2dxQzjvrkmRGSa5gwgaQn1oLtRo5pXS94oJ"
      },
      "asset": {
        "type": "explicit",
        "asset": "6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d",
        "label": "liquid_bitcoin"
      },
      "value": {
        "type": "explicit",
        "value": 262
      },
      "nonce": {
        "type": "null"
      },
      "witness": {
        "surjection_proof": null,
        "rangeproof": null
      },
      "is_fee": false
    },
    {
      "script_pub_key": {
        "hex": "6a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab3",
        "asm": "OP_RETURN OP_PUSHBYTES_36 aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab3",
        "type": "opreturn"
      },
      "asset": {
        "type": "explicit",
        "asset": "6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d",
        "label": "liquid_bitcoin"
      },
      "value": {
        "type": "explicit",
        "value": 0
      },
      "nonce": {
        "type": "null"
      },
      "witness": {
        "surjection_proof": null,
        "rangeproof": null
      },
      "is_fee": false
    }
  ]
}"#;
	assert_cmd(&["tx", "decode", "0200000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffffff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000"],
		tx_decode,
		"");
	assert_cmd(&["tx", "decode", "-r", "0200000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffffff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000"],
		tx_decode,
		"");
	// -v works but seems to do nothing
	assert_cmd(&["tx", "decode", "-v", "0200000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffffff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000"],
		tx_decode,
		"");
	assert_cmd(&["tx", "decode", "--liquid", "0200000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffffff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000"],
		tx_decode.replace("2dxQzjvrkmRGSa5gwgaQn1oLtRo5pXS94oJ", "QLFdUboUPJnUzvsXKu83hUtrQ1DuxyggRg"),
		"");
	// FIXME both -r and --liquid are allowed, and it seems that -r wins. Should error out instead.
	assert_cmd(&["tx", "decode", "-r", "--liquid", "0200000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffffff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000"],
		tx_decode,
		"");
	// -v works but seems to do nothing
	assert_cmd(&["tx", "decode", "-y", "0200000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffffff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000000120000000000000000000000000000000000000000000000000000000000000000000000000000000"],
		r#"---
txid: 9523d75b48b3411a3f4ebd31b6005898deebbe748875aa6ee084b94aa8422ba6
wtxid: c1107130eaa29002ceac7c7fc9a93cd46a15a030a8f21ad579a4a06a3deff008
hash: c1107130eaa29002ceac7c7fc9a93cd46a15a030a8f21ad579a4a06a3deff008
size: 334
weight: 1207
vsize: 301
version: 2
locktime:
  Blocks: 0
inputs:
  - prevout: "0000000000000000000000000000000000000000000000000000000000000000:4294967295"
    txid: "0000000000000000000000000000000000000000000000000000000000000000"
    vout: 4294967295
    script_sig:
      hex: 03a730180101
      asm: OP_PUSHBYTES_3 a73018 OP_PUSHBYTES_1 01
    sequence: 4294967295
    is_pegin: false
    has_issuance: false
    witness:
      amount_rangeproof: ~
      inflation_keys_rangeproof: ~
      script_witness:
        - "0000000000000000000000000000000000000000000000000000000000000000"
outputs:
  - script_pub_key:
      hex: 6a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000
      asm: OP_RETURN OP_PUSHBYTES_36 0a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e27905000000000000000000
      type: opreturn
    asset:
      type: explicit
      asset: 6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d
      label: liquid_bitcoin
    value:
      type: explicit
      value: 0
    nonce:
      type: "null"
    witness:
      surjection_proof: ~
      rangeproof: ~
    is_fee: false
  - script_pub_key:
      hex: 76a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac
      asm: OP_DUP OP_HASH160 OP_PUSHBYTES_20 fc26751a5025129a2fd006c6fbfa598ddd67f7e1 OP_EQUALVERIFY OP_CHECKSIG
      type: p2pkh
      address: 2dxQzjvrkmRGSa5gwgaQn1oLtRo5pXS94oJ
    asset:
      type: explicit
      asset: 6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d
      label: liquid_bitcoin
    value:
      type: explicit
      value: 262
    nonce:
      type: "null"
    witness:
      surjection_proof: ~
      rangeproof: ~
    is_fee: false
  - script_pub_key:
      hex: 6a24aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab3
      asm: OP_RETURN OP_PUSHBYTES_36 aa21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab3
      type: opreturn
    asset:
      type: explicit
      asset: 6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d
      label: liquid_bitcoin
    value:
      type: explicit
      value: 0
    nonce:
      type: "null"
    witness:
      surjection_proof: ~
      rangeproof: ~
    is_fee: false"#,
		"");
}

// Stick some big constants down here
static BLOCK_HEADER_1585319: &str = concat!(
	"000000a0176409e0a34e5bde1640a618a8910ce27af4157140f7531e8fde47ddcdaf65338ce0c95a",
	"86c8cf32ca810bdb15d0333e1b5cb67981b284f558f7c61207442f2494229c61a730180001220020",
	"e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c880500007e755ded",
	"4e96bdcc0f5db0f6d21a46e3c91ab474f1a8c95a04ad3452e8600fff000d00483045022100c44868",
	"fef7440e0a826d46dd53114d9d5c37163fe04fbceb5fc92abf0032475f02200d148c282a5285eb26",
	"b72d1b20f53b333e72fe94218e85544bd381bf06105a5901483045022100f8506df43d1daf76f331",
	"1426bb736b67b0f3180a9cef697ea3d4e908fe99823c022006782ef8308bf9e1d79d1535e4fbc23e",
	"cd1cd2517968372e99e2bb47c2e11dda01473044022043c69b9f466f7f21eec9e537481fc3dd2d45",
	"7d49b452d15eb41d349c7762ad37022071b817ca37414dfebe7cde1c45b270aedc63ea001886521a",
	"201b45c0ecbc7fc301483045022100b1bf654ae2e1df62e94ebf0556ee4c41c75e129cdbeeccab91",
	"44aa1e2748307d022075c9811300107ab5b61c0b8f0c8740c6da2561f2ff70a974157d995f0bd04f",
	"da01483045022100d3a10b1d49775fb34006ca482510e5284950994a028cea45ad7d251c5af3c87b",
	"02205ea89e4a3bdffa3cd8802c0048a8375074fcb042883319c542fe6ef09bda37e7014830450221",
	"00defd7e485760479e5f7bca3fd1dcbb0b7239f2675d234e6d03645a9092587f1002202dc6f316ee",
	"ef700729347a1e37d9edeb80554cf65ae8e5161c54342407a789b201483045022100f5ab571aed3f",
	"e613a88a70373bac3e9d32f33a2ad911516d5181dc748de9df9702202780bdfde630dc66f4358ef8",
	"9d7893396a74b7e33badd2b3041484b36b39534901473044022002835ed51d51ea57074cf2b30472",
	"b07d8819e61ee496c2377882ac973ce128e002206e7944db89d08150226e3513f4bfa4d59a6388fc",
	"7eeff7fee3ebf5dd296d56c201483045022100ca4756437d2dfe8b56cee02da12183eb8f451bb27f",
	"7c886852d6e106d667f95202203a29ea3dafd725d496cc6508ba62de42d9b7ff3fafcb528b0a6a3a",
	"2a13ecfd11014730440220212d552bc35aac010dd546467cf0d15fe3f2b3349ba6e554d10cadd2b3",
	"7d975802201ede6c1f518056dd843bf7338f6b3d31f4811d9590db3a4c2679311ea6f9bf1a014830",
	"45022100fb4aee60b6157f7942e720e893e39676c6bd97e5bca37e1248ce6133a6b2b65302200de5",
	"611208eb3c12f713b2eee904f7d70a19f74491bbe4fcf11210d7c1c46b9c01fd01025b21026a2a10",
	"6ec32c8a1e8052e5d02a7b0a150423dbd9b116fc48d46630ff6e6a05b92102791646a8b49c274035",
	"2b4495c118d876347bf47d0551c01c4332fdc2df526f1a2102888bda53a424466b0451627df22090",
	"143bbf7c060e9eacb1e38426f6b07f2ae12102aee8967150dee220f613de3b239320355a49880808",
	"4a93eaf39a34dcd62024852102d46e9259d0a0bb2bcbc461a3e68f34adca27b8d08fbe985853992b",
	"4b104e27412102e9944e35e5750ab621e098145b8e6cf373c273b7c04747d1aa020be0af40ccd621",
	"02f9a9d4b10a6d6c56d8c955c547330c589bb45e774551d46d415e51cd9ad5116321033b421566c1",
	"24dfde4db9defe4084b7aa4e7f36744758d92806b8f72c2e943309210353dcc6b4cf6ad28aceb7f7",
	"b2db92a4bf07ac42d357adf756f3eca790664314b621037f55980af0455e4fb55aad9b85a55068bb",
	"6dc4740ea87276dc693f4598db45fa210384001daa88dabd23db878dbb1ce5b4c2a5fa72c3113e35",
	"14bf602325d0c37b8e21039056d089f2fe72dbc0a14780b4635b0dc8a1b40b7a59106325dd1bc45c",
	"c70493210397ab8ea7b0bf85bc7fc56bb27bf85e75502e94e76a6781c409f3f2ec3d1122192103b0",
	"0e3b5b77884bf3cae204c4b4eac003601da75f96982ffcb3dcb29c5ee419b92103c1f3c0874cfe34",
	"b8131af34699589aacec4093399739ae352e8a46f80a6f68375fae"
);

static HEADER_DECODE_1585319: &str = r#"{
  "block_hash": "5f37039a5ae15d9239bb2e137643a51d3a525d6e850b5e8974b4323c9e13a39b",
  "version": 536870912,
  "previous_block_hash": "3365afcddd47de8f1e53f7407115f47ae20c91a818a64016de5b4ea3e0096417",
  "merkle_root": "242f440712c6f758f584b28179b65c1b3e33d015db0b81ca32cfc8865ac9e08c",
  "time": 1637622420,
  "height": 1585319,
  "dynafed": true,
  "dynafed_current": {
    "params_type": "compact",
    "signblockscript": "0020e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c",
    "signblock_witness_limit": 1416,
    "elided_root": "ff0f60e85234ad045ac9a8f174b41ac9e3461ad2f6b05d0fccbd964eed5d757e"
  },
  "dynafed_proposed": {
    "params_type": "null",
    "signblockscript": null,
    "signblock_witness_limit": null
  },
  "dynafed_witness": [
    "",
    "3045022100c44868fef7440e0a826d46dd53114d9d5c37163fe04fbceb5fc92abf0032475f02200d148c282a5285eb26b72d1b20f53b333e72fe94218e85544bd381bf06105a5901",
    "3045022100f8506df43d1daf76f3311426bb736b67b0f3180a9cef697ea3d4e908fe99823c022006782ef8308bf9e1d79d1535e4fbc23ecd1cd2517968372e99e2bb47c2e11dda01",
    "3044022043c69b9f466f7f21eec9e537481fc3dd2d457d49b452d15eb41d349c7762ad37022071b817ca37414dfebe7cde1c45b270aedc63ea001886521a201b45c0ecbc7fc301",
    "3045022100b1bf654ae2e1df62e94ebf0556ee4c41c75e129cdbeeccab9144aa1e2748307d022075c9811300107ab5b61c0b8f0c8740c6da2561f2ff70a974157d995f0bd04fda01",
    "3045022100d3a10b1d49775fb34006ca482510e5284950994a028cea45ad7d251c5af3c87b02205ea89e4a3bdffa3cd8802c0048a8375074fcb042883319c542fe6ef09bda37e701",
    "3045022100defd7e485760479e5f7bca3fd1dcbb0b7239f2675d234e6d03645a9092587f1002202dc6f316eeef700729347a1e37d9edeb80554cf65ae8e5161c54342407a789b201",
    "3045022100f5ab571aed3fe613a88a70373bac3e9d32f33a2ad911516d5181dc748de9df9702202780bdfde630dc66f4358ef89d7893396a74b7e33badd2b3041484b36b39534901",
    "3044022002835ed51d51ea57074cf2b30472b07d8819e61ee496c2377882ac973ce128e002206e7944db89d08150226e3513f4bfa4d59a6388fc7eeff7fee3ebf5dd296d56c201",
    "3045022100ca4756437d2dfe8b56cee02da12183eb8f451bb27f7c886852d6e106d667f95202203a29ea3dafd725d496cc6508ba62de42d9b7ff3fafcb528b0a6a3a2a13ecfd1101",
    "30440220212d552bc35aac010dd546467cf0d15fe3f2b3349ba6e554d10cadd2b37d975802201ede6c1f518056dd843bf7338f6b3d31f4811d9590db3a4c2679311ea6f9bf1a01",
    "3045022100fb4aee60b6157f7942e720e893e39676c6bd97e5bca37e1248ce6133a6b2b65302200de5611208eb3c12f713b2eee904f7d70a19f74491bbe4fcf11210d7c1c46b9c01",
    "5b21026a2a106ec32c8a1e8052e5d02a7b0a150423dbd9b116fc48d46630ff6e6a05b92102791646a8b49c2740352b4495c118d876347bf47d0551c01c4332fdc2df526f1a2102888bda53a424466b0451627df22090143bbf7c060e9eacb1e38426f6b07f2ae12102aee8967150dee220f613de3b239320355a498808084a93eaf39a34dcd62024852102d46e9259d0a0bb2bcbc461a3e68f34adca27b8d08fbe985853992b4b104e27412102e9944e35e5750ab621e098145b8e6cf373c273b7c04747d1aa020be0af40ccd62102f9a9d4b10a6d6c56d8c955c547330c589bb45e774551d46d415e51cd9ad5116321033b421566c124dfde4db9defe4084b7aa4e7f36744758d92806b8f72c2e943309210353dcc6b4cf6ad28aceb7f7b2db92a4bf07ac42d357adf756f3eca790664314b621037f55980af0455e4fb55aad9b85a55068bb6dc4740ea87276dc693f4598db45fa210384001daa88dabd23db878dbb1ce5b4c2a5fa72c3113e3514bf602325d0c37b8e21039056d089f2fe72dbc0a14780b4635b0dc8a1b40b7a59106325dd1bc45cc70493210397ab8ea7b0bf85bc7fc56bb27bf85e75502e94e76a6781c409f3f2ec3d1122192103b00e3b5b77884bf3cae204c4b4eac003601da75f96982ffcb3dcb29c5ee419b92103c1f3c0874cfe34b8131af34699589aacec4093399739ae352e8a46f80a6f68375fae"
  ]
}"#;

static FULL_BLOCK_1585319: &str = concat!(
	"000000a0176409e0a34e5bde1640a618a8910ce27af4157140f7531e8fde47ddcdaf65338ce0c95a",
	"86c8cf32ca810bdb15d0333e1b5cb67981b284f558f7c61207442f2494229c61a730180001220020",
	"e51211e91d9cf4aec3bdc370a0303acde5d24baedb12235fdd2786885069d91c880500007e755ded",
	"4e96bdcc0f5db0f6d21a46e3c91ab474f1a8c95a04ad3452e8600fff000d00483045022100c44868",
	"fef7440e0a826d46dd53114d9d5c37163fe04fbceb5fc92abf0032475f02200d148c282a5285eb26",
	"b72d1b20f53b333e72fe94218e85544bd381bf06105a5901483045022100f8506df43d1daf76f331",
	"1426bb736b67b0f3180a9cef697ea3d4e908fe99823c022006782ef8308bf9e1d79d1535e4fbc23e",
	"cd1cd2517968372e99e2bb47c2e11dda01473044022043c69b9f466f7f21eec9e537481fc3dd2d45",
	"7d49b452d15eb41d349c7762ad37022071b817ca37414dfebe7cde1c45b270aedc63ea001886521a",
	"201b45c0ecbc7fc301483045022100b1bf654ae2e1df62e94ebf0556ee4c41c75e129cdbeeccab91",
	"44aa1e2748307d022075c9811300107ab5b61c0b8f0c8740c6da2561f2ff70a974157d995f0bd04f",
	"da01483045022100d3a10b1d49775fb34006ca482510e5284950994a028cea45ad7d251c5af3c87b",
	"02205ea89e4a3bdffa3cd8802c0048a8375074fcb042883319c542fe6ef09bda37e7014830450221",
	"00defd7e485760479e5f7bca3fd1dcbb0b7239f2675d234e6d03645a9092587f1002202dc6f316ee",
	"ef700729347a1e37d9edeb80554cf65ae8e5161c54342407a789b201483045022100f5ab571aed3f",
	"e613a88a70373bac3e9d32f33a2ad911516d5181dc748de9df9702202780bdfde630dc66f4358ef8",
	"9d7893396a74b7e33badd2b3041484b36b39534901473044022002835ed51d51ea57074cf2b30472",
	"b07d8819e61ee496c2377882ac973ce128e002206e7944db89d08150226e3513f4bfa4d59a6388fc",
	"7eeff7fee3ebf5dd296d56c201483045022100ca4756437d2dfe8b56cee02da12183eb8f451bb27f",
	"7c886852d6e106d667f95202203a29ea3dafd725d496cc6508ba62de42d9b7ff3fafcb528b0a6a3a",
	"2a13ecfd11014730440220212d552bc35aac010dd546467cf0d15fe3f2b3349ba6e554d10cadd2b3",
	"7d975802201ede6c1f518056dd843bf7338f6b3d31f4811d9590db3a4c2679311ea6f9bf1a014830",
	"45022100fb4aee60b6157f7942e720e893e39676c6bd97e5bca37e1248ce6133a6b2b65302200de5",
	"611208eb3c12f713b2eee904f7d70a19f74491bbe4fcf11210d7c1c46b9c01fd01025b21026a2a10",
	"6ec32c8a1e8052e5d02a7b0a150423dbd9b116fc48d46630ff6e6a05b92102791646a8b49c274035",
	"2b4495c118d876347bf47d0551c01c4332fdc2df526f1a2102888bda53a424466b0451627df22090",
	"143bbf7c060e9eacb1e38426f6b07f2ae12102aee8967150dee220f613de3b239320355a49880808",
	"4a93eaf39a34dcd62024852102d46e9259d0a0bb2bcbc461a3e68f34adca27b8d08fbe985853992b",
	"4b104e27412102e9944e35e5750ab621e098145b8e6cf373c273b7c04747d1aa020be0af40ccd621",
	"02f9a9d4b10a6d6c56d8c955c547330c589bb45e774551d46d415e51cd9ad5116321033b421566c1",
	"24dfde4db9defe4084b7aa4e7f36744758d92806b8f72c2e943309210353dcc6b4cf6ad28aceb7f7",
	"b2db92a4bf07ac42d357adf756f3eca790664314b621037f55980af0455e4fb55aad9b85a55068bb",
	"6dc4740ea87276dc693f4598db45fa210384001daa88dabd23db878dbb1ce5b4c2a5fa72c3113e35",
	"14bf602325d0c37b8e21039056d089f2fe72dbc0a14780b4635b0dc8a1b40b7a59106325dd1bc45c",
	"c70493210397ab8ea7b0bf85bc7fc56bb27bf85e75502e94e76a6781c409f3f2ec3d1122192103b0",
	"0e3b5b77884bf3cae204c4b4eac003601da75f96982ffcb3dcb29c5ee419b92103c1f3c0874cfe34",
	"b8131af34699589aacec4093399739ae352e8a46f80a6f68375fae02020000000101000000000000",
	"0000000000000000000000000000000000000000000000000000ffffffff0603a730180101ffffff",
	"ff03016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f0100000000",
	"0000000000266a240a8ce26fdbb51a2d03d4e62fdafd4a06dd7faa0d1c083aa7e279050000000000",
	"00000000016d521c38ec1ea15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f010000",
	"000000000106001976a914fc26751a5025129a2fd006c6fbfa598ddd67f7e188ac016d521c38ec1e",
	"a15734ae22b7c46064412829c0d0579f0a713d1c04ede979026f01000000000000000000266a24aa",
	"21a9ede8497768bc893ee587244bf5303ac3cf482bab8e4b3fd22e8b114c2a52525ab30000000000",
	"00012000000000000000000000000000000000000000000000000000000000000000000000000000",
	"00000200000001027fb3be5d23fa6969d3635fc4f9b0b4010d61dfe46f38044f731475cb0b90e01d",
	"0000000017160014508f86975a0adca90da0b16cd2a88edb8a9afa8bfeffffffc10afe1da8fc0016",
	"4ada5a987fd60dc7993c1494ee37ebb3e171e26adae0f5dd000000001716001490cd10e6e8f89e1f",
	"ddc4576a681acb5070e8562ffeffffff030ae372253ede7a2f25c59019dccd4140ac6c99f00bf988",
	"a5c9157779e73cc6d22b085f8f4ed6205bc0d9bc8dc0f2073650303c1ccd5bf8a37b48dd1f097984",
	"f6a50b03bb3fa7ffb337705d32fa2ba39223e07622d7dc8b522255938f5f5b4053f9bf6f17a914d9",
	"6d23a467b3245554b4290d4a4b12d008f3ba82870a7defda6c8dd67ad3ea1397c13410a1447d7191",
	"c0e6d3eee1bc58971c4267012409333fd77145c985dda789d118337951e5276bf727fe4ce21b7578",
	"103338a5957c03ca688cd8e71101777d89885e50ddc890bd51fac5908bd65580a1f90b293cf11417",
	"a914f7f1d98cc5edebb87129ab642bf80c3774dbc67587016d521c38ec1ea15734ae22b7c4606441",
	"2829c0d0579f0a713d1c04ede979026f0100000000000001060000a6301800000002473044022070",
	"e7027d08384a21455037958689743da7f94453f3da766d8cec9be27e30cbf902203159961c7b8daa",
	"79e1e2766c648706fa5ead7de56f4cd528ed0c9e37aee0516b012102cd330ecd3c98172c086f3d54",
	"fa4291e5f7b0fee9f3a650a77caa1bfadfebc535000000024730440220102c9f5209a3d57bf6982d",
	"261d40157432f41012a994ccf4883ba854d519770e02206660d876e2233d202ee367c7835911d262",
	"02aea1ac50036cd3614b05d8fced7c01210343f8aa15777fe09c3b3fae8f5b44b307f17f2dae66d8",
	"c03813bff2609dd63588006302000353bd1ea30d4025337bd06b8393a4b43e82939b820e7aab2e7d",
	"70a551d281dffe17fb216f9bc5b6a11ae5708fb9391a130860d8d0d52d9daeda9e82b57280873185",
	"ac62cbe92b459fa3c4dfd60e3a6ccc336a6d72c2b170bb8d80173036ed97e4fd4e10603300000000",
	"0000000186d9e001ed553e90ce549d95e78b8767d0ec3a991bef5c717475127b8a181d88555f856c",
	"fbc24fe7eea5bcbd4bc2db03a9b1ad516bdb843b229dd0c0db50aca236f1d889c0d820cd58671b4c",
	"57d712d6097f092ce8c09bf169df27daac1a850ffa1742e4e6cfb8486ee5da140eaef25ffc0cab76",
	"023736a76dbaf7461bc889cbd4e8619207624ac84609f6baacf2de185d801dbd305b4c6063bfd0ab",
	"dde47c54a4cf2245394a3081681ffd2475f80b4f6b4fbbba0dbc7e8cf991292d309ff64c4c0f20b8",
	"c64062c49e86c10879de229c3f5665f49bde3de9f0159998b70d0d0542de6f19772b41b26b7645e7",
	"38eb23cbd6d1e6c21f0e1255ead7a256f75d9755f9dcb77bd44b408a9261467df18a75377e4d15f0",
	"2b663b5221ed66510d89acddcab574aee3e246734d71e0c804193bc9ee68e489622ef225f430b21f",
	"a90c4f77beaba64ff8af77f397901e6781eb08e62d7bf62a49ac63ba965f212879c0180b015e0414",
	"f7dd5d670f03c0d210daadb818576a6bcb0f49201c55c09d1579ef8ff4c71ddf429f03d88c305114",
	"d66bfc8f2cc4fd38a9e81363f3ffca8a1a904e333d3debb2f3d17237876f66ef3bb3f6f8e661edba",
	"e7c5c96fe020317b55bc3182a4462cc791a89c0258dc302e3351b04dc8125dbdf94c20295562c492",
	"49cda76000b299a8421ace866138273fbbcfc5316a5c222f550bd54c1ab0f3617ecd0d6900347291",
	"b1a589aa7b6ab9c0294cc2b189ad1a2b27460f42fe5975922fa06595c5ed0d059d8a39fdb3b8fd58",
	"db57f653b118e9359973231f34365e8b31575ffb2967c86d66dc376226cca4ef59a2352be4e691d4",
	"ccfbb879524842815f5c4bbeb0fbb7e4d3eb54aa733eea4da929009ee25b1e3e41ab59d81a2f0c51",
	"066da7d610b537104930b726627d5a4de99c87a3fecb324a5855e59a553c2e07174dbbb10ab73125",
	"853ea6fb6cc1ba91560f1fd6b35f3dec779209a8f6285b14a0f7772cce3f7a0be3ecaed93ca15589",
	"c6f274cff3629e78a0290f26f3e1aee9b39b02128dddbf93d20dda252ff8c87d6d4ece2f51bd3fbc",
	"0fa0e61b1d2992e6efc183f2d4102b80d577d8bb357b48af7a3ca2d06c7609cd98680098df331763",
	"0678a58710a83483acb528aa05a9b953c9cedeffd9bdc0e1874540908bcc06a47eb5b94a4102935c",
	"63a42a79296c290e0d12cf50a0eb8df39cfb936b45b310a45e5412a616c41cc3d45af285affb66c9",
	"44cf54ac7a0c9b9d94360ca50a4bcc6f4954856d6af2b2b1ec3adf19441bf594834f65172cfba7f6",
	"3c94658667cd3f341df59e137738a754ae27779a4bbb5b335d05e5a0a8f7cb993bb597c50c1cb46f",
	"2971902c921df5d4701ebaacf8e0ddb4f2f65a36093dc050ae432db4ed6d3cb2919e25b6d014fd98",
	"7eb5b74eb86ab559507dac3fd8986852146b9fa733d7032f577516b6265f93a78e6bc03d1c4f988c",
	"261e37c103634546d6519a3791665d6286af598b0ed654c215dab4e049c3d5b82337f29e7e20c6f4",
	"d5f1827887dad736d305d251713b98c3bb4ada05f9f75f74810b194a9ea8a01b93aeb3ef9b9d1534",
	"827b2e82f33afd6720351bdbecf78b92ed00da885ca868c9cee2a13acc2eaceeb1fc8249c2b0ff1a",
	"6d46ff3e0ede62bf0065910ee5ed9ffb3751c6d0a7b403ac7398ce546760801c25c5ec37daf3e83f",
	"960082ee91ee8d98261ac5656deabe517b645e3af396225fee94994592dab320986942451c0f13ca",
	"6d11cb807a1a284567e667cc79b08d3803180fa76b8f5d91e0a64bad8a30155145f040655a0a4bf7",
	"7cd57e12af0fb7907a2431169ae0910c0c345b0a5111eb4110342ec02d08929b6cf65fc413e9dc4e",
	"bde2bff4cfed6343237f494fef6c04fbb3e7b23de0153d7c42dd58b672cce1e473e4600272147534",
	"15d60e413988b91684acdbf41b43b04eecb1c848c5a0ac227e77841164a9517a7294360b7279f28b",
	"d9bd19e4a81687e41247d3ae8753e26533fbc9f22001265d0616c2adc1f552d4ee1b5667a810f353",
	"8eb438599d8bd9a666d9beb0517f754e48079cb3ef8074f72d9f1688142769843e0f634a1c215bbd",
	"cbe54ce09c3f9d773845f371185eeef6e93c498deae0a455b42b615bf7e0dc02cff916c6f634c68b",
	"34f7781e8cf13916f161af7f71504b899285776f49bc783328bad2ac5cecbd06b64fbe46929d6daf",
	"227c7f38a7264707fe857cdf3f40447c0e793156208c68b98f65edc4d7e0f5aaf2463b023b647bf9",
	"420f41544edaad39ff480e7846f676ad4696094fe02d19b08fbfabd5b43688b77a63f75edf9d72de",
	"25025c2d9744a2116aa0cbdef6cf31d7fd310c866bbe671b1ebce70e37185640d77274f643bebe45",
	"919a20bd1a65221ecf075cd979f64ecfd35d32f8107e051adfbe45df68bf9bd72ecede8614b3841c",
	"00ac6a63ef2114717b2eca1d3a0307072e33f82bb34d3a460007eb0ddab294337557e8b87a5cfd93",
	"7a5faf7caffc192f281c94ed0659e901d12e93b10de7b43e8a5214b06c4cb3d7961a46581e2ffdf1",
	"23957e1175a82ac0cb24b206c1d826fabf8fa634a9240dcb7a7def61c1bcf6d0270c11234f0876b2",
	"777cc19fbe21b4f01ade7dd9a1ef4a75dc7ec25545fb9507c85cc4545d78b19bae531e6bca2903a1",
	"9c12f9e63ebe2d058ed18b80de8adb5c44c1c699a4f3eb058536b3bda9a9e9b5ed0a9f21f6bb2aaa",
	"c9e0c6db4aaa3f2736b4e428dc5b7c31669e4b79d8773a4a3e9d2add5b38e205d5b402dc73178ebc",
	"83e5efde88cae3ad35361bbe06363b894421d6a6f20912f615e4f4bbe661169b4463f6eb2c50cbe9",
	"0d6b3e137e99e79ccb4f0cc2e37f232a703bd8f86df6a08aed1f49a5f3d9b805671f1d942cd27e0a",
	"6b4ed14f6d39d26a05cda253cd18a9d14901a426bd4368f027bc96980efb1cdc8b705360c10748e3",
	"6e90d10f86756f0c79082df68da7b505ff61d156bff249fc30de64123e31c148c76371f3d29684a4",
	"28fdbfc7091b6c45ee5e26afcf3ce9698f95c65c4b857b7d4b87e6ee9fdfe362814ff398b7e967e9",
	"e86be1329eef688949c9a03b6e9a3e3bf48e1fa6e451f62f0942a59295e9c24b665570ee6e10c1da",
	"6bf8f770764989a6003295d908b0555e5318a2fdaf86cca03090f82d1216632878a9f67a8b209ba0",
	"03a1764bc5f7fd401fde553eefea36477ebb4f3ad9ad020490d469ba210ff3ec83ad75ee452630aa",
	"4ae6378bfa66eef28714c00acdd39a20a483b543d81d5f942d22357713d6c20029d07a2c75cdd1fd",
	"6ecefe43a5f872cec7458d1999b258a836bebeaca00d80afc562738576d5d7137d70770784540f58",
	"b98d9557b47a376088faed6afbe4f3f651109fd718c6a73d30b032e2f6ea02b9bd83f5a92d3f35ff",
	"8a82fc4c11e3550883f40a08bc2f37ce60146e392358636798a4e5f217c684499161e9deab84237c",
	"3f46e1811cda9a27bc1cbb4870d4e78b6980c968a845f263db1f814b1e408785a369542c74d40909",
	"9580e128144162c783047e901c2a559c72f89a22dd70d5d62af09bb6d14922cfa700f7f2f039b6a1",
	"6f1165ac8b6d767a22eccbec917bec8a0f940fd9946ba628bb487fc08045f7304eefb183e8b9345b",
	"36ffee9cf37b8472b04d1d8db8b6b70ae33a6ab6738e57a4a41ad5616e46a495e2e1250d8540a71c",
	"5fdabc85ee1a5cfe4d22af38c23a09e9d31f7276e1c31cabc87726ead96e833c5c66a07f917f964a",
	"f311a1c4a975fa0e67f891f73722710d314285ce0a04e0e0be787909f3cd52e4862a75cf9564642c",
	"7281bcc4315c25722ac4df0ed8a541b0e828311b4ec90872965198d8eefe4b1cd2669875c80bd7b6",
	"b549cd81c119d7f0452f06ea568a7eeef06a713a378ce39f2540ba4b633db55d5cbdde36c404f63c",
	"0caf44f2f3f0d1827b6ff7d9a7173b6e3aa01d9f38c3e0ffd8ed30f60608ed7897c6c49cc76d254e",
	"a4abe35ffc17642c692586ca1cccd8477e9a3933149fbc1e942a2526de6b81165a6f203870d796c2",
	"d140e46a4199ef5becef779306982bbf5d15772f270fbf7f452da3a22fe5056f58857586f25fbc7f",
	"059422759d2522d0075ad9c88f13ab259b08168aa6f37ffd77335e5868c1a89a0ba9b02ab7dca743",
	"beb2b6d36b056a9ba15ba69d45b218a1e21766ef46e2f3bf4fce2bb86a1dc830b19a0d3e433cefcf",
	"3ffc3c66b32236121213d5e6a5e679966a6d95e93eb4326f56a8d1582a9c6b071143bc561e102fcc",
	"41c94736ed91486c6faf3464aa5a751ade6042481b2cb2b7de953f52d810a44e01d4d04a695cdef7",
	"9019cdd021baacad9c1d4f4f43e5505db44cccb53d7b5608edf3ee02200deb31b4872074cdbb5d19",
	"b0aab03e78bdb49139c725b6541216a8cf4192c0633f8aa924b6e96e7444745e34d90ae842db708d",
	"c8ac2ba1becc51dcdbd9ce94d917b6572c9cfec0a1ad4a13ea2c56edb8016d60178631b4eb21cf07",
	"06f1018c27465902ad38a703f51d08221e612935acb9886cf38f9cb70368976e8a52cd19a62a1912",
	"6eb9bc78c195dcb38fabda33c38ed968ed598639a78bb6e0bda79a62ffbe985baf34b0ef2a07e318",
	"76c8ca4f147e73ff036f44603cc0c17fc0f37f9f4bf1c147b659a97164b0c49e81890eb2d0c3a708",
	"6deed8f00c3aed1ff21a8595a11cfdb86f7a06d6f4c9d2422f00c1c04ad6252a29607e50044f2512",
	"61f77b6fb57542a4e9b6e0adb0267b1baf3435730f70f95e24e6b68c55deaaeb99667f70e90be0a3",
	"52139da41a3ce4e9b9e5ce8f672f1d3c2c777f9df2977179c32b2429fec99e36562330a202e4c9aa",
	"2c256caf1b3cc6c5165a6645ae727c3b0c3048ac7e251981623452f9bd43f07cbf1e470ab50761e7",
	"128f21ebc97c443973fe4508652d513f041a8fbce1ca8039ce864b384bfebf16f8b4f3e06e9a212f",
	"3f2714f35428e7d69b7bc6c76bdd352f1ab25c80da2f59281abe1946c562130463686c3b299a66be",
	"0e9a38ad0b4d1f9cbc46a71266f4707db670c1df3113fa57b90755706b2f05c2ab6789ed697c789f",
	"f7b262720c309bc4a3437e46dd0a0bfef476f988998b63ac5539ae523107dd4a2d7b31feebad2eaa",
	"d23907d8ada42880d0b416a623f439e53425e15874a0adf73b7277af119d30b45598a765c3cc215c",
	"3bb2cfb415cc45711397a8c249187c061f83845b5d6d6960aaa226445c83d63c5c01b0409d126bba",
	"6ff0760d79915234336c04e8cdb5967f579d6b31a1a80fe82a4a4ce8570db4bb62143119152866fb",
	"8018aae560153ff2f5e216f9bc625940ebf449d40e9ed0ffb918394ad6e548961ccd06d81563163c",
	"1fbfa2abdac0e9aa08db11d1583a9b67e2b0b5f65875cde9eee4512890c9fe8a64508853b52ceafe",
	"5fd6e7fb84e9fbe90beec9b1d930c5dde014e9a7e6d7956a01d7c9109654b1c54a95b2dae0ba7d38",
	"7932780277797c810790197024214bf51e6aeafbc2c73264158a9b1734929c479b2ecf20fd99b173",
	"3441d90cc45ff45a1f246630287bfd917695bda206a58c3134a1b20df3e0326d0a49d629952910b9",
	"093528e5b76a7ded3b23c03b41605383941d472eb2c086ff094ced5578c0b284adb2bef1d90381f7",
	"2c573ae4414c137d8fbb89a876cf41a6d7b19b69c59f8f1e4cb46389f15cbc7c098f5cdcf969ca67",
	"916cc46818b8a388bdcbd4f754a932d4f49808f93ab584528cc1541c89894106f562a77558e40f2b",
	"908ce945534d40a87fca0279b92ffc121a66030901cb353aa0dddc07f1805c29d2dc6e5ce9e16bc0",
	"aa57c1c989666afc9ae87ac54ab0f29d518c4bc2cbad04fda67ee4ac3d3253cb2968e4f54ee13cc6",
	"869759cb956bdb38e80d7ce3034fd53baa5472192cc09e297b5a8f0e13f152e3671f32ef102bbb8d",
	"ed4726cac195821f63020003ecee044a27d5f691082eb3fa0d7b800f0134fa702c2d0007f1fc2ce3",
	"0142d9e456b543d6b43943ccc9e83b005f06099cca1656b65f0311865618a52bbc961649f784d707",
	"59c522034c31a423d9e727c4956ce4ca2079a4bea9f08237d2164089fd4e10603300000000000000",
	"0101ca98006991e5f0d71568c120f4824cb36a02bc5db22222b3e400299603cd9727ffb6b1a1fbb3",
	"8a33b65e939c2c665a5c6d998e42deb823b1340a06a0fb27b91b04be3b01c0d50b348ee71f32f30a",
	"51280f2671c5df0a642295019562fc99a6697ddb733d9db5bac2fad9f0c3f793a2a95bb50a53ccfd",
	"752c04fcdc0564b94b3095b8f7f4512f9a960817097529f813ecc19993dbd2cc66519723c219963f",
	"d9827869d221a7091341fd807df2c7c565adeff61d74dbe7dd073cc5d86f6c51834d5bc734fdaf0d",
	"5587548d4d279b1858fa400e99f6b3af5ea11f9baee5120f8a2f87258c1bbea524928a35840446b2",
	"b7b8b2ecd2c583017084315f0cde477f0c858c9f71b63fa65e411f04bde992df8633eb467924fc7f",
	"847ce5cc63ac7ad2781da72d90dbe6697967ed7b695a786f6647b118afebcbe7f706dc587072a50a",
	"c20212d36fb009ba7f427fad1d1b289196050a776b033f638feb1beb8cab21504cca86e25ffe02c9",
	"ec8e1a20e814e305308f1cb815f7875805c6c927dc9e234092ddf7fc5d3e3fd0c12f477d3305aed5",
	"c5452b127825084456420e20adc0eb613c107fc1cd24bb880675e2263f6d74b954218c0829aa7af0",
	"853ca5a43ebed67826021334740801ec0205b11784ae4ed4394d7d5adaa74fb26d176dc4903abb43",
	"8c28056789bc8c6f31116bce660507e1ae6c46918e1378c01aec3dfef1b0806b0afef30d39e6b2ca",
	"d40fc29820eaa3d657c3c20c953c754a59e556e555dd28777b83d11b5d634dc5d96e73b663e6961a",
	"e9fca5271fa737e85ca280961ef6da6b701e2932c3f2d519657415d633c1b02248593020fb65191c",
	"a8701651c370f74e90e43fd8757ad43ff2bd705b7b365f889239437533e4b01e98894359f5b85336",
	"cb5bba6f2ceca2c61426d8fe0447747708cdf84b95f5227c2bf3adbb7c4fab8efebb23aac0161c94",
	"23089e1594a2bd924f0c8f5eabba83d52aa3cb136eb482e074c2db104e44baa441dcb7d401cdef39",
	"83a2b7aa3e055bb2791311cbcd97482e8149b59a3e6a48367baf7e10125df3fafc5e8c1956bc4b30",
	"fc04622193f9eab4cfe4ffe53f61edc7209999860461eae28b8d2e0dea3975983f767cbc8d884c12",
	"825116c41b01d911059aa6262cbc82e5910cf1e333369fee18e96f0eeec699494bd8e45d47d94a6e",
	"49841d83261d4a2bdc161bc1b366bad5fa8466be09013685b182466cbf6b9a5f8f0d4a2b414b5234",
	"48c9435b58dc78b4203861fd43b1267b74f8c1c969918e18ca76233e79cd1756eac12b774e071d3c",
	"3a63f3a6e4a512d32e7669423084bf77a5cdf8f126aba81a1f02b61177079c8cf7fce8e28ed2b7ce",
	"8c5f30a10c8294060e4c0c7c3fb97b74aef69daba412100f57277feb9a7dc7f6253f0aebe75f1529",
	"ad0b49e4ed90219d195212568d9805d748b047def7c28f06553ab18bc94c67a0a2fa6fb82bfeea92",
	"34fad3a1b3f1c5f5c3e75a768bc994a85a1a1f41c57aa766d0f3514a914652b0d7a4627a158c644e",
	"bae9ba9d6fb333290abf13f63830519a4c3400c7649a45bb34d5409825e3d697ded4248131a24b83",
	"693af34de171d6097f5a8ed0934949f5607b0cbc1a2eaa26ecddf35cadb9e198fe3f4522ca726747",
	"6e689f3aebab2459d6066302426b11c2a864f45bf971f0e05902dfa6fe3527f3a0df8e247d11a9d8",
	"859a74ba7858046877cb7e85624e87462b75da749c85d42f1d40adaba3b20532f3a71c42890da4a3",
	"a71b1332cd4f5216adabcb321e0c9ec6a381c528f42d7a230c28d30a181f89e688d25f2277929a18",
	"13c81daaf7a362d79a22925de60e9f74fcc008271365caffa2a840490a88e637a0aaba6f8d903dab",
	"19315c30f5b604060b9d338ae5a261a5c546752288a3350e5f89cbc6143fbcfa0ec31009b07fcf89",
	"b7d983e0c2e65a8f86fad466b146d88d1c1177f49b502d7d8d154407561084c15b84c14dbade7d68",
	"2478655564cb181cb4d5e241a6ba6f0a347016f83c986fb74e02d3cc9197fa99b24f4093a7d9415a",
	"f9ff19413ee0fb770252921f1b87aeee64f459ed080693212c535dee68a614d63abcc0d08905a7d4",
	"abbf06a829a86ae4364bcd8b8a412999ca58715c099977abbbfd6e7e2498611a79202dcd402005be",
	"5fe03f24c0d8d46b2bf01e7f7dbada43f2441b07a24bd5a07ad44e7869b46690f6c5dd13bddcf812",
	"10caf26fbea73386098400288370a2a5973784803b97ca6f340e9875ee883993294ed5afca991de9",
	"981b9dcb874a569212fdb857759f247732bd7bde0016a294b36381489c20381c98a245d5c3a3ca2d",
	"6535a11354bfa32d06ea1e2b72b1f1d6e96201b9b9d18c58a79a6f6ef7547017666fca41d564dd7d",
	"a8084294da5b72713774f31d66620fd5017bdb64b962ced8acd2b2a6eca6fc70105af4fe4a321442",
	"2cc07e18c41acd4ca1cec1952c6df66691599b8f291af4d73605b6238c59c93e12a78ec192eec302",
	"83c176bf0227287c2a81c968eadbcbd3709e3b26671174b8542aec1293518b7b0abccf08693e71ec",
	"8347475f22044894c30e6f3105521bef1efcd25700f6b828ee8df2c329e24f4e7e43ca6db0022fa1",
	"5ef6980d61cfd29c1a7cbf531da714a5d47b82eb2282be4536f538453ac4c146c15bb53f94428714",
	"5d14ded8390658e1910c57aecdb12a45a581461ba98189ad80dbb9f0a12285c2e18412f8f0b33805",
	"92c7de212fd2d88d66a980a75578ebced6c5c7ea9864d2df7964efaf043a410c5149009d88c21300",
	"3464dbeca9e8517b8a3a145afb65578d3ab525672229a52a1cfacb4a67112e7d4265c2c19ef6c537",
	"6223162544975d38d497f713fa921fdae5fa44de7d46eb4492af847910db132604e1ccc5ce33683e",
	"92c61a033505c917ec4f842eecdea39d0c4afe98f63b3ede798e82981e5def4ef4657262f2888951",
	"ecaac670672846ffdca023e3238dfabcf1b736bdcc0d74d5027fc7c43d87cbce1685703b9c9d819b",
	"0ef2aa6165d870e2920eaab28a1bc69bf4202dffd961cafe3d5f0c2882a688458777c80c24268ce5",
	"259d6e3be4dc8f3b45947f08e50d7544b7921da933d1bd777c1b27b1fcd864e5ceaa7a56ff0d08f6",
	"4179ac4a150f06704bd5086e7d8db9d0b17d607b086bcc8627c9dc6b1d9746facac0c117801bba22",
	"c33276f2bcda7e1ba7c354584cab51a8332ee96eae8eebdf0257414f1c4710d20438db98930197f3",
	"a085193e2ab1298fe91f12921f82f786187d96c636835045c5852e829158daa92198220f57d9a291",
	"bb6a8942d429a6ba48d063f82f3f5bde82b72651e6b19169bb0c778fd22fb4237eab13b6ae95c3d9",
	"2a8a172d8b95e52b41ec76237a56c9fa4541c993fa8d1a1fa72da279801e9a398272d0018c1c52dd",
	"5350dd34f298eb54e847dd9acdeae43bc4abfc62e2f5dbff5f8b3013d2c5d4df737b9f0b4ad7bced",
	"68b7dab99a2814ac095b42a1f9bc144ba7f266f5da61dad4fd465a010e452f010f5f6b53a31ca880",
	"eebc733a34fd58cc6c7ebe0ac9d5a03020ff37da78ce97bc0b3fc3dde03d4104d9ab3808be23dad2",
	"7ae011592b63e8c67437af85ebeabde2e023e489eef3e6659b19d6e76d53aca2f399212c9d1cc27b",
	"34aa1fa23cfddc0866cc5cc25319b19dad6b5acfa3fe32a0e1240dc76e0a4541b58cbf84c84a18b5",
	"c4801fdf892b25f31dd178a8ebd73bec5994e8ce007232cb9130ea59ad21b3d465b0dc5a279f5458",
	"71aac3cec3603f9ce9df1b4b890e0a2ccef1a362043edfa2f96a1cfd52ae046f509929c1db87d234",
	"71c5407f7739221a03769b3e290cd1c74c6cf86aef016099625f6ad6b8018aeec3f5cbc27cc4e64e",
	"060a2bd6ae779164ad3661ebad7ebaa9b43420bf34d916e52a6eb1f1ac885a733fe048fa13b71b11",
	"6220edd39cc32444d7a8894ac25ee79ecd7d371573a1b147f0bf064c46adf499b14ba6ce1e9c7634",
	"8df9df83555f18f7ff00ccfd2dc8431ba4437266fb9f75f29256be2633e22deb83ed64c9ab9842cc",
	"de92184a918c7e2302ad580b6322225b3e71f2abfdee521303cca7a50611c08d26bff4acd5d7f7d2",
	"b08e239b20c1bb3ba5764a456f531aa8c1da4a65ec71ee3b092e5f89ba1e6c48355ce6ddef40c062",
	"ed2cbdb633bf522c5825dd927cf4e3c29e2990633492b1a66b65c5cd93a091aff6c2fd0ae4808553",
	"b9aabd00544e45dfee0a3d3c17d69cccd7fca378938a53283f820aa8bfc647d0e598161d869f4e18",
	"fd1c7ea37c21b26cf9af0d19cac42f401b070b4067fc6bdcdb1bcec5aed941bc39f060b4a100d03a",
	"ac3f3a44c2e2b0a8769d51de12012e08ba09e3faea69763bb08768b9f901ad7cafe5b000870c03b3",
	"efc301ff0f1dae8101c4bdb2d5a9fe5ef1fde40c124c61658bcb3dc738c0b38dc293f98e3042ab00",
	"ae26812bdedf2af968d63ee9f2dc60f831b32d87f30cd6288c0c5256a4516f33ba3db5884cdccc07",
	"bf6bb89ddeb765bcc7a3f533bcb8648d1e37864831173d050e6b056302e3273d56cb3d61b2f109fa",
	"367f67a4b9c7042532a56c968379e613e1357f62558ff40018e87f4753409123883af3648292d89e",
	"dce8f3482b0066718279d3946c2736ae93658a4568a24a54b5e6fd8d6ac138ef7b87a27a3dfa992b",
	"3c2baf1de8f06ac44e10042ca0b7a05f1140036c43506d1388abc5c44fe88fdf20e51e5a10e9d9a7",
	"5e20206cbadbcf6badc505d46d92d4d4f1c54992877f36d3c72197c56246b9b8daf738472be6afee",
	"9b9c31017562a1c1360502b36e4d5f01fecdd8ec737a25d03aa545ff7e8d7be379e3da680a2bf081",
	"cd2a49b99fbb6a1ae6f076336f8760ebea16d543194c4b5dfb792f13b39e972ae31e18dd4a00384e",
	"db5510a39999c8c7076a7ec650853b9e2e4214c4b25088a4790f34ca62ea2b5a461260494a12fbe1",
	"2bcd94d423b574b760a9e644083cd20a712ec5bc87aa4236c2eab4dd354264b0d433a750981041d7",
	"aed96880254801dd06660e6050b408b691a447afb0d07d0f2a8eeaa42e8600c726cce7fdec135299",
	"45f135ed4a238c009e217d15ebd02b302fd250025f028a51b9dc5edc18472b9442e192ecf7c001e4",
	"5242021f3ff802b7a63debd89a8a279ba38fca342724a4ebdbe9483e1f21aa105c02bb7c5a8e7285",
	"8f01b047a3a3fa333887e6b12399576eeccebcd579d27e28dcbd42d08aabd6d36d6ec508a84c9d97",
	"e0999befe672a36e8b706cb9f2f86458959070cb5dce845f9721ac95aed31aa1988a07266cbfb2b4",
	"2253290ddff6085f1d730f4004577b4e61c56673acae09263bf05a9ddda265dc6c909469a92d9913",
	"969e90e1a89256fc5fe689e346dcafe01d9ba7e6a1f471738bcd021dc38ca7e4e636c94ca6f4798f",
	"5847a372f75fe881236f33cd0ec97144936f4ba14c4f41f22c4b41c1f31b7e4f3382d376ba79cdac",
	"0aeea6b4098cb59177490ffab463ea5d6a8c638a868f49c32f114cfad827918cfee18fab024530db",
	"dcbbb611c23cc9aa26c43f314887f647a7ee982b81976a66fb07154f6657f7bfadc11ecd305b094d",
	"4bfc0ccd1ae52e37e0849f1006c60854810c874948ebd56c7ded4371107c9dc1361565c51f745e2c",
	"a4be55a12b26d0015c7d8d9774814c20d5b06823e66c306c0a282b787f6d6e79dccef4647462f543",
	"c771db5deb45f1dee9280c4fbba7cbe44510bf01fff07178362a76832832152fa0ff62f9c1f2bfd7",
	"eaaa1e7ac1c27148f7f1e498e851e314f7d793bd7f992dbcbc13c5f4bf95669003864b4d4c2dfa98",
	"470129fab5b9c34cb2febaf8b17393e9d3695ea4626280f2818ad48a4ea0b6c5eaecd8a71075590a",
	"5d988f5792c74202e8c4dad8d8b46423b3cbd0943cbafeaeeaf4cdc7b1ceaad213d56d49d5e14580",
	"98a340b9ba0000",
);
