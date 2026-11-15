use super::BuildError;

/// Precise planning context; `kind` is the unchanged legacy error category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanError {
    pub kind: BuildError,
    pub context: PlanErrorContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanErrorContext {
    pub field: PlanField,
    pub tensor_index: Option<usize>,
    pub operation: PlanOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanOperation {
    Validate,
    Add,
    Multiply,
    Align,
    ConvertOffset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanField {
    TensorCount,
    EntryScratch,
    PageSize,
    ProfileFlags,
    ProfileSections,
    ManifoldCount,
    ScheduleTable,
    TensorTable,
    StringTable,
    ManifoldTable,
    Tokenizer,
    ChecksumTable,
    BlobRegion,
    TensorRank,
    TensorManifold,
    TensorName,
    TensorBlobSize,
    TensorBlobOffset,
    TensorBlobEnd,
    ContainerEnd,
    OutputBudget,
}

impl core::fmt::Display for PlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at {:?}", self.kind, self.context)
    }
}

impl core::error::Error for PlanError {}

impl PlanErrorContext {
    pub(super) fn at(field: PlanField, tensor_index: Option<usize>) -> Self {
        Self {
            field,
            tensor_index,
            operation: PlanOperation::Validate,
        }
    }

    pub(super) fn error(self, kind: BuildError) -> PlanError {
        PlanError {
            kind,
            context: self,
        }
    }

    fn arithmetic(self, operation: PlanOperation) -> PlanError {
        Self { operation, ..self }.error(BuildError::ArithmeticOverflow)
    }

    pub(super) fn add(self, left: usize, right: usize) -> Result<usize, PlanError> {
        left.checked_add(right)
            .ok_or_else(|| self.arithmetic(PlanOperation::Add))
    }

    pub(super) fn mul(self, left: usize, right: usize) -> Result<usize, PlanError> {
        left.checked_mul(right)
            .ok_or_else(|| self.arithmetic(PlanOperation::Multiply))
    }

    pub(super) fn align(self, value: usize, alignment: usize) -> Result<usize, PlanError> {
        value
            .checked_add(alignment - 1)
            .map(|n| n & !(alignment - 1))
            .ok_or_else(|| self.arithmetic(PlanOperation::Align))
    }

    pub(super) fn offset(self, value: usize) -> Result<u32, PlanError> {
        u32::try_from(value).map_err(|_| {
            Self {
                operation: PlanOperation::ConvertOffset,
                ..self
            }
            .error(BuildError::OffsetLimit)
        })
    }

    pub(super) fn usize(self, value: u64) -> Result<usize, PlanError> {
        usize::try_from(value).map_err(|_| {
            Self {
                operation: PlanOperation::ConvertOffset,
                ..self
            }
            .error(BuildError::OffsetLimit)
        })
    }
}
