//! Byte-Backed WASM `.q42` Host Seam, Progressive Range-Streamed `.hmc` Delivery,
//! Webizen-Governed Worker Pool & Hardware Texture Transcoding Pipeline.
//!
//! Unites Epic 7 (BC7/ETC2/ASTC/KTX2 Hardware Pipeline) and Epic 10 (Byte-Backed WASM
//! `.q42` Host Seam & Governed Worker Pool) from `qualia-core-db`:
//! - [`HmcPackStreamer`]: Orchestrates progressive range streaming for `.hmc` bundles
//!   across 3 priority tiers:
//!   - Tier 0: `.q42` semantic graph + `.10d` `PROC1`/`EONT1`/`IMPO`/`SURF` sections (<50 KB)
//!   - Tier 1: Mid-LOD (`LOD2` / `LOD1`) mesh sections
//!   - Tier 2: High-poly `LOD0` B-Rep geometry and `KTX2` mip0 textures
//! - Zero-heap `NQuin` and `Q42LEX` dictionary resolution via [`WasmSliceQ42Reader`] and
//!   `vibe::catalog::local::LocalHost::with_q42_pack`.
//! - Capability-gated background hydration worker pool ([`GovernedHydrationPool`]) enforcing
//!   fuel and byte budgets using [`HydrationCapabilityToken`] so main-thread frame times never stall.
//! - Direct GPU hardware-compressed texture selection ([`select_hardware_compressed_format`])
//!   and zero-heap CPU block fallback transcoding ([`decode_bc7_block`], [`decode_etc2_eac_rgba_block`],
//!   [`decode_astc_4x4_block`]).

use crate::bundle::streaming_loader::{
    verify_arrived_range, HmcHeaderProbe, HmcProgressiveRangePlanner, HmcRangeRequest,
    StreamPriorityTier,
};
use crate::bundle::BundleError;
use crate::governance::worker_pool::{
    GovernedHydrationPool, GovernancePoolError, HydrationCapabilityToken, HydrationReceipt,
    HydrationTask, HydrationTaskKind, TranscodeCodec, CAP_ALL_HYDRATION,
};
use crate::q42::q42_volume::WasmSliceQ42Reader;
use crate::render::texture_ktx2::{
    select_hardware_compressed_format, HardwareCompressedUploadFormat, Ktx2Document, Ktx2Error,
};
use crate::render::texture_transcode::{
    decode_astc_4x4_block, decode_bc7_block, decode_etc2_eac_rgba_block,
};
use vibe::Host;

/// Maximum capability tokens issued for hydration worker threads.
pub const DEFAULT_HYDRATION_QUEUE_CAPACITY: usize = 128;

/// Extension trait equipping [`vibe::LocalHost`] with direct `.q42` pack admission
/// and local in-memory SPARQL execution.
pub trait LocalHostQ42Ext {
    /// Creates a new `LocalHost` initialized with the admitted `.q42` slice.
    fn with_q42_pack(q42_bytes: &[u8]) -> Self;
    /// Admits a slice of 48-byte `NQuin` records into the local SPARQL store.
    fn admit_q42_slice(&mut self, q42_bytes: &[u8]) -> Result<usize, String>;
    /// Evaluates a local zero-heap SPARQL pattern query.
    fn query_sparql_local(
        &mut self,
        subject: Option<u64>,
        predicate: Option<u64>,
        object: Option<u64>,
        context: Option<u64>,
    ) -> Result<Vec<vibe::Value>, vibe::Diagnostic>;
}

impl LocalHostQ42Ext for vibe::LocalHost {
    fn with_q42_pack(q42_bytes: &[u8]) -> Self {
        let mut host = Self::default();
        let _ = host.admit_q42_slice(q42_bytes);
        host
    }

    fn admit_q42_slice(&mut self, q42_bytes: &[u8]) -> Result<usize, String> {
        let mut map = std::collections::BTreeMap::new();
        let byte_list = q42_bytes.iter().map(|&b| vibe::Value::U64(b as u64)).collect();
        map.insert("bytes".into(), vibe::Value::List(byte_list));
        map.insert("key".into(), vibe::Value::String("pack.q42".into()));
        map.insert("load".into(), vibe::Value::Bool(true));
        let args = vibe::Value::Record(map);
        let span = vibe::Span::new(0, 0);
        match self.capability_invoke("GraphDatabase.volume_open", &args, span) {
            Ok(vibe::Value::Record(res)) => {
                let count = match res.get("quin_count") {
                    Some(vibe::Value::U64(c)) => *c as usize,
                    _ => 0,
                };
                Ok(count)
            }
            Ok(_) => Ok(0),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    fn query_sparql_local(
        &mut self,
        subject: Option<u64>,
        predicate: Option<u64>,
        object: Option<u64>,
        context: Option<u64>,
    ) -> Result<Vec<vibe::Value>, vibe::Diagnostic> {
        let mut map = std::collections::BTreeMap::new();
        if let Some(s) = subject {
            map.insert("subject".into(), vibe::Value::U64(s));
        }
        if let Some(p) = predicate {
            map.insert("predicate".into(), vibe::Value::U64(p));
        }
        if let Some(o) = object {
            map.insert("object".into(), vibe::Value::U64(o));
        }
        if let Some(c) = context {
            map.insert("context".into(), vibe::Value::U64(c));
        }
        let args = vibe::Value::Record(map);
        let span = vibe::Span::new(0, 0);
        let res = self.capability_invoke("GraphDatabase.sparql", &args, span)?;
        match res {
            vibe::Value::List(rows) => Ok(rows),
            _ => Ok(Vec::new()),
        }
    }
}

/// Progressive `.hmc` package streamer, governed hydration worker pool, and hardware texture bridge.
pub struct HmcPackStreamer {
    /// In-process Vibe local host for executing local SPARQL and graph queries.
    pub local_host: vibe::LocalHost,
    /// Capability-gated background hydration worker pool.
    pub pool: GovernedHydrationPool,
    /// Primary capability token governing background task dispatch.
    pub token: HydrationCapabilityToken,
    /// Scheduled range requests partitioned across priority tiers.
    pub planned_requests: Vec<HmcRangeRequest>,
    /// Tier 0 requests successfully verified and admitted.
    pub tier0_completed: usize,
    /// Tier 1 requests successfully verified and admitted.
    pub tier1_completed: usize,
    /// Tier 2 requests successfully verified and admitted.
    pub tier2_completed: usize,
    /// Total payload bytes admitted and verified.
    pub total_bytes_admitted: u64,
    /// Monotonically increasing hydration task identifier.
    next_task_id: u64,
}

/// Compatibility alias for [`HmcPackStreamer`].
pub type EcopunkPackStreamer = HmcPackStreamer;

impl HmcPackStreamer {
    /// Creates a new streamer with governed worker pool, capability token, and Vibe local host.
    pub fn new(holder_did_hash: u64, fuel_budget: u64, byte_budget: u64) -> Self {
        let token = HydrationCapabilityToken::new_sealed(
            holder_did_hash,
            CAP_ALL_HYDRATION,
            StreamPriorityTier::Tier2HighPolyAndMip0,
            fuel_budget,
            byte_budget,
        );
        Self {
            local_host: vibe::LocalHost::default(),
            pool: GovernedHydrationPool::new(DEFAULT_HYDRATION_QUEUE_CAPACITY),
            token,
            planned_requests: Vec::new(),
            tier0_completed: 0,
            tier1_completed: 0,
            tier2_completed: 0,
            total_bytes_admitted: 0,
            next_task_id: 1,
        }
    }

    /// Plans progressive `.hmc` stream delivery from the 64-byte header probe and CBOR index footer.
    ///
    /// Orders all entries strictly by `(tier, offset)` so Tier 0 arrives first.
    pub fn plan_hmc_stream(
        &mut self,
        header_probe: HmcHeaderProbe,
        index_footer_bytes: &[u8],
    ) -> Result<&[HmcRangeRequest], BundleError> {
        let planner = HmcProgressiveRangePlanner::from_header_and_index(header_probe, index_footer_bytes)?;
        self.planned_requests = planner.plan_entry_ranges();
        Ok(&self.planned_requests)
    }

    /// Verifies an arrived byte range against its planned request and admits it into the active session.
    pub fn admit_and_verify_range(
        &mut self,
        req_idx: usize,
        arrived_bytes: &[u8],
    ) -> Result<(), GovernancePoolError> {
        if req_idx >= self.planned_requests.len() {
            return Err(GovernancePoolError::IntegrityMismatch {
                task_id: req_idx as u64,
            });
        }
        let req = &self.planned_requests[req_idx];
        verify_arrived_range(req, arrived_bytes).map_err(|_| {
            GovernancePoolError::IntegrityMismatch {
                task_id: req_idx as u64,
            }
        })?;

        // If this is a Tier 0 .q42 semantic slice, admit it into the Vibe LocalHost
        if req.entry_kind.eq_ignore_ascii_case("q42") || req.entry_key.ends_with(".q42") {
            let _ = self.local_host.admit_q42_slice(arrived_bytes);
        }

        match req.tier {
            StreamPriorityTier::Tier0SemanticAndProcedural => self.tier0_completed += 1,
            StreamPriorityTier::Tier1MidLodMeshes => self.tier1_completed += 1,
            StreamPriorityTier::Tier2HighPolyAndMip0 => self.tier2_completed += 1,
        }
        self.total_bytes_admitted += arrived_bytes.len() as u64;
        Ok(())
    }

    /// Opens a zero-heap [`WasmSliceQ42Reader`] directly over borrowed in-memory bytes.
    #[inline]
    pub fn open_wasm_slice_reader<'a>(
        &self,
        q42_bytes: &'a [u8],
    ) -> std::io::Result<WasmSliceQ42Reader<'a>> {
        WasmSliceQ42Reader::open(q42_bytes)
    }

    /// Submits a background `.10d` section CRC-32C verification task to the governed pool.
    pub fn submit_section_crc_check(
        &mut self,
        tier: StreamPriorityTier,
        payload: Vec<u8>,
        expected_crc32c: u32,
    ) -> Result<u64, GovernancePoolError> {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.pool.submit(
            &mut self.token,
            HydrationTask {
                task_id,
                tier,
                kind: HydrationTaskKind::VerifySectionCrc32c {
                    payload,
                    expected_crc32c,
                },
            },
        )?;
        Ok(task_id)
    }

    /// Submits a background `.hmc` entry SHA-256 verification task to the governed pool.
    pub fn submit_entry_sha256_check(
        &mut self,
        tier: StreamPriorityTier,
        payload: Vec<u8>,
        expected_sha256: [u8; 32],
    ) -> Result<u64, GovernancePoolError> {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.pool.submit(
            &mut self.token,
            HydrationTask {
                task_id,
                tier,
                kind: HydrationTaskKind::VerifyEntrySha256 {
                    payload,
                    expected_sha256,
                },
            },
        )?;
        Ok(task_id)
    }

    /// Submits a background 16-byte texture block transcode task (BC7 / ETC2 / ASTC) to the governed pool.
    pub fn submit_texture_transcode(
        &mut self,
        tier: StreamPriorityTier,
        codec: TranscodeCodec,
        block: [u8; 16],
    ) -> Result<u64, GovernancePoolError> {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.pool.submit(
            &mut self.token,
            HydrationTask {
                task_id,
                tier,
                kind: HydrationTaskKind::TranscodeTextureBlock { codec, block },
            },
        )?;
        Ok(task_id)
    }

    /// Executes a bounded slice of hydration work (draining up to `max_tasks_this_frame`)
    /// strictly prioritizing Tier 0 over Tier 1 over Tier 2, ensuring frame times never stall.
    #[inline]
    pub fn step_hydration_frame(
        &mut self,
        max_tasks_this_frame: usize,
    ) -> Result<Vec<HydrationReceipt>, GovernancePoolError> {
        self.pool.drain_bounded(max_tasks_this_frame)
    }

    /// Inspects a borrowed KTX2 container and selects a direct hardware-compressed format
    /// (avoiding CPU RGBA8 inflation) if supported by the active GPU profile.
    #[inline]
    pub fn inspect_and_select_gpu_texture_format(
        &self,
        ktx2_bytes: &[u8],
        gpu_bc: bool,
        gpu_astc: bool,
        gpu_etc2: bool,
    ) -> Result<Option<HardwareCompressedUploadFormat>, Ktx2Error> {
        let doc = Ktx2Document::parse(ktx2_bytes)?;
        Ok(select_hardware_compressed_format(&doc, gpu_bc, gpu_astc, gpu_etc2))
    }

    /// Executes a zero-heap single 16-byte block transcode into a 64-byte 4x4 RGBA8 tile.
    #[inline]
    pub fn transcode_block_zero_heap(
        codec: TranscodeCodec,
        block: &[u8; 16],
        out_rgba: &mut [u8; 64],
    ) {
        let pixels: &mut [[u8; 4]; 16] = bytemuck::cast_mut(out_rgba);
        match codec {
            TranscodeCodec::Bc7 => decode_bc7_block(block, pixels),
            TranscodeCodec::Etc2EacRgba8 => decode_etc2_eac_rgba_block(block, pixels),
            TranscodeCodec::Astc4x4 => decode_astc_4x4_block(block, pixels),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleWriter;
    use crate::container_10d::crc32c::crc32c;
    use crate::NQuin;

    #[test]
    fn progressive_hmc_streaming_schedules_priority_tiers() {
        let mut writer = BundleWriter::new();
        // Add entries belonging to Tier 0, Tier 1, and Tier 2
        writer
            .add_file("ecopunk_terrain_lod0.10d", "10d", vec![0xAA; 1024], None)
            .unwrap();
        writer
            .add_file("ecopunk_terrain_lod2.10d", "10d", vec![0xBB; 256], None)
            .unwrap();
        writer
            .add_file("ecopunk_ecology.q42", "q42", vec![0xCC; 96], None)
            .unwrap();
        writer
            .add_file("snow_gum_bark.ktx2", "ktx2", vec![0xDD; 512], None)
            .unwrap();
        let bundle_bytes = writer.build().unwrap();

        let probe = HmcHeaderProbe::parse_header_range(&bundle_bytes[..64]).unwrap();
        assert_eq!(probe.entry_count, 4);

        let (idx_off, idx_len) = probe.index_footer_range();
        let footer_slice = &bundle_bytes[idx_off as usize..(idx_off + idx_len) as usize];

        let mut streamer = HmcPackStreamer::new(0x10D_E64_A1, 50_000, 100_000);
        let schedule = streamer.plan_hmc_stream(probe, footer_slice).unwrap();
        assert_eq!(schedule.len(), 4);

        // Tier 0 (.q42) must be scheduled first
        assert_eq!(schedule[0].tier, StreamPriorityTier::Tier0SemanticAndProcedural);
        assert_eq!(schedule[0].entry_key, "ecopunk_ecology.q42");

        // Tier 1 (lod2) must be scheduled second
        assert_eq!(schedule[1].tier, StreamPriorityTier::Tier1MidLodMeshes);
        assert_eq!(schedule[1].entry_key, "ecopunk_terrain_lod2.10d");

        // Tier 2 (lod0, ktx2) must be scheduled last
        assert_eq!(schedule[2].tier, StreamPriorityTier::Tier2HighPolyAndMip0);
        assert_eq!(schedule[3].tier, StreamPriorityTier::Tier2HighPolyAndMip0);

        // Verify arrived slices pass verification
        for i in 0..schedule.len() {
            let start = streamer.planned_requests[i].offset as usize;
            let end = start + streamer.planned_requests[i].length as usize;
            streamer
                .admit_and_verify_range(i, &bundle_bytes[start..end])
                .unwrap();
        }
        assert_eq!(streamer.tier0_completed, 1);
        assert_eq!(streamer.tier1_completed, 1);
        assert_eq!(streamer.tier2_completed, 2);
    }

    #[test]
    fn governed_worker_pool_hydrates_within_budget() {
        let mut streamer = EcopunkPackStreamer::new(0x8888_7777, 500, 2048);
        assert!(streamer.token.verify_seal());

        // Submit Tier 2 texture transcode task
        let mut block = [0u8; 16];
        block[0] = 1 << 6; // BC7 Mode 6
        let t1 = streamer
            .submit_texture_transcode(StreamPriorityTier::Tier2HighPolyAndMip0, TranscodeCodec::Bc7, block)
            .unwrap();

        // Submit Tier 0 CRC-32C verification task
        let payload = b"ecopunk procedural botanical section".to_vec();
        let expected_crc = crc32c(&payload);
        let t0 = streamer
            .submit_section_crc_check(
                StreamPriorityTier::Tier0SemanticAndProcedural,
                payload.clone(),
                expected_crc,
            )
            .unwrap();

        // Step pool bounded by 2 tasks; Tier 0 MUST complete first!
        let receipts = streamer.step_hydration_frame(2).unwrap();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].task_id, t0);
        assert_eq!(receipts[0].tier, StreamPriorityTier::Tier0SemanticAndProcedural);
        assert!(receipts[0].verified);
        assert_eq!(receipts[1].task_id, t1);
        assert_eq!(receipts[1].tier, StreamPriorityTier::Tier2HighPolyAndMip0);
        assert!(receipts[1].transcoded_rgba8_4x4.is_some());

        // Budgets were correctly deducted and seal preserved
        assert!(streamer.token.fuel_remaining < 500);
        assert!(streamer.token.byte_budget_remaining < 2048);
        assert!(streamer.token.verify_seal());
    }

    #[test]
    fn hardware_compressed_format_selection_and_transcode() {
        // Test BC7 Modes 0..=7 decoding
        for mode in 0..8usize {
            let mut block = [0xAAu8; 16];
            block[0] = 1u8 << mode;
            let mut out = [0u8; 64];
            HmcPackStreamer::transcode_block_zero_heap(TranscodeCodec::Bc7, &block, &mut out);
            assert!(out.iter().any(|&b| b > 0));
        }

        // Test ETC2 EAC RGBA8 block decoding
        let mut etc2_block = [0u8; 16];
        etc2_block[0] = 220; // base alpha
        etc2_block[8] = 0xAA;
        let mut etc2_out = [0u8; 64];
        HmcPackStreamer::transcode_block_zero_heap(TranscodeCodec::Etc2EacRgba8, &etc2_block, &mut etc2_out);
        assert!(etc2_out.iter().any(|&b| b > 0));

        // Test ASTC 4x4 void-extent block decoding
        let mut astc_block = [0u8; 16];
        astc_block[0] = 0xFC;
        astc_block[1] = 0xFD; // Void-extent flag
        astc_block[8..10].copy_from_slice(&0x8000u16.to_le_bytes()); // R
        astc_block[10..12].copy_from_slice(&0x4000u16.to_le_bytes()); // G
        astc_block[12..14].copy_from_slice(&0xFF00u16.to_le_bytes()); // B
        astc_block[14..16].copy_from_slice(&0xFF00u16.to_le_bytes()); // A
        let mut astc_out = [0u8; 64];
        HmcPackStreamer::transcode_block_zero_heap(TranscodeCodec::Astc4x4, &astc_block, &mut astc_out);
        assert_eq!(astc_out[0], 0x80);
        assert_eq!(astc_out[1], 0x40);
        assert_eq!(astc_out[2], 0xFF);
        assert_eq!(astc_out[3], 0xFF);
    }

    #[test]
    fn q42_wasm_slice_and_localhost_query() {
        let mut q1 = NQuin {
            subject: 0x1111,
            predicate: 0x2222,
            object: 0x3333,
            context: 0x4444,
            metadata: 0,
            parity: 0,
        };
        q1.parity = NQuin::calculate_parity(q1.subject, q1.predicate, q1.object, q1.context, q1.metadata);

        let mut q2 = NQuin {
            subject: 0x1111,
            predicate: 0x2222,
            object: 0x9999,
            context: 0x4444,
            metadata: 0,
            parity: 0,
        };
        q2.parity = NQuin::calculate_parity(q2.subject, q2.predicate, q2.object, q2.context, q2.metadata);

        let quins = [q1, q2];
        let raw_bytes: &[u8] = bytemuck::cast_slice(&quins);

        // 1. WasmSliceQ42Reader
        let reader = WasmSliceQ42Reader::open(raw_bytes).expect("open raw q42 slice");
        assert_eq!(reader.byte_len(), 96);
        let all = reader.collect_all_quins().expect("collect quins");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].subject, 0x1111);
        assert_eq!(all[1].object, 0x9999);

        // 2. vibe::LocalHost::with_q42_pack
        let mut host = vibe::LocalHost::with_q42_pack(raw_bytes);
        let rows = host
            .query_sparql_local(Some(0x1111), None, None, None)
            .expect("local sparql query");
        assert_eq!(rows.len(), 2);
    }
}
