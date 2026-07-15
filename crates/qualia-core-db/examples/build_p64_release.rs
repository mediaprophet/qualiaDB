//! Build a browser-release model package without linking the full Qualia CLI.
//!
//! Usage: `cargo run --release -p qualia-core-db --example build_p64_release -- <in.gguf> <out-dir>`

use qualia_core_db::gguf_sharder::GgufTokenizer;
use qualia_core_db::model_helper::{ModelHelper, ModelHelperTokenizer};
use qualia_core_db::p64_weight::{
    compile_gguf_to_p64_with_layout, P64ConvertLayout, P64TensorIndex,
};
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: build_p64_release <input.gguf> <out-dir>".to_string())?;
    let out_dir = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: build_p64_release <input.gguf> <out-dir>".to_string())?;
    if args.next().is_some() || !input.is_file() {
        return Err("expected one existing GGUF and one output directory".into());
    }

    std::fs::create_dir_all(&out_dir).map_err(|e| format!("create output directory: {e}"))?;
    let file = std::fs::File::open(&input).map_err(|e| format!("open input: {e}"))?;
    let mmap = unsafe { memmap2::Mmap::map(&file).map_err(|e| format!("mmap input: {e}"))? };
    println!("Compiling {} ({:.1} MiB)", input.display(), mmap.len() as f64 / 1_048_576.0);

    // Browser releases keep the source quant blocks byte-for-byte. Expanded/SoA layouts
    // are device-specific native optimizations and can exceed GitHub's asset-size ceiling.
    let p64 = compile_gguf_to_p64_with_layout(&mmap, 14, P64ConvertLayout::Verbatim)?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "input filename is not UTF-8".to_string())?;
    let p64_path = out_dir.join(format!("{stem}.p64"));
    std::fs::write(&p64_path, &p64).map_err(|e| format!("write P64: {e}"))?;

    let index = P64TensorIndex::from_p64(&p64).map_err(|e| format!("P64 self-check: {e}"))?;
    let tokenizer = GgufTokenizer::from_gguf(&mmap);
    let stop_ids = tokenizer.stop_tokens().to_vec();
    let stop_names = stop_ids
        .iter()
        .filter_map(|&id| tokenizer.vocab.get(id as usize).cloned())
        .collect();
    let helper = ModelHelper::new(
        file_name(&input),
        file_name(&p64_path),
        14,
        "Verbatim",
        ModelHelperTokenizer {
            bos_token_id: tokenizer.bos_token_id,
            eos_token_id: tokenizer.eos_token_id,
            add_bos_token: tokenizer.add_bos_token,
            chat_family: format!("{:?}", tokenizer.chat_family()),
            stop_token_ids: stop_ids,
            stop_token_strings: stop_names,
            vocab_len: tokenizer.vocab_len(),
        },
    );
    let q42_path = helper.write_beside_p64(&p64_path)?;
    let round_trip = ModelHelper::load_beside_p64(&p64_path)?
        .ok_or_else(|| "Q42 self-check did not find the emitted volume".to_string())?;
    if round_trip != helper {
        return Err("Q42 self-check changed model metadata".into());
    }

    println!(
        "Built {} ({:.1} MiB, {} tensors) + {} (Q42 v3, family {})",
        p64_path.display(),
        p64.len() as f64 / 1_048_576.0,
        index.entries.len(),
        q42_path.display(),
        helper.tokenizer.chat_family,
    );
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string()
}
