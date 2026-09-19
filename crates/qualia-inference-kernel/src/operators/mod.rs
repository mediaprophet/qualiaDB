//! Portable executable-operator contracts (W1 / W3).
//!
//! No heap in `apply_into` / `workspace_requirement`. No `unsafe`.
//! W3 adds scalar Q4_K activation lookup apply (EOS-030).

mod descriptor;
mod error;
mod lookup;
mod oracle;
mod q4k;
mod view;
mod workspace;

pub use descriptor::{
    AccumKind, OperatorDescriptor, OperatorKind, ScaleLayout, Q4K_SUPERBLOCK_BYTES,
    Q4K_SUPERBLOCK_ELEMS,
};
pub use error::OperatorError;
pub use lookup::{
    apply_q4k_lookup, prepare_q4k_activation_table, q4k_lookup_workspace_floats,
    FLOATS_PER_SUBBLOCK_LUT,
};
pub use oracle::apply_into;
pub use q4k::reconstruct_q4k_into;
pub use view::{validate_operator, OperatorView, PayloadView};
pub use workspace::{
    workspace_requirement, MatrixView, MatrixViewMut, OperatorWorkspace, ScheduleKind,
    WorkspaceRequirement,
};
