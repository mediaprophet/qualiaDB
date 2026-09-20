//! Exact Qwen4Exp PLE row selection.
//!
//! The PLE table is a trained, hash-addressed n-gram embedding.  This module
//! computes its selected rows using only caller-provided token history and
//! fixed arrays; it never materialises the table or substitutes a generic
//! string hash.

use crate::gguf_sharder::Qwen4ExpPleConfig;

pub const MAX_PLE_NGRAM: usize = 8;
pub const MAX_PLE_HEADS: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PleTokenHistory {
    previous: [u32; MAX_PLE_NGRAM - 1],
    count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PleNgramError {
    IncompleteConfig,
    UnsupportedNgramSize,
    OutputTooSmall,
    RowOutOfRange,
}

impl core::fmt::Display for PleNgramError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IncompleteConfig => write!(f, "Qwen4Exp PLE metadata is incomplete"),
            Self::UnsupportedNgramSize => write!(f, "Qwen4Exp PLE n-gram size is unsupported"),
            Self::OutputTooSmall => write!(f, "PLE row-ID output buffer is too small"),
            Self::RowOutOfRange => write!(f, "PLE row ID exceeds the extracted table"),
        }
    }
}

impl std::error::Error for PleNgramError {}

impl PleTokenHistory {
    /// Compute row IDs for `token` and then retain it for the following step.
    ///
    /// Missing history, and history preceding EOS, is represented by the
    /// checkpoint's EOS token. This is the reference `shift_right` behaviour:
    /// a n-gram never crosses an EOS boundary.
    pub fn select_and_push(
        &mut self,
        config: &Qwen4ExpPleConfig,
        token: u32,
        table_rows: u64,
        out: &mut [u64],
    ) -> Result<usize, PleNgramError> {
        let count = select_rows(config, token, self, table_rows, out)?;
        if token == config.eos_token_id {
            self.count = 0;
            return Ok(count);
        }
        let retain = self.previous.len();
        if self.count < retain {
            for index in (1..=self.count).rev() {
                self.previous[index] = self.previous[index - 1];
            }
            self.count += 1;
        } else {
            for index in (1..retain).rev() {
                self.previous[index] = self.previous[index - 1];
            }
        }
        self.previous[0] = token;
        Ok(count)
    }
}

/// Generate the model's global PLE row IDs for one token.
///
/// For a 3-gram model this emits eight bigram and eight trigram row IDs.  The
/// signed Euclidean remainder matches tensor-runtime modulo for a negative
/// i64 XOR result.
pub fn select_rows(
    config: &Qwen4ExpPleConfig,
    token: u32,
    history: &PleTokenHistory,
    table_rows: u64,
    out: &mut [u64],
) -> Result<usize, PleNgramError> {
    if !config.is_complete() {
        return Err(PleNgramError::IncompleteConfig);
    }
    let ngram_size = config.ngram_size as usize;
    if !(2..=MAX_PLE_NGRAM).contains(&ngram_size) {
        return Err(PleNgramError::UnsupportedNgramSize);
    }
    let required = (ngram_size - 1)
        .checked_mul(config.heads_per_ngram as usize)
        .ok_or(PleNgramError::OutputTooSmall)?;
    if required > MAX_PLE_HEADS || out.len() < required {
        return Err(PleNgramError::OutputTooSmall);
    }

    let mut shifted = [config.eos_token_id; MAX_PLE_NGRAM];
    shifted[0] = token;
    for shift in 1..ngram_size {
        if shift <= history.count {
            shifted[shift] = history.previous[shift - 1];
        }
    }

    let heads_per_ngram = config.heads_per_ngram as usize;
    let mut written = 0usize;
    for ngram in 2..=ngram_size {
        let mut mixed = (shifted[0] as i64).wrapping_mul(config.layer_multipliers[0]);
        for position in 1..ngram {
            mixed ^= (shifted[position] as i64).wrapping_mul(config.layer_multipliers[position]);
        }
        let first_head = (ngram - 2) * heads_per_ngram;
        for head in first_head..first_head + heads_per_ngram {
            let size = config.head_vocab_sizes[head];
            let offset = config.head_offsets[head];
            let row = mixed
                .rem_euclid(size)
                .checked_add(offset)
                .ok_or(PleNgramError::RowOutOfRange)?;
            if row < 0 || row as u64 >= table_rows {
                return Err(PleNgramError::RowOutOfRange);
            }
            out[written] = row as u64;
            written += 1;
        }
    }
    Ok(written)
}

/// Sort and deduplicate a selected row list in caller storage.
pub fn sort_dedup_rows(rows: &mut [u64]) -> usize {
    rows.sort_unstable();
    let mut kept = 0usize;
    for read in 0..rows.len() {
        if kept == 0 || rows[read] != rows[kept - 1] {
            rows[kept] = rows[read];
            kept += 1;
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Qwen4ExpPleConfig {
        let mut config = Qwen4ExpPleConfig {
            ngram_size: 3,
            heads_per_ngram: 2,
            eos_token_id: 99,
            embedding_row_width: 8,
            layer_count: 1,
            layers: [1; 8],
            multiplier_count: 3,
            layer_multipliers: [3, 5, 7, 0, 0, 0, 0, 0],
            head_count: 4,
            head_offsets: [0; 32],
            head_vocab_sizes: [0; 32],
        };
        config.head_offsets[..4].copy_from_slice(&[0, 11, 24, 41]);
        config.head_vocab_sizes[..4].copy_from_slice(&[11, 13, 17, 19]);
        config
    }

    #[test]
    fn rows_are_stable_and_stay_in_their_head_ranges() {
        let cfg = config();
        let mut history = PleTokenHistory::default();
        let mut rows = [0u64; 4];
        assert_eq!(history.select_and_push(&cfg, 7, 60, &mut rows).unwrap(), 4);
        assert!(rows[0] < 11 && (11..24).contains(&rows[1]));
        assert!((24..41).contains(&rows[2]) && (41..60).contains(&rows[3]));
        let first = rows;
        let mut fresh = PleTokenHistory::default();
        assert_eq!(fresh.select_and_push(&cfg, 7, 60, &mut rows).unwrap(), 4);
        assert_eq!(rows, first);
    }

    #[test]
    fn eos_resets_history_before_the_following_token() {
        let cfg = config();
        let mut history = PleTokenHistory::default();
        let mut rows = [0u64; 4];
        history.select_and_push(&cfg, 7, 60, &mut rows).unwrap();
        history
            .select_and_push(&cfg, cfg.eos_token_id, 60, &mut rows)
            .unwrap();
        let after_eos = {
            history.select_and_push(&cfg, 8, 60, &mut rows).unwrap();
            rows
        };
        let mut fresh = PleTokenHistory::default();
        fresh.select_and_push(&cfg, 8, 60, &mut rows).unwrap();
        assert_eq!(after_eos, rows);
    }

    #[test]
    fn dedup_is_in_place() {
        let mut rows = [9, 2, 9, 7, 2];
        assert_eq!(sort_dedup_rows(&mut rows), 3);
        assert_eq!(&rows[..3], &[2, 7, 9]);
    }
}
