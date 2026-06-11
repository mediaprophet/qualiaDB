//! Zero-Copy LoRA Multiplexing — context-driven neural adaptation.
//!
//! Maintains a single quantized base model in memory and streams tiny LoRA
//! (Low-Rank Adaptation) adapters based on context triggers encoded in the
//! `NQuin` 5th metadata vector.  Context switching takes <10 ms; the
//! extra memory footprint is ≤15 MB per cached adapter.
//!
//! # Architecture
//!
//! ```text
//! Prompt / NQuin
//!       │
//!       ▼
//!  ContextDetector  ──► ContextType (Medical / Legal / Chemical / …)
//!       │
//!       ▼
//!  LoRAAdapterManager (LRU-10 cache)
//!       ├── adapter_cache: HashMap<ContextType, LoRAAdapter>
//!       │      LoRAAdapter { lora_a: [rank × n_in], lora_b: [n_out × rank] }
//!       │
//!       ▼
//!  CPU apply: output += B @ (A @ x) * scaling
//!  GPU apply: lora_apply.wgsl dispatch (additive delta on hidden state)
//!       │
//!       ▼
//!  Modified hidden-state fed into fused_transformer.wgsl
//! ```
//!
//! # NQuin metadata encoding (bits 60–48)
//!
//! | Bits  | Field          | Notes                  |
//! |-------|----------------|------------------------|
//! | 63–60 | ContextType    | 0=General … 5=Technical |
//! | 59–56 | AdapterID      | 0–15 (4 bits)          |
//! | 55–48 | Confidence     | 0–255 → 0.0–1.0        |

pub mod adapter_manager;
pub mod context_detector;

#[cfg(not(target_arch = "wasm32"))]
pub mod webgpu_lora;

pub use adapter_manager::{LoRAAdapter, LoRAAdapterManager, LoRAError, LoRAMetadata, LoRATensor};
pub use context_detector::{ContextDetector, ContextType};
