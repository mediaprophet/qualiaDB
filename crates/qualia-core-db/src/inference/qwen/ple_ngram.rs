#[cfg(test)]
use crate::gguf_sharder::Qwen4ExpPleConfig;
#[cfg(test)]
use qualia_inference_kernel::qwen::ple::PleNgramConfigSource;
// Compatibility with the canonical inference PLE selector.
pub use etalon_inference::qwen::ple_ngram::*;
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
    #[test]
    fn host_projection_borrows_exact_arrays_and_preserves_completeness() {
        let mut cfg = config();
        cfg.layer_count = 0;
        cfg.layers = [u32::MAX; 8];
        let view = cfg.ple_ngram_config();
        assert!(core::ptr::eq(
            view.layer_multipliers,
            &cfg.layer_multipliers
        ));
        assert!(core::ptr::eq(view.head_offsets, &cfg.head_offsets));
        assert!(core::ptr::eq(view.head_vocab_sizes, &cfg.head_vocab_sizes));
        assert_eq!(view.ngram_size, cfg.ngram_size);
        assert_eq!(view.heads_per_ngram, cfg.heads_per_ngram);
        assert_eq!(view.eos_token_id, cfg.eos_token_id);
        assert_eq!(view.embedding_row_width, cfg.embedding_row_width);
        assert_eq!(view.multiplier_count, cfg.multiplier_count);
        assert_eq!(view.head_count, cfg.head_count);
        assert_eq!(view.is_complete(), cfg.is_complete());
        for ngram in [0, 1, 2, 8, 9, u32::MAX] {
            cfg.ngram_size = ngram;
            assert_eq!(cfg.ple_ngram_config().is_complete(), cfg.is_complete());
        }
        cfg = config();
        cfg.embedding_row_width = 0;
        assert!(!cfg.ple_ngram_config().is_complete());
    }

    #[test]
    fn concrete_legacy_calls_share_canonical_history_and_error_types() {
        use qualia_inference_kernel::qwen::ple as canonical;
        type Select = fn(
            &Qwen4ExpPleConfig,
            u32,
            &PleTokenHistory,
            u64,
            &mut [u64],
        ) -> Result<usize, PleNgramError>;
        type Advance = fn(
            &mut PleTokenHistory,
            &Qwen4ExpPleConfig,
            u32,
            u64,
            &mut [u64],
        ) -> Result<usize, PleNgramError>;
        let select: Select = select_rows;
        let advance: Advance = PleTokenHistory::select_and_push;
        let cfg = config();
        let mut history: canonical::PleTokenHistory = PleTokenHistory::default();
        let mut rows = [u64::MAX; 6];
        let mut expected = rows;
        assert_eq!(
            select(&cfg, 7, &history, 60, &mut rows),
            canonical::select_rows(&cfg, 7, &history, 60, &mut expected)
        );
        assert_eq!(rows, expected);
        assert_eq!(advance(&mut history, &cfg, 7, 60, &mut rows), Ok(4));
        let error: canonical::PleNgramError = select(&cfg, 8, &history, 60, &mut []).unwrap_err();
        assert_eq!(error, PleNgramError::OutputTooSmall);
        assert_eq!(MAX_PLE_NGRAM, canonical::MAX_PLE_NGRAM);
        assert_eq!(MAX_PLE_HEADS, canonical::MAX_PLE_HEADS);
    }
}
