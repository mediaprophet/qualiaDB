//! MCP Resource endpoints and QApp Host Surface schema handlers.
//!
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::McpSystemError;
use serde_json::json;

pub const QAPP_SURFACE_SCHEMA_URI: &str = "qualia://qapp-surface-schema";

/// Return static JSON-schema describing the QApp Host Surface.
pub fn get_qapp_surface_schema() -> String {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "QualiaQappSurfaceSchema",
        "description": "Schema defining the execution contract and resource surface for Qualia QApps.",
        "type": "object",
        "required": ["surfaceVersion", "capabilities", "modalities"],
        "properties": {
            "surfaceVersion": { "type": "string", "const": "2026.1" },
            "protocolVersion": { "type": "string", "const": "2024-11-05" },
            "capabilities": {
                "type": "array",
                "items": { "type": "string" },
                "default": [
                    "sparql-query",
                    "shacl-validation",
                    "vibe-scripting",
                    "geomechanics-simulation",
                    "space-weather-simulation",
                    "fire-ecology-simulation",
                    "geospatial-indexing",
                    "biosense-rppg",
                    "spatial-audio-hrtf",
                    "qpu-optimization",
                    "ambient-orchestration"
                ]
            },
            "modalities": {
                "type": "array",
                "items": { "type": "string" },
                "default": ["ltl", "asp", "deontic", "epistemic", "paraconsistent", "probabilistic"]
            }
        }
    })
    .to_string()
}

/// `describe_qapp_surface_schema` tool handler.
pub fn describe_qapp_surface_schema(_args: &[u8]) -> Result<String, McpSystemError> {
    Ok(get_qapp_surface_schema())
}
