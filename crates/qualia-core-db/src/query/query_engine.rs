use crate::NQuin;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

/// Memory-maps a flat `.q42` file (packed `NQuin` records) and returns
/// all quins whose `subject` field matches `subject_id`.
pub fn mmap_query_subject(
    file_path: &str,
    subject_id: u64,
) -> Result<Vec<NQuin>, Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use memmap2::MmapOptions;
        use std::fs::File;

        const QUIN_SIZE: usize = std::mem::size_of::<NQuin>();

        let file = File::open(file_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let len = mmap.len();

        if len % QUIN_SIZE != 0 {
            return Err(format!(
                "File size {} is not a multiple of NQuin ({} bytes)",
                len, QUIN_SIZE
            )
            .into());
        }

        let count = len / QUIN_SIZE;
        let quins: &[NQuin] = unsafe {
            std::slice::from_raw_parts(mmap.as_ptr() as *const NQuin, count)
        };

        Ok(quins
            .iter()
            .filter(|q| q.subject == subject_id)
            .copied()
            .collect())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = file_path;
        let _ = subject_id;
        Err("mmap_query_subject is not available on wasm32".into())
    }
}

/// Telemetry counters for `lazy_superblock_query`.
pub struct TelemetryHook {
    pub blocks_loaded: usize,
    pub bytes_decompressed: usize,
    /// Reserved for future WebRTC P2P streaming telemetry.
    pub remote_blocks_streamed: usize,
}

/// Reads a SuperBlock file lazily: scans 16-byte block headers and decompresses
/// only blocks selected by `target_percent` (0–100).  Skipped blocks are O(1) seeks.
pub fn lazy_superblock_query(
    file_path: &str,
    target_percent: u8,
) -> Result<TelemetryHook, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let path = Path::new(file_path);

    let mut file = File::open(path)?;
    let mut telemetry = TelemetryHook {
        blocks_loaded: 0,
        bytes_decompressed: 0,
        remote_blocks_streamed: 0,
    };

    let file_len = file.metadata()?.len();
    let mut offset = 0u64;
    let mut block_index = 0u64;

    while offset < file_len {
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).is_err() {
            break;
        }
        offset += 16;

        let _block_id = u64::from_le_bytes(header[0..8].try_into().unwrap());
        let compressed_len = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
        let uncompressed_len = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;

        // Load blocks up to target_percent of total; skip the rest with an O(1) seek.
        let is_relevant = (block_index % 100) < target_percent as u64;

        if is_relevant {
            let mut compressed_buf = vec![0u8; compressed_len];
            file.read_exact(&mut compressed_buf)?;
            telemetry.blocks_loaded += 1;
            let _uncompressed = lz4_flex::decompress_size_prepended(&compressed_buf)?;
            telemetry.bytes_decompressed += uncompressed_len;
        } else {
            file.seek(SeekFrom::Current(compressed_len as i64))?;
        }

        offset += compressed_len as u64;
        block_index += 1;
    }

    let _duration = start_time.elapsed();

    Ok(telemetry)
}

/// Filter a slice of NQuin by context hash
pub fn filter_by_context(quins: &[NQuin], context_hash: u64) -> Vec<NQuin> {
    if context_hash == 0 {
        return quins.to_vec();
    }
    quins.iter().filter(|q| q.context == context_hash).copied().collect()
}

/// Filter a slice of NQuin by multiple context hashes
pub fn filter_by_contexts(quins: &[NQuin], context_hashes: &[u64]) -> Vec<NQuin> {
    if context_hashes.is_empty() {
        return quins.to_vec();
    }
    let context_set: std::collections::HashSet<u64> = context_hashes.iter().copied().collect();
    quins.iter().filter(|q| context_set.contains(&q.context)).copied().collect()
}

/// Count Quins per context hash
pub fn count_by_context(quins: &[NQuin]) -> std::collections::HashMap<u64, usize> {
    let mut counts = std::collections::HashMap::new();
    for quin in quins {
        *counts.entry(quin.context).or_insert(0) += 1;
    }
    counts
}

/// Get unique context hashes from a slice of NQuin
pub fn unique_contexts(quins: &[NQuin]) -> Vec<u64> {
    let mut contexts = std::collections::HashSet::new();
    for quin in quins {
        contexts.insert(quin.context);
    }
    contexts.into_iter().collect()
}

/// Filter Quins by context and subject
pub fn filter_by_context_and_subject(quins: &[NQuin], context_hash: u64, subject: u64) -> Vec<NQuin> {
    quins.iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.subject == subject)
        .copied()
        .collect()
}

/// Filter Quins by context and predicate
pub fn filter_by_context_and_predicate(quins: &[NQuin], context_hash: u64, predicate: u64) -> Vec<NQuin> {
    quins.iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.predicate == predicate)
        .copied()
        .collect()
}

/// Filter Quins by context and object
pub fn filter_by_context_and_object(quins: &[NQuin], context_hash: u64, object: u64) -> Vec<NQuin> {
    quins.iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.object == object)
        .copied()
        .collect()
}

#[cfg(test)]
mod context_tests {
    use super::*;

    #[test]
    fn test_filter_by_context() {
        let quins = vec![
            NQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 4, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
            NQuin { subject: 7, predicate: 8, object: 9, context: 100, metadata: 0, parity: 0 },
        ];
        
        let filtered = filter_by_context(&quins, 100);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].context, 100);
        assert_eq!(filtered[1].context, 100);
    }

    #[test]
    fn test_filter_by_context_wildcard() {
        let quins = vec![
            NQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 4, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
        ];
        
        let filtered = filter_by_context(&quins, 0);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_count_by_context() {
        let quins = vec![
            NQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 4, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
            NQuin { subject: 7, predicate: 8, object: 9, context: 100, metadata: 0, parity: 0 },
        ];
        
        let counts = count_by_context(&quins);
        assert_eq!(counts.get(&100), Some(&2));
        assert_eq!(counts.get(&200), Some(&1));
    }

    #[test]
    fn test_filter_by_context_and_subject() {
        let quins = vec![
            NQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 1, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
            NQuin { subject: 7, predicate: 8, object: 9, context: 100, metadata: 0, parity: 0 },
        ];
        
        let filtered = filter_by_context_and_subject(&quins, 100, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].subject, 1);
        assert_eq!(filtered[0].context, 100);
    }
}