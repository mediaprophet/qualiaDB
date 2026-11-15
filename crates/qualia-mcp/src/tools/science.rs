//! MCP tools for scientific, algebraic, chemistry, clinical, and geometric compute.
//!
//! Exposes CAS, linear algebra, ODE solvers, bioinformatics, chemistry, and computational geometry.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::McpSystemError;

fn map_db_error(e: qualia_core_db::mcp::mcp_server::McpSystemError) -> McpSystemError {
    match e {
        qualia_core_db::mcp::mcp_server::McpSystemError::InvalidParameters => {
            McpSystemError::InvalidParameters
        }
        qualia_core_db::mcp::mcp_server::McpSystemError::ParseError => McpSystemError::ParseError,
        qualia_core_db::mcp::mcp_server::McpSystemError::SanctuaryGateTriggered => {
            McpSystemError::SanctuaryGateTriggered
        }
        _ => McpSystemError::ToolNotReady,
    }
}

pub fn matrix_operation(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::matrix_operation(args).map_err(map_db_error)
}

pub fn algebra_solve_polynomial(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::algebra_solve_polynomial(args).map_err(map_db_error)
}

pub fn algebra_matrix_analyze(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::algebra_matrix_analyze(args).map_err(map_db_error)
}

pub fn cas(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::cas(args).map_err(map_db_error)
}

pub fn ode_solve(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::ode_solve(args).map_err(map_db_error)
}

pub fn chemical_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::chemical_analysis(args).map_err(map_db_error)
}

pub fn chemical_descriptors(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::chemical_descriptors(args).map_err(map_db_error)
}

pub fn statistical_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::statistical_analysis(args).map_err(map_db_error)
}

pub fn financial_model(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::financial_model(args).map_err(map_db_error)
}

pub fn medical_score(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::medical_score(args).map_err(map_db_error)
}

pub fn clinical_risk(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::clinical_risk(args).map_err(map_db_error)
}

pub fn engineering_analysis_op(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::engineering_analysis(args).map_err(map_db_error)
}

pub fn computational_geometry(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::computational_geometry(args).map_err(map_db_error)
}

pub fn geometry_manifests(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::geometry_manifests(args).map_err(map_db_error)
}

pub fn geometric_algebra_op(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::geometric_algebra_op(args).map_err(map_db_error)
}

pub fn bioinformatics_align(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::bioinformatics_align(args).map_err(map_db_error)
}

pub fn conditioning_compile(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::conditioning_compile(args).map_err(map_db_error)
}

pub fn conditioning_validate(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::conditioning_validate(args).map_err(map_db_error)
}
