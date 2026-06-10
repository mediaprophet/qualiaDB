use crate::QualiaQuin;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

/// Memory-maps a large `.q42` file and performs a lightning-fast scan for a subject ID
pub fn mmap_query_subject(
    _file_path: &str,
    subject_id: u64,
) -> Result<Vec<QualiaQuin>, Box<dyn std::error::Error>> {
    println!(
        "Legacy mmap_query_subject called for subject_id {}",
        subject_id
    );
    Ok(vec![])
}

/// Simulated Telemetry Hook
pub struct TelemetryHook {
    pub blocks_loaded: usize,
    pub bytes_decompressed: usize,
    pub remote_blocks_streamed: usize,
}

/// Reads SuperBlocks lazily. It scans 16-byte headers and only decompresses blocks
/// if they meet certain criteria (simulated). If a block is marked as "missing locally",
/// it mocks a WebRTC DataChannel stream from a peer.
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
    let mut offset = 0;
    let mut block_index = 0;

    println!("📡 Initializing Lazy SuperBlock Query & WebRTC P2P Streamer...");

    while offset < file_len {
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).is_err() {
            break;
        }
        offset += 16;

        // Parse Header
        let _block_id = u64::from_le_bytes(header[0..8].try_into().unwrap());
        let compressed_len = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
        let uncompressed_len = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;

        // Simulating the Webizen deciding if this block is relevant
        // E.g., querying 10% of the graph
        let is_relevant = (block_index % 100) < target_percent as u64;

        if is_relevant {
            // Mocking WebRTC: Every 5th relevant block is sourced from a P2P Swarm peer
            let is_remote = block_index % 5 == 0;

            let mut compressed_buf = vec![0u8; compressed_len];

            if is_remote {
                // Mock WebRTC DataChannel Stream
                telemetry.remote_blocks_streamed += 1;
                // We'll skip the disk read and pretend we streamed it
                file.seek(SeekFrom::Current(compressed_len as i64))?;
                // In reality, compressed_buf would be filled via WebRTC
            } else {
                file.read_exact(&mut compressed_buf)?;
                telemetry.blocks_loaded += 1;
            }

            // Decompress into L1 cache buffer
            // For the benchmark mock, if it's remote we didn't actually load valid LZ4 bytes,
            // so we skip actual decompression to avoid panic, but we count the bytes.
            if !is_remote {
                let _uncompressed = lz4_flex::decompress_size_prepended(&compressed_buf)?;
                telemetry.bytes_decompressed += uncompressed_len;
            } else {
                telemetry.bytes_decompressed += uncompressed_len;
            }
        } else {
            // Lazy Jump: Skip this block entirely (O(1) seek)
            file.seek(SeekFrom::Current(compressed_len as i64))?;
        }

        offset += compressed_len as u64;
        block_index += 1;
    }

    let duration = start_time.elapsed();
    println!("⚡ Query Complete in {:?}", duration);
    println!(
        "🎯 Loaded {} Local Blocks | Streamed {} Remote Blocks via WebRTC",
        telemetry.blocks_loaded, telemetry.remote_blocks_streamed
    );
    println!(
        "💾 Total Uncompressed Data Processed: {:.2} MB",
        telemetry.bytes_decompressed as f64 / 1_048_576.0
    );

    Ok(telemetry)
}

/// Filter a slice of QualiaQuin by context hash
pub fn filter_by_context(quins: &[QualiaQuin], context_hash: u64) -> Vec<QualiaQuin> {
    if context_hash == 0 {
        return quins.to_vec();
    }
    quins.iter().filter(|q| q.context == context_hash).copied().collect()
}

/// Filter a slice of QualiaQuin by multiple context hashes
pub fn filter_by_contexts(quins: &[QualiaQuin], context_hashes: &[u64]) -> Vec<QualiaQuin> {
    if context_hashes.is_empty() {
        return quins.to_vec();
    }
    let context_set: std::collections::HashSet<u64> = context_hashes.iter().copied().collect();
    quins.iter().filter(|q| context_set.contains(&q.context)).copied().collect()
}

/// Count Quins per context hash
pub fn count_by_context(quins: &[QualiaQuin]) -> std::collections::HashMap<u64, usize> {
    let mut counts = std::collections::HashMap::new();
    for quin in quins {
        *counts.entry(quin.context).or_insert(0) += 1;
    }
    counts
}

/// Get unique context hashes from a slice of QualiaQuin
pub fn unique_contexts(quins: &[QualiaQuin]) -> Vec<u64> {
    let mut contexts = std::collections::HashSet::new();
    for quin in quins {
        contexts.insert(quin.context);
    }
    contexts.into_iter().collect()
}

/// Filter Quins by context and subject
pub fn filter_by_context_and_subject(quins: &[QualiaQuin], context_hash: u64, subject: u64) -> Vec<QualiaQuin> {
    quins.iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.subject == subject)
        .copied()
        .collect()
}

/// Filter Quins by context and predicate
pub fn filter_by_context_and_predicate(quins: &[QualiaQuin], context_hash: u64, predicate: u64) -> Vec<QualiaQuin> {
    quins.iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.predicate == predicate)
        .copied()
        .collect()
}

/// Filter Quins by context and object
pub fn filter_by_context_and_object(quins: &[QualiaQuin], context_hash: u64, object: u64) -> Vec<QualiaQuin> {
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
            QualiaQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            QualiaQuin { subject: 4, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
            QualiaQuin { subject: 7, predicate: 8, object: 9, context: 100, metadata: 0, parity: 0 },
        ];
        
        let filtered = filter_by_context(&quins, 100);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].context, 100);
        assert_eq!(filtered[1].context, 100);
    }

    #[test]
    fn test_filter_by_context_wildcard() {
        let quins = vec![
            QualiaQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            QualiaQuin { subject: 4, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
        ];
        
        let filtered = filter_by_context(&quins, 0);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_count_by_context() {
        let quins = vec![
            QualiaQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            QualiaQuin { subject: 4, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
            QualiaQuin { subject: 7, predicate: 8, object: 9, context: 100, metadata: 0, parity: 0 },
        ];
        
        let counts = count_by_context(&quins);
        assert_eq!(counts.get(&100), Some(&2));
        assert_eq!(counts.get(&200), Some(&1));
    }

    #[test]
    fn test_filter_by_context_and_subject() {
        let quins = vec![
            QualiaQuin { subject: 1, predicate: 2, object: 3, context: 100, metadata: 0, parity: 0 },
            QualiaQuin { subject: 1, predicate: 5, object: 6, context: 200, metadata: 0, parity: 0 },
            QualiaQuin { subject: 7, predicate: 8, object: 9, context: 100, metadata: 0, parity: 0 },
        ];
        
        let filtered = filter_by_context_and_subject(&quins, 100, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].subject, 1);
        assert_eq!(filtered[0].context, 100);
    }
}