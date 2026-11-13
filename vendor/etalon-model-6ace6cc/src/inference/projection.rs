//! Shared matrix projection descriptor with optional per-row scales.
use super::TensorInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projection {
    pub weight: TensorInfo,
    pub row_scale: Option<TensorInfo>,
}

impl Projection {
    pub fn info(&self) -> TensorInfo {
        self.weight
    }
    pub fn row_scales(&self) -> Option<&TensorInfo> {
        self.row_scale.as_ref()
    }
    pub fn cols(&self) -> usize {
        self.weight.dims[0] as usize
    }
    pub fn rows(&self) -> usize {
        self.weight.dims[1] as usize
    }
}
