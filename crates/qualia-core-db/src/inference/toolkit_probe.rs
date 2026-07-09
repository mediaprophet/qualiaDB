//! Stage-by-stage **library toolkit probe** for the inference pipeline.
//!
//! Exercises existing functions (P64 convert, ModelHelper CBOR-LD, dequant,
//! stack GEMV, forge GEMM, top-k, ternary, passport) with simple timings.
//! Not a full decode bench — a map of what the library can do at each stage.
//!
//! ```text
//! cargo test -p qualia-core-db --lib toolkit_probe -- --nocapture
//! ```

#[cfg(test)]
mod tests {
    use std::time::Instant;

    // ── Stage 1: Import → P64 ────────────────────────────────────────────────

    #[test]
    fn stage1_import_convert_p64_verbatim_and_f16() {
        let gguf = minimal_gguf_f32_weight();
        assert!(gguf.starts_with(b"GGUF"));

        let t0 = Instant::now();
        let p64_v = crate::p64_weight::compile_gguf_to_p64_with_layout(
            &gguf,
            12,
            crate::p64_weight::P64ConvertLayout::Verbatim,
        )
        .expect("verbatim");
        let ms_v = t0.elapsed().as_secs_f64() * 1e3;
        assert!(crate::p64_weight::has_p64_magic(&p64_v));
        let idx_v = crate::p64_weight::P64TensorIndex::from_p64(&p64_v).unwrap();

        let t1 = Instant::now();
        let p64_f = crate::p64_weight::compile_gguf_to_p64_with_layout(
            &gguf,
            12,
            crate::p64_weight::P64ConvertLayout::F16Expand,
        )
        .expect("f16");
        let ms_f = t1.elapsed().as_secs_f64() * 1e3;
        let idx_f = crate::p64_weight::P64TensorIndex::from_p64(&p64_f).unwrap();
        assert_eq!(idx_v.entries.len(), idx_f.entries.len());

        eprintln!(
            "[stage1 convert] verbatim {:.2}ms {}B | f16-expand {:.2}ms {}B tensors={}",
            ms_v,
            p64_v.len(),
            ms_f,
            p64_f.len(),
            idx_v.entries.len()
        );
    }

    // ── Stage 2: CBOR-LD helper + stop merge ─────────────────────────────────

    #[test]
    fn stage2_helper_cbor_ld_and_stop_merge() {
        use crate::gguf_sharder::GgufTokenizer;
        use crate::model_helper::{ModelHelper, ModelHelperTokenizer};

        let helper = ModelHelper::new(
            "probe.gguf",
            "probe.p64",
            14,
            "Verbatim",
            ModelHelperTokenizer {
                bos_token_id: 1,
                eos_token_id: 2,
                add_bos_token: true,
                chat_family: "Llama3".into(),
                stop_token_ids: vec![2, 128009],
                stop_token_strings: vec!["</s>".into(), "<|eot_id|>".into()],
                vocab_len: 128_256,
            },
        );
        let t0 = Instant::now();
        let bytes = helper.to_cbor_ld().unwrap();
        let back = ModelHelper::from_cbor_ld(&bytes).unwrap();
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        assert!(crate::model_helper::has_model_helper_magic(&bytes));

        let mut tok = GgufTokenizer::default();
        tok.eos_token_id = 2;
        tok.rebuild_stop_token_ids();
        back.apply_stops_to_tokenizer(&mut tok);
        assert!(tok.is_stop_token(128009));

        eprintln!(
            "[stage2 helper] {} B cbor-ld {:.3}ms stops={:?}",
            bytes.len(),
            ms,
            tok.stop_tokens()
        );
    }

    // ── Stage 3: Dequant + stack_gemm_quant ≡ substrate matvec ───────────────

    #[test]
    fn stage3_dequant_stack_gemm_substrate_parity() {
        use crate::gguf_sharder::GgufTensorInfo;
        use crate::solvers::linear_algebra::gemm::{matvec, Transpose};

        let n_in = 64usize;
        let n_out = 32usize;
        let row_bytes = crate::llm_kernel_parity::q8_0_bytes(n_in);
        let mut raw = vec![0u8; row_bytes * n_out];
        let mut row = vec![0f32; n_in];
        let mut s = 7u64;
        let mut rng = || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
        };
        for r in 0..n_out {
            for x in row.iter_mut() {
                *x = rng();
            }
            assert!(crate::llm_kernel_parity::quantize_q8_0_from_f32(
                &row,
                &mut raw[r * row_bytes..(r + 1) * row_bytes],
            ));
        }
        let input: Vec<f32> = (0..n_in).map(|_| rng()).collect();
        let info = GgufTensorInfo {
            dims: [n_in as u64, n_out as u64, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q8_0,
            byte_offset: 0,
        };

        let t0 = Instant::now();
        let mut out_llm = vec![0f32; n_out];
        assert!(crate::gguf_bridge::stack_gemm_quant(
            &raw, &info, &input, &mut out_llm, n_in, n_out
        ));
        let ms_llm = t0.elapsed().as_secs_f64() * 1e3;

        let t1 = Instant::now();
        let mut w = vec![0f64; n_in * n_out];
        let mut deq = vec![0f32; n_in];
        for i in 0..n_out {
            let n = crate::ggml_quants::dequant_matrix_row_into(&raw, &info, i, &mut deq).unwrap();
            assert_eq!(n, n_in);
            for j in 0..n_in {
                w[i * n_in + j] = deq[j] as f64;
            }
        }
        let x: Vec<f64> = input.iter().map(|&v| v as f64).collect();
        let mut out_sub = vec![0f64; n_out];
        matvec(Transpose::No, n_out, n_in, &w, &x, &mut out_sub).unwrap();
        let ms_sub = t1.elapsed().as_secs_f64() * 1e3;

        let mut max_err = 0f32;
        for i in 0..n_out {
            max_err = max_err.max((out_llm[i] - out_sub[i] as f32).abs());
        }
        assert!(max_err < 1e-3, "max_err={max_err}");
        eprintln!(
            "[stage3] stack_gemm_quant {:.3}ms | dequant+matvec {:.3}ms max_err={max_err:.2e} ({n_in}x{n_out})",
            ms_llm, ms_sub
        );
    }

    // ── Stage 4: Forge GEMM floor + TC selector ──────────────────────────────

    #[test]
    fn stage4_forge_gemm_f32_and_tc_selector() {
        use crate::wgsl_forge::dispatch::{gemm_f32, gemm_f32_tc};

        let (m, k, n) = (32usize, 64usize, 32usize);
        let a: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.01).collect();
        let b: Vec<f32> = (0..k * n).map(|i| (i as f32) * 0.02).collect();

        let t0 = Instant::now();
        let plain = gemm_f32(m, k, n, &a, &b).expect("gemm_f32 floor");
        let ms_plain = t0.elapsed().as_secs_f64() * 1e3;

        // Must not panic when CUDA/NVRTC absent — falls to plain floor.
        let t1 = Instant::now();
        let tc = gemm_f32_tc(m, k, n, &a, &b).expect("gemm_f32_tc must soft-fail to floor");
        let ms_tc = t1.elapsed().as_secs_f64() * 1e3;

        let mut max_err = 0f32;
        for i in 0..plain.len() {
            max_err = max_err.max((plain[i] - tc[i]).abs());
        }
        assert!(max_err < 1e-2, "max_err={max_err}");
        eprintln!(
            "[stage4 forge] gemm_f32 {:.3}ms | gemm_f32_tc {:.3}ms max_err={max_err:.2e} ({m}x{k}x{n})",
            ms_plain, ms_tc
        );
    }

    // ── Stage 5: Top-k (output projection primitive) ─────────────────────────

    #[test]
    fn stage5_topk_cpu() {
        use crate::topk::topk_cpu;

        let logits: Vec<f32> = (0..4096)
            .map(|i| ((i * 17) % 4096) as f32 * 0.001 - 1.0)
            .collect();
        let t0 = Instant::now();
        let top = topk_cpu(&logits, 8);
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        assert_eq!(top.len(), 8);
        for w in top.windows(2) {
            assert!(w[0].logit >= w[1].logit);
        }
        let argmax = logits
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i as u32)
            .unwrap();
        assert_eq!(top[0].token_id, argmax);
        eprintln!(
            "[stage5 topk] topk_cpu k=8 / 4096 logits {:.3}ms top1={}",
            ms, top[0].token_id
        );
    }

    // ── Stage 6: Ternary novel representation ────────────────────────────────

    #[test]
    fn stage6_ternary_blob_gemm_vs_dense() {
        use crate::ternary::{quantize_ternary, ternary_blob, ternary_gemm_cpu, dequantize_blob};

        let n_in = 48usize;
        let n_out = 16usize;
        let mut w = vec![0f32; n_in * n_out];
        let mut s = 99u64;
        for x in w.iter_mut() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *x = ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0;
        }
        let input: Vec<f32> = (0..n_in).map(|i| (i as f32) * 0.1).collect();

        let t0 = Instant::now();
        let blob = ternary_blob(&w);
        let (scale, _) = quantize_ternary(&w);
        let packed = &blob[4..];
        let mut out_t = vec![0f32; n_out];
        ternary_gemm_cpu(
            &input, packed, scale, n_in, n_out, 1, 0, 0, &mut out_t,
        );
        let ms_t = t0.elapsed().as_secs_f64() * 1e3;

        let t1 = Instant::now();
        let mut out_d = vec![0f32; n_out];
        for i in 0..n_out {
            let mut acc = 0f32;
            for j in 0..n_in {
                acc += w[i * n_in + j] * input[j];
            }
            out_d[i] = acc;
        }
        let ms_d = t1.elapsed().as_secs_f64() * 1e3;

        // Ternary is lossy — just prove both run and blob is smaller than f32.
        assert!(blob.len() < w.len() * 4);
        let mut recon = vec![0f32; w.len()];
        dequantize_blob(&blob, &mut recon);
        eprintln!(
            "[stage6 ternary] pack+GEMV {:.3}ms dense GEMV {:.3}ms blob {}B vs f32 {}B ratio={:.2}",
            ms_t,
            ms_d,
            blob.len(),
            w.len() * 4,
            blob.len() as f64 / (w.len() * 4) as f64
        );
    }

    // ── Stage 7: Capability matrix / passport ────────────────────────────────

    #[test]
    fn stage7_capability_matrix_and_passport() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::device_benchmark::benchmark_devices;
            use crate::hardware_passport::{default_cache_path, load_or_probe};

            let t0 = Instant::now();
            let matrix = benchmark_devices(256);
            let ms = t0.elapsed().as_secs_f64() * 1e3;
            assert!(!matrix.circuits.is_empty());
            eprintln!("[stage7 matrix] {:.1}ms\n{}", ms, matrix.summary());

            let (p, cached) = load_or_probe(&default_cache_path(), 256);
            eprintln!(
                "[stage7 passport] cached={cached} best={:?}",
                p.matrix
                    .best()
                    .map(|c| (c.label.as_str(), c.backend.as_str(), c.ms_per_gemv))
            );
        }
    }

    // ── Stage 8: Live p64 + helper on disk (optional) ────────────────────────

    #[test]
    fn stage8_live_p64_and_helper_if_present() {
        use crate::p64_weight::IntegrityMode;
        use std::path::Path;

        let live = Path::new(r"C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64");
        if !live.is_file() {
            eprintln!("[stage8] no live p64 — skip");
            return;
        }
        let bytes = std::fs::read(live).unwrap();
        assert!(crate::p64_weight::has_p64_magic(&bytes));

        let t_full = Instant::now();
        let idx = crate::p64_weight::P64TensorIndex::from_p64_with_integrity(
            &bytes,
            IntegrityMode::Full,
        )
        .unwrap();
        let ms_full = t_full.elapsed().as_secs_f64() * 1e3;

        let t_meta = Instant::now();
        let idx_m = crate::p64_weight::P64TensorIndex::from_p64_with_integrity(
            &bytes,
            IntegrityMode::Metadata,
        )
        .unwrap();
        let ms_meta = t_meta.elapsed().as_secs_f64() * 1e3;
        assert_eq!(idx.entries.len(), idx_m.entries.len());

        let helper = crate::model_helper::ModelHelper::load_beside_p64(live).unwrap();
        eprintln!(
            "[stage8 live] Full CRC {:.1}ms | Metadata-only {:.1}ms tensors={} helper={:?}",
            ms_full,
            ms_meta,
            idx.entries.len(),
            helper.as_ref().map(|h| (
                h.layout.as_str(),
                h.tokenizer.chat_family.as_str(),
                h.tokenizer.stop_token_ids.clone()
            ))
        );
        if helper.is_none() {
            eprintln!("[stage8] ⚑ re-convert to attach .q42.cbor-ld");
        }
        // Metadata mode must be materially faster on multi-hundred-MB models.
        if ms_full > 500.0 {
            assert!(
                ms_meta < ms_full * 0.5,
                "metadata integrity should cut activate cost; full={ms_full} meta={ms_meta}"
            );
        }
    }

    // ── Novel-rep insight test: f16 expand size vs verbatim for synthetic ────

    #[test]
    fn stage9_layout_representation_sizes() {
        let gguf = minimal_gguf_f32_weight();
        let v = crate::p64_weight::compile_gguf_to_p64_with_layout(
            &gguf,
            12,
            crate::p64_weight::P64ConvertLayout::Verbatim,
        )
        .unwrap();
        let f = crate::p64_weight::compile_gguf_to_p64_with_layout(
            &gguf,
            12,
            crate::p64_weight::P64ConvertLayout::F16Expand,
        )
        .unwrap();
        // Source is already F32; f16 expand of 2-D weights should shrink those blobs.
        // (Norms/others may stay — total can still grow with page align.)
        let iv = crate::p64_weight::P64TensorIndex::from_p64(&v).unwrap();
        let if_ = crate::p64_weight::P64TensorIndex::from_p64(&f).unwrap();
        eprintln!(
            "[stage9 layout] verbatim {}B f16 {}B Δ={:+} tensors v/f={}/{}",
            v.len(),
            f.len(),
            f.len() as i64 - v.len() as i64,
            iv.entries.len(),
            if_.entries.len()
        );
        // Dtypes: at least one entry may flip to F16 under expand when source was quant.
        // With pure F32 source synthetic, expand may copy or re-encode — just log.
    }

    fn minimal_gguf_f32_weight() -> Vec<u8> {
        fn put_str(out: &mut Vec<u8>, s: &str) {
            out.extend_from_slice(&(s.len() as u64).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        fn put_kv_u32(out: &mut Vec<u8>, key: &str, v: u32) {
            put_str(out, key);
            out.extend_from_slice(&4u32.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        fn put_tensor(out: &mut Vec<u8>, name: &str, dims: &[u64], offset: u64) {
            put_str(out, name);
            out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
            for d in dims {
                out.extend_from_slice(&d.to_le_bytes());
            }
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
        }
        fn align_up(n: usize, a: usize) -> usize {
            (n + a - 1) & !(a - 1)
        }

        let mut output = Vec::new();
        output.extend_from_slice(b"GGUF");
        output.extend_from_slice(&3u32.to_le_bytes());
        output.extend_from_slice(&2u64.to_le_bytes());
        output.extend_from_slice(&3u64.to_le_bytes());
        put_kv_u32(&mut output, "llama.block_count", 1);
        put_kv_u32(&mut output, "llama.embedding_length", 4);
        put_kv_u32(&mut output, "llama.attention.head_count", 1);
        put_tensor(&mut output, "blk.0.attn_q.weight", &[4, 4], 0);
        put_tensor(&mut output, "token_embd.weight", &[4, 2], 64);
        let pad_to = align_up(output.len(), 32);
        output.resize(pad_to, 0);
        for i in 0..24 {
            output.extend_from_slice(&(i as f32 * 0.1).to_le_bytes());
        }
        output
    }
}
