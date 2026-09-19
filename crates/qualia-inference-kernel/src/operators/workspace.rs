//! Caller-owned scratch. Capacity is reported before any apply.

use super::error::OperatorError;
use super::view::OperatorView;

/// Schedule bucket. W1 implements decode batch-1 only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleKind {
    DecodeBatch1,
}

/// Exact scratch the caller must supply. Units are elements (`numeric_f32`) and bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceRequirement {
    pub numeric_f32: usize,
    pub bytes: usize,
}

/// Borrowed caller buffers. Not grown by the operator.
pub struct OperatorWorkspace<'a> {
    pub numeric: &'a mut [f32],
    pub bytes: &'a mut [u8],
}

/// Row-major f32 matrix view (borrowed).
#[derive(Debug, Clone, Copy)]
pub struct MatrixView<'a> {
    pub data: &'a [f32],
    pub rows: usize,
    pub cols: usize,
}

/// Mutable row-major f32 matrix view.
pub struct MatrixViewMut<'a> {
    pub data: &'a mut [f32],
    pub rows: usize,
    pub cols: usize,
}

/// Bytes required for `batch` under `schedule`. Does not allocate.
pub fn workspace_requirement(
    operator: &OperatorView<'_>,
    batch: usize,
    schedule: ScheduleKind,
) -> Result<WorkspaceRequirement, OperatorError> {
    if batch == 0 {
        return Err(OperatorError::UnsupportedShape);
    }
    match schedule {
        ScheduleKind::DecodeBatch1 => {
            if batch != 1 {
                return Err(OperatorError::UnsupportedShape);
            }
            match operator.descriptor.kind {
                super::descriptor::OperatorKind::DenseF32 => Ok(WorkspaceRequirement {
                    numeric_f32: 0,
                    bytes: 0,
                }),
                super::descriptor::OperatorKind::Q4KBitPlane => {
                    let inn = operator.descriptor.in_features as usize;
                    let need_f32 = super::lookup::q4k_lookup_workspace_floats(inn)?;
                    Ok(WorkspaceRequirement {
                        numeric_f32: need_f32,
                        bytes: 0,
                    })
                }
            }
        }
    }
}

impl OperatorWorkspace<'_> {
    pub fn check(&self, need: WorkspaceRequirement) -> Result<(), OperatorError> {
        if self.numeric.len() < need.numeric_f32 || self.bytes.len() < need.bytes {
            return Err(OperatorError::WorkspaceTooSmall);
        }
        Ok(())
    }
}

impl MatrixView<'_> {
    pub fn len(&self) -> usize {
        self.rows.saturating_mul(self.cols)
    }

    pub fn check_len(&self) -> Result<(), OperatorError> {
        if self.data.len() < self.len() {
            return Err(OperatorError::UnsupportedShape);
        }
        Ok(())
    }
}

impl MatrixViewMut<'_> {
    pub fn len(&self) -> usize {
        self.rows.saturating_mul(self.cols)
    }

    pub fn check_len(&self) -> Result<(), OperatorError> {
        if self.data.len() < self.len() {
            return Err(OperatorError::UnsupportedShape);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::descriptor::{
        AccumKind, OperatorDescriptor, OperatorKind, ScaleLayout,
    };
    use crate::operators::view::{validate_operator, PayloadView};

    #[test]
    fn batch_zero_rejected() {
        let desc = OperatorDescriptor {
            kind: OperatorKind::DenseF32,
            in_features: 2,
            out_features: 2,
            batch_hint: 1,
            tile_elems: 2,
            scale_layout: ScaleLayout::None,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        };
        let buf = [0u8; 16];
        let payloads = [PayloadView { bytes: &buf }];
        let view = validate_operator(&desc, &payloads).unwrap();
        assert_eq!(
            workspace_requirement(&view, 0, ScheduleKind::DecodeBatch1).err(),
            Some(OperatorError::UnsupportedShape)
        );
    }

    #[test]
    fn undersized_workspace_fails_check() {
        let mut n = [0f32; 1];
        let mut b = [0u8; 0];
        let ws = OperatorWorkspace {
            numeric: &mut n,
            bytes: &mut b,
        };
        assert_eq!(
            ws.check(WorkspaceRequirement {
                numeric_f32: 4,
                bytes: 0,
            }),
            Err(OperatorError::WorkspaceTooSmall)
        );
    }
}
