//! Cold P64 numerical conversion and compatibility serialization.
use super::build::write::write_entry as write_tensor_entry;
use super::integrity::crc32c;
use super::layout::*;
use crate::inference::{
    write_blk_tensor_name, HyperparamsSource, ModelHyperparams, TensorIndex,
    TensorInfo as GgufTensorInfo,
};
pub mod codecs;
mod converted;
mod ffn;
mod legacy;
mod quantized;
mod vision;
pub use converted::compile_converted;
pub use ffn::{requantize_ffn, FfnQuant};
pub use legacy::{compile_legacy, LegacyInput, LegacyTensor};
pub use quantized::{transcode_quantized, QuantizedTensor, SafetensorQuantization};
pub use vision::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertLayout {
    F16Expand,
    Q4kSoa,
}
#[derive(Clone, Copy, Debug)]
pub struct ConversionBudget {
    pub output_bytes: usize,
    pub scratch_bytes: usize,
}
impl Default for ConversionBudget {
    fn default() -> Self {
        Self {
            output_bytes: u32::MAX as usize,
            scratch_bytes: 256 * 1024 * 1024,
        }
    }
}
impl ConversionBudget {
    fn output(self, bytes: usize) -> Result<(), String> {
        if bytes > self.output_bytes {
            return Err(format!(
                "p64: output budget exceeded: {bytes} > {}",
                self.output_bytes
            ));
        }
        Ok(())
    }
    fn scratch(self, bytes: usize) -> Result<(), String> {
        if bytes > self.scratch_bytes {
            return Err(format!(
                "p64: scratch budget exceeded: {bytes} > {}",
                self.scratch_bytes
            ));
        }
        Ok(())
    }
}
#[derive(Debug)]
pub struct ConversionInput<'a> {
    pub tensor_data_start: usize,
    pub tensors: &'a [(u16, u16, u64, GgufTensorInfo)],
    pub names: &'a [String],
    pub hparams: P64HParams,
    pub tokenizer: &'a [u8],
    pub header_reserved: [u8; 20],
    pub manifold_bytes: &'a [u8],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscodeReport {
    pub n_tensors: usize,
    pub bytes_written: usize,
    pub largest_tensor_bytes: usize,
    pub total_tensor_bytes: usize,
    pub peak_working_bytes: usize,
}

/// Summary of a byte-exact P64 <-> GGUF round-trip verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct P64RoundTripReport {
    pub tensor_count: usize,
    pub tensor_bytes: u64,
    pub manifold_count: usize,
}

/// Build a GGUF-equivalent [`TensorIndex`] from a parsed P64 entry table and hyperparameters.
pub fn synthesize_gguf_index<H: HyperparamsSource>(
    entries: &[P64TensorEntry],
    n_layer: u32,
    hyperparams: H,
) -> TensorIndex<H> {
    let mut named: Vec<(Vec<u8>, GgufTensorInfo)> = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = if entry.manifold_idx == n_layer {
            match entry.role_id {
                P64_ROLE_TOKEN_EMBD => b"token_embd.weight".to_vec(),
                P64_ROLE_OUTPUT => b"output.weight".to_vec(),
                P64_ROLE_OUTPUT_NORM => b"output_norm.weight".to_vec(),
                _ => continue,
            }
        } else if let Some(suffix) = p64_role_suffix(entry.role_id) {
            let mut buffer = [0u8; 96];
            let length = write_blk_tensor_name(entry.manifold_idx, suffix, &mut buffer);
            buffer[..length].to_vec()
        } else {
            continue;
        };
        named.push((
            name,
            GgufTensorInfo {
                dims: entry.dimensions.map(u64::from),
                n_dims: entry.rank,
                ggml_type: entry.dtype as u32,
                byte_offset: entry.blob_offset as u64,
            },
        ));
    }
    let references: Vec<(&[u8], GgufTensorInfo)> = named
        .iter()
        .map(|(name, info)| (name.as_slice(), *info))
        .collect();
    TensorIndex::from_components(&references, hyperparams, 0)
}

/// Prove that a parsed P64 contains every GGUF tensor with identical shape,
/// type and bytes. This is a cold-path validation routine and intentionally
/// performs a complete byte comparison in addition to the stored CRCs.
pub fn validate_p64_against_gguf(
    entries: &[P64TensorEntry],
    n_layer: u32,
    p64_data: &[u8],
    gguf_data: &[u8],
) -> Result<P64RoundTripReport, String> {
    let source = TensorIndex::<ModelHyperparams>::from_gguf(gguf_data);
    if source.entries.len() != entries.len() {
        return Err(format!(
            "p64: tensor count differs (GGUF {}, P64 {})",
            source.entries.len(),
            entries.len()
        ));
    }
    let source_data_start = source.tensor_data_start as usize;
    let mut tensor_bytes = 0u64;
    for (position, entry) in entries.iter().enumerate() {
        let (source_name_hash, source_info) = source
            .entries
            .iter()
            .find(|(_, info)| info.byte_offset == entry.source_offset)
            .ok_or_else(|| format!("p64: tensor {position} source offset is missing"))?;
        if *source_name_hash != entry.source_name_hash
            || source_info.ggml_type != entry.dtype as u32
            || source_info.n_dims != entry.rank
            || source_info.dims != entry.dimensions.map(u64::from)
        {
            return Err(format!("p64: tensor {position} metadata differs from GGUF"));
        }
        let source_len = crate::ggml::tensor_byte_len(source_info)
            .ok_or_else(|| format!("p64: tensor {position} has unsupported source type"))?;
        if source_len != entry.blob_size as usize {
            return Err(format!("p64: tensor {position} byte length differs"));
        }
        let source_start = source_data_start + source_info.byte_offset as usize;
        let source_end = source_start + source_len;
        let p64_blob = entry
            .blob_slice(p64_data)
            .ok_or_else(|| format!("p64: tensor {position} bytes differ from GGUF"))?;
        if source_end > gguf_data.len() || p64_blob != &gguf_data[source_start..source_end] {
            return Err(format!("p64: tensor {position} bytes differ from GGUF"));
        }
        tensor_bytes += source_len as u64;
    }
    Ok(P64RoundTripReport {
        tensor_count: entries.len(),
        tensor_bytes,
        manifold_count: n_layer as usize + 1,
    })
}

fn q_hash(s: &str) -> u64 {
    s.as_bytes().iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

fn workspace_sum(values: &[usize]) -> Result<usize, String> {
    values
        .iter()
        .try_fold(0usize, |n, v| n.checked_add(*v))
        .ok_or_else(|| "p64: workspace overflow".into())
}
fn names_len<'a>(mut names: impl Iterator<Item = &'a str>) -> Result<usize, String> {
    names
        .try_fold(1usize, |n, s| n.checked_add(s.len())?.checked_add(1))
        .ok_or_else(|| "p64: string table overflow".into())
}
fn write_zeros(out: &mut impl std::io::Write, mut count: usize) -> Result<(), String> {
    let zeros = [0u8; 4096];
    while count > 0 {
        let n = count.min(zeros.len());
        out.write_all(&zeros[..n]).map_err(|e| e.to_string())?;
        count -= n;
    }
    Ok(())
}
