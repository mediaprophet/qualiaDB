//! Exact consumed cold numerical inputs. File names and asserted checksums never
//! replace the loaded adapter weights or the prompt's actual encoded token IDs.
use sha2::{Digest, Sha256};

pub(super) fn lora_digest(adapter: Option<&crate::lora::LoRAAdapter>) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"qualia:prepared-lora-input:v1\0");
    if let Some(adapter) = adapter {
        hash.update([1, adapter.context_type as u8, adapter.meta.adapter_id]);
        for value in [
            adapter.meta.rank as u64,
            adapter.meta.n_in as u64,
            adapter.meta.n_out as u64,
        ] {
            hash.update(value.to_le_bytes());
        }
        hash.update(adapter.meta.alpha.to_bits().to_le_bytes());
        for tensor in [&adapter.lora_a, &adapter.lora_b] {
            hash.update((tensor.rows as u64).to_le_bytes());
            hash.update((tensor.cols as u64).to_le_bytes());
            hash.update((tensor.data.len() as u64).to_le_bytes());
            for value in &tensor.data {
                hash.update(value.to_bits().to_le_bytes());
            }
        }
    } else {
        hash.update([0]);
    }
    hash.finalize().into()
}

pub(super) fn request_digest(
    prompt: &str,
    graph: u64,
    encoded: &[u32],
    model: [u8; 32],
    tokenizer: [u8; 32],
    lora: [u8; 32],
    settings: [u8; 32],
    max_tokens: u64,
    thinking_tokens: u32,
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"qualia:prepared-native-request:v1\0");
    hash.update((prompt.len() as u64).to_le_bytes());
    hash.update(prompt.as_bytes());
    hash.update(graph.to_le_bytes());
    hash.update((encoded.len() as u64).to_le_bytes());
    for token in encoded {
        hash.update(token.to_le_bytes());
    }
    for digest in [model, tokenizer, lora, settings] {
        hash.update(digest);
    }
    hash.update(max_tokens.to_le_bytes());
    hash.update(thinking_tokens.to_le_bytes());
    hash.finalize().into()
}
