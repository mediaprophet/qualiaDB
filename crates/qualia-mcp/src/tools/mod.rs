//! Modular tool implementations for Qualia Model Context Protocol (MCP).
//!
//! Organized into focused submodules per domain, adhering to `AGENTS.md` §0-B.

pub mod audio;
pub mod formats;
pub mod geospatial;
pub mod governance;
pub mod graph;
pub mod llm;
pub mod logic;
pub mod poet;
pub mod qapps;
pub mod quantum;
pub mod science;
pub mod shacl;
pub mod sim_fire_ecology;
pub mod sim_geomechanics;
pub mod sim_space_weather;
pub mod tasks;
pub mod vibe;
pub mod vision;
pub mod wgsl_forge;

pub use audio::*;
pub use formats::*;
pub use geospatial::*;
pub use governance::*;
pub use graph::*;
pub use llm::*;
pub use logic::*;
pub use poet::*;
pub use qapps::*;
pub use quantum::*;
pub use science::*;
pub use shacl::*;
pub use sim_fire_ecology::*;
pub use sim_geomechanics::*;
pub use sim_space_weather::*;
pub use tasks::*;
pub use vibe::*;
pub use vision::*;
pub use wgsl_forge::*;
