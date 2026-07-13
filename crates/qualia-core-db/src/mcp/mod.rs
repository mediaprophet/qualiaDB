//! Model-Context-Protocol server surface (moved from the crate root per
//! MODULE_REORG_PLAN.md; `crate::mcp_server` / `crate::mcp_cooperation` paths are
//! preserved by re-exports in lib.rs). mcp_server owns the *_impls submodules.
#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_server;
// Cooperation gate composes modalities::interaction_governance, so it compiles
// only where modalities does (native or the logic-enabled wasm profiles).
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod mcp_cooperation;
