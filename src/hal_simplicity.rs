// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use std::sync::Arc;

use elements::taproot::{TaprootBuilder, TaprootSpendInfo};
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
	///
	/// The canonical representation of witnesses is hex, but old versions of simc
	/// (e.g. every released version, and master, as of 2025-10-25) output base64.
	pub fn from_str(prog_b64: &str, wit_hex: Option<&str>) -> Result<Self, ParseError> {
		let prog_bytes = crate::hex_or_base64(prog_b64).map_err(ParseError::Base64)?;
		let iter = BitIter::new(prog_bytes.iter().copied());
		let commit_prog = CommitNode::decode(iter).map_err(ParseError::Decode)?;

		let redeem_prog = wit_hex
			.map(|wit_hex| {
				let wit_bytes = crate::hex_or_base64(wit_hex).map_err(ParseError::Base64)?;
				let prog_iter = BitIter::new(prog_bytes.into_iter());
				let wit_iter = BitIter::new(wit_bytes.into_iter());
				RedeemNode::decode(prog_iter, wit_iter).map_err(ParseError::Decode)
			})
			.transpose()?;

		Ok(Self {
			commit_prog,
			redeem_prog,
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
	pub fn redeem_node(&self) -> Option<&Arc<RedeemNode<J>>> {
		self.redeem_prog.as_ref()
	}
}

/// The unspendable internal key specified in BIP-0341.
///
/// This is a "nothing up my sleeve" (NUMS) point. See the text of BIP-0341
/// for its derivation.
#[rustfmt::skip] // mangles byte vectors
pub fn unspendable_internal_key() -> secp256k1::XOnlyPublicKey {
	secp256k1::XOnlyPublicKey::from_slice(&[
		0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
		0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0, 
	])
	.expect("key should be valid")
}

fn script_ver(cmr: simplicity::Cmr) -> (elements::Script, elements::taproot::LeafVersion) {
	let script = elements::script::Script::from(cmr.as_ref().to_vec());
	(script, simplicity::leaf_version())
}

/// Given a Simplicity CMR and an internal key, computes the [`TaprootSpendInfo`]
/// for a Taptree with this CMR as its single leaf.
pub fn taproot_spend_info(
	internal_key: secp256k1::XOnlyPublicKey,
	cmr: simplicity::Cmr,
) -> TaprootSpendInfo {
	let builder = TaprootBuilder::new();
	let (script, version) = script_ver(cmr);
	let builder = builder.add_leaf_with_ver(0, script, version).expect("tap tree should be valid");
	builder.finalize(secp256k1::SECP256K1, internal_key).expect("tap tree should be valid")
}

/// Given a Simplicity CMR, computes an unconfidential Elements address
/// (for the given network) corresponding to a Taptree with an unspendable
/// internal key and this CMR as its single leaf.
pub fn elements_address(
	cmr: simplicity::Cmr,
	params: &'static elements::AddressParams,
) -> elements::Address {
	let info = taproot_spend_info(unspendable_internal_key(), cmr);
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
