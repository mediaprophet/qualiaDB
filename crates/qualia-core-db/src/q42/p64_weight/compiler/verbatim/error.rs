use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GgufCompileBuffer {
    Order,
    Descriptors,
    Entries,
    Manifolds,
    Hparams,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GgufCompileError {
    NoTensors,
    Page(u16),
    LayerIndex,
    ManifoldCount,
    WorkspaceOverflow,
    NameCapacity,
    UnsupportedType(u32),
    SourceOffset,
    SourceLength,
    SourceBounds(usize),
    Dimension,
    Dtype,
    Capacity {
        buffer: GgufCompileBuffer,
        required: usize,
        available: usize,
    },
    ScratchBudget {
        required: usize,
        available: usize,
    },
    Planning(PlanError),
    Writing(BuildError),
}
impl core::fmt::Display for GgufCompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoTensors => f.write_str("p64: GGUF parse yielded no tensors"),
            Self::Page(page) => write!(f, "p64: page_log2 {page} out of range"),
            Self::LayerIndex => f.write_str("p64: layer index exceeds u16"),
            Self::ManifoldCount => f.write_str("p64: manifold count overflow"),
            Self::WorkspaceOverflow => f.write_str("p64: workspace size overflow"),
            Self::NameCapacity => f.write_str("p64: canonical tensor name exceeds workspace"),
            Self::UnsupportedType(dtype) => write!(f, "p64: unsupported GGML type {dtype}"),
            Self::SourceOffset => f.write_str("p64: source tensor offset overflow"),
            Self::SourceLength => f.write_str("p64: source tensor length overflow"),
            Self::SourceBounds(position) => {
                write!(f, "p64: source tensor {position} is out of bounds")
            }
            Self::Dimension => f.write_str("p64: tensor dimension exceeds u32"),
            Self::Dtype => f.write_str("p64: GGML type exceeds u16"),
            Self::Capacity {
                buffer,
                required,
                available,
            } => write!(
                f,
                "p64: {buffer:?} workspace needs {required}, available {available}"
            ),
            Self::ScratchBudget {
                required,
                available,
            } => write!(
                f,
                "p64: workspace budget needs {required}, available {available}"
            ),
            Self::Planning(error) => match (error.kind, error.context.field) {
                (BuildError::OffsetLimit, PlanField::TensorBlobOffset) => {
                    f.write_str("p64: container exceeds 4 GiB")
                }
                (BuildError::OffsetLimit, PlanField::TensorBlobSize) => {
                    f.write_str("p64: tensor exceeds 4 GiB")
                }
                (BuildError::OffsetLimit, PlanField::ContainerEnd) => {
                    f.write_str("p64: 32-bit relative-offset container exceeds 4 GiB")
                }
                (BuildError::ArithmeticOverflow, PlanField::TensorBlobEnd) => {
                    f.write_str("p64: container size overflow")
                }
                (_, PlanField::ScheduleTable) => f.write_str("p64: schedule overflow"),
                (_, PlanField::TensorTable) => f.write_str("p64: tensor table overflow"),
                (_, PlanField::ManifoldTable) => f.write_str("p64: manifold table overflow"),
                (_, PlanField::ChecksumTable) => f.write_str("p64: checksum table overflow"),
                (_, PlanField::TensorName | PlanField::StringTable) => {
                    f.write_str("p64: string table exceeds 4 GiB")
                }
                _ => write!(f, "p64: {error}"),
            },
            Self::Writing(error) => write!(f, "p64: {error}"),
        }
    }
}
impl std::error::Error for GgufCompileError {}
