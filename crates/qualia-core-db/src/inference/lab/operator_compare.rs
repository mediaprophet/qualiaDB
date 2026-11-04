//! Canonical comparison execution with host-specific checkpoint discovery tests.
pub use etalon_inference_lab::operator_compare::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_real_model_checkpoint_or_not_measured() {
        let candidates = [
            std::env::var("QUALIA_TEST_MODEL_PATH").unwrap_or_default(),
            r"E:\LLM_Models\lmstudio-community\granite-4.0-h-tiny-GGUF\granite-4.0-h-tiny-Q4_K_M.gguf".to_string(),
            r"E:\LLM_Models\lmstudio-community\gemma-4-26B-A4B-it-GGUF\gemma-4-26B-A4B-it-Q4_K_M.gguf".to_string(),
            r"E:\LLM_Models\lmstudio-community\Qwen3.6-35B-A3B-GGUF\Qwen3.6-35B-A3B-Q4_K_M.gguf".to_string(),
            "models/SmolLM2-360M-Instruct-Q4_K_M.gguf".to_string(),
        ];

        let mut found_path = None;
        for c in &candidates {
            if !c.is_empty() && std::path::Path::new(c).exists() {
                found_path = Some(c.clone());
                break;
            }
        }

        let path_str = match found_path {
            Some(p) => p,
            None => {
                println!("CARGO_TEST_STATUS: not measured: No candidate GGUF model found");
                return;
            }
        };

        println!("CARGO_TEST_STATUS: evaluating real model at {}", path_str);
        let file = match std::fs::File::open(&path_str) {
            Ok(f) => f,
            Err(e) => {
                println!(
                    "CARGO_TEST_STATUS: not measured: failed to open {}: {}",
                    path_str, e
                );
                return;
            }
        };
        let mmap = match unsafe { memmap2::MmapOptions::new().map(&file) } {
            Ok(m) => m,
            Err(e) => {
                println!(
                    "CARGO_TEST_STATUS: not measured: failed to mmap {}: {}",
                    path_str, e
                );
                return;
            }
        };
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        if index.entries.is_empty() {
            println!(
                "CARGO_TEST_STATUS: not measured: empty tensor index for {}",
                path_str
            );
            return;
        }

        // Find any 2D Q4_K weight matrix in the model
        let mut target_info = None;
        for (_h, info) in &index.entries {
            if info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K
                && info.n_dims == 2
                && info.dims[0] >= 256
                && info.dims[1] >= 256
                && info.dims[0] % 256 == 0
            {
                target_info = Some(*info);
                break;
            }
        }

        let target_info = match target_info {
            Some(info) => info,
            None => {
                println!(
                    "CARGO_TEST_STATUS: not measured: no 2D Q4_K tensor found in {}",
                    path_str
                );
                return;
            }
        };

        let in_features = target_info.dims[0] as usize;
        let out_features = target_info.dims[1] as usize;
        let raw_bytes = match crate::ggml_quants::fetch_tensor_bytes(
            &mmap,
            index.tensor_data_start,
            &target_info,
        ) {
            Ok(b) => b,
            Err(e) => {
                println!(
                    "CARGO_TEST_STATUS: not measured: failed to fetch tensor bytes: {:?}",
                    e
                );
                return;
            }
        };

        let mut input = vec![0.0f32; in_features];
        for (i, v) in input.iter_mut().enumerate() {
            *v = ((i % 128) as f32 * 0.02).sin();
        }

        let receipt = compare_single_tensor(raw_bytes, in_features, out_features, &input);
        println!(
            "CARGO_TEST_STATUS: real tensor compare [in={}, out={}]: max_err={:e}, mean_err={:e}, existing_ns={}, lookup_ns={}",
            in_features, out_features, receipt.numerical_max_error, receipt.numerical_mean_error, receipt.lut_setup_ns, receipt.warm_latency_ns
        );
        assert!(
            receipt.fidelity_satisfied,
            "real tensor fidelity must be satisfied"
        );
        assert!(
            receipt.numerical_max_error < 1e-3,
            "real tensor max error < 1e-3"
        );
    }
}
