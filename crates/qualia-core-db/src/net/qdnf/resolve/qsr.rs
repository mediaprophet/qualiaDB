//! QSR scoped exact lookup. Authenticated covers; not a Kademlia overlay.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub const RADIX_DIGITS: usize = 96;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverInterval {
    pub start: u8,
    pub end: u8,
}

pub fn validate_cover(children: &[CoverInterval]) -> Result<(), QdnfError> {
    if children.is_empty() {
        return Err(QdnfError::Incomplete);
    }
    let mut prev_end: Option<u8> = None;
    for child in children {
        if child.end < child.start {
            return Err(QdnfError::Malformed);
        }
        if child.end as usize >= RADIX_DIGITS {
            return Err(QdnfError::Range);
        }
        if let Some(end) = prev_end {
            if child.start <= end {
                return Err(QdnfError::Overlap);
            }
            if child.start > end + 1 {
                return Err(QdnfError::Incomplete);
            }
        }
        prev_end = Some(child.end);
    }
    Ok(())
}

pub fn lookup_exact(key: &StrongDigest, covers: &[CoverInterval]) -> Result<bool, QdnfError> {
    validate_cover(covers)?;
    let digit = key.0[0] % RADIX_DIGITS as u8;
    Ok(covers
        .iter()
        .any(|c| digit >= c.start && digit <= c.end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_cover_is_rejected() {
        let children = [
            CoverInterval { start: 0, end: 10 },
            CoverInterval { start: 8, end: 20 },
        ];
        assert_eq!(validate_cover(&children), Err(QdnfError::Overlap));
    }
}
