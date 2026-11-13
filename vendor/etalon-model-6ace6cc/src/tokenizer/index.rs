use super::{budget::Work, MergeEntry, TokenizerError, VocabEntry};
use core::cmp::Ordering;

pub(super) fn sort<T>(
    values: &mut [T],
    work: &mut Work,
    compare: impl Fn(&T, &T, &mut Work) -> Result<Ordering, TokenizerError>,
) -> Result<(), TokenizerError> {
    fn sift<T>(
        v: &mut [T],
        mut root: usize,
        end: usize,
        work: &mut Work,
        compare: &impl Fn(&T, &T, &mut Work) -> Result<Ordering, TokenizerError>,
    ) -> Result<(), TokenizerError> {
        while root < end / 2 {
            work.charge(1)?;
            let mut child = root * 2 + 1;
            if child + 1 < end && compare(&v[child], &v[child + 1], work)? == Ordering::Less {
                child += 1;
            }
            if compare(&v[root], &v[child], work)? != Ordering::Less {
                break;
            }
            work.charge(1)?;
            v.swap(root, child);
            root = child;
        }
        Ok(())
    }
    for root in (0..values.len() / 2).rev() {
        sift(values, root, values.len(), work, &compare)?;
    }
    for end in (1..values.len()).rev() {
        work.charge(1)?;
        values.swap(0, end);
        sift(values, 0, end, work, &compare)?;
    }
    Ok(())
}

pub(super) fn vocab(
    entries: &[VocabEntry<'_>],
    text: &[u8],
    work: &mut Work,
) -> Result<Option<u32>, TokenizerError> {
    let (mut lo, mut hi) = (0, entries.len());
    while lo < hi {
        work.charge(1)?;
        let mid = lo + (hi - lo) / 2;
        if work.compare(entries[mid].text.as_bytes(), text)? == Ordering::Greater {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    if lo != 0 && work.equal(entries[lo - 1].text.as_bytes(), text)? {
        Ok(Some(entries[lo - 1].id))
    } else {
        Ok(None)
    }
}

pub(super) fn merge(
    entries: &[MergeEntry<'_>],
    left: &[u8],
    right: &[u8],
    work: &mut Work,
) -> Result<Option<usize>, TokenizerError> {
    let (mut lo, mut hi) = (0, entries.len());
    while lo < hi {
        work.charge(1)?;
        let mid = lo + (hi - lo) / 2;
        let e = &entries[mid];
        let first = work.compare(e.left.as_bytes(), left)?;
        let order = if first == Ordering::Equal {
            work.compare(e.right.as_bytes(), right)?
        } else {
            first
        };
        if order == Ordering::Less {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if let Some(e) = entries.get(lo) {
        if work.equal(e.left.as_bytes(), left)? && work.equal(e.right.as_bytes(), right)? {
            return Ok(Some(e.rank));
        }
    }
    Ok(None)
}
