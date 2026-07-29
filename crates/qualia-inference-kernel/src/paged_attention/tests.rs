use super::*;

#[test]
fn config_accounts_for_k_and_v_without_hidden_capacity() {
    let config = PagedKvConfig::new(2, 4, 64, 1024).expect("valid paged-KV shape");

    assert_eq!(config.logical_blocks_per_layer(), 64);
    assert_eq!(config.required_single_sequence_blocks(), 128);
    assert_eq!(config.block_elems(), 16 * 4 * 64 * 2);
    assert_eq!(
        config.arena_bytes(),
        Some(128 * 16 * 4 * 64 * 2 * core::mem::size_of::<f32>())
    );
}

#[test]
fn scalar_oracle_obeys_a_non_identity_page_table() {
    let mut config = PagedKvConfig::new(1, 1, 2, 32).expect("valid paged-KV shape");
    config.physical_blocks = 2;
    let mut arena = [0.0f32; 128];
    let block_table = [1u32, 0u32];

    // Logical token zero lives in physical page one.
    let page_one = config.block_elems();
    arena[page_one] = 1.0;
    arena[page_one + 1] = 0.0;
    arena[page_one + 2] = 7.0;
    arena[page_one + 3] = 11.0;

    let mut output = [0.0f32; 2];
    paged_gqa_attention_into(
        &[1.0, 0.0],
        &arena,
        &block_table,
        &config,
        0,
        1,
        &mut output,
    )
    .expect("attention succeeds");

    assert_eq!(output, [7.0, 11.0]);
}

#[test]
fn scalar_oracle_rejects_missing_pages() {
    let config = PagedKvConfig::new(1, 1, 2, 16).expect("valid paged-KV shape");
    let arena = [0.0f32; 64];
    let mut output = [0.0f32; 2];

    assert_eq!(
        paged_gqa_attention_into(
            &[1.0, 0.0],
            &arena,
            &[INVALID_BLOCK],
            &config,
            0,
            1,
            &mut output,
        ),
        Err(AttentionError::MissingPage)
    );
}
