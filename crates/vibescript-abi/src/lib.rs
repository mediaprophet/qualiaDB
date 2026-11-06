//! `vibescript-abi` — Lightweight, `#![no_std]`, zero-heap extension ABI,
//! self-describing ontological contract, and Sentinel lifecycle interface for VibeScript.

#![no_std]

pub mod descriptor;
pub mod effects;
pub mod identity;
pub mod lifecycle;
pub mod memory;
pub mod version;

pub use descriptor::{
    FuncDescriptor, OntologicalContract, ParamSpec, ProviderDescriptor, SubNamespaceDescriptor,
    VibeNamespaceProvider,
};
pub use effects::{
    AsyncExecutionClass, DeterminismGuarantee, EffectSet, ExecutionMode, ResourceEnvelope,
    ReversibilityKind, TargetProfileMask,
};
pub use identity::{
    q_hash, q_hash_str, CanonicalIri, PreparedCallHandle, RegistryGeneration, StableFuncId,
    VibeOpCode, FNV1A_64_OFFSET_BASIS, FNV1A_64_PRIME,
};
pub use lifecycle::{
    AdmissionRequest, AdmissionTicket, CompletionStatus, ExecutionReceiptHeader, FailureStage,
    SentinelPolicyGate, VibeTrap,
};
pub use memory::{BorrowedSliceView, CallerOutputSlice, NQuin48, PrimitiveKind, ResourceHandle};
pub use version::{ContractVersionAxes, SemVer64, SemVerReq};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nquin48_layout_and_parity() {
        assert_eq!(core::mem::size_of::<NQuin48>(), 48);
        assert_eq!(core::mem::align_of::<NQuin48>(), 16);
        let q = NQuin48::from_parts(11, 22, 33, 44, 55);
        assert!(q.verify_parity());
        assert_eq!(q.parity(), 11 ^ 22 ^ 33 ^ 44);

        let mut corrupted = q;
        corrupted.0[2] ^= 1;
        assert!(!corrupted.verify_parity());
    }

    #[test]
    fn semver64_caret_handles_zero_major_and_zero_minor_correctly() {
        // ^0.4.0 -> >=0.4.0, <0.5.0
        let req_0_4 = SemVerReq::Caret(SemVer64::release(0, 4, 0));
        assert!(!req_0_4.matches(SemVer64::release(0, 3, 9)));
        assert!(req_0_4.matches(SemVer64::release(0, 4, 0)));
        assert!(req_0_4.matches(SemVer64::release(0, 4, 9)));
        assert!(!req_0_4.matches(SemVer64::release(0, 5, 0)));
        assert!(!req_0_4.matches(SemVer64::release(1, 0, 0)));

        // ^0.0.3 -> ==0.0.3
        let req_0_0_3 = SemVerReq::Caret(SemVer64::release(0, 0, 3));
        assert!(!req_0_0_3.matches(SemVer64::release(0, 0, 2)));
        assert!(req_0_0_3.matches(SemVer64::release(0, 0, 3)));
        assert!(!req_0_0_3.matches(SemVer64::release(0, 0, 4)));
        assert!(!req_0_0_3.matches(SemVer64::release(0, 1, 0)));

        // ^1.2.3 -> >=1.2.3, <2.0.0
        let req_1_2_3 = SemVerReq::Caret(SemVer64::release(1, 2, 3));
        assert!(!req_1_2_3.matches(SemVer64::release(1, 2, 2)));
        assert!(req_1_2_3.matches(SemVer64::release(1, 2, 3)));
        assert!(req_1_2_3.matches(SemVer64::release(1, 9, 0)));
        assert!(!req_1_2_3.matches(SemVer64::release(2, 0, 0)));

        // Prerelease / abi_flags != 0 rejected by stable range
        assert!(!req_1_2_3.matches(SemVer64::new(1, 2, 3, 1)));
    }

    #[test]
    fn q_hash_matches_canonical_fnv1a_64() {
        let iri = CanonicalIri::new("https://qualiadb.org/ext/etalon/mechanics/verlet_step");
        assert!(iri.is_valid());
        assert_eq!(
            iri.fnv1a_hash,
            q_hash(b"https://qualiadb.org/ext/etalon/mechanics/verlet_step")
        );

        let bad = CanonicalIri::with_explicit_hash("https://qualiadb.org/ext/test", 0xDEAD_BEEF);
        assert!(!bad.is_valid());
    }

    #[test]
    fn vibe_opcode_packs_and_unpacks_cleanly() {
        let stable = StableFuncId::new(7, 42);
        let op = VibeOpCode::pack(19, stable);
        assert_eq!(op.ns_slot(), 19);
        assert_eq!(op.sub_namespace_id(), 7);
        assert_eq!(op.func_id(), 42);
        assert_eq!(op.stable_id(), stable);
    }

    #[test]
    fn slice_views_and_output_buffer_enforce_bounds_and_parity() {
        let mut raw_out = [0u8; 16];
        let mut out = CallerOutputSlice::new(&mut raw_out, PrimitiveKind::F64).unwrap();
        assert_eq!(out.capacity_elems(), 2);
        assert_eq!(
            out.ensure_elem_capacity(3),
            Err(VibeTrap::OutputBufferTooSmall)
        );
        out.push_f64_le(3.5).unwrap();
        out.push_f64_le(-1.25).unwrap();
        assert_eq!(out.push_f64_le(9.0), Err(VibeTrap::OutputBufferTooSmall));
        assert_eq!(out.written_elems(), 2);

        let view = BorrowedSliceView::from_raw_bytes(out.written_byte_slice(), PrimitiveKind::F64, 2)
            .unwrap();
        assert_eq!(view.read_f64_le(0).unwrap(), 3.5);
        assert_eq!(view.read_f64_le(1).unwrap(), -1.25);
        assert_eq!(view.read_f64_le(2), Err(VibeTrap::SliceBoundsOverflow));
    }

    #[test]
    fn resource_handle_validates_owner_and_generation() {
        let owner = q_hash_str("https://qualiadb.org/ext/etalon/mechanics");
        let handle = ResourceHandle::new(owner, 3, 0, 5, 12);
        assert!(handle.validate(owner, 3, 5).is_ok());
        assert_eq!(
            handle.validate(owner ^ 1, 3, 5),
            Err(VibeTrap::InvalidResourceHandleOwner)
        );
        assert_eq!(
            handle.validate(owner, 3, 6),
            Err(VibeTrap::StaleResourceHandleGeneration)
        );
    }

    #[test]
    fn execution_receipt_seals_to_parity_valid_nquins() {
        let receipt = ExecutionReceiptHeader {
            ticket_id: 101,
            actor_did_hash: q_hash_str("did:q42:alice"),
            provider_iri_hash: q_hash_str("https://qualiadb.org/ext/etalon/mechanics"),
            func_iri_hash: q_hash_str("https://qualiadb.org/ext/etalon/mechanics/verlet_step"),
            stable_id: StableFuncId::new(0, 1),
            mode: ExecutionMode::GovernedApplication,
            status: CompletionStatus::Committed,
            simulated: false,
            reserved: [0; 5],
            policy_generation: 9,
            registry_generation: RegistryGeneration(2),
            ticks_consumed: 14,
            output_bytes_written: 24,
            trap_code: 0,
            failure_stage: 0,
            input_output_fold_hash: 0x1234_5678_9ABC_DEF0,
            lamport_timestamp: 500,
        };
        let quins = receipt.to_audit_nquins();
        assert!(quins[0].verify_parity());
        assert!(quins[1].verify_parity());
    }
}
