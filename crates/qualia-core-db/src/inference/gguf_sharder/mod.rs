//! Q-GGUF Hybrid Packaging
//! Parses monolithic `.gguf` files: vocabulary (KV section) and tensor names/offsets
//! (tensor-info section) are extracted into native Rust types; multi-gigabyte tensor
//! payloads are left on disk for direct VRAM mapping via `gguf_bridge.rs`.

mod hyperparams;
mod sharder;
mod tensor_index;
mod tokenizer;
mod types;

pub use hyperparams::*;
pub use sharder::*;
pub use tensor_index::*;
pub use tokenizer::*;
pub use types::*;

// ─── Module-level GGUF helpers ───────────────────────────────────────────────

/// FNV-1a hash over raw bytes — same algorithm as `crate::q_hash` but for
/// byte slices parsed at runtime (e.g. tensor names from the binary header).
fn gguf_name_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Skip over one GGUF KV value of the given type without storing it.
/// Returns `None` on any parse error (truncated data, unknown type).
/// Used by both `GgufTokenizer` and `GgufTensorIndex`.
fn gguf_skip_value(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<()> {
    match vtype {
        0 | 1 | 7 => {
            if *pos + 1 > mmap.len() {
                return None;
            }
            *pos += 1;
        }
        2 | 3 => {
            if *pos + 2 > mmap.len() {
                return None;
            }
            *pos += 2;
        }
        4 | 5 | 6 => {
            if *pos + 4 > mmap.len() {
                return None;
            }
            *pos += 4;
        }
        10 | 11 | 12 => {
            if *pos + 8 > mmap.len() {
                return None;
            }
            *pos += 8;
        }
        8 => {
            if *pos + 8 > mmap.len() {
                return None;
            }
            let slen = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos + slen > mmap.len() {
                return None;
            }
            *pos += slen;
        }
        9 => {
            if *pos + 12 > mmap.len() {
                return None;
            }
            let etype = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().ok()?);
            *pos += 4;
            let cnt = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            for _ in 0..cnt {
                gguf_skip_value(mmap, pos, etype)?;
            }
        }
        _ => return None,
    }
    Some(())
}

#[cfg(test)]
mod tests;
