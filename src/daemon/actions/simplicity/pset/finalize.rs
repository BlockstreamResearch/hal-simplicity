use super::PsetError;
use crate::daemon::actions::types::{PsetFinalizeRequest, PsetFinalizeResponse};
use crate::hal_simplicity::Program;
use crate::simplicity::jet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PsetFinalizeError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("Failed to decode PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("Failed to parse program: {0}")]
	ProgramParse(simplicity::ParseError),

	#[error("Program does not have a redeem node")]
	NoRedeemNode,

	#[error("Failed to prune program: {0}")]
	ProgramPrune(simplicity::bit_machine::ExecutionError),
}

pub fn finalize(req: PsetFinalizeRequest) -> Result<PsetFinalizeResponse, PsetFinalizeError> {
	let mut pset: elements::pset::PartiallySignedTransaction =
		req.pset.parse().map_err(PsetFinalizeError::PsetDecode)?;

	let input_idx_usize = req.input_index as usize;

	let program = Program::<jet::Elements>::from_str(&req.program, Some(&req.witness))
		.map_err(PsetFinalizeError::ProgramParse)?;

	let (tx_env, control_block, tap_leaf) = super::execution_environment(
		&pset,
		input_idx_usize,
		program.cmr(),
		req.genesis_hash.as_deref(),
	)?;

	let cb_serialized = control_block.serialize();

	let redeem_node = program.redeem_node().ok_or_else(|| PsetFinalizeError::NoRedeemNode)?;

	let pruned = redeem_node.prune(&tx_env).map_err(PsetFinalizeError::ProgramPrune)?;

	let (prog, witness) = pruned.to_vec_with_witness();

	let input = &mut pset.inputs_mut()[input_idx_usize];
	input.final_script_witness = Some(vec![witness, prog, tap_leaf.into_bytes(), cb_serialized]);

	Ok(PsetFinalizeResponse {
		pset: pset.to_string(),
		updated_values: vec!["final_script_witness".to_string()],
	})
}
