//! Helpers shared between unit tests in different modules.

use elements::taproot::ControlBlock;
use elements::Script;

use crate::hal_simplicity::{script_ver, taproot_spend_info, unspendable_internal_key};

/// The control block and scriptPubKey for a Taptree with this CMR as its single leaf.
pub fn test_leaf(cmr: simplicity::Cmr) -> (ControlBlock, Script) {
	let spend_info = taproot_spend_info(unspendable_internal_key(), None, cmr);
	let control_block = spend_info.control_block(&script_ver(cmr)).expect("leaf is in tree");
	(control_block, Script::new_v1_p2tr_tweaked(spend_info.output_key()))
}
