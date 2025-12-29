use crate::hal_simplicity::{elements_address, Program};
use crate::simplicity::hex::parse::FromHex as _;
use crate::simplicity::{jet, Amr, Cmr, Ihr};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum SimplicityInfoError {
	#[error("invalid program: {0}")]
	ProgramParse(simplicity::ParseError),

	#[error("invalid state: {0}")]
	StateParse(elements::hashes::hex::HexToArrayError),
}

#[derive(Serialize)]
pub struct RedeemInfo {
	pub redeem_base64: String,
	pub witness_hex: String,
	pub amr: Amr,
	pub ihr: Ihr,
}

#[derive(Serialize)]
pub struct ProgramInfo {
	pub jets: &'static str,
	pub commit_base64: String,
	pub commit_decode: String,
	pub type_arrow: String,
	pub cmr: Cmr,
	pub liquid_address_unconf: String,
	pub liquid_testnet_address_unconf: String,
	pub is_redeem: bool,
	#[serde(flatten)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub redeem_info: Option<RedeemInfo>,
}

/// Parse and analyze a Simplicity program.
pub fn simplicity_info(
	program: &str,
	witness: Option<&str>,
	state: Option<&str>,
) -> Result<ProgramInfo, SimplicityInfoError> {
	// In the future we should attempt to parse as a Bitcoin program if parsing as
	// Elements fails. May be tricky/annoying in Rust since Program<Elements> is a
	// different type from Program<Bitcoin>.
	let program = Program::<jet::Elements>::from_str(program, witness)
		.map_err(SimplicityInfoError::ProgramParse)?;

	let redeem_info = program.redeem_node().map(|node| {
		let disp = node.display();
		let redeem_base64 = disp.program().to_string();
		let witness_hex = disp.witness().to_string();
		RedeemInfo {
			redeem_base64,
			witness_hex,
			amr: node.amr(),
			ihr: node.ihr(),
		}
	});

	let state =
		state.map(<[u8; 32]>::from_hex).transpose().map_err(SimplicityInfoError::StateParse)?;

	Ok(ProgramInfo {
		jets: "core",
		commit_base64: program.commit_prog().to_string(),
		// FIXME this is, in general, exponential in size. Need to limit it somehow; probably need upstream support
		commit_decode: program.commit_prog().display_expr().to_string(),
		type_arrow: program.commit_prog().arrow().to_string(),
		cmr: program.cmr(),
		liquid_address_unconf: elements_address(
			program.cmr(),
			state,
			&elements::AddressParams::LIQUID,
		)
		.to_string(),
		liquid_testnet_address_unconf: elements_address(
			program.cmr(),
			state,
			&elements::AddressParams::LIQUID_TESTNET,
		)
		.to_string(),
		is_redeem: redeem_info.is_some(),
		redeem_info,
	})
}
