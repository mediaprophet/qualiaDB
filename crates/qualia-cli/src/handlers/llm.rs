use std::path::PathBuf;

use crate::cli::LlmAction;
use crate::llm_lifecycle;
use crate::llm_testing;

pub async fn handle(action: &LlmAction) -> Result<(), Box<dyn std::error::Error>> {
    let vault = |opt: &Option<PathBuf>| {
        opt.clone()
            .unwrap_or_else(llm_lifecycle::default_vault_path)
    };
    match action {
        LlmAction::List { vault_path } => {
            llm_lifecycle::run_list(&vault(vault_path))?;
        }
        LlmAction::Duplicates { vault_path } => {
            llm_lifecycle::run_duplicate_audit(&vault(vault_path))?;
        }
        LlmAction::Load { model, vault_path } => {
            llm_lifecycle::run_load(&vault(vault_path), model)?;
        }
        LlmAction::Status => {
            llm_lifecycle::run_status()?;
        }
        LlmAction::Eval {
            prompt,
            orchestrated,
            stream,
            lora,
        } => {
            if let Some(l) = lora {
                println!("Multiplexing LoRA adapter from {:?}", l);
            }
            llm_lifecycle::run_eval(prompt, *orchestrated, *stream)?;
        }
        LlmAction::Evict { model_id } => {
            llm_lifecycle::run_evict(model_id)?;
        }
        LlmAction::Test {
            vault_path,
            models,
            quantization,
            verbose,
        } => {
            llm_testing::run_test_models(
                vault_path.clone(),
                models.clone(),
                quantization.clone(),
                *verbose,
            )?;
        }
        LlmAction::Validate { vault_path, strict } => {
            llm_testing::run_validate_models(vault_path.clone(), *strict)?;
        }
        LlmAction::ComprehensiveTest {
            vault_path,
            model,
            verbose,
        } => {
            llm_testing::run_comprehensive_llm_test(vault_path.clone(), model.clone(), *verbose)?;
        }
        LlmAction::Benchmark {
            vault_path,
            models,
            iterations,
            warmup,
        } => {
            llm_testing::run_benchmark_models(
                vault_path.clone(),
                models.clone(),
                *iterations,
                *warmup,
            )?;
        }
        LlmAction::Report {
            vault_path,
            output,
            format,
        } => {
            llm_testing::run_generate_report(vault_path.clone(), output.clone(), format.clone())?;
        }
        LlmAction::Convert {
            input,
            out,
            page_log2,
            layout,
        } => {
            llm_testing::run_convert_gguf_to_p64(input, out, *page_log2, layout)?;
        }
        LlmAction::Optimize {
            input,
            out,
            skip_passport,
        } => {
            llm_testing::run_optimize_pipeline(input, out.clone(), *skip_passport)?;
        }
        LlmAction::Passport {
            reprobe,
            gemv_n,
            cache,
            apply_env_hint,
            decode_proxy,
            decode_proxy_tokens,
        } => {
            llm_testing::run_hardware_passport(
                *reprobe,
                *gemv_n,
                cache.clone(),
                *apply_env_hint,
                decode_proxy.clone(),
                *decode_proxy_tokens,
            )?;
        }
        LlmAction::DecodeProxy { model, tokens } => {
            llm_testing::run_decode_proxy(model, *tokens)?;
        }
        LlmAction::RawDecodeBench {
            model,
            steps,
            warmups,
            runs,
            quantization,
            prompt,
            target_prompt_tokens,
            retain_artifacts,
        } => {
            crate::llm_raw_bench::run(crate::llm_raw_bench::CommandConfig {
                model,
                steps: *steps,
                warmups: *warmups,
                runs: *runs,
                quantization,
                prompt,
                target_prompt_tokens: *target_prompt_tokens,
                retain_artifacts: retain_artifacts.as_deref(),
            })?;
        }
        LlmAction::Mode { name } => {
            llm_testing::run_inference_mode(name.as_deref())?;
        }
        LlmAction::PathSelect { reprobe, apply } => {
            llm_testing::run_path_select(*reprobe, *apply)?;
        }
        LlmAction::Profile { name } => {
            llm_testing::run_app_profile(name.as_deref())?;
        }
        LlmAction::Lab {
            action,
            model,
            tokens,
            n_in,
            n_out,
            gemv_n,
            out,
            hours,
            max_generations,
            ollama_model,
            ollama_url,
            no_ollama,
        } => {
            llm_testing::run_lab(
                action,
                model.as_deref(),
                *tokens,
                *n_in,
                *n_out,
                *gemv_n,
                out.as_deref(),
                *hours,
                *max_generations,
                ollama_model.as_deref(),
                ollama_url,
                *no_ollama,
            )?;
        }
        LlmAction::Ground { prompt, answer } => {
            llm_testing::run_ground_check(prompt, answer)?;
        }
        LlmAction::SeedGrounding => {
            llm_testing::run_seed_grounding()?;
        }
        LlmAction::CudaTcBench { side } => {
            llm_testing::run_cuda_tc_microbench(*side)?;
        }
        LlmAction::Explore {
            input,
            out,
            tokens,
            layouts,
            skip_convert,
            sweep_ffn_f16,
            modes,
        } => {
            llm_testing::run_explore_pipeline(
                input,
                out.clone(),
                *tokens,
                layouts,
                *skip_convert,
                *sweep_ffn_f16,
                modes.as_deref(),
            )?;
        }
    }
    Ok(())
}
