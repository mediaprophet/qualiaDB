//! Packet `EVAL-22`: Unified Tri-Source Evaluation Aggregator
//! (Etalon + Network -> QualiaDB `.q42` v4).
//!
//! Combines:
//! 1. **Etalon Evaluation Plane**: `EtalonNQuinEvalReceipt` (48-byte `NQuinWire` GPU/CPU sieve &
//!    LSD radix sort parity), bridged sensor sidecar custody receipts, and `ExactnessProfile`
//!    (`ADR 0003`) tolerance checks (`mismatches == 0`).
//! 2. **Network Evaluation Plane (`QDNF` / `QPR` / `CSCP`)**: Handshake/HKDF/rekeying (`R01`–`R04`),
//!    384-bit radix-16 `QSR` + `RAR` route selection & Bellman–Ford dual shortest-path witness
//!    (`R06`–`R07`), chunk-to-SuperBlock `.q42` replication & lease accounting (`R08`–`R11`),
//!    and 40-scenario `policy_labels` egress audit receipts.
//! 3. **QualiaDB Canonical Enumeration Plane**: SHACL-first plane split (`idf:` shapes),
//!    5-field `NQuin` XOR parity (`subject ^ predicate ^ object ^ context ^ metadata`),
//!    `attempt_join` non-entailment across all 4 Identifier Fabric planes, and `USRI` / `CB-USRI`
//!    cosmic coordinate grounding.
//!
//! Never synthesizes a pass percentage that hides a single failed security gate or an unrun
//! hardware lane; exact `passed` / `failed` / `unsupported` counts are reported per source plane.

use std::collections::HashMap;

use crate::foundation::frame_layout::{INLINE_TAG_BOOLEAN, INLINE_TAG_INTEGER};
use crate::net::qdnf::authority::Plane;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::fabric::{attempt_join, JoinAttempt, JoinKey};
use crate::net::qdnf::policy_labels::egress::{enumerate_egress_audit_quin, EgressAuditReceipt};
use crate::q42::execution_profile::{admit_for_policy_decision, ExactnessProfile};
use crate::q42::q42_volume::{UnifiedVolumeBuilder, WasmSliceQ42Reader};
use crate::{q_hash, NQuin};

/// Exact per-plane tally of `passed`, `failed`, and `unsupported` evaluation gates.
///
/// Architectural invariant (`EVAL-22`): never collapse these counts into a synthetic
/// percentage that masks a failed security gate or an unrun/unsupported hardware lane.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourcePlaneCounts {
    pub passed: u32,
    pub failed: u32,
    pub unsupported: u32,
}

impl SourcePlaneCounts {
    #[inline]
    pub const fn is_strictly_qualified(&self) -> bool {
        self.passed > 0 && self.failed == 0 && self.unsupported == 0
    }
}

/// Etalon hardware, sensing custody, and CPU-oracle qualification receipt summary (`ETA-14`–`ETA-17`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EtalonSourceReceipt {
    pub device_backend_hash: u64,
    pub quin_count: u32,
    pub sieve_matches_cpu: bool,
    pub radix_order_matches_cpu: bool,
    pub parity_verified_all: bool,
    pub oracle_mismatches: u32,
    pub sidecar_custody_intact: bool,
    pub hardware_lane_executed: bool,
    pub exactness_profile: ExactnessProfile,
}

impl EtalonSourceReceipt {
    /// Classify this receipt into `(passed, failed, unsupported)` without hiding unrun lanes.
    #[inline]
    pub fn classify(&self) -> (bool, bool, bool) {
        if !self.hardware_lane_executed || self.device_backend_hash == 0 || self.quin_count == 0 {
            return (false, false, true);
        }
        let policy_ok = admit_for_policy_decision(&self.exactness_profile).is_ok();
        let ok = self.sieve_matches_cpu
            && self.radix_order_matches_cpu
            && self.parity_verified_all
            && self.oracle_mismatches == 0
            && self.sidecar_custody_intact
            && policy_ok;
        (ok, !ok, false)
    }

    /// Enumerate this Etalon receipt into 6 canonical 48-byte `NQuin`s with 5-field XOR parity.
    pub fn enumerate_to_quins(&self, epoch: u64) -> [NQuin; 6] {
        let subj = self.device_backend_hash & 0x0FFF_FFFF_FFFF_FFFF;
        let ctx = q_hash("urn:qualia:q42:profile:etalon:eval:1");
        let meta = (epoch & 0x1FFF_FFFF) << 32;
        let (passed, failed, unsupported) = self.classify();
        let make = |pred_iri: &str, obj: u64| -> NQuin {
            let pred = q_hash(pred_iri);
            NQuin {
                subject: subj,
                predicate: pred,
                object: obj,
                context: ctx,
                metadata: meta,
                parity: NQuin::calculate_parity(subj, pred, obj, ctx, meta),
            }
        };
        [
            make(
                "urn:qualia:etalon:eval:quinCount",
                INLINE_TAG_INTEGER | u64::from(self.quin_count),
            ),
            make(
                "urn:qualia:etalon:eval:sieveMatchesCpu",
                INLINE_TAG_BOOLEAN | u64::from(self.sieve_matches_cpu),
            ),
            make(
                "urn:qualia:etalon:eval:radixMatchesCpu",
                INLINE_TAG_BOOLEAN | u64::from(self.radix_order_matches_cpu),
            ),
            make(
                "urn:qualia:etalon:eval:oracleZeroMismatches",
                INLINE_TAG_BOOLEAN | u64::from(self.oracle_mismatches == 0),
            ),
            make(
                "urn:qualia:etalon:eval:sidecarCustodyIntact",
                INLINE_TAG_BOOLEAN | u64::from(self.sidecar_custody_intact),
            ),
            make(
                "urn:qualia:etalon:eval:qualifiedPass",
                INLINE_TAG_BOOLEAN | u64::from(passed && !failed && !unsupported),
            ),
        ]
    }
}

/// Network (`QDNF` / `QPR` / `CSCP`) evaluation receipt summary (`NET-18`–`NET-21`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkSourceReceipt {
    pub session_id_hash: u64,
    pub handshake_transcript_bound: bool,
    pub rekey_generation_verified: bool,
    pub qsr_384bit_verified: bool,
    pub spf_dual_witness_verified: bool,
    pub superblock_replication_verified: bool,
    pub locator_no_proxy_launder_verified: bool,
    pub egress_receipt: EgressAuditReceipt,
}

impl NetworkSourceReceipt {
    #[inline]
    pub fn is_passing(&self) -> bool {
        let egress_quin = enumerate_egress_audit_quin(
            &self.egress_receipt,
            q_hash("urn:qualia:q42:profile:network:1"),
            1,
        );
        self.session_id_hash != 0
            && self.handshake_transcript_bound
            && self.rekey_generation_verified
            && self.qsr_384bit_verified
            && self.spf_dual_witness_verified
            && self.superblock_replication_verified
            && self.locator_no_proxy_launder_verified
            && self.egress_receipt.allowed
            && egress_quin.verify_ecc_parity()
    }

    /// Enumerate this QDNF network evaluation receipt into 4 canonical 48-byte `NQuin`s.
    pub fn enumerate_to_quins(&self, epoch: u64) -> [NQuin; 4] {
        let subj = self.session_id_hash & 0x0FFF_FFFF_FFFF_FFFF;
        let ctx = q_hash("urn:qualia:q42:profile:network:1");
        let meta = (epoch & 0x1FFF_FFFF) << 32;
        let make = |pred_iri: &str, obj: u64| -> NQuin {
            let pred = q_hash(pred_iri);
            NQuin {
                subject: subj,
                predicate: pred,
                object: obj,
                context: ctx,
                metadata: meta,
                parity: NQuin::calculate_parity(subj, pred, obj, ctx, meta),
            }
        };
        [
            make(
                "urn:qualia:qdnf:eval:handshakeAndRekeyVerified",
                INLINE_TAG_BOOLEAN
                    | u64::from(self.handshake_transcript_bound && self.rekey_generation_verified),
            ),
            make(
                "urn:qualia:qdnf:eval:qsrAndSpfDualWitnessVerified",
                INLINE_TAG_BOOLEAN
                    | u64::from(self.qsr_384bit_verified && self.spf_dual_witness_verified),
            ),
            make(
                "urn:qualia:qdnf:eval:superblockReplicationVerified",
                INLINE_TAG_BOOLEAN
                    | u64::from(
                        self.superblock_replication_verified
                            && self.locator_no_proxy_launder_verified,
                    ),
            ),
            enumerate_egress_audit_quin(&self.egress_receipt, ctx, epoch),
        ]
    }
}

/// QualiaDB semantic, SHACL, 5-field parity, non-entailment, and cosmic grounding invariants.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualiaDbInvariantsReceipt {
    pub shacl_plane_shapes_valid: bool,
    pub five_field_parity_verified: bool,
    pub plane_join_prohibition_verified: bool,
    pub usri_cb_usri_grounded: bool,
    pub cb_usri_context_token: u64,
    pub contains_sanctuary_claims: bool,
}

impl QualiaDbInvariantsReceipt {
    /// Verify `qdnf::fabric::attempt_join` non-entailment across all `Plane` pairs and `JoinKey::ALL`.
    pub fn verify_all_plane_joins_denied() -> bool {
        let planes = [
            Plane::Entity,
            Plane::Claim,
            Plane::Handle,
            Plane::Instrument,
        ];
        let mut i = 0;
        while i < planes.len() {
            let mut j = 0;
            while j < planes.len() {
                let mut k = 0;
                while k < JoinKey::ALL.len() {
                    if attempt_join(JoinAttempt {
                        left: planes[i],
                        right: planes[j],
                        key: JoinKey::ALL[k],
                    })
                    .is_ok()
                    {
                        return false;
                    }
                    k += 1;
                }
                j += 1;
            }
            i += 1;
        }
        true
    }

    #[inline]
    pub fn is_passing(&self) -> bool {
        self.shacl_plane_shapes_valid
            && self.five_field_parity_verified
            && self.plane_join_prohibition_verified
            && self.usri_cb_usri_grounded
            && (self.cb_usri_context_token & (1u64 << 63)) == 0
            && Self::verify_all_plane_joins_denied()
    }

    pub fn enumerate_to_quins(&self, epoch: u64) -> [NQuin; 4] {
        let subj = q_hash("urn:qualia:eval:invariants:root");
        let ctx = if self.cb_usri_context_token != 0 {
            self.cb_usri_context_token & 0x0FFF_FFFF_FFFF_FFFF
        } else {
            q_hash("urn:omni:v1:physical:L4:sol:earth")
        };
        let meta = (epoch & 0x1FFF_FFFF) << 32;
        let make = |pred_iri: &str, obj: u64| -> NQuin {
            let pred = q_hash(pred_iri);
            NQuin {
                subject: subj,
                predicate: pred,
                object: obj,
                context: ctx,
                metadata: meta,
                parity: NQuin::calculate_parity(subj, pred, obj, ctx, meta),
            }
        };
        [
            make(
                "urn:qualia:eval:shaclPlaneShapesValid",
                INLINE_TAG_BOOLEAN | u64::from(self.shacl_plane_shapes_valid),
            ),
            make(
                "urn:qualia:eval:fiveFieldParityVerified",
                INLINE_TAG_BOOLEAN | u64::from(self.five_field_parity_verified),
            ),
            make(
                "urn:qualia:eval:planeJoinProhibitionVerified",
                INLINE_TAG_BOOLEAN
                    | u64::from(
                        self.plane_join_prohibition_verified
                            && Self::verify_all_plane_joins_denied(),
                    ),
            ),
            make(
                "urn:qualia:eval:usriCosmicGroundingValid",
                INLINE_TAG_BOOLEAN | u64::from(self.usri_cb_usri_grounded),
            ),
        ]
    }
}

/// Aggregated Tri-Source Evaluation Report (`EVAL-22`).
#[derive(Clone, Debug)]
pub struct TriSourceEvaluationReport<'a> {
    pub etalon_receipts: &'a [EtalonSourceReceipt],
    pub network_receipts: &'a [NetworkSourceReceipt],
    pub qualiadb_invariants: QualiaDbInvariantsReceipt,
    pub evaluation_epoch: u64,
}

/// Per-source breakdown returned by [`TriSourceEvaluationReport::summarize_counts`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TriSourceSummaryCounts {
    pub etalon: SourcePlaneCounts,
    pub network: SourcePlaneCounts,
    pub qualiadb: SourcePlaneCounts,
}

impl TriSourceSummaryCounts {
    /// Overall qualification holds iff all three planes have `passed > 0`, `failed == 0`,
    /// and `unsupported == 0`.
    #[inline]
    pub const fn is_qualified(&self) -> bool {
        self.etalon.is_strictly_qualified()
            && self.network.is_strictly_qualified()
            && self.qualiadb.is_strictly_qualified()
    }
}

impl<'a> TriSourceEvaluationReport<'a> {
    /// Compute exact `passed` / `failed` / `unsupported` counts per source plane.
    pub fn summarize_counts(&self) -> TriSourceSummaryCounts {
        let mut summary = TriSourceSummaryCounts::default();

        for r in self.etalon_receipts {
            let (passed, failed, unsupported) = r.classify();
            if passed {
                summary.etalon.passed += 1;
            }
            if failed {
                summary.etalon.failed += 1;
            }
            if unsupported {
                summary.etalon.unsupported += 1;
            }
        }

        for r in self.network_receipts {
            if r.is_passing() {
                summary.network.passed += 1;
            } else {
                summary.network.failed += 1;
            }
        }

        if self.qualiadb_invariants.is_passing() {
            summary.qualiadb.passed = 1;
        } else {
            summary.qualiadb.failed = 1;
        }

        summary
    }

    /// Strict fail-closed qualification gate across all three sources.
    pub fn verify_qualified(&self) -> Result<TriSourceSummaryCounts, QdnfError> {
        let counts = self.summarize_counts();
        if !counts.is_qualified() {
            return Err(QdnfError::Denied);
        }
        Ok(counts)
    }

    /// Serialize the entire tri-source evaluation graph into a unified `.q42` v4 volume
    /// (`Q42_VERSION_V4 = 4`, namespaced `Q42LEX` v4, `PCAT` diagnostic summary, and SHA-256
    /// Merkle root), verifying 5-field XOR parity on every emitted `NQuin`.
    pub fn bake_evaluation_to_q42_v4(&self) -> Result<Vec<u8>, String> {
        let counts = self.summarize_counts();
        let mut quins = Vec::with_capacity(
            self.etalon_receipts.len() * 6 + self.network_receipts.len() * 4 + 4,
        );

        for r in self.etalon_receipts {
            for q in r.enumerate_to_quins(self.evaluation_epoch) {
                if !q.verify_ecc_parity() {
                    return Err("Etalon evaluation Quin failed 5-field XOR parity".to_string());
                }
                quins.push(q);
            }
        }

        for r in self.network_receipts {
            for q in r.enumerate_to_quins(self.evaluation_epoch) {
                if !q.verify_ecc_parity() {
                    return Err("QDNF network evaluation Quin failed 5-field XOR parity".to_string());
                }
                quins.push(q);
            }
        }

        for q in self.qualiadb_invariants.enumerate_to_quins(self.evaluation_epoch) {
            if !q.verify_ecc_parity() {
                return Err("QualiaDB invariant Quin failed 5-field XOR parity".to_string());
            }
            quins.push(q);
        }

        let mut lex_map = HashMap::new();
        for iri in [
            "urn:qualia:q42:profile:etalon:eval:1",
            "urn:qualia:q42:profile:network:1",
            "urn:qualia:etalon:eval:quinCount",
            "urn:qualia:etalon:eval:sieveMatchesCpu",
            "urn:qualia:etalon:eval:radixMatchesCpu",
            "urn:qualia:etalon:eval:oracleZeroMismatches",
            "urn:qualia:etalon:eval:sidecarCustodyIntact",
            "urn:qualia:etalon:eval:qualifiedPass",
            "urn:qualia:qdnf:eval:handshakeAndRekeyVerified",
            "urn:qualia:qdnf:eval:qsrAndSpfDualWitnessVerified",
            "urn:qualia:qdnf:eval:superblockReplicationVerified",
            "urn:qualia:q42:network:egressDecision",
            "urn:qualia:q42:network:EgressAllow",
            "urn:qualia:q42:network:EgressDeny",
            "urn:qualia:eval:invariants:root",
            "urn:qualia:eval:shaclPlaneShapesValid",
            "urn:qualia:eval:fiveFieldParityVerified",
            "urn:qualia:eval:planeJoinProhibitionVerified",
            "urn:qualia:eval:usriCosmicGroundingValid",
            "urn:omni:v1:physical:L4:sol:earth",
        ] {
            lex_map.insert(q_hash(iri), iri.to_string());
        }

        let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex_map)
            .map_err(|e| format!("failed to build Q42LEX v4: {e:?}"))?;
        builder = if self.qualiadb_invariants.contains_sanctuary_claims {
            builder.with_sanctuary()
        } else {
            builder.with_permissive_commons()
        };

        quins.sort_unstable_by_key(|q| q.object);
        builder
            .push_block(0, &quins)
            .map_err(|e| format!("failed to push tri-source SuperBlock: {e}"))?;

        let diagnostic_json = format!(
            "{{\"schema\":\"urn:qualia:eval:tri-source:1\",\"version\":4,\"qualified\":{},\"etalon\":{{\"passed\":{},\"failed\":{},\"unsupported\":{}}},\"network\":{{\"passed\":{},\"failed\":{},\"unsupported\":{}}},\"qualiadb\":{{\"passed\":{},\"failed\":{},\"unsupported\":{}}}}}",
            counts.is_qualified(),
            counts.etalon.passed,
            counts.etalon.failed,
            counts.etalon.unsupported,
            counts.network.passed,
            counts.network.failed,
            counts.network.unsupported,
            counts.qualiadb.passed,
            counts.qualiadb.failed,
            counts.qualiadb.unsupported,
        );
        builder
            .add_payload_catalogue_utf8(
                "urn:qualia:eval:tri-source:diagnostic-summary",
                "application/json",
                Some("en"),
                "tri-source-diagnostic-summary",
                &diagnostic_json,
            )
            .map_err(|e| format!("failed to attach PCAT diagnostic summary: {e}"))?;

        let bytes = builder.finish_to_bytes();
        let reader = WasmSliceQ42Reader::open(&bytes)
            .map_err(|e| format!("baked tri-source Q42 v4 volume failed slice validation: {e}"))?;
        if reader.block_count() == 0 {
            return Err("baked tri-source Q42 v4 volume has zero SuperBlocks".to_string());
        }
        Ok(bytes)
    }
}
