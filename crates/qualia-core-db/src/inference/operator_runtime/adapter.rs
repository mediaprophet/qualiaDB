//! Prepared operator adapter binding representation digest and residency generation (W3: EOS-031).
//!
//! Provides the execution boundary connecting companion packages with the inference runtime:
//! - Binds immutable representation digest into residency identity.
//! - Checks residency generation before every execution (invalidated/stale plans fail closed).
//! - Zero heap allocation during execution hot path.
//! - Strictly no dependencies on or modifications to `gguf_bridge` or `decode.rs`.

use std::fmt;
use qualia_inference_kernel::operators::{
    apply_into, validate_operator, MatrixView, MatrixViewMut, OperatorDescriptor, OperatorError,
    OperatorWorkspace, PayloadView,
};
use crate::inference::operator_package::{OperatorPackage, PackageError, SegmentError};

/// Errors occurring during operator preparation or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedOperatorError {
    OperatorNotFound(String),
    DigestMismatch { expected: u64, actual: u64 },
    StaleResidencyGeneration { expected: u64, actual: u64 },
    WorkspaceTooSmall { need: usize, actual: usize },
    InvalidShape,
    KernelError(OperatorError),
    PackageError(PackageError),
    SegmentError(SegmentError),
}

impl fmt::Display for PreparedOperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperatorNotFound(name) => write!(f, "operator '{name}' not found in package manifest"),
            Self::DigestMismatch { expected, actual } => {
                write!(f, "representation digest mismatch: expected {expected:#018x}, actual {actual:#018x}")
            }
            Self::StaleResidencyGeneration { expected, actual } => {
                write!(f, "residency generation mismatch: plan prepared with gen {expected}, current is {actual}")
            }
            Self::WorkspaceTooSmall { need, actual } => {
                write!(f, "workspace too small: need {need} floats, provided {actual}")
            }
            Self::InvalidShape => write!(f, "input or output buffer length mismatch with operator dimensions"),
            Self::KernelError(ke) => write!(f, "kernel error during operator execution: {ke:?}"),
            Self::PackageError(pe) => write!(f, "package error: {pe}"),
            Self::SegmentError(se) => write!(f, "segment error: {se}"),
        }
    }
}

impl std::error::Error for PreparedOperatorError {}

/// An operator prepared for execution within a residency generation.
#[derive(Debug, Clone)]
pub struct PreparedOperator {
    pub name: String,
    pub descriptor: OperatorDescriptor,
    pub representation_digest: u64,
    pub residency_generation: u64,
    pub primary_payload: Vec<u8>,
}

impl PreparedOperator {
    /// Prepare an operator from an existing validated companion package.
    pub fn prepare(
        package: &OperatorPackage,
        operator_name: &str,
        residency_generation: u64,
    ) -> Result<Self, PreparedOperatorError> {
        let op_record = package
            .manifest()
            .operators
            .iter()
            .find(|op| op.name == operator_name)
            .ok_or_else(|| PreparedOperatorError::OperatorNotFound(operator_name.to_string()))?;

        // Verify that representation digest matches the manifest
        if op_record.descriptor.representation_digest != package.manifest().representation_digest
            && op_record.descriptor.representation_digest != 0
        {
            return Err(PreparedOperatorError::DigestMismatch {
                expected: package.manifest().representation_digest,
                actual: op_record.descriptor.representation_digest,
            });
        }

        // Fetch and copy primary segment bytes for prepared residency
        let seg_view = package
            .get_segment_view(op_record.primary_segment_id)
            .map_err(PreparedOperatorError::SegmentError)?;

        Ok(Self {
            name: operator_name.to_string(),
            descriptor: op_record.descriptor,
            representation_digest: op_record.descriptor.representation_digest,
            residency_generation,
            primary_payload: seg_view.as_slice().to_vec(),
        })
    }

    /// Execute the prepared operator against caller-supplied input, output, and workspace buffers.
    ///
    /// Fails closed immediately if `current_generation` does not match `self.residency_generation`.
    pub fn execute(
        &self,
        input: &[f32],
        output: &mut [f32],
        workspace_numeric: &mut [f32],
        current_generation: u64,
    ) -> Result<(), PreparedOperatorError> {
        if current_generation != self.residency_generation {
            return Err(PreparedOperatorError::StaleResidencyGeneration {
                expected: self.residency_generation,
                actual: current_generation,
            });
        }

        let inn = self.descriptor.in_features as usize;
        let out_n = self.descriptor.out_features as usize;

        if input.len() != inn || output.len() != out_n {
            return Err(PreparedOperatorError::InvalidShape);
        }

        let payloads = [PayloadView {
            bytes: &self.primary_payload,
        }];
        let op_view = validate_operator(&self.descriptor, &payloads)
            .map_err(PreparedOperatorError::KernelError)?;

        let in_view = MatrixView {
            data: input,
            rows: 1,
            cols: inn,
        };
        let out_view = MatrixViewMut {
            data: output,
            rows: 1,
            cols: out_n,
        };
        let mut byte_scratch = [];
        let ws = OperatorWorkspace {
            numeric: workspace_numeric,
            bytes: &mut byte_scratch,
        };

        apply_into(&op_view, in_view, out_view, ws).map_err(PreparedOperatorError::KernelError)?;
        Ok(())
    }
}
