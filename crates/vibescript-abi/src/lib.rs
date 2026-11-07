//! `vibescript-abi` — Lightweight, `#![no_std]`, zero-heap extension ABI,
//! self-describing ontological contract, and Sentinel lifecycle interface for VibeScript.

#![no_std]

pub mod descriptor;
pub mod effects;
pub mod identity;
pub mod lifecycle;
pub mod memory;
pub mod sdk;
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
pub use sdk::{
    VIBE_HAS_POSTCONDITION_IRI, VIBE_HAS_PRECONDITION_IRI, VIBE_PRESERVES_INVARIANT_IRI,
    VIBE_PUBLIC_CONTEXT_IRI,
};
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
        assert_eq!(q.parity(), 11 ^ 22 ^ 33 ^ 44 ^ 55);

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

    #[test]
    fn sdk_const_constructors_and_ontology_nquins_verify_5field_parity() {
        const PARAMS: &[ParamSpec] = &[
            ParamSpec::slice("vec3", PrimitiveKind::F64, 3, 3),
            ParamSpec::scalar("scale", PrimitiveKind::F64),
        ];
        const FUNC: FuncDescriptor = FuncDescriptor::const_new(
            "scale_vec3",
            "https://qualiadb.org/ext/test/math/scale_vec3",
            StableFuncId::new(0, 1),
            "(f64[3],f64)->f64[3]",
            PARAMS,
            PrimitiveKind::F64,
            3,
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicTolerance,
            ReversibilityKind::PureNoState,
            ResourceEnvelope::hot_zero_heap(16, 24),
            (0, 1),
            (1, 1),
            (2, 1),
        );
        const SUBS: &[SubNamespaceDescriptor] = &[SubNamespaceDescriptor {
            name: "math",
            sub_id: 0,
            functions: &[FUNC],
        }];
        const PROVIDER: ProviderDescriptor = ProviderDescriptor::const_new(
            "https://qualiadb.org/ext/test",
            "tmath",
            SemVer64::release(0, 2, 1),
            SemVer64::release(0, 1, 0),
            TargetProfileMask::ALL_CPU,
            SUBS,
            8,
        );
        const QUINS: [NQuin48; 3] = [
            NQuin48::precondition(
                "https://qualiadb.org/ext/test/math/scale_vec3",
                "https://qualiadb.org/schema/vibe#FiniteRealInputs",
                10,
            ),
            NQuin48::postcondition(
                "https://qualiadb.org/ext/test/math/scale_vec3",
                "https://qualiadb.org/schema/vibe#FiniteRealOutputs",
                20,
            ),
            NQuin48::invariant(
                "https://qualiadb.org/ext/test/math/scale_vec3",
                "https://qualiadb.org/schema/vibe#Linearity",
                30,
            ),
        ];

        assert!(PROVIDER.canonical_iri.is_valid());
        assert!(PROVIDER.versions.is_engine_compatible());
        assert!(FUNC.canonical_iri.is_valid());
        assert!(FUNC.contract.validate_spans(QUINS.len()));
        assert_eq!(
            FUNC.contract.contract_digest,
            OntologicalContract::canonical_digest(
                FUNC.canonical_iri.fnv1a_hash,
                FUNC.stable_id,
                q_hash_str("(f64[3],f64)->f64[3]"),
                FUNC.effects,
                FUNC.determinism,
                FUNC.reversibility,
                FUNC.envelope,
                (0, 1),
                (1, 1),
                (2, 1),
            )
        );
        for q in &QUINS {
            assert!(q.verify_parity());
            assert_eq!(
                q.parity(),
                q.subject() ^ q.predicate() ^ q.object() ^ q.context() ^ q.metadata()
            );
        }
    }

    #[test]
    fn sdk_finite_slice_readers_and_writers_reject_non_finite_values() {
        let mut buf_f64 = [0u8; 24];
        let mut out_f64 = CallerOutputSlice::new(&mut buf_f64, PrimitiveKind::F64).unwrap();
        out_f64.push_f64_slice(&[1.25, -2.5, 4.0]).unwrap();
        assert_eq!(out_f64.written_elems(), 3);

        let view_f64 =
            BorrowedSliceView::from_raw_bytes(out_f64.written_byte_slice(), PrimitiveKind::F64, 3)
                .unwrap();
        assert_eq!(
            view_f64.read_finite_f64_array::<3>(0).unwrap(),
            [1.25, -2.5, 4.0]
        );

        let nan_bytes = f64::NAN.to_le_bytes();
        let nan_view = BorrowedSliceView::from_raw_bytes(&nan_bytes, PrimitiveKind::F64, 1).unwrap();
        assert_eq!(
            nan_view.read_finite_f64(0),
            Err(VibeTrap::NumericalDomainError)
        );

        let mut buf_f32 = [0u8; 8];
        let mut out_f32 = CallerOutputSlice::new(&mut buf_f32, PrimitiveKind::F32).unwrap();
        out_f32.write_f32_slice(&[0.5f32, 1.5f32]).unwrap();
        let view_f32 =
            BorrowedSliceView::from_raw_bytes(out_f32.written_byte_slice(), PrimitiveKind::F32, 2)
                .unwrap();
        assert_eq!(view_f32.read_finite_f32_array::<2>(0).unwrap(), [0.5, 1.5]);

        let inf_bytes = f32::INFINITY.to_le_bytes();
        let inf_view = BorrowedSliceView::from_raw_bytes(&inf_bytes, PrimitiveKind::F32, 1).unwrap();
        assert_eq!(
            inf_view.read_finite_f32(0),
            Err(VibeTrap::NumericalDomainError)
        );
    }
}
