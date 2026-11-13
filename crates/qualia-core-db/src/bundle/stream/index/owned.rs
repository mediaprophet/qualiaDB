//! Fallible cold materialization; no serde-owned decode before admission.
use super::{
    AdmittedIndex, CheckedHeader, EntryScratch, IndexError, IndexReadLimits, admit_index,
    measure_index,
};
use crate::bundle::BundleEntry;

fn text(value: &str) -> Result<String, IndexError> {
    let mut out = String::new();
    out.try_reserve_exact(value.len())
        .map_err(|_| IndexError::Allocation)?;
    out.push_str(value);
    Ok(out)
}
fn buffer(len: usize) -> Result<Vec<u8>, IndexError> {
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| IndexError::Allocation)?;
    out.resize(len, 0);
    Ok(out)
}

pub fn materialize_entries(
    index: &AdmittedIndex<'_>,
    l: IndexReadLimits,
) -> Result<Vec<BundleEntry>, IndexError> {
    // A stricter new budget must not bypass validation through an old handle.
    let sizing = measure_index(&index.header(), index.encoded_bytes(), l)?;
    let mut out = Vec::new();
    out.try_reserve_exact(sizing.entry_count())
        .map_err(|_| IndexError::Allocation)?;
    for row in index.entries() {
        let row = row?;
        let mut sha256 = buffer(32)?;
        sha256.copy_from_slice(row.sha256());
        let meta = if let Some(len) = row.metadata_len() {
            let mut bytes = buffer(len)?;
            row.copy_metadata_into(&mut bytes)?;
            Some(bytes)
        } else {
            None
        };
        out.push(BundleEntry {
            key: text(row.key())?,
            kind: text(row.kind())?,
            offset: row.offset(),
            length: row.length(),
            sha256,
            meta,
        });
    }
    Ok(out)
}

pub(crate) fn decode_owned_index(
    h: &CheckedHeader,
    bytes: &[u8],
    l: IndexReadLimits,
) -> Result<Vec<BundleEntry>, IndexError> {
    let sizing = measure_index(h, bytes, l)?;
    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(sizing.entry_count())
        .map_err(|_| IndexError::Allocation)?;
    scratch.resize(sizing.entry_count(), EntryScratch::EMPTY);
    let index = admit_index(h, bytes, l, &mut scratch)?;
    // The handle borrows immutable index bytes, not this temporary scratch.
    drop(scratch);
    materialize_entries(&index, l)
}
