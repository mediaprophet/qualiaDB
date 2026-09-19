//! Borrowed, validated views over operator payloads.

use super::descriptor::{
    OperatorDescriptor, OperatorKind, Q4K_SUPERBLOCK_BYTES, Q4K_SUPERBLOCK_ELEMS,
};
use super::error::OperatorError;

/// One immutable payload segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadView<'a> {
    pub bytes: &'a [u8],
}

/// Validated operator: descriptor plus borrowed payloads.
#[derive(Debug, Clone, Copy)]
pub struct OperatorView<'a> {
    pub descriptor: OperatorDescriptor,
    pub payloads: &'a [PayloadView<'a>],
}

/// Validate shape and payload lengths. Does not write output.
pub fn validate_operator<'a>(
    descriptor: &OperatorDescriptor,
    payloads: &'a [PayloadView<'a>],
) -> Result<OperatorView<'a>, OperatorError> {
    descriptor.check_shape()?;
    match descriptor.kind {
        OperatorKind::DenseF32 => validate_dense(descriptor, payloads)?,
        OperatorKind::Q4KBitPlane => validate_q4k(descriptor, payloads)?,
    }
    Ok(OperatorView {
        descriptor: *descriptor,
        payloads,
    })
}

fn validate_dense(
    descriptor: &OperatorDescriptor,
    payloads: &[PayloadView<'_>],
) -> Result<(), OperatorError> {
    let needed = descriptor
        .matrix_elems()?
        .checked_mul(4)
        .ok_or(OperatorError::UnsupportedShape)?;
    let bytes = first_payload(payloads)?;
    if bytes.len() != needed {
        return Err(OperatorError::InvalidPayload);
    }
    Ok(())
}

fn validate_q4k(
    descriptor: &OperatorDescriptor,
    payloads: &[PayloadView<'_>],
) -> Result<(), OperatorError> {
    let n_elems = descriptor.matrix_elems()?;
    let n_blocks = n_elems.div_ceil(Q4K_SUPERBLOCK_ELEMS as usize);
    let needed = n_blocks
        .checked_mul(Q4K_SUPERBLOCK_BYTES)
        .ok_or(OperatorError::UnsupportedShape)?;
    let bytes = first_payload(payloads)?;
    if bytes.len() < needed {
        return Err(OperatorError::InvalidPayload);
    }
    Ok(())
}

fn first_payload<'a>(payloads: &'a [PayloadView<'a>]) -> Result<&'a [u8], OperatorError> {
    match payloads.first() {
        Some(p) => Ok(p.bytes),
        None => Err(OperatorError::InvalidPayload),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::descriptor::{AccumKind, ScaleLayout};

    fn dense_desc() -> OperatorDescriptor {
        OperatorDescriptor {
            kind: OperatorKind::DenseF32,
            in_features: 2,
            out_features: 2,
            batch_hint: 1,
            tile_elems: 2,
            scale_layout: ScaleLayout::None,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        }
    }

    #[test]
    fn dense_payload_length_must_match() {
        let desc = dense_desc();
        let buf = [0u8; 16];
        let payloads = [PayloadView { bytes: &buf }];
        assert!(validate_operator(&desc, &payloads).is_ok());
        let short = [PayloadView {
            bytes: &buf[..12],
        }];
        assert_eq!(
            validate_operator(&desc, &short).err(),
            Some(OperatorError::InvalidPayload)
        );
    }

    #[test]
    fn empty_payloads_rejected() {
        let desc = dense_desc();
        assert_eq!(
            validate_operator(&desc, &[]).err(),
            Some(OperatorError::InvalidPayload)
        );
    }
}
