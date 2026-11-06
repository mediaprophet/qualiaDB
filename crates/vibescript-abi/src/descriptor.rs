//! Self-describing ontological contract (`OntologicalContract`), function/provider
//! descriptors (`FuncDescriptor`, `ProviderDescriptor`), and `VibeNamespaceProvider`.

use crate::effects::{
    DeterminismGuarantee, EffectSet, ResourceEnvelope, ReversibilityKind, TargetProfileMask,
};
use crate::identity::{CanonicalIri, StableFuncId};
use crate::lifecycle::VibeTrap;
use crate::memory::{BorrowedSliceView, CallerOutputSlice, NQuin48, PrimitiveKind};
use crate::version::ContractVersionAxes;

/// Self-describing ontological contract attached to an extension function.
///
/// Combines a full 32-byte SHA-256 contract digest with a 64-bit `q_hash` schema
/// fingerprint and slice spans into the provider's `static_ontology_nquins()` table.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OntologicalContract {
    /// Full 32-byte SHA-256 digest of the canonical function contract.
    pub contract_digest: [u8; 32],
    /// 64-bit `q_hash` of the parameter/return schema signature.
    pub schema_hash: u64,
    /// `(start, len)` span in `static_ontology_nquins()` encoding preconditions.
    pub precondition_quin_span: (u32, u32),
    /// `(start, len)` span in `static_ontology_nquins()` encoding postconditions.
    pub postcondition_quin_span: (u32, u32),
    /// `(start, len)` span in `static_ontology_nquins()` encoding conservation/invariants.
    pub invariant_quin_span: (u32, u32),
}

impl OntologicalContract {
    /// Verifies that all `(start, len)` spans lie within `total_ontology_quins`.
    #[inline]
    pub const fn validate_spans(&self, total_ontology_quins: usize) -> bool {
        let total = total_ontology_quins as u64;
        let pre_end =
            (self.precondition_quin_span.0 as u64) + (self.precondition_quin_span.1 as u64);
        let post_end =
            (self.postcondition_quin_span.0 as u64) + (self.postcondition_quin_span.1 as u64);
        let inv_end = (self.invariant_quin_span.0 as u64) + (self.invariant_quin_span.1 as u64);
        pre_end <= total && post_end <= total && inv_end <= total
    }

    /// Computes a deterministic 32-byte canonical contract digest at compile time
    /// from the function's identity, schema hash, effects, envelope, and ontology spans.
    #[inline]
    pub const fn canonical_digest(
        func_iri_hash: u64,
        stable_id: StableFuncId,
        schema_hash: u64,
        effects: EffectSet,
        determinism: DeterminismGuarantee,
        reversibility: ReversibilityKind,
        envelope: ResourceEnvelope,
        pre_span: (u32, u32),
        post_span: (u32, u32),
        inv_span: (u32, u32),
    ) -> [u8; 32] {
        let w0 = func_iri_hash
            ^ ((stable_id.to_u32() as u64) << 32)
            ^ 0x5653_3033_434E_5452; // "VS03CNTR" domain separator
        let w1 = schema_hash
            ^ ((effects.0 as u64) << 32)
            ^ ((determinism as u64) << 16)
            ^ (reversibility as u64);
        let w2 = envelope.max_ticks
            ^ ((envelope.max_scratch_bytes as u64) << 32)
            ^ (envelope.max_vram_bytes as u64)
            ^ ((envelope.max_output_bytes as u64) << 16);
        let w3 = ((pre_span.0 as u64) << 48)
            ^ ((pre_span.1 as u64) << 32)
            ^ ((post_span.0 as u64) << 24)
            ^ ((post_span.1 as u64) << 16)
            ^ ((inv_span.0 as u64) << 8)
            ^ (inv_span.1 as u64);

        let b0 = w0.to_le_bytes();
        let b1 = w1.to_le_bytes();
        let b2 = w2.to_le_bytes();
        let b3 = w3.to_le_bytes();
        [
            b0[0], b0[1], b0[2], b0[3], b0[4], b0[5], b0[6], b0[7], b1[0], b1[1], b1[2], b1[3],
            b1[4], b1[5], b1[6], b1[7], b2[0], b2[1], b2[2], b2[3], b2[4], b2[5], b2[6], b2[7],
            b3[0], b3[1], b3[2], b3[3], b3[4], b3[5], b3[6], b3[7],
        ]
    }
}

/// Parameter specification for an extension function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamSpec {
    /// Parameter name (UTF-8 static identifier).
    pub name: &'static str,
    /// Element primitive kind expected in the corresponding `BorrowedSliceView`.
    pub kind: PrimitiveKind,
    /// Minimum element count required (`1` for scalar, `N` for fixed vector/tuple).
    pub min_elems: u32,
    /// Maximum element count permitted (`u32::MAX` for unbounded slice within envelope).
    pub max_elems: u32,
}

impl ParamSpec {
    /// Scalar parameter (`min_elems = 1`, `max_elems = 1`).
    #[inline]
    pub const fn scalar(name: &'static str, kind: PrimitiveKind) -> Self {
        Self {
            name,
            kind,
            min_elems: 1,
            max_elems: 1,
        }
    }

    /// Bounded slice parameter (`min_elems..=max_elems`).
    #[inline]
    pub const fn slice(
        name: &'static str,
        kind: PrimitiveKind,
        min_elems: u32,
        max_elems: u32,
    ) -> Self {
        Self {
            name,
            kind,
            min_elems,
            max_elems,
        }
    }
}

/// Static descriptor for a single extension function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuncDescriptor {
    /// Short function name within its sub-namespace (e.g. `"verlet_step"`).
    pub name: &'static str,
    /// Globally unique canonical IRI + `q_hash`.
    pub canonical_iri: CanonicalIri,
    /// Provider-scoped stable `(sub_namespace_id, func_id)`.
    pub stable_id: StableFuncId,
    /// Static parameter specifications.
    pub params: &'static [ParamSpec],
    /// Output element primitive kind written to `CallerOutputSlice`.
    pub output_kind: PrimitiveKind,
    /// Exact or worst-case number of output elements required per call.
    pub required_output_elems: u32,
    /// Declared effect bitmask.
    pub effects: EffectSet,
    /// Declared determinism guarantee.
    pub determinism: DeterminismGuarantee,
    /// Declared reversibility / undo classification.
    pub reversibility: ReversibilityKind,
    /// Worst-case resource envelope enforced at admission and runtime.
    pub envelope: ResourceEnvelope,
    /// Self-describing ontological contract and `NQuin48` span offsets.
    pub contract: OntologicalContract,
}

/// Static descriptor for a sub-namespace within a provider.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubNamespaceDescriptor {
    /// Sub-namespace name (e.g. `"kinematics"`).
    pub name: &'static str,
    /// Immutable sub-namespace ID (`matches StableFuncId::sub_namespace_id`).
    pub sub_id: u16,
    /// Functions exported by this sub-namespace.
    pub functions: &'static [FuncDescriptor],
}

/// Top-level descriptor for a VibeScript extension provider.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    /// Canonical namespace IRI + `q_hash` (e.g. `"https://qualiadb.org/ext/etalon/mechanics"`).
    pub canonical_iri: CanonicalIri,
    /// Default short import alias (e.g. `"mech"`).
    pub short_alias: &'static str,
    /// Five independent version axes (`language`, `abi`, `host`, `provider`, `ontology`).
    pub versions: ContractVersionAxes,
    /// Supported execution target profiles.
    pub supported_targets: TargetProfileMask,
    /// Exported sub-namespaces.
    pub sub_namespaces: &'static [SubNamespaceDescriptor],
    /// Maximum allowed ontology `NQuin48` records for this provider.
    pub max_ontology_quins: u32,
}

/// Trait implemented by zero-heap VibeScript extension providers (`crates/vibe`, `etalon`, `qualia-core-db`).
pub trait VibeNamespaceProvider: Sync {
    /// Returns the provider's static metadata and function catalog.
    fn descriptor(&self) -> &'static ProviderDescriptor;

    /// Returns the provider's static, compile-time sealed `NQuin48` ontology triples.
    fn static_ontology_nquins(&self) -> &'static [NQuin48];

    /// Executes the function identified by `stable_id` using caller-supplied input/output slices.
    ///
    /// Returns the number of logical ticks consumed on success, or a deterministic `VibeTrap`.
    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap>;
}
