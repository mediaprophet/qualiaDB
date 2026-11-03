use crate::safetensor::SafeTensorPlan;
use etalon_model::p64::build::{
    BuildError, PayloadWriteRefusal, PlanError, PlanField, PlanOperation, StreamFailure,
    StreamFailureKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetensorTranscodeValidationError {
    Page(u16),
    Empty,
    DataStart,
    Offsets(usize),
    UnsupportedType(usize),
    LowPrecision(usize),
    UnsupportedWidth,
    ShapeOverflow(usize),
    ByteSizeOverflow(usize),
    ShapeMismatch {
        index: usize,
        declared: usize,
        expected: usize,
    },
    Dimension,
    TensorSize,
    NameTable,
    TensorTotal,
    WorkspaceOverflow,
    WorkspaceCapacity {
        field: &'static str,
        required: usize,
        available: usize,
    },
    ScratchBudget {
        required: usize,
        available: usize,
    },
}

#[derive(Debug)]
pub enum SafetensorTranscodeError {
    Validation(SafetensorTranscodeValidationError),
    Plan(PlanError),
    Stream(StreamFailure<PayloadWriteRefusal, std::io::Error>),
}

impl From<SafetensorTranscodeValidationError> for SafetensorTranscodeError {
    fn from(value: SafetensorTranscodeValidationError) -> Self {
        Self::Validation(value)
    }
}
impl core::fmt::Display for SafetensorTranscodeValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Safetensor transcode validation: {self:?}")
    }
}
impl core::error::Error for SafetensorTranscodeValidationError {}
impl core::fmt::Display for SafetensorTranscodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Safetensor transcode failure: {self:?}")
    }
}
impl core::error::Error for SafetensorTranscodeError {}

pub(super) fn validation_message(
    error: SafetensorTranscodeValidationError,
    plan: &SafeTensorPlan,
) -> String {
    use SafetensorTranscodeValidationError::*;
    match error {
        Page(page) => format!("p64: page_log2 {page} out of range"),
        Empty => "p64: Safetensors source contains no tensors".into(),
        DataStart => "safetensor: header length exceeds file".into(),
        Offsets(index) => format!(
            "safetensor: tensor '{}' offsets out of bounds",
            plan.tensors[index].name
        ),
        UnsupportedType(index) => format!(
            "p64: tensor '{}' dtype {} is not a supported high-fidelity source",
            plan.tensors[index].name, plan.tensors[index].dtype
        ),
        LowPrecision(index) => format!(
            "p64: tensor '{}' is low precision and cannot be transcoded",
            plan.tensors[index].name
        ),
        UnsupportedWidth => "p64: unsupported element width".into(),
        ShapeOverflow(index) => {
            format!("p64: tensor '{}' shape overflow", plan.tensors[index].name)
        }
        ByteSizeOverflow(index) => format!(
            "p64: tensor '{}' byte-size overflow",
            plan.tensors[index].name
        ),
        ShapeMismatch {
            index,
            declared,
            expected,
        } => format!(
            "p64: tensor '{}' declares {declared} bytes but shape/dtype requires {expected}",
            plan.tensors[index].name
        ),
        Dimension => "p64: tensor dimension exceeds u32".into(),
        TensorSize => "p64: tensor exceeds 4 GiB".into(),
        NameTable => "p64: string table exceeds 4 GiB".into(),
        TensorTotal => "p64: tensor byte total overflow".into(),
        other => other.to_string(),
    }
}

pub(super) fn legacy_message(error: SafetensorTranscodeError, plan: &SafeTensorPlan) -> String {
    match error {
        SafetensorTranscodeError::Validation(error) => validation_message(error, plan),
        SafetensorTranscodeError::Stream(StreamFailure {
            kind: StreamFailureKind::Sink(error),
            ..
        }) => error.to_string(),
        SafetensorTranscodeError::Plan(error) => {
            let message = match (error.kind, error.context.field, error.context.operation) {
                (BuildError::OffsetLimit, PlanField::TensorName, _) => {
                    Some("p64: string table exceeds 4 GiB")
                }
                (BuildError::OffsetLimit, PlanField::TensorBlobSize, _) => {
                    Some("p64: tensor exceeds 4 GiB")
                }
                (BuildError::OffsetLimit, PlanField::TensorBlobOffset, _) => {
                    Some("p64: container exceeds 4 GiB")
                }
                (BuildError::OffsetLimit, PlanField::ContainerEnd, _) => {
                    Some("p64: 32-bit relative-offset container exceeds 4 GiB")
                }
                (
                    BuildError::ArithmeticOverflow,
                    PlanField::EntryScratch,
                    PlanOperation::Multiply,
                ) => Some("p64: tensor table overflow"),
                (
                    BuildError::ArithmeticOverflow,
                    PlanField::ManifoldTable,
                    PlanOperation::Multiply,
                ) => Some("p64: manifold table overflow"),
                (
                    BuildError::ArithmeticOverflow,
                    PlanField::ChecksumTable,
                    PlanOperation::Multiply,
                ) => Some("p64: checksum table overflow"),
                (BuildError::ArithmeticOverflow, PlanField::TensorBlobEnd, PlanOperation::Add) => {
                    Some("p64: container size overflow")
                }
                _ => None,
            };
            message
                .map(str::to_owned)
                .unwrap_or_else(|| format!("p64: {error}"))
        }
        other => other.to_string(),
    }
}
