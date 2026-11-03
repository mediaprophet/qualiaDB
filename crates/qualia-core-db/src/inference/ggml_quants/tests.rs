use super::*;

#[test]
fn q4k_soa_roundtrip_matches_aos_dequant() {
    // Build one synthetic Q4_K superblock and check SoA dequant ≈ stock dequant.
    let mut aos = [0u8; 144];
    // d = 1.0 f16, dmin = 0.1 f16
    aos[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
    aos[2..4].copy_from_slice(&half::f16::from_f32(0.1).to_le_bytes());
    // scales: all 1 for sc, 0 for m (low 6 bits)
    for i in 0..4 {
        aos[4 + i] = 1;
        aos[8 + i] = 0;
    }
    for i in 0..4 {
        aos[12 + i] = 0;
    }
    // qs: low nibble = 3, high nibble = 5
    for i in 16..144 {
        aos[i] = 0x53;
    }
    let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
    q4k_block_to_soa(&aos, &mut soa).unwrap();
    let mut out_aos = [0f32; 256];
    let mut out_soa = [0f32; 256];
    dequant_q4_k(&aos, 256, &mut out_aos).unwrap();
    dequant_q4_k_soa(&soa, 256, &mut out_soa).unwrap();
    for i in 0..256 {
        let d = (out_aos[i] - out_soa[i]).abs();
        assert!(
            d < 1e-3,
            "elem {i}: aos={} soa={} δ={d}",
            out_aos[i],
            out_soa[i]
        );
    }
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K_SOA, 256), Some(160));
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K_SOA, 512), Some(320));
}

#[test]
fn quantize_f32_to_soa_roundtrip() {
    // Synthetic weights: 256 values with varied ranges per sub-block.
    let mut src = [0f32; 256];
    for j in 0..8 {
        let base = j as f32 * 0.1 - 0.4;
        let amp = (j as f32 + 1.0) * 0.05;
        for l in 0..32 {
            src[j * 32 + l] = base + amp * (l as f32 / 31.0);
        }
    }
    let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
    quantize_block_f32_to_q4_k_soa(&src, &mut soa);
    let mut deq = [0f32; 256];
    dequant_q4_k_soa(&soa, 256, &mut deq).unwrap();
    // Q4_K has 4-bit precision (~1/15 of the sub-block range). Check that
    // the round-trip error is within the quantization step size.
    for j in 0..8 {
        let sub_min = src[j * 32..j * 32 + 32]
            .iter()
            .cloned()
            .fold(f32::MAX, f32::min);
        let sub_max = src[j * 32..j * 32 + 32]
            .iter()
            .cloned()
            .fold(f32::MIN, f32::max);
        let step = (sub_max - sub_min) / 15.0;
        for l in 0..32 {
            let i = j * 32 + l;
            let err = (src[i] - deq[i]).abs();
            assert!(
                err <= step + 1e-4,
                "elem {i}: src={} deq={} err={} step={}",
                src[i],
                deq[i],
                err,
                step
            );
        }
    }
}

#[test]
fn quantize_f32_to_soa_tensor_roundtrip() {
    // 2 rows × 512 elements (2 blocks per row).
    let n_row_elems = 512;
    let n_rows = 2;
    let mut src = vec![0f32; n_row_elems * n_rows];
    for i in 0..src.len() {
        src[i] = ((i as f32) * 0.01 - 5.0).sin() * 0.5;
    }
    let dst_row = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_row_elems).unwrap();
    let mut out = vec![0u8; dst_row * n_rows];
    quantize_f32_to_q4_k_soa_tensor(&src, n_row_elems, n_rows, &mut out).unwrap();
    let mut deq = vec![0f32; n_row_elems * n_rows];
    for r in 0..n_rows {
        let off = r * dst_row;
        dequant_q4_k_soa(
            &out[off..off + dst_row],
            n_row_elems,
            &mut deq[r * n_row_elems..(r + 1) * n_row_elems],
        )
        .unwrap();
    }
    // Check average error is reasonable for 4-bit quantization.
    let mut total_err = 0.0;
    for i in 0..src.len() {
        total_err += (src[i] - deq[i]).abs();
    }
    let avg_err = total_err / src.len() as f32;
    assert!(avg_err < 0.05, "avg quantization error too high: {avg_err}");
}

#[test]
fn bf16_row_bytes_and_dequant() {
    // Gemma-4 norms/scales use ggml_type 30 (BF16): 2 bytes/elem.
    assert_eq!(ggml_row_bytes(GGML_TYPE_BF16, 1024), Some(2048));
    // 1.0_bf16 = 0x3F80, -2.0_bf16 = 0xC000
    let raw: [u8; 4] = [0x80, 0x3F, 0x00, 0xC0];
    let mut out = [0.0f32; 2];
    assert_eq!(
        dequantize_row_into(&raw, GGML_TYPE_BF16, 2, &mut out),
        Ok(2)
    );
    assert!((out[0] - 1.0).abs() < 1e-6, "got {}", out[0]);
    assert!((out[1] + 2.0).abs() < 1e-6, "got {}", out[1]);
    let info = GgufTensorInfo {
        dims: [128, 1, 0, 0],
        n_dims: 2,
        ggml_type: GGML_TYPE_BF16,
        byte_offset: 0,
    };
    assert_eq!(tensor_byte_len(&info), Some(256));
}

#[test]
fn q4_0_row_bytes_stride() {
    // hidden_dim=4096 → (4096/32)*18 = 2304
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_0, 4096), Some(2304));
}

#[test]
fn q5_0_row_bytes_stride() {
    // SmolLM2 attn_k row: hidden_dim=960 → (960/32)*22 = 660
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q5_0, 960), Some(660));
}

#[test]
fn ternary_tensor_byte_len_is_whole_blob_not_row_strided() {
    // A1b inc 2a: a ternary FFN tensor is ONE `[scale f32][5-trits/byte]` blob over all
    // dims[0]*dims[1] elements (per-tensor scale), so `tensor_byte_len`/`fetch_tensor_bytes`
    // must return the whole-blob length — NOT the (None) row-based path that choked before.
    let info = GgufTensorInfo {
        dims: [960, 2560, 0, 0], // SmolLM2 ffn_gate-shape
        n_dims: 2,
        ggml_type: crate::ternary::GGML_TYPE_TERNARY_158,
        byte_offset: 0,
    };
    let n = 960 * 2560;
    assert_eq!(
        tensor_byte_len(&info),
        Some(crate::ternary::ternary_blob_len(n))
    );

    // and fetch returns exactly that slice from a buffer holding a real ternary blob.
    let weights: Vec<f32> = (0..n).map(|i| (i as f32 * 0.001).sin()).collect();
    let blob = crate::ternary::ternary_blob(&weights);
    assert_eq!(blob.len(), crate::ternary::ternary_blob_len(n));
    let got = fetch_tensor_bytes(&blob, 0, &info).expect("ternary fetch must slice");
    assert_eq!(got.len(), blob.len());
    assert_eq!(&got[..4], &blob[..4]); // scale preserved at the front
}

#[test]
fn q5_0_dequant_matches_gguf_smollm2_row0() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
    if !path.exists() {
        return;
    }
    let mmap = std::fs::read(&path).expect("read gguf");
    let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
    let info = index.get_layer_tensors(0).attn_k.expect("blk.0.attn_k");
    let raw = fetch_tensor_bytes(&mmap, index.tensor_data_start, &info).expect("fetch attn_k");
    let row_bytes = ggml_row_bytes(GGML_TYPE_Q5_0, info.dims[0] as usize).unwrap();
    assert_eq!(row_bytes, 660);
    let mut out = [0f32; 960];
    dequantize_row_into(&raw[..row_bytes], GGML_TYPE_Q5_0, 960, &mut out).unwrap();
    // Reference: gguf-py dequantize_row_q5_0 on blk.0.attn_k.weight row 0
    let expected = [
        -0.0f32,
        -0.02502441,
        -0.10009766,
        -0.07507324,
        -0.05004883,
        -0.3503418,
        -0.0,
        0.05004883,
        -0.07507324,
        0.1751709,
    ];
    for (i, &exp) in expected.iter().enumerate() {
        assert!(
            (out[i] - exp).abs() < 1e-5,
            "elem {i}: got {} expected {}",
            out[i],
            exp
        );
    }
}

#[test]
fn smollm_q4km_tensor_types_and_q4k_row0() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
    if !path.exists() {
        return;
    }
    let mmap = std::fs::read(&path).expect("read gguf");
    let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
    let emb = index.token_embd_info().expect("token_embd");
    println!("token_embd type={} dims={:?}", emb.ggml_type, emb.dims);
    let lt = index.get_layer_tensors(0);
    for (name, info) in [
        ("attn_q", lt.attn_q),
        ("attn_k", lt.attn_k),
        ("attn_v", lt.attn_v),
        ("ffn_gate", lt.ffn_gate),
        ("ffn_down", lt.ffn_down),
    ] {
        if let Some(i) = info {
            println!("{name} type={} dims={:?}", i.ggml_type, i.dims);
        }
    }
    // token 504 = "The" in naked prompt
    let mut out = [0f32; 960];
    let n = index.dequantize_token_embedding_into(&mmap, 504, &mut out);
    assert_eq!(n, 960);
    println!(
        "token504 first10: {:?}",
        &out[..10]
            .iter()
            .map(|v| format!("{v:.6}"))
            .collect::<Vec<_>>()
    );
    assert!(out.iter().any(|&v| v.is_finite() && v != 0.0));
}

#[test]
fn q4_k_row_bytes_stride() {
    // hidden_dim=2560 → (2560/256)*144 = 1440
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K, 2560), Some(1440));
}

#[test]
fn q5_k_uses_its_high_bit_plane_and_scale_min_pairs() {
    let mut raw = [0u8; 176];
    raw[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
    raw[2..4].copy_from_slice(&half::f16::from_f32(0.5).to_le_bytes());
    // First low/high 32-value groups: scale=2/min=1 and scale=3/min=2.
    raw[4] = 2;
    raw[5] = 3;
    raw[8] = 1;
    raw[9] = 2;
    raw[16] = 0b0000_0010; // lane zero high bit for the second 32 values.
    raw[48] = 5 | (6 << 4);
    let mut out = [0.0f32; 256];
    assert_eq!(dequant_q5_k(&raw, 256, &mut out), Ok(256));
    assert!((out[0] - 9.5).abs() < 1e-6); // 2*5 - 0.5*1
    assert!((out[32] - 65.0).abs() < 1e-6); // 3*(6|16) - 0.5*2
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q5_K, 256), Some(176));
}

#[test]
fn q6_k_row_bytes_stride() {
    // Gemma 4B token_embd: hidden_dim=2560 → (2560/256)*210 = 2100
    assert_eq!(ggml_row_bytes(GGML_TYPE_Q6_K, 2560), Some(2100));
}

#[test]
fn q6_k_dequant_matches_gguf_smollm2_ffn_down_row0() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
    if !path.exists() {
        return;
    }
    let mmap = std::fs::read(&path).expect("read gguf");
    let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
    let info = index.get_layer_tensors(0).ffn_down.expect("blk.0.ffn_down");
    assert_eq!(info.ggml_type, GGML_TYPE_Q6_K);
    let raw =
        fetch_tensor_bytes(&mmap, index.tensor_data_start, &info).expect("fetch ffn_down");
    let row_bytes = ggml_row_bytes(GGML_TYPE_Q6_K, info.dims[0] as usize).unwrap();
    assert_eq!(row_bytes, 2100);
    let mut out = [0f32; 2560];
    dequantize_row_into(&raw[..row_bytes], GGML_TYPE_Q6_K, 2560, &mut out).unwrap();
    // Reference: llama.cpp dequantize_row_q6_K (signed int8 scales) on row 0
    let expected = [
        -0.11712998f32,
        -0.16535997,
        -0.0,
        0.15157998,
        -0.08956999,
        -0.02067,
        0.04133999,
        0.03445,
        0.17913999,
        0.22047997,
    ];
    for (i, &exp) in expected.iter().enumerate() {
        assert!(
            (out[i] - exp).abs() < 1e-5,
            "elem {i}: got {} expected {}",
            out[i],
            exp
        );
    }
    assert!(out.iter().any(|&v| v.is_finite() && v != 0.0));
}

#[test]
fn block_q6k_layout_matches_ggml() {
    assert_eq!(std::mem::size_of::<BlockQ6K>(), BLOCK_Q6K_BYTES);
    assert_eq!(std::mem::align_of::<BlockQ6K>(), 2);
}

#[test]
fn fetch_row_range_bounds() {
    use crate::gguf_sharder::GgufTensorInfo;
    let info = GgufTensorInfo {
        dims: [2560, 100, 0, 0],
        n_dims: 2,
        ggml_type: GGML_TYPE_Q6_K,
        byte_offset: 0,
    };
    let row = tensor_row_byte_len(&info).unwrap();
    let total = tensor_byte_len(&info).unwrap();
    assert_eq!(total, row * 100);
    let fake = vec![0u8; total];
    let chunk = fetch_tensor_row_range_bytes(&fake, 0, &info, 10, 8).unwrap();
    assert_eq!(chunk.len(), row * 8);
}

#[test]
fn tensor_byte_len_covers_all_expert_dimensions() {
    let info = GgufTensorInfo {
        dims: [32, 7, 11, 0],
        n_dims: 3,
        ggml_type: GGML_TYPE_F32,
        byte_offset: 0,
    };
    // 11 experts × 7 rows/expert × 32 f32 values/row.
    assert_eq!(tensor_byte_len(&info), Some(32 * 7 * 11 * 4));

    let ternary = GgufTensorInfo {
        ggml_type: crate::ternary::GGML_TYPE_TERNARY_158,
        ..info
    };
    assert_eq!(
        tensor_byte_len(&ternary),
        Some(crate::ternary::ternary_blob_len(32 * 7 * 11))
    );
}

#[test]
fn q8_0_block_roundtrip() {
    let mut block = [0u8; 34];
    block[0] = 0x00;
    block[1] = 0x3C; // f16 1.0
    for i in 0..32 {
        block[2 + i] = (i + 1) as u8;
    }
    let mut out = [0f32; 32];
    dequant_q8_0(&block, 32, &mut out).unwrap();
    assert!((out[0] - 1.0).abs() < 0.01);
    assert!((out[31] - 32.0).abs() < 0.01);
}

#[test]
fn iq4_xs_block_layout_and_lane_order_match_ggml() {
    assert_eq!(ggml_row_bytes(GGML_TYPE_IQ4_XS, 160), Some(136));
    let info = GgufTensorInfo {
        dims: [160, 2, 0, 0],
        n_dims: 2,
        ggml_type: GGML_TYPE_IQ4_XS,
        byte_offset: 0,
    };
    assert_eq!(tensor_byte_len(&info), Some(272));

    let mut block = [0x88u8; 136]; // nonlinear code 8 == +1
    block[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
    // group 0 scale code = 33: low four bits in scales_l, high two in scales_h.
    block[2..4].copy_from_slice(&2u16.to_le_bytes());
    block[4] = 1;
    let mut out = [0.0f32; 256];
    assert_eq!(
        dequantize_row_into(&block, GGML_TYPE_IQ4_XS, 256, &mut out),
        Ok(256)
    );
    assert!(out[..32]
        .iter()
        .all(|value| (*value - 1.0).abs() < f32::EPSILON));
    // The next group has the default scale code 0, thus -32 times code 8.
    assert!(out[32..64]
        .iter()
        .all(|value| (*value + 32.0).abs() < f32::EPSILON));
}

#[test]
fn iq4_nl_block_layout_and_lane_order_match_ggml() {
    assert_eq!(ggml_row_bytes(GGML_TYPE_IQ4_NL, 160), Some(90));
    let mut block = [0x88u8; 18]; // code 8 is +1 in the non-linear table.
    block[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
    let mut out = [0.0f32; 32];
    assert_eq!(
        dequantize_row_into(&block, GGML_TYPE_IQ4_NL, 32, &mut out),
        Ok(32)
    );
    assert!(out.iter().all(|value| (*value - 1.0).abs() < f32::EPSILON));
}
