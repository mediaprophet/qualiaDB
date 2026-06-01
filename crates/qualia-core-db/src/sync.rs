// no_std is declared at the crate level if needed

use crate::QualiaQuin;

/// Represents a 12-byte structural pointer used for $O(N)$ Merkle DAG diffing.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JumpTableEntry {
    pub chunk_id: u64,
    pub byte_offset: u32,
}

/// The Zero-Allocation CRDT Synchronizer for the Qualia Engine.
/// Operates strictly within pre-allocated stack limits for offline-first mesh syncs.
pub struct MerkleCrdtSynchronizer;

impl MerkleCrdtSynchronizer {
    /// Extracts the 12-bit Lamport clock integer from the 5th Vector (Metadata).
    /// The Lamport clock is embedded in bits 32-43.
    #[inline(always)]
    pub fn extract_lamport_time(quin: &QualiaQuin) -> u16 {
        ((quin.metadata >> 32) & 0x0FFF) as u16
    }

    /// Compares two arrays of JumpTableEntry to find the divergent 128KB frames.
    /// Returns a stack-allocated slice of byte offsets indicating which blocks to pull.
    /// Operates in exactly $O(N)$ time with zero heap allocation.
    pub fn diff_jump_tables<'a>(
        local_table: &[JumpTableEntry],
        remote_table: &[JumpTableEntry],
        diff_buffer: &'a mut [u32],
    ) -> &'a [u32] {
        let mut local_idx = 0;
        let mut remote_idx = 0;
        let mut diff_count = 0;

        // $O(N)$ linear scan comparing Virtual Chunk IDs (Merkle Hashes)
        while local_idx < local_table.len() && remote_idx < remote_table.len() && diff_count < diff_buffer.len() {
            let local_chunk = local_table[local_idx].chunk_id;
            let remote_chunk = remote_table[remote_idx].chunk_id;

            if local_chunk == remote_chunk {
                // Frames match, advance both pointers
                local_idx += 1;
                remote_idx += 1;
            } else if local_chunk < remote_chunk {
                local_idx += 1;
            } else {
                // Remote has a divergent frame
                diff_buffer[diff_count] = remote_table[remote_idx].byte_offset;
                diff_count += 1;
                remote_idx += 1;
            }
        }

        // Add any remaining remote frames
        while remote_idx < remote_table.len() && diff_count < diff_buffer.len() {
            diff_buffer[diff_count] = remote_table[remote_idx].byte_offset;
            diff_count += 1;
            remote_idx += 1;
        }

        &diff_buffer[..diff_count]
    }

    /// Resolves structural conflicts between an incoming 128KB divergent frame and the local frame.
    /// Follows a strict selectable compaction policy triggered by the context metadata.
    pub fn resolve_frame_conflict(
        local_frame: &mut [QualiaQuin],
        incoming_frame: &[QualiaQuin],
        compaction_policy_metadata: u16,
    ) {
        const MASK_STRICT_HISTORY: u16 = 0x0010;
        const MASK_EPOCH_COMPACT: u16 = 0x0020;

        if (compaction_policy_metadata & MASK_EPOCH_COMPACT) != 0 {
            // Epoch Compaction: Zero-out Tombstone Quins to shrink the active data footprint
            for incoming in incoming_frame.iter() {
                // In an epoch compact, we look for tombstones (e.g. Quins with metadata flags marking deletion)
                let is_tombstone = (incoming.metadata & 0x1) != 0; // Simulated tombstone flag
                if is_tombstone {
                    for local in local_frame.iter_mut() {
                        if local.subject == incoming.subject
                            && local.predicate == incoming.predicate
                            && local.object == incoming.object
                        {
                            // Match found. Zero-out both to compress the dataset.
                            local.subject = 0;
                            local.predicate = 0;
                            local.object = 0;
                            local.context = 0;
                            local.metadata = 0;
                            local.parity = 0;
                            break;
                        }
                    }
                } else {
                    // Regular merge logic via Lamport clock comparison
                    // Find empty slot or matching subject/predicate
                    for local in local_frame.iter_mut() {
                        if local.subject == incoming.subject && local.predicate == incoming.predicate {
                            let local_time = Self::extract_lamport_time(local);
                            let incoming_time = Self::extract_lamport_time(incoming);
                            
                            // Keep the Quin with the higher Lamport clock
                            if incoming_time > local_time {
                                *local = *incoming;
                            }
                            break;
                        } else if local.subject == 0 {
                            // Empty slot found, insert
                            *local = *incoming;
                            break;
                        }
                    }
                }
            }
        } else if (compaction_policy_metadata & MASK_STRICT_HISTORY) != 0 {
            // Strict History: Append Tombstone Quins (never delete)
            for incoming in incoming_frame.iter() {
                // Find empty slot in local frame to append to
                for local in local_frame.iter_mut() {
                    if local.subject == 0 { // 0 denotes empty slot
                        *local = *incoming;
                        break;
                    }
                }
            }
        }
    }
}
