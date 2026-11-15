//! Stage S6 (`VS03-13`) Named provider migration: `StagedGraphMutationProvider` (`qgraph`)
//! and `DeonticTemporalProvider` (`modality`) with direct-call reference oracles.

use vibescript_abi::{
    q_hash_str, BorrowedSliceView, CallerOutputSlice, CanonicalIri, ContractVersionAxes,
    DeterminismGuarantee, EffectSet, FuncDescriptor, NQuin48, OntologicalContract, ParamSpec,
    PrimitiveKind, ProviderDescriptor, ResourceEnvelope, ReversibilityKind, SemVer64, StableFuncId,
    SubNamespaceDescriptor, TargetProfileMask, VibeNamespaceProvider, VibeTrap,
};

const SEAL_QUIN_PARAMS: &[ParamSpec] = &[ParamSpec::slice(
    "fields_spocm",
    PrimitiveKind::U64,
    5,
    5,
)];

const VERIFY_QUIN_PARAMS: &[ParamSpec] = &[ParamSpec::slice(
    "quin_words",
    PrimitiveKind::U64,
    6,
    6,
)];

const SEAL_ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(24, 48);
const VERIFY_ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(16, 8);

const STAGING_FUNCS: &[FuncDescriptor] = &[
    FuncDescriptor {
        name: "seal_quin",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/qualia/graph/staging/seal_quin",
        ),
        stable_id: StableFuncId::new(0, 0),
        params: SEAL_QUIN_PARAMS,
        output_kind: PrimitiveKind::U64,
        required_output_elems: 6,
        effects: EffectSet::GRAPH_MUTATE,
        determinism: DeterminismGuarantee::DeterministicExact,
        reversibility: ReversibilityKind::StagedReversible,
        envelope: SEAL_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/qualia/graph/staging/seal_quin"),
                StableFuncId::new(0, 0),
                q_hash_str("(u64[5])->u64[6]"),
                EffectSet::GRAPH_MUTATE,
                DeterminismGuarantee::DeterministicExact,
                ReversibilityKind::StagedReversible,
                SEAL_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(u64[5])->u64[6]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
    FuncDescriptor {
        name: "verify_quin_parity",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/qualia/graph/staging/verify_quin_parity",
        ),
        stable_id: StableFuncId::new(0, 1),
        params: VERIFY_QUIN_PARAMS,
        output_kind: PrimitiveKind::U64,
        required_output_elems: 1,
        effects: EffectSet::GRAPH_READ,
        determinism: DeterminismGuarantee::DeterministicExact,
        reversibility: ReversibilityKind::PureNoState,
        envelope: VERIFY_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/qualia/graph/staging/verify_quin_parity"),
                StableFuncId::new(0, 1),
                q_hash_str("(u64[6])->u64"),
                EffectSet::GRAPH_READ,
                DeterminismGuarantee::DeterministicExact,
                ReversibilityKind::PureNoState,
                VERIFY_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(u64[6])->u64"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
];

const GRAPH_SUB_NAMESPACES: &[SubNamespaceDescriptor] = &[SubNamespaceDescriptor {
    name: "staging",
    sub_id: 0,
    functions: STAGING_FUNCS,
}];

const GRAPH_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/qualia/graph"),
    short_alias: "qgraph",
    versions: ContractVersionAxes::v0_3_default(
        SemVer64::release(0, 3, 0),
        SemVer64::release(0, 1, 0),
    ),
    supported_targets: TargetProfileMask::ALL_CPU,
    sub_namespaces: GRAPH_SUB_NAMESPACES,
    max_ontology_quins: 16,
};

const GRAPH_ONTOLOGY_QUINS: &[NQuin48] = &[
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/graph"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPrecondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#CanonicalFiveFieldTuple"),
        q_hash_str("https://qualiadb.org/context/public"),
        1,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/graph"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPostcondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#XorParitySealedNQuin48"),
        q_hash_str("https://qualiadb.org/context/public"),
        2,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/graph"),
        q_hash_str("https://qualiadb.org/schema/vibe#preservesInvariant"),
        q_hash_str("https://qualiadb.org/schema/vibe#ReversibleBeforeReceiptCommit"),
        q_hash_str("https://qualiadb.org/context/public"),
        3,
    ),
];

/// Staged semantic graph provider (`VS03-13`).
#[derive(Debug, Clone, Copy, Default)]
pub struct StagedGraphMutationProvider;

impl VibeNamespaceProvider for StagedGraphMutationProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &GRAPH_DESCRIPTOR
    }

    fn static_ontology_nquins(&self) -> &'static [NQuin48] {
        GRAPH_ONTOLOGY_QUINS
    }

    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap> {
        match (stable_id.sub_namespace_id, stable_id.func_id) {
            (0, 0) => {
                // staging::seal_quin
                let mut spocm = [0u64; 5];
                for (i, slot) in spocm.iter_mut().enumerate() {
                    *slot = inputs[0].read_u64_le(i)?;
                }
                let q = oracle_seal_quin(spocm);
                for &w in &q.0 {
                    output.push_u64_le(w)?;
                }
                Ok(12)
            }
            (0, 1) => {
                // staging::verify_quin_parity
                let mut words = [0u64; 6];
                for (i, slot) in words.iter_mut().enumerate() {
                    *slot = inputs[0].read_u64_le(i)?;
                }
                let ok = oracle_verify_quin_parity(words);
                output.push_u64_le(if ok { 1 } else { 0 })?;
                Ok(8)
            }
            _ => Err(VibeTrap::FunctionNotFound),
        }
    }
}

/// Direct-call oracle for `qgraph::staging::seal_quin`.
#[inline]
pub const fn oracle_seal_quin(spocm: [u64; 5]) -> NQuin48 {
    NQuin48::from_parts(spocm[0], spocm[1], spocm[2], spocm[3], spocm[4])
}

/// Direct-call oracle for `qgraph::staging::verify_quin_parity`.
#[inline]
pub const fn oracle_verify_quin_parity(words: [u64; 6]) -> bool {
    NQuin48(words).verify_parity()
}

// ---------------------------------------------------------------------------
// DeonticTemporalProvider (VS03-13)
// ---------------------------------------------------------------------------

const ALLEN_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("interval_a", PrimitiveKind::I64, 2, 2),
    ParamSpec::slice("interval_b", PrimitiveKind::I64, 2, 2),
];

const DEONTIC_PACK_PARAMS: &[ParamSpec] = &[
    ParamSpec::scalar("prop_hash", PrimitiveKind::U64),
    ParamSpec::scalar("opcode", PrimitiveKind::U64),
    ParamSpec::scalar("is_defeater", PrimitiveKind::U64),
];

const ALLEN_ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(16, 8);
const DEONTIC_ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(12, 8);

const ALLEN_FUNCS: &[FuncDescriptor] = &[FuncDescriptor {
    name: "interval_relation",
    canonical_iri: CanonicalIri::new(
        "https://qualiadb.org/ext/qualia/modality/allen/interval_relation",
    ),
    stable_id: StableFuncId::new(0, 0),
    params: ALLEN_PARAMS,
    output_kind: PrimitiveKind::U64,
    required_output_elems: 1,
    effects: EffectSet::PURE_COMPUTE,
    determinism: DeterminismGuarantee::DeterministicExact,
    reversibility: ReversibilityKind::PureNoState,
    envelope: ALLEN_ENV,
    contract: OntologicalContract {
        contract_digest: OntologicalContract::canonical_digest(
            q_hash_str("https://qualiadb.org/ext/qualia/modality/allen/interval_relation"),
            StableFuncId::new(0, 0),
            q_hash_str("(i64[2],i64[2])->u64"),
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::PureNoState,
            ALLEN_ENV,
            (0, 1),
            (1, 1),
            (2, 1),
        ),
        schema_hash: q_hash_str("(i64[2],i64[2])->u64"),
        precondition_quin_span: (0, 1),
        postcondition_quin_span: (1, 1),
        invariant_quin_span: (2, 1),
    },
}];

const DEONTIC_FUNCS: &[FuncDescriptor] = &[FuncDescriptor {
    name: "pack_norm_predicate",
    canonical_iri: CanonicalIri::new(
        "https://qualiadb.org/ext/qualia/modality/deontic/pack_norm_predicate",
    ),
    stable_id: StableFuncId::new(1, 0),
    params: DEONTIC_PACK_PARAMS,
    output_kind: PrimitiveKind::U64,
    required_output_elems: 1,
    effects: EffectSet::PURE_COMPUTE,
    determinism: DeterminismGuarantee::DeterministicExact,
    reversibility: ReversibilityKind::PureNoState,
    envelope: DEONTIC_ENV,
    contract: OntologicalContract {
        contract_digest: OntologicalContract::canonical_digest(
            q_hash_str("https://qualiadb.org/ext/qualia/modality/deontic/pack_norm_predicate"),
            StableFuncId::new(1, 0),
            q_hash_str("(u64,u64,u64)->u64"),
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::PureNoState,
            DEONTIC_ENV,
            (0, 1),
            (1, 1),
            (2, 1),
        ),
        schema_hash: q_hash_str("(u64,u64,u64)->u64"),
        precondition_quin_span: (0, 1),
        postcondition_quin_span: (1, 1),
        invariant_quin_span: (2, 1),
    },
}];

const MODALITY_SUB_NAMESPACES: &[SubNamespaceDescriptor] = &[
    SubNamespaceDescriptor {
        name: "allen",
        sub_id: 0,
        functions: ALLEN_FUNCS,
    },
    SubNamespaceDescriptor {
        name: "deontic",
        sub_id: 1,
        functions: DEONTIC_FUNCS,
    },
];

const MODALITY_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/qualia/modality"),
    short_alias: "modality",
    versions: ContractVersionAxes::v0_3_default(
        SemVer64::release(0, 3, 0),
        SemVer64::release(0, 1, 0),
    ),
    supported_targets: TargetProfileMask::ALL_CPU,
    sub_namespaces: MODALITY_SUB_NAMESPACES,
    max_ontology_quins: 16,
};

const MODALITY_ONTOLOGY_QUINS: &[NQuin48] = &[
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/modality"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPrecondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#WellFormedTemporalIntervals"),
        q_hash_str("https://qualiadb.org/context/public"),
        1,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/modality"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPostcondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#CanonicalAllenRelationOrDeonticOpcode"),
        q_hash_str("https://qualiadb.org/context/public"),
        2,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/modality"),
        q_hash_str("https://qualiadb.org/schema/vibe#preservesInvariant"),
        q_hash_str("https://qualiadb.org/schema/vibe#OpcodesAbove0x10ReservedForModalities"),
        q_hash_str("https://qualiadb.org/context/public"),
        3,
    ),
];

/// Deontic & Allen temporal modality provider (`VS03-13`).
#[derive(Debug, Clone, Copy, Default)]
pub struct DeonticTemporalProvider;

impl VibeNamespaceProvider for DeonticTemporalProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &MODALITY_DESCRIPTOR
    }

    fn static_ontology_nquins(&self) -> &'static [NQuin48] {
        MODALITY_ONTOLOGY_QUINS
    }

    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap> {
        match (stable_id.sub_namespace_id, stable_id.func_id) {
            (0, 0) => {
                // allen::interval_relation
                let a0 = inputs[0].read_i64_le(0)?;
                let a1 = inputs[0].read_i64_le(1)?;
                let b0 = inputs[1].read_i64_le(0)?;
                let b1 = inputs[1].read_i64_le(1)?;
                let rel = oracle_allen_interval_relation([a0, a1], [b0, b1])?;
                output.push_u64_le(rel)?;
                Ok(10)
            }
            (1, 0) => {
                // deontic::pack_norm_predicate
                let prop_hash = inputs[0].read_u64_le(0)?;
                let opcode = inputs[1].read_u64_le(0)?;
                let is_defeater = inputs[2].read_u64_le(0)? != 0;
                let packed = oracle_pack_deontic_predicate(prop_hash, opcode, is_defeater)?;
                output.push_u64_le(packed)?;
                Ok(6)
            }
            _ => Err(VibeTrap::FunctionNotFound),
        }
    }
}

/// Direct-call oracle for `modality::allen::interval_relation`.
///
/// Returns canonical Allen relation code `1..=7`:
/// `1 = Before`, `2 = Meets`, `3 = Overlaps`, `4 = Starts`, `5 = During`, `6 = Finishes`, `7 = Equals`, `8 = Other/Inverse`.
#[inline]
pub const fn oracle_allen_interval_relation(
    a: [i64; 2],
    b: [i64; 2],
) -> Result<u64, VibeTrap> {
    if a[0] > a[1] || b[0] > b[1] {
        return Err(VibeTrap::NumericalDomainError);
    }
    let code = if a[1] < b[0] {
        1 // Before
    } else if a[1] == b[0] {
        2 // Meets
    } else if a[0] == b[0] && a[1] == b[1] {
        7 // Equals
    } else if a[0] == b[0] && a[1] < b[1] {
        4 // Starts
    } else if a[0] > b[0] && a[1] < b[1] {
        5 // During
    } else if a[0] > b[0] && a[1] == b[1] {
        6 // Finishes
    } else if a[0] < b[0] && a[1] > b[0] && a[1] < b[1] {
        3 // Overlaps
    } else {
        8 // Inverse / After
    };
    Ok(code)
}

/// Direct-call oracle for `modality::deontic::pack_norm_predicate`.
#[inline]
pub const fn oracle_pack_deontic_predicate(
    prop_hash: u64,
    opcode: u64,
    is_defeater: bool,
) -> Result<u64, VibeTrap> {
    // Deontic opcodes must be in 0x10..=0x12 per AGENTS.md §1/§2
    if opcode < 0x10 || opcode > 0x12 {
        return Err(VibeTrap::NumericalDomainError);
    }
    let mut pred = ((prop_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode & 0xFF);
    if is_defeater {
        pred |= 1u64 << 63;
    }
    Ok(pred)
}
