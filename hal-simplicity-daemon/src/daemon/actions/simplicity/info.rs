use crate::simplicity::jet;
use crate::types::{RedeemInfo, SimplicityInfoRequest, SimplicityInfoResponse};
use crate::utils::hal_simplicity::{elements_address, Program};
use simplicity::hex::parse::FromHex as _;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimplicityInfoError {
	#[error("Failed to parse program: {0}")]
	ProgramParse(simplicity::ParseError),

	#[error("Failed to parse state (32-byte hex): {0}")]
	StateParse(elements::hashes::hex::HexToArrayError),
}

pub fn info(req: SimplicityInfoRequest) -> Result<SimplicityInfoResponse, SimplicityInfoError> {
	let program = Program::<jet::Elements>::from_str(&req.program, req.witness.as_deref())
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

	let state = req
		.state
		.as_deref()
		.map(<[u8; 32]>::from_hex)
		.transpose()
		.map_err(SimplicityInfoError::StateParse)?;

	Ok(SimplicityInfoResponse {
		jets: "core".to_string(),
		commit_base64: program.commit_prog().to_string(),
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
