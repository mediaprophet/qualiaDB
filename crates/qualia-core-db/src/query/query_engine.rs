use crate::NQuin;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

/// Bounded sample of a `.q42` graph — unified v3 first, then flat packed NQuins.
pub fn mmap_sample_quins(
    file_path: &str,
    max_quins: usize,
) -> Result<Vec<NQuin>, Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::q42_volume::{Q42Volume, Q42VolumeSet};

        if max_quins == 0 {
            return Ok(Vec::new());
        }
        let path = Path::new(file_path);
        if crate::q42_volume::is_unified_volume(path)? {
            let volume = Q42Volume::open(path)?;
            if volume.volume_manifest()?.is_some() {
                let set = Q42VolumeSet::open_root(path)?;
                let mut out = Vec::new();
                for segment in set.segments() {
                    sample_volume_blocks(&segment, max_quins.saturating_sub(out.len()), &mut out)?;
                    if out.len() >= max_quins {
                        break;
                    }
                }
                return Ok(out);
            }
            let mut out = Vec::new();
            sample_volume_blocks(&volume, max_quins, &mut out)?;
            return Ok(out);
        }

        if let Ok(quins) = crate::q42_reader::read_q42_quins(path) {
            return Ok(stride_sample(&quins, max_quins));
        }

        use memmap2::MmapOptions;
        const QUIN_SIZE: usize = std::mem::size_of::<NQuin>();
        let file = File::open(path)?;
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
        let quins: &[NQuin] =
            unsafe { std::slice::from_raw_parts(mmap.as_ptr() as *const NQuin, count) };
        Ok(stride_sample(quins, max_quins))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = file_path;
        let _ = max_quins;
        Err("mmap_sample_quins is not available on wasm32".into())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn sample_volume_blocks(
    volume: &crate::q42_volume::Q42Volume,
    max_quins: usize,
    out: &mut Vec<NQuin>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::q42_volume::{decode_superblock_quins, SUPERBLOCK_SIZE};
    let blocks = volume.block_count() as usize;
    if blocks == 0 || max_quins == 0 {
        return Ok(());
    }
    let mut decoded = [0u8; SUPERBLOCK_SIZE];
    let stride = (blocks / max_quins).max(1);
    let mut index = 0usize;
    while out.len() < max_quins && index < blocks {
        volume.read_superblock_into(index, &mut decoded)?;
        for quin in decode_superblock_quins(&decoded)? {
            if quin.subject != 0 || quin.predicate != 0 {
                out.push(quin);
                if out.len() >= max_quins {
                    break;
                }
            }
        }
        index = index.saturating_add(stride);
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_subject(
    volume: &crate::q42_volume::Q42Volume,
    subject_id: u64,
    out: &mut Vec<NQuin>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::q42_volume::{decode_superblock_quins, SUPERBLOCK_SIZE};
    let mut decoded = [0u8; SUPERBLOCK_SIZE];
    for index in 0..volume.block_count() as usize {
        volume.read_superblock_into(index, &mut decoded)?;
        for quin in decode_superblock_quins(&decoded)? {
            if quin.subject == subject_id {
                out.push(quin);
            }
        }
    }
    Ok(())
}

fn stride_sample(quins: &[NQuin], max_quins: usize) -> Vec<NQuin> {
    if max_quins == 0 || quins.is_empty() {
        return Vec::new();
    }
    let cap = max_quins.min(quins.len());
    let stride = (quins.len() / cap).max(1);
    let mut out = Vec::with_capacity(cap);
    let mut idx = 0usize;
    while out.len() < cap && idx < quins.len() {
        let quin = quins[idx];
        if quin.subject != 0 || quin.predicate != 0 {
            out.push(quin);
        }
        idx += stride;
    }
    out
}

/// Query a `.q42` for Quins whose `subject` matches `subject_id`.
/// Unified v3 volumes are decoded; flat packed files remain a fallback.
pub fn mmap_query_subject(
    file_path: &str,
    subject_id: u64,
) -> Result<Vec<NQuin>, Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::q42_volume::{Q42Volume, Q42VolumeSet};

        let path = Path::new(file_path);
        if crate::q42_volume::is_unified_volume(path)? {
            let volume = Q42Volume::open(path)?;
            if volume.volume_manifest()?.is_some() {
                let set = Q42VolumeSet::open_root(path)?;
                let mut out = Vec::new();
                for segment in set.segments() {
                    collect_subject(&segment, subject_id, &mut out)?;
                }
                return Ok(out);
            }
            let mut out = Vec::new();
            collect_subject(&volume, subject_id, &mut out)?;
            return Ok(out);
        }

        if let Ok(quins) = crate::q42_reader::read_q42_quins(path) {
            return Ok(quins
                .into_iter()
                .filter(|q| q.subject == subject_id)
                .collect());
        }

        use memmap2::MmapOptions;
        const QUIN_SIZE: usize = std::mem::size_of::<NQuin>();
        let file = File::open(path)?;
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
        let quins: &[NQuin] =
            unsafe { std::slice::from_raw_parts(mmap.as_ptr() as *const NQuin, count) };
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

/// Reads a SuperBlock file lazily: unified v3 volumes decode selected blocks;
/// legacy 16-byte framed transport remains a fallback (O(1) seek on skip).
pub fn lazy_superblock_query(
    file_path: &str,
    target_percent: u8,
) -> Result<TelemetryHook, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let path = Path::new(file_path);
    let mut telemetry = TelemetryHook {
        blocks_loaded: 0,
        bytes_decompressed: 0,
        remote_blocks_streamed: 0,
    };

    #[cfg(not(target_arch = "wasm32"))]
    if crate::q42_volume::is_unified_volume(path).ok() == Some(true) {
        use crate::q42_volume::{Q42Volume, SUPERBLOCK_SIZE};
        let volume = Q42Volume::open(path)?;
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        for index in 0..volume.block_count() {
            let is_relevant = (index % 100) < target_percent as u64;
            if !is_relevant {
                continue;
            }
            let n = volume.read_superblock_into(index as usize, &mut decoded)?;
            telemetry.blocks_loaded += 1;
            telemetry.bytes_decompressed += n;
        }
        let _duration = start_time.elapsed();
        return Ok(telemetry);
    }

    let mut file = File::open(path)?;
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
    quins
        .iter()
        .filter(|q| q.context == context_hash)
        .copied()
        .collect()
}

/// Filter a slice of NQuin by multiple context hashes
pub fn filter_by_contexts(quins: &[NQuin], context_hashes: &[u64]) -> Vec<NQuin> {
    if context_hashes.is_empty() {
        return quins.to_vec();
    }
    let context_set: std::collections::HashSet<u64> = context_hashes.iter().copied().collect();
    quins
        .iter()
        .filter(|q| context_set.contains(&q.context))
        .copied()
        .collect()
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
pub fn filter_by_context_and_subject(
    quins: &[NQuin],
    context_hash: u64,
    subject: u64,
) -> Vec<NQuin> {
    quins
        .iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.subject == subject)
        .copied()
        .collect()
}

/// Filter Quins by context and predicate
pub fn filter_by_context_and_predicate(
    quins: &[NQuin],
    context_hash: u64,
    predicate: u64,
) -> Vec<NQuin> {
    quins
        .iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.predicate == predicate)
        .copied()
        .collect()
}

/// Filter Quins by context and object
pub fn filter_by_context_and_object(quins: &[NQuin], context_hash: u64, object: u64) -> Vec<NQuin> {
    quins
        .iter()
        .filter(|q| (context_hash == 0 || q.context == context_hash) && q.object == object)
        .copied()
        .collect()
}

#[cfg(test)]
mod volume_query_tests {
    use super::*;
    use crate::q42_volume::write_sorted_quins_volume;

    fn quin(subject: u64, object: u64) -> NQuin {
        NQuin {
            subject,
            predicate: 2,
            object,
            context: 0,
            metadata: 0,
            parity: NQuin::calculate_parity(subject, 2, object, 0, 0),
        }
    }

    #[test]
    fn sample_and_subject_query_read_unified_v3() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.q42");
        write_sorted_quins_volume(&path, &[quin(7, 1), quin(9, 2), quin(7, 3)]).unwrap();
        let sampled = mmap_sample_quins(path.to_str().unwrap(), 8).unwrap();
        assert_eq!(sampled.len(), 3);
        let hits = mmap_query_subject(path.to_str().unwrap(), 7).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|q| q.subject == 7));
        let telemetry = lazy_superblock_query(path.to_str().unwrap(), 100).unwrap();
        assert_eq!(telemetry.blocks_loaded, 1);
        assert!(telemetry.bytes_decompressed >= crate::q42_volume::SUPERBLOCK_SIZE);
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;

    #[test]
    fn test_filter_by_context() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 4,
                predicate: 5,
                object: 6,
                context: 200,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 7,
                predicate: 8,
                object: 9,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let filtered = filter_by_context(&quins, 100);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].context, 100);
        assert_eq!(filtered[1].context, 100);
    }

    #[test]
    fn test_filter_by_context_wildcard() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 4,
                predicate: 5,
                object: 6,
                context: 200,
                metadata: 0,
                parity: 0,
            },
        ];

        let filtered = filter_by_context(&quins, 0);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_count_by_context() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 4,
                predicate: 5,
                object: 6,
                context: 200,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 7,
                predicate: 8,
                object: 9,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let counts = count_by_context(&quins);
        assert_eq!(counts.get(&100), Some(&2));
        assert_eq!(counts.get(&200), Some(&1));
    }

    #[test]
    fn test_filter_by_context_and_subject() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 1,
                predicate: 5,
                object: 6,
                context: 200,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 7,
                predicate: 8,
                object: 9,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let filtered = filter_by_context_and_subject(&quins, 100, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].subject, 1);
        assert_eq!(filtered[0].context, 100);
    }
}
