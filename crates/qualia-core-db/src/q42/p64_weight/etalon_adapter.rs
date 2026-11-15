use super::{IntegrityMode, P64TensorIndex};
use etalon_model::p64::{P64Error, P64OverflowContext, P64View};

pub(super) fn parse_owned(data: &[u8], integrity: IntegrityMode) -> Result<P64TensorIndex, String> {
    let mode = match integrity {
        IntegrityMode::Full => etalon_model::p64::IntegrityMode::Full,
        IntegrityMode::Metadata => etalon_model::p64::IntegrityMode::Metadata,
        IntegrityMode::Structure => etalon_model::p64::IntegrityMode::Structure,
    };
    let view = P64View::parse(data, mode).map_err(legacy_error)?;
    // Preserve the existing cold, owned manifest API; weight bytes stay with the caller.
    let mut entries = Vec::with_capacity(view.tensor_count());
    for index in 0..view.tensor_count() {
        entries.push(view.tensor(index).map_err(legacy_error)?.entry());
    }
    Ok(P64TensorIndex {
        header: *view.header(),
        hparams: *view.hparams(),
        entries,
    })
}

fn legacy_error(error: P64Error) -> String {
    match error {
        P64Error::TruncatedHeader => "p64: truncated header".into(),
        P64Error::InvalidMagic => "p64: invalid magic".into(),
        P64Error::UnsupportedVersion(version) => format!("p64: unsupported version {version}"),
        P64Error::UnsupportedEndian => "p64: non-little-endian container is unsupported".into(),
        P64Error::InvalidPageSize => "p64: invalid page size".into(),
        P64Error::HyperparametersOutOfBounds => "p64: hyperparameters out of bounds".into(),
        P64Error::MetadataBounds => "p64: metadata sections overlap or are out of bounds".into(),
        P64Error::ManifoldAlignment => "p64: manifold table is not cache-line aligned".into(),
        P64Error::MetadataChecksum => "p64: metadata CRC-32C mismatch".into(),
        P64Error::InvalidRank(index) => format!("p64: tensor {index} has invalid rank"),
        P64Error::InvalidManifold(index) => {
            format!("p64: tensor {index} has invalid manifold index")
        }
        P64Error::InvalidName(index) => format!("p64: tensor {index} has invalid name"),
        P64Error::InvalidBlob(index) => {
            format!("p64: tensor {index} is unaligned, overlapping, or out of bounds")
        }
        P64Error::TensorChecksum(index) => format!("p64: tensor {index} CRC-32C mismatch"),
        P64Error::ArithmeticOverflow(context) => match context {
            P64OverflowContext::HyperparameterOffset => "p64: hyperparameter offset overflow",
            P64OverflowContext::TensorTable => "p64: tensor table overflow",
            P64OverflowContext::ManifoldCount => "p64: manifold count overflow",
            P64OverflowContext::ManifoldTable => "p64: manifold table overflow",
            P64OverflowContext::ChecksumTable => "p64: checksum table overflow",
            P64OverflowContext::TensorName => "p64: tensor name offset overflow",
            P64OverflowContext::TensorBlob => "p64: tensor blob overflow",
            P64OverflowContext::BlobAlignment => "p64: blob alignment overflow",
            P64OverflowContext::ManifoldRecord => "p64: manifold offset overflow",
            P64OverflowContext::ScheduleTable => "p64: schedule table overflow",
        }
        .into(),
        P64Error::IndexOutOfBounds => "p64: tensor index out of bounds".into(),
        P64Error::ScheduleBounds => "p64: schedule table out of bounds".into(),
    }
}
