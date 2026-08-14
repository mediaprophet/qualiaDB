//! Choose resident vs range query from file size. Small graphs stay in RAM;
//! everything above the cap must use the range/BIDX path.

/// 4 MiB decoded-file threshold. Above this, callers must not `read_all_quins`.
pub const RESIDENT_QUERY_MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Q42QueryMode {
    Resident,
    Range,
}

impl Q42QueryMode {
    pub fn for_file_bytes(file_bytes: u64) -> Self {
        if file_bytes <= RESIDENT_QUERY_MAX_BYTES {
            Self::Resident
        } else {
            Self::Range
        }
    }

    pub fn allows_read_all_quins(self) -> bool {
        matches!(self, Self::Resident)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_files_may_reside_large_files_must_range() {
        assert_eq!(Q42QueryMode::for_file_bytes(1024), Q42QueryMode::Resident);
        assert_eq!(
            Q42QueryMode::for_file_bytes(RESIDENT_QUERY_MAX_BYTES + 1),
            Q42QueryMode::Range
        );
        assert!(!Q42QueryMode::Range.allows_read_all_quins());
    }
}
