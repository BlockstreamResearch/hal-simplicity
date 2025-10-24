// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use std::sync::Arc;

use simplicity::bitcoin::secp256k1;
use simplicity::jet::Jet;
use simplicity::{BitIter, CommitNode, DecodeError, ParseError, RedeemNode};

/// A representation of a hex or base64-encoded Simplicity program, as seen by
/// hal-simplicity.
pub struct Program<J: Jet> {
	/// A commitment-time program. This should have no hidden branches (though the
	/// rust-simplicity encoding allows this) and no witness data.
	///
	/// When parsing a redeem-time program, we first parse it as a commitment-time
	/// program (which will always succeed, despite the potentially hidden branches)
	/// because this lets the tool provide information like CMRs or addresses even
	/// if there is no witness data available or if the program is improperly
	/// pruned.
	commit_prog: Arc<CommitNode<J>>,
	/// A redemption-time program. This should be pruned (though an unpruned or
	/// improperly-pruned program can still be parsed) and have witness data.
	redeem_prog: Option<Arc<RedeemNode<J>>>,
}

impl<J: Jet> Program<J> {
	/// Constructs a program from a hex representation.
	///
	/// The canonical representation of Simplicity programs is base64, but hex is a
	/// common output mode from rust-simplicity and what you will probably get when
	/// decoding data straight off the blockchain.
	pub fn from_str(prog_b64: &str, wit_hex: Option<&str>) -> Result<Self, ParseError> {
		// Attempt to decode a program from base64, and failing that, try hex.
		let commit_prog = match CommitNode::from_str(prog_b64) {
			Ok(prog) => prog,
			Err(e) => {
				use simplicity::hex::FromHex as _;
				if let Ok(bytes) = Vec::from_hex(prog_b64) {
					let iter = simplicity::BitIter::new(bytes.into_iter());
					if let Ok(node) = CommitNode::decode(iter) {
						node
					} else {
						return Err(e);
					}
				} else {
					return Err(e);
				}
			}
		};

		Ok(Self {
			commit_prog,
			redeem_prog: wit_hex.map(|hex| RedeemNode::from_str(prog_b64, hex)).transpose()?,
		})
	}

	/// Constructs a program from raw bytes.
	pub fn from_bytes(prog_bytes: &[u8], wit_bytes: Option<&[u8]>) -> Result<Self, DecodeError> {
		let prog_iter = BitIter::from(prog_bytes);
		let wit_iter = wit_bytes.map(BitIter::from);
		Ok(Self {
			commit_prog: CommitNode::decode(prog_iter.clone())?,
			redeem_prog: wit_iter.map(|iter| RedeemNode::decode(prog_iter, iter)).transpose()?,
		})
	}

	/// The CMR of the program.
	pub fn cmr(&self) -> simplicity::Cmr {
		self.commit_prog.cmr()
	}

	/// The AMR of the program, if it exists.
	pub fn amr(&self) -> Option<simplicity::Amr> {
		self.redeem_prog.as_ref().map(Arc::as_ref).map(RedeemNode::amr)
	}

	/// The IHR of the program, if it exists.
	pub fn ihr(&self) -> Option<simplicity::Ihr> {
		self.redeem_prog.as_ref().map(Arc::as_ref).map(RedeemNode::ihr)
	}

	/// Accessor for the commitment-time program.
	pub fn commit_prog(&self) -> &CommitNode<J> {
		&self.commit_prog
	}

	/// Accessor for the commitment-time program.
	pub fn redeem_node(&self) -> Option<&RedeemNode<J>> {
		self.redeem_prog.as_ref().map(Arc::as_ref)
	}
}

// Stolen from simplicity-webide
fn unspendable_internal_key() -> secp256k1::XOnlyPublicKey {
	secp256k1::XOnlyPublicKey::from_slice(&[
		0xf5, 0x91, 0x9f, 0xa6, 0x4c, 0xe4, 0x5f, 0x83, 0x06, 0x84, 0x90, 0x72, 0xb2, 0x6c, 0x1b,
		0xfd, 0xd2, 0x93, 0x7e, 0x6b, 0x81, 0x77, 0x47, 0x96, 0xff, 0x37, 0x2b, 0xd1, 0xeb, 0x53,
		0x62, 0xd2,
	])
	.expect("key should be valid")
}

fn script_ver(cmr: simplicity::Cmr) -> (elements::Script, elements::taproot::LeafVersion) {
	let script = elements::script::Script::from(cmr.as_ref().to_vec());
	(script, simplicity::leaf_version())
}

fn taproot_spend_info(cmr: simplicity::Cmr) -> elements::taproot::TaprootSpendInfo {
	let builder = elements::taproot::TaprootBuilder::new();
	let (script, version) = script_ver(cmr);
	let builder = builder.add_leaf_with_ver(0, script, version).expect("tap tree should be valid");
	builder
		.finalize(secp256k1::SECP256K1, unspendable_internal_key())
		.expect("tap tree should be valid")
}

pub fn elements_address(
	cmr: simplicity::Cmr,
	params: &'static elements::AddressParams,
) -> elements::Address {
	let info = taproot_spend_info(cmr);
	let blinder = None;
	elements::Address::p2tr(
		secp256k1::SECP256K1,
		info.internal_key(),
		info.merkle_root(),
		blinder,
		params,
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fixed_hex_vector_1() {
		// Taken from rust-simplicity `assert_lr`. This program works with no witness data.
		let b64 = "zSQIS29W33fvVt9371bfd+9W33fvVt9371bfd+9W33fvVt93hgGA";
		let prog = Program::<simplicity::jet::Core>::from_str(b64, Some("")).unwrap();

		assert_eq!(
			prog.cmr(),
			"abdd773fc7a503908739b4a63198416fdd470948830cb5a6516b98fe0a3bfa85".parse().unwrap()
		);
		assert_eq!(
			prog.amr(),
			Some(
				"1362ee53ae75218ed51dc4bd46cdbfa585f934ac6c6c3ff787e27dce91ccd80b".parse().unwrap()
			)
		);
		assert_eq!(
			prog.ihr(),
			Some(
				"251c6778129e0f12da3f2388ab30184e815e9d9456b5931e54802a6715d9ca42".parse().unwrap()
			),
		);

		// The same program with no provided witness has no AMR or IHR, even though
		// the provided witness was merely the empty string.
		//
		// Maybe in the UI we should detect this case and output some sort of warning?
		let b64 = "zSQIS29W33fvVt9371bfd+9W33fvVt9371bfd+9W33fvVt93hgGA";
		let prog = Program::<simplicity::jet::Core>::from_str(b64, None).unwrap();

		assert_eq!(
			prog.cmr(),
			"abdd773fc7a503908739b4a63198416fdd470948830cb5a6516b98fe0a3bfa85".parse().unwrap()
		);
		assert_eq!(prog.amr(), None);
		assert_eq!(prog.ihr(), None);
	}
}
