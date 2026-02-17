use std::str::FromStr;

use simplicity::jet;

use crate::hal_simplicity::Program;

#[derive(Debug, thiserror::Error)]
pub enum SimplicityGraphError {
	#[error("invalid program: {0}")]
	ProgramParse(simplicity::ParseError),
}

#[derive(Debug, Clone, Copy)]
pub enum SharingLevel {
	NoSharing,
	MaxSharing,
}

impl FromStr for SharingLevel {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"none" => Ok(SharingLevel::NoSharing),
			"max" => Ok(SharingLevel::MaxSharing),
			other => Err(format!("unknown sharing level: {}", other)),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum GraphFormat {
	Dot,
	Mermaid,
}

impl FromStr for GraphFormat {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"graphviz" | "dot" => Ok(GraphFormat::Dot),
			"mermaid" | "mermaidjs" => Ok(GraphFormat::Mermaid),
			other => Err(format!("unknown graph format: {}", other)),
		}
	}
}

pub fn simplicity_graph(
	program: &str,
	witness: Option<&str>,
	sharing_level: SharingLevel,
	graph_format: GraphFormat,
) -> Result<String, SimplicityGraphError> {
	let program = Program::<jet::Elements>::from_str(program, witness)
		.map_err(SimplicityGraphError::ProgramParse)?;

	let sharing_level = match sharing_level {
		SharingLevel::MaxSharing => simplicity::node::SharingLevel::Max,
		SharingLevel::NoSharing => simplicity::node::SharingLevel::None,
	};
	let graph_format = match graph_format {
		GraphFormat::Dot => simplicity::node::GraphFormat::Dot,
		GraphFormat::Mermaid => simplicity::node::GraphFormat::Mermaid,
	};

	let res = if let Some(node) = program.redeem_node() {
		let graph = node.display_as_graph(graph_format, sharing_level);
		graph.to_string()
	} else {
		let graph = program.commit_prog().display_as_graph(graph_format, sharing_level);
		graph.to_string()
	};
	Ok(res)
}
