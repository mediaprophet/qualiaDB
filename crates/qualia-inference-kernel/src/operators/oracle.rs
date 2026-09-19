//! Scalar `apply_into` for DenseF32. Q4_K lookup is EOS-030, not this packet.

use super::descriptor::OperatorKind;
use super::error::OperatorError;
use super::view::OperatorView;
use super::workspace::{
    workspace_requirement, MatrixView, MatrixViewMut, OperatorWorkspace, ScheduleKind,
};

/// Overwrite `output` with `Y = W X` for the declared operator.
///
/// Accumulation is IEEE-754 f32, left-to-right over `in_features` for each output row,
/// then over batch columns. On error, `output` is not written.
pub fn apply_into(
    operator: &OperatorView<'_>,
    input: MatrixView<'_>,
    mut output: MatrixViewMut<'_>,
    workspace: OperatorWorkspace<'_>,
) -> Result<(), OperatorError> {
    let batch = 1usize;
    let need = workspace_requirement(operator, batch, ScheduleKind::DecodeBatch1)?;
    workspace.check(need)?;
    input.check_len()?;
    output.check_len()?;

    let inn = operator.descriptor.in_features as usize;
    let out_n = operator.descriptor.out_features as usize;

    match operator.descriptor.kind {
        OperatorKind::DenseF32 => apply_dense_f32(operator, inn, out_n, input, &mut output),
        OperatorKind::Q4KBitPlane => {
            super::lookup::apply_q4k_lookup(operator, inn, out_n, input, &mut output, workspace)
        }
    }
}

fn apply_dense_f32(
    operator: &OperatorView<'_>,
    inn: usize,
    out_n: usize,
    input: MatrixView<'_>,
    output: &mut MatrixViewMut<'_>,
) -> Result<(), OperatorError> {
    // Decode batch-1: x is an `inn` vector; y is `out_n`.
    if input.rows * input.cols != inn && !(input.rows == inn && input.cols == 1) {
        if !(input.rows == 1 && input.cols == inn) {
            return Err(OperatorError::UnsupportedShape);
        }
    }
    if output.rows * output.cols != out_n && !(output.rows == out_n && output.cols == 1) {
        if !(output.rows == 1 && output.cols == out_n) {
            return Err(OperatorError::UnsupportedShape);
        }
    }
    let weights = operator
        .payloads
        .first()
        .map(|p| p.bytes)
        .ok_or(OperatorError::InvalidPayload)?;
    let need_w = out_n
        .checked_mul(inn)
        .and_then(|e| e.checked_mul(4))
        .ok_or(OperatorError::UnsupportedShape)?;
    if weights.len() != need_w {
        return Err(OperatorError::InvalidPayload);
    }

    for o in 0..out_n {
        let mut acc = 0.0f32;
        let row = o * inn;
        for i in 0..inn {
            let w = read_f32_le(weights, row + i);
            acc += w * input.data[i];
        }
        output.data[o] = acc;
    }
    Ok(())
}

#[inline]
fn read_f32_le(bytes: &[u8], elem: usize) -> f32 {
    let o = elem * 4;
    f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::descriptor::{
        AccumKind, OperatorDescriptor, OperatorKind, ScaleLayout,
    };
    use crate::operators::view::{validate_operator, PayloadView};

    fn pack_row_major(vals: &[f32], out: &mut [u8]) {
        for (i, &v) in vals.iter().enumerate() {
            let b = v.to_le_bytes();
            out[i * 4..i * 4 + 4].copy_from_slice(&b);
        }
    }

    #[test]
    fn dense_gemv_2x3() {
        // W = [[1, 2, 3], [4, 5, 6]], x = [1, 0, 1] → y = [4, 10]
        let desc = OperatorDescriptor {
            kind: OperatorKind::DenseF32,
            in_features: 3,
            out_features: 2,
            batch_hint: 1,
            tile_elems: 3,
            scale_layout: ScaleLayout::None,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        };
        let mut wbytes = [0u8; 24];
        pack_row_major(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &mut wbytes);
        let payloads = [PayloadView { bytes: &wbytes }];
        let view = validate_operator(&desc, &payloads).unwrap();
        let x = [1.0f32, 0.0, 1.0];
        let mut y = [99.0f32, 99.0];
        let mut n = [];
        let mut b = [];
        apply_into(
            &view,
            MatrixView {
                data: &x,
                rows: 3,
                cols: 1,
            },
            MatrixViewMut {
                data: &mut y,
                rows: 2,
                cols: 1,
            },
            OperatorWorkspace {
                numeric: &mut n,
                bytes: &mut b,
            },
        )
        .unwrap();
        assert_eq!(y, [4.0, 10.0]);
    }

    #[test]
    fn error_does_not_write_output() {
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
        let wbytes = [0u8; 16];
        let payloads = [PayloadView { bytes: &wbytes }];
        let view = validate_operator(&desc, &payloads).unwrap();
        let x = [1.0f32]; // too short
        let mut y = [7.0f32, 8.0];
        let mut n = [];
        let mut b = [];
        let err = apply_into(
            &view,
            MatrixView {
                data: &x,
                rows: 1,
                cols: 1,
            },
            MatrixViewMut {
                data: &mut y,
                rows: 2,
                cols: 1,
            },
            OperatorWorkspace {
                numeric: &mut n,
                bytes: &mut b,
            },
        )
        .err();
        assert_eq!(err, Some(OperatorError::UnsupportedShape));
        assert_eq!(y, [7.0, 8.0]);
    }

    #[test]
    fn q4k_apply_checks_workspace_and_computes() {
        let desc = OperatorDescriptor {
            kind: OperatorKind::Q4KBitPlane,
            in_features: 256,
            out_features: 1,
            batch_hint: 1,
            tile_elems: 256,
            scale_layout: ScaleLayout::GgmlQ4K,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        };
        let wbytes = [0u8; 144];
        let payloads = [PayloadView { bytes: &wbytes }];
        let view = validate_operator(&desc, &payloads).unwrap();
        let x = [0.0f32; 256];
        let mut y = [1.0f32];
        let mut n_undersized = [];
        let mut b = [];

        // Undersized workspace fails closed
        assert_eq!(
            apply_into(
                &view,
                MatrixView {
                    data: &x,
                    rows: 256,
                    cols: 1,
                },
                MatrixViewMut {
                    data: &mut y,
                    rows: 1,
                    cols: 1,
                },
                OperatorWorkspace {
                    numeric: &mut n_undersized,
                    bytes: &mut b,
                },
            )
            .err(),
            Some(OperatorError::WorkspaceTooSmall)
        );
        assert_eq!(y[0], 1.0);

        // Sufficient workspace succeeds
        let mut n_ok = vec![0.0f32; 8 * 513];
        assert!(apply_into(
            &view,
            MatrixView {
                data: &x,
                rows: 256,
                cols: 1,
            },
            MatrixViewMut {
                data: &mut y,
                rows: 1,
                cols: 1,
            },
            OperatorWorkspace {
                numeric: &mut n_ok,
                bytes: &mut b,
            },
        ).is_ok());
    }
}
