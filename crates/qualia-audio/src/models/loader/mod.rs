//! Common P64 learned-head loader — fails closed (`BackendUnavailable`) without weights.
//! Re-exports only (AU-LEARNED). See ADR 007.

pub mod fail_closed;
pub mod weight_file;

pub use fail_closed::{require_weights, WeightState};
pub use weight_file::{
    make_blob, parse_weight_blob, write_weight_blob, WeightBlob, MAX_DATA, MAX_DIMS, WEIGHT_MAGIC,
    WEIGHT_VERSION,
};
