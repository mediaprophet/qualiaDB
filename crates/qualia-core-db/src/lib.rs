use std::fs::File;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_core::Stream;
use zeroize::Zeroize;

pub mod query_engine;
pub mod n3_parser;
pub mod n3_compiler;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_reader;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_lex;
pub mod llm_agent;
pub mod profiles;
pub mod solid_ldp;
pub mod wasm_bridge;
pub mod modalities;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_ingest;
#[cfg(not(target_arch = "wasm32"))]
pub mod chat_relay_daemon;
#[cfg(not(target_arch = "wasm32"))]
pub mod webtorrent_seeder;
#[cfg(not(target_arch = "wasm32"))]
pub mod webtorrent_routes;
#[cfg(not(target_arch = "wasm32"))]
pub mod comorbidity_eval;

/// The Global Capability Registry exposes which features are compiled into the
/// current qualia-core-db binary. This allows the CLI to dynamically self-document
/// and progressively expose features like SHACL extensions or specific logic modalities.
/// Crate semver baked in at compile time — shared by daemon `/health`, CLI, and WASM `get_engine_version()`.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CAPABILITY_REGISTRY: &[&str] = &[
    "SHACL",
    "Memory",
    "Database",
    "Migration",
    "DeonticLogic",
    "EpistemicLogic",
    "ParaconsistentLogic",
    "DialecticalLogic",
    "TemporalLTL",
    "Bioinformatics",
    "OrganicChemistry",
    "Economics",
    "DicomImaging",
    "ComorbidityEval",
    "CogAI",
    "N3Compiler",
];

/// Bare-metal 40-byte continuous statement container for the Qualia engine.
/// Fully optimized for zero-copy memory operations on post-2020 architectures.
#[repr(C, align(16))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Zeroize, bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct QualiaQuin {
    /// Subject identifier code reference index
    pub subject: u64,
    /// Predicate relation code reference index
    pub predicate: u64,
    /// Object value or entity code reference index
    pub object: u64,
    /// Graph Context identifier code reference index
    pub context: u64,
    /// The Fifth Vector: Metadata, Policy bitmasks, and geometric traits
    pub metadata: u64,
    /// The Sixth Vector: ECC Parity and Checksum bits (making the Quin 48 bytes)
    pub parity: u64,
}

impl QualiaQuin {
    const NESTED_BIT_MASK: u64 = 1 << 63;

    #[inline(always)]
    pub fn is_subject_nested(&self) -> bool {
        (self.subject & Self::NESTED_BIT_MASK) != 0
    }

    #[inline(always)]
    pub fn get_subject_literal_id(&self) -> u64 {
        self.subject & !Self::NESTED_BIT_MASK
    }

    // Extract the two bits (61-62) of the metadata slot for fast lane classification
    const LANE_MASK: u64 = 0b11 << 61;
    const SHIFT_LANE: u32 = 61;

    #[inline(always)]
    pub fn identify_routing_lane(&self) -> PermissiveRoutingLane {
        // Since it's packed, taking reference to field might be unsafe in some contexts,
        // but passing self by reference and copying the field is usually fine, 
        // though `self.metadata` directly copies if it's Copy. 
        // Let's use `let metadata = { self.metadata };` to safely copy if needed, 
        // but `self.metadata` usually works if we don't take a reference to it.
        let metadata = self.metadata;
        let lane_bits = (metadata & Self::LANE_MASK) >> Self::SHIFT_LANE;
        match lane_bits {
            0x01 => PermissiveRoutingLane::EnforcePermissiveCommons,
            0x02 => PermissiveRoutingLane::EnforceBilateralMicroCommons,
            0x03 => PermissiveRoutingLane::SpatiotemporalAmbiguous,
            _    => PermissiveRoutingLane::PassthroughStandard,
        }
    }

    pub const SENSITIVITY_PUBLIC: u8 = 0x00;
    pub const SENSITIVITY_RESTRICTED: u8 = 0x01;
    pub const SENSITIVITY_CLASSIFIED: u8 = 0x02;

    #[inline(always)]
    pub fn get_sensitivity_byte(&self) -> u8 {
        (self.context >> 56) as u8
    }

    #[inline(always)]
    pub fn set_sensitivity_byte(&mut self, sensitivity: u8) {
        // Clear top 8 bits
        self.context &= 0x00FF_FFFF_FFFF_FFFF;
        // Set new sensitivity
        self.context |= (sensitivity as u64) << 56;
    }

    /// Extracts the Lamport Logical Clock embedded in bits 32-60 of the metadata.
    #[inline(always)]
    pub fn extract_lamport_clock(&self) -> u32 {
        ((self.metadata >> 32) & 0x1FFF_FFFF) as u32
    }

    /// Sets the Lamport Logical Clock in bits 32-60, preserving payload and routing lanes.
    #[inline(always)]
    pub fn set_lamport_clock(&mut self, clock: u32) {
        // Clear bits 32..60
        self.metadata &= !(0x1FFF_FFFFu64 << 32);
        // Set new clock
        self.metadata |= ((clock as u64) & 0x1FFF_FFFF) << 32;
    }

    /// Extracts the geometric pruning sector ID from the raw metadata payload.
    /// Payload is stored in bits 0-31 to reserve 32-60 for the CRDT Lamport clock.
    #[inline(always)]
    pub fn extract_clean_metadata_value(&self) -> u64 {
        self.metadata & 0xFFFF_FFFF
    }

    #[inline(always)]
    pub fn verify_ecc_parity(&self) -> bool {
        // Mock ECC parity check. In real implementation, this would compute CRC-64.
        // For testing, we just assume it's valid unless parity is u64::MAX.
        self.parity != u64::MAX
    }

    #[inline(always)]
    pub fn new_conduct_violation(reason: &[u8]) -> Self {
        let mut quin = Self::default();
        quin.predicate = 0x42_0000_0000_0000; // Fake hash for q42:conductViolation
        // Truncate reason to 8 bytes for object for simplicity
        let mut obj_bytes = [0u8; 8];
        let len = core::cmp::min(reason.len(), 8);
        obj_bytes[..len].copy_from_slice(&reason[..len]);
        quin.object = u64::from_le_bytes(obj_bytes);
        quin
    }
}

pub const MODALITY_FLAG_LLM_TENSOR: u8 = 0b1001;
pub const MODALITY_FLAG_DENSE_PHYSICS: u8 = 0b1000;
/// CLIP / mmproj vision encoder tensors in a multimodal GGUF bundle.
pub const MODALITY_FLAG_VISION_TENSOR: u8 = 0b1010;

pub trait QuinPointerExt {
    fn extract_modality_flag(&self) -> u8;
    fn extract_byte_offset(&self) -> u64;
}

impl QuinPointerExt for QualiaQuin {
    #[inline(always)]
    fn extract_modality_flag(&self) -> u8 {
        (self.object >> 60) as u8
    }

    #[inline(always)]
    fn extract_byte_offset(&self) -> u64 {
        self.object & 0x0FFF_FFFF_FFFF_FFFF
    }
}

pub const QUINS_PER_BLOCK: usize = 850;
pub const BLOCK_MULTIPLIER_SIZE: usize = 40960; // Exact alignment across 10 sectors

#[repr(C, align(4096))]
pub struct QualiaSuperBlock {
    /// Monotonically increasing sequencing page tracker index ID
    pub block_sequence_id: u64,
    /// Binary token identifying the decentralized micro-commons owner DID root node
    pub storage_owner_did: u64,
    /// Active, filled quin array items within current page focus
    pub active_quin_count: u64,
    /// Validation value checksum bit flags
    pub validation_checksum: u32,
    /// Hard-coded sector configuration properties context (and FEA bounds)
    pub hardware_profile_flags: u32,
    /// Identifier for attached 3D voxel/tetrahedra FEA structural mesh layer
    pub fea_mesh_index_id: u64,
    /// Fixed trailing block buffer space to force page-header normalization 
    pub layout_padding: [u8; 120], // Adjusted padding to maintain exactly 160 bytes header
    /// Contiguous un-padded sequential database array zones
    pub quin_ledger: [QualiaQuin; QUINS_PER_BLOCK],
}

pub mod archive;

/// Target lanes configuration identifiers for Qualia data pipelines
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]

/// Standard payload mask denoting Ambient Telemetry (Passthrough routing) Path: Local sensor traces, timeline events, and internal files
pub enum PermissiveRoutingLane {
    /// Passthrough Fast Path: Local sensor traces, timeline events, and internal files
    PassthroughStandard = 0x00,
    /// Enforces Permissive Commons compensation milestone evaluation gates
    EnforcePermissiveCommons = 0x01,
    /// Enforces absolute multi-signatory safeguards for sensitive personal links
    EnforceBilateralMicroCommons = 0x02,
    /// Triggers GPU/NPU to run bounding hull math and linguistic semantic checking
    SpatiotemporalAmbiguous = 0x03,
}

// Bitwise parameters checked for targeted DID tracks
pub const MASK_AUTHENTICATED_NATURAL_PERSON: u16 = 0b0000_0001;
pub const MASK_BILATERAL_IDENTITY_LOCKED:   u16 = 0b0000_0010;
pub const MASK_COMMERCIAL_BILLABLE_GATE:    u16 = 0b0000_0100;
pub const MASK_WORK_OBLIGATION_SATISFIED:   u16 = 0b0000_1000;

#[inline(always)]
pub fn evaluate_permissive_runtime_gate(
    entry_policy_mask: u16, 
    requesting_agent_signature_flags: u16
) -> bool {
    // If permissive commons work metrics or cost recoupments are met, data opens at zero cost
    if (entry_policy_mask & MASK_WORK_OBLIGATION_SATISFIED) != 0 {
        return true;
    }

    // Halt corporate analytics data mining if programmatic micro-payment ticks fail
    if (requesting_agent_signature_flags & MASK_COMMERCIAL_BILLABLE_GATE) != 0 
        && (entry_policy_mask & MASK_COMMERCIAL_BILLABLE_GATE) != 0 
    {
        return false; 
    }

    // Multi-signatory guardian/ward validation constraints check
    if (entry_policy_mask & MASK_BILATERAL_IDENTITY_LOCKED) != 0 
        && (requesting_agent_signature_flags & MASK_AUTHENTICATED_NATURAL_PERSON) == 0 
    {
        return false;
    }

    true
}

pub struct QuinIncrementalScanner<'a> {
    pub file_descriptor: &'a File,
    pub block_sector_offsets: &'a [u64],
    pub current_cursor: usize,
    pub agent_signature_attributes: u16,
    /// Stack pre-allocated workspace ensures the app memory footprint remains flatlined
    pub allocated_working_buffer: QualiaSuperBlock,
}

impl<'a> Stream for QuinIncrementalScanner<'a> {
    type Item = Result<Vec<QualiaQuin>, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current_cursor >= self.block_sector_offsets.len() {
            return Poll::Ready(None); // Stream scan sequence completed
        }

        let file_offset = self.block_sector_offsets[self.current_cursor];
        if file_offset == 0 { return Poll::Ready(None); }

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::FileExt;
            
            // Unpack layout buffer straight into register space via raw block copy paths
            let destination_ptr = &mut self.allocated_working_buffer as *mut _ as *mut u8;
            let byte_slice = unsafe { std::slice::from_raw_parts_mut(destination_ptr, BLOCK_MULTIPLIER_SIZE) };

            if let Err(e) = self.file_descriptor.read_exact_at(byte_slice, file_offset) {
                return Poll::Ready(Some(Err(e)));
            }
        }

        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::FileExt;
            
            let destination_ptr = &mut self.allocated_working_buffer as *mut _ as *mut u8;
            let byte_slice = unsafe { std::slice::from_raw_parts_mut(destination_ptr, BLOCK_MULTIPLIER_SIZE) };

            let mut bytes_read = 0;
            while bytes_read < BLOCK_MULTIPLIER_SIZE {
                match self.file_descriptor.seek_read(&mut byte_slice[bytes_read..], file_offset + bytes_read as u64) {
                    Ok(0) => return Poll::Ready(Some(Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "failed to fill whole buffer")))),
                    Ok(n) => bytes_read += n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => return Poll::Ready(Some(Err(e))),
                }
            }
        }

        // Taking a reference to a field of a packed struct is fine if we just extract it.
        // Wait, we can't take a reference to a packed struct element without caution.
        // Using `std::ptr::addr_of!` or just making a local copy is safe.
        // `self.allocated_working_buffer.quin_ledger[0]` copies the 40-byte struct because it implements Copy.
        let sampling_quin = self.allocated_working_buffer.quin_ledger[0];
        
        if !sampling_quin.verify_ecc_parity() {
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Quin failed ECC parity check - Sector corrupted",
            ))));
        }

        match sampling_quin.identify_routing_lane() {
            PermissiveRoutingLane::EnforcePermissiveCommons => {
                let bitmask = sampling_quin.extract_clean_metadata_value() as u16;
                if !evaluate_permissive_runtime_gate(bitmask, self.agent_signature_attributes) {
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Target resource permissive commons access criteria unfulfilled",
                    ))));
                }
            },
            PermissiveRoutingLane::EnforceBilateralMicroCommons => {
                let _relation_token = sampling_quin.extract_clean_metadata_value();
                // Core evaluation checks require signature presence before output emission
                if (self.agent_signature_attributes & MASK_AUTHENTICATED_NATURAL_PERSON) == 0 {
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Protected Bilateral Micro-Commons authorization token missing",
                    ))));
                }
            },
            PermissiveRoutingLane::PassthroughStandard => {
                // Directly bypasses permission check matrices for regular local database paths
            },
            PermissiveRoutingLane::SpatiotemporalAmbiguous => {
                // Routed to the Geometric Pruning Pipeline and Agent Orchestrator
            }
        }

        self.current_cursor += 1;
        let elements_in_frame = self.allocated_working_buffer.active_quin_count as usize;
        
        // Cannot take a slice of an unaligned array. However, `QualiaQuin` is 40 bytes, which is a multiple of 8.
        // But `#[repr(C, packed)]` causes the elements in `quin_ledger` to be tightly packed with 1-byte alignment.
        // But since it's 40 bytes (multiple of 8), they end up exactly where they would be if aligned to 8!
        // We can safely create a Vec by copying element by element to avoid unaligned reference warnings, or just use `to_vec()` if it compiles.
        // Let's use a safe iterator to copy:
        let mut emitted_vector_slice = Vec::with_capacity(elements_in_frame);
        for i in 0..elements_in_frame {
            emitted_vector_slice.push(self.allocated_working_buffer.quin_ledger[i]);
        }

        Poll::Ready(Some(Ok(emitted_vector_slice)))
    }
}

impl Drop for QualiaSuperBlock {
    fn drop(&mut self) {
        // Safe volatile memory scrubbing to clear tracking signatures.
        unsafe {
            std::ptr::write_volatile(self as *mut _, std::mem::zeroed());
        }
    }
}

pub mod geometric;
pub mod indexing;
pub mod logic;
pub mod orchestrator;
pub mod resolver;
pub mod spatial;
pub mod rules;
#[cfg(not(target_arch = "wasm32"))]
pub mod npu_ffi;
#[cfg(not(target_arch = "wasm32"))]
pub mod daemon;
pub mod daemon_graph;
pub mod tee_ffi;
#[cfg(not(target_arch = "wasm32"))]
pub mod wal;
pub mod crdt;
#[cfg(not(target_arch = "wasm32"))]
pub mod sync;
pub mod cbor_compiler;
pub mod git_bridge;
pub mod tax_schema;
pub mod spatial_sieve;
pub mod webizen;
pub mod shacl_compiler;
pub mod owl_to_shacl;
pub mod agency;
pub mod query_compiler;
pub mod fuzz_testing;
pub mod ingestion;
pub mod lexicon;
pub mod storage;
pub mod telemetry;
#[cfg(not(target_arch = "wasm32"))]
pub mod rpc;
#[cfg(not(target_arch = "wasm32"))]
pub mod ilp_dispatcher;
#[cfg(not(target_arch = "wasm32"))]
pub mod nym_adapter;

pub mod mini_parser;
pub mod webizen_bytecode;
pub mod identifier;
pub mod bioinformatics;
pub mod clinical_engine;
pub mod organic_chemistry;
pub mod ode_solver;
pub mod quantum_dft;
pub mod qubo_compiler;
pub mod qpu_ingress;
pub mod thermodynamics;
#[cfg(not(target_arch = "wasm32"))]
pub mod daemon_swarm;
#[cfg(not(target_arch = "wasm32"))]
pub mod gguf_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod gguf_sharder;
#[cfg(target_os = "windows")]
pub mod directml_bridge;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod metal_bridge;
pub mod resource_catalog;
#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_server;

#[cfg(target_os = "android")]
pub mod jni_bridge;

#[cfg(target_arch = "wasm32")]

#[cfg(target_arch = "wasm32")]
pub mod wasm_edge;

/// A zero-allocation compile-time hashing function for Q-Turtle macros.
/// Uses the FNV-1a algorithm to hash strings into 64-bit Quin vectors natively.
pub const fn q_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = hash ^ (bytes[i] as u64);
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

/// Advanced 2026 Q-Turtle Macro
/// Translates terse semantic triples into physical 48-byte hardware Quins 
/// strictly at compile time. Eliminates runtime string allocations entirely.
#[macro_export]
macro_rules! q_turtle {
    ($s:expr, $p:expr, $o:expr) => {
        $crate::QualiaQuin {
            subject: $crate::q_hash($s),
            predicate: $crate::q_hash($p),
            object: $crate::q_hash($o),
            context: 0,
            metadata: 0b01 << 61, // Default to Permissive Commons routing
            parity: 0,
        }
    };
}

// Tests for Antigravity Validation Pipeline
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualia_spatial_val() {
        use crate::spatial::{SpatiotemporalQuadTree, embed_h3_context};
        
        let h3_index = 0x8a2a1072b59ffff; // Mock H3 cell index
        let context_val = embed_h3_context(h3_index);
        assert_eq!(context_val, h3_index, "Failed to embed H3 index into context");

        let quad_tree = SpatiotemporalQuadTree {
            root_bounds: (0.0, 0.0, 100.0, 100.0),
        };
        
        let results = quad_tree.query_region(10.0, 10.0, 20.0, 20.0);
        // We expect it to be empty since it's a structural mock
        assert_eq!(results.len(), 0, "SpatiotemporalQuadTree placeholder query failed");
    }

    #[test]
    fn qualia_logic_val() {
        use crate::logic::{WebizenVM, WebizenOpcode, WebizenCompiler};
        let q = QualiaQuin { subject: 0, predicate: 100, object: 18, context: 0, metadata: 0, parity: 0 };
        
        let mut vm = WebizenVM::new();
        // Use the Compiler mock to generate bytecode for the constraint: 
        // Must have predicate 100 and object 18.
        let bytecode = WebizenCompiler::compile_mock_constraint();
        vm.load_bytecode(&bytecode);
        
        let result = vm.execute_constraint(&q);
        assert_eq!(result, true, "Webizen VM failed to validate constraint byte-code");
    }

    #[test]
    fn qualia_webizen_guardianship() {
        use crate::logic::{WebizenVM, WebizenOpcode};
        
        // 0b11 << 61 signals SpatiotemporalAmbiguous for bounding logic
        let q = QualiaQuin { subject: 0, predicate: 0, object: 0, context: 0, metadata: 0b11 << 61 | 500, parity: 0 };
        
        let mut vm = WebizenVM::new();
        let bytecode = vec![
            WebizenOpcode::EvalMetadataMask(499), // Try to match exactly 499 on the lower 16 bits
            WebizenOpcode::HaltIfFalse,
        ];
        vm.load_bytecode(&bytecode);
        
        let result = vm.execute_constraint(&q);
        assert_eq!(result, false, "Webizen VM failed to deny mismatched EvalMetadataMask");
    }

    #[test]
    fn qualia_ldp_rdf_star_mapping() {
        use crate::solid_ldp::SolidLdpFacade;
        let q = QualiaQuin { subject: 1, predicate: 2, object: 3, context: 4, metadata: 0b11 << 61 | 555, parity: 0 };
        
        let rdf_output = SolidLdpFacade::serialize_to_rdf_star(&q);
        
        // Ensure it mapped to RDF quads with context
        assert!(rdf_output.contains("GRAPH <urn:qualia:context:4>"));
        // Ensure RDF-star reification with GeoSPARQL WKT is present because it's SpatiotemporalAmbiguous
        assert!(rdf_output.contains("geo:asWKT"));
        assert!(rdf_output.contains("qualia:hardwareIntegrity \"VERIFIED_ECC_PASS\""));
    }

    #[test]
    fn qualia_vector_density() {
        use crate::geometric::{VectorSectorMap, BoundingHull, extract_spatial_projection};
        let q = QualiaQuin { subject: 0, predicate: 0, object: 0, context: 0, metadata: 42, parity: 0 };
        let projection = extract_spatial_projection(&q);
        
        let sector_map = VectorSectorMap { sector_id: 2, active: true }; // 42 % 10 = 2
        assert_eq!(sector_map.contains(projection), true, "VectorSectorMap failed to include point within bounding hull");
        
        let out_of_bounds_map = VectorSectorMap { sector_id: 3, active: true };
        assert_eq!(out_of_bounds_map.contains(projection), false, "VectorSectorMap failed to prune out-of-bounds point");
    }

    #[test]
    fn qualia_validate_volatile_drop() {
        let mut block = Box::new(unsafe { std::mem::zeroed::<QualiaSuperBlock>() });
        block.block_sequence_id = 12345;
        assert_eq!(block.block_sequence_id, 12345);
        drop(block);
    }

    #[test]
    fn qualia_validate_quin() {
        assert_eq!(std::mem::size_of::<QualiaQuin>(), 48, "QualiaQuin must be exactly 48 bytes");
    }

    #[test]
    fn qualia_validate_ecc() {
        let mut q = QualiaQuin { subject: 0, predicate: 0, object: 0, context: 0, metadata: 0, parity: 0 };
        assert_eq!(q.verify_ecc_parity(), true, "Valid ECC parity should pass");
        
        q.parity = u64::MAX;
        assert_eq!(q.verify_ecc_parity(), false, "Corrupted ECC parity should fail");
    }

    #[test]
    fn qualia_validate_alignment() {
        assert_eq!(std::mem::size_of::<QualiaSuperBlock>(), 40960, "QualiaSuperBlock must be exactly 40960 bytes (10 sectors)");
        assert_eq!(std::mem::align_of::<QualiaSuperBlock>(), 4096, "QualiaSuperBlock must be page aligned (4096 bytes)");
    }

    #[test]
    fn qualia_validate_routing() {
        // Test 1: Passthrough Standard
        let q1 = QualiaQuin {
            subject: 0, predicate: 0, object: 0, context: 0,
            metadata: 0b00 << 61 | 12345, parity: 0
        };
        assert_eq!(q1.identify_routing_lane(), PermissiveRoutingLane::PassthroughStandard);
        assert_eq!(q1.extract_clean_metadata_value(), 12345);

        // Test 2: Permissive Commons
        let q2 = QualiaQuin {
            subject: 0, predicate: 0, object: 0, context: 0,
            metadata: 0b01 << 61 | 67890, parity: 0
        };
        assert_eq!(q2.identify_routing_lane(), PermissiveRoutingLane::EnforcePermissiveCommons);
        assert_eq!(q2.extract_clean_metadata_value(), 67890);

        // Test 3: Bilateral Micro Commons
        let q3 = QualiaQuin {
            subject: 0, predicate: 0, object: 0, context: 0,
            metadata: 0b10 << 61 | 42, parity: 0
        };
        assert_eq!(q3.identify_routing_lane(), PermissiveRoutingLane::EnforceBilateralMicroCommons);
        assert_eq!(q3.extract_clean_metadata_value(), 42);

        // Test 4: Spatiotemporal Ambiguous
        let q4 = QualiaQuin {
            subject: 0, predicate: 0, object: 0, context: 0,
            metadata: 0b11 << 61 | 999, parity: 0
        };
        assert_eq!(q4.identify_routing_lane(), PermissiveRoutingLane::SpatiotemporalAmbiguous);
        assert_eq!(q4.extract_clean_metadata_value(), 999);
    }

    #[test]
    fn engine_version_matches_cargo_pkg_version() {
        assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!ENGINE_VERSION.is_empty());
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod key_vault;

#[cfg(not(target_arch = "wasm32"))]
pub mod p2p;

#[cfg(not(target_arch = "wasm32"))]
pub mod webizen_sync;

#[cfg(not(target_arch = "wasm32"))]
pub mod web_civics;

pub mod economics;
pub mod deontic_logic;
