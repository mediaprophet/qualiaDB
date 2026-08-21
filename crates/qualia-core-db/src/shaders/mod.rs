//! WGSL shader sources embedded in the Qualia WASM binary.
//!
//! - **Root** (`*.wgsl`) — U0/U1 compute (LLM, tensor query)
//! - **`wasm/`** — copies of WASM LLM inference shaders (fused attention/FFN/transformer, GEMV, dequant, top-K, ternary, LoRA)
//! - **`viewport/`** — U2 display (ambient, projector, epistemic, screen)

pub mod viewport;

pub const PGA_SCLERP_EVAL_WGSL: &str = include_str!("pga_sclerp_eval.wgsl");
pub const SPRING_DAMPER_GRID_WGSL: &str = include_str!("spring_damper_grid.wgsl");
pub const HUD_GLASS_BLUR_WGSL: &str = include_str!("hud_glass_blur.wgsl");
pub const WAVE_INTERFERENCE_WGSL: &str = include_str!("wave_interference.wgsl");
