//! Zero-heap Provider SDK helpers for domain package authors (`const fn` descriptor
//! constructors, canonical ontology `NQuin48` builders, and checked finite slice I/O).

use crate::descriptor::{
    FuncDescriptor, OntologicalContract, ParamSpec, ProviderDescriptor, SubNamespaceDescriptor,
};
use crate::effects::{
    DeterminismGuarantee, EffectSet, ResourceEnvelope, ReversibilityKind, TargetProfileMask,
};
use crate::identity::{q_hash_str, CanonicalIri, StableFuncId};
use crate::lifecycle::VibeTrap;
use crate::memory::{BorrowedSliceView, CallerOutputSlice, NQuin48, PrimitiveKind};
use crate::version::{ContractVersionAxes, SemVer64};

/// Canonical predicate IRI for function preconditions in `static_ontology_nquins()`.
pub const VIBE_HAS_PRECONDITION_IRI: &str = "https://qualiadb.org/schema/vibe#hasPrecondition";

/// Canonical predicate IRI for function postconditions in `static_ontology_nquins()`.
pub const VIBE_HAS_POSTCONDITION_IRI: &str = "https://qualiadb.org/schema/vibe#hasPostcondition";

/// Canonical predicate IRI for function invariants in `static_ontology_nquins()`.
pub const VIBE_PRESERVES_INVARIANT_IRI: &str =
    "https://qualiadb.org/schema/vibe#preservesInvariant";

/// Default public ontology context IRI for provider contract triples.
pub const VIBE_PUBLIC_CONTEXT_IRI: &str = "https://qualiadb.org/context/public";

impl FuncDescriptor {
    /// Compile-time constructor that computes `CanonicalIri`, `schema_hash`, and the
    /// canonical 32-byte `OntologicalContract::canonical_digest` automatically.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub const fn const_new(
        name: &'static str,
        canonical_iri_str: &'static str,
        stable_id: StableFuncId,
        schema_sig_str: &str,
        params: &'static [ParamSpec],
        output_kind: PrimitiveKind,
        required_output_elems: u32,
        effects: EffectSet,
        determinism: DeterminismGuarantee,
        reversibility: ReversibilityKind,
        envelope: ResourceEnvelope,
        pre_span: (u32, u32),
        post_span: (u32, u32),
        inv_span: (u32, u32),
    ) -> Self {
        let canonical_iri = CanonicalIri::new(canonical_iri_str);
        let schema_hash = q_hash_str(schema_sig_str);
        let contract_digest = OntologicalContract::canonical_digest(
            canonical_iri.fnv1a_hash,
            stable_id,
            schema_hash,
            effects,
            determinism,
            reversibility,
            envelope,
            pre_span,
            post_span,
            inv_span,
        );
        Self {
            name,
            canonical_iri,
            stable_id,
            params,
            output_kind,
            required_output_elems,
            effects,
            determinism,
            reversibility,
            envelope,
            contract: OntologicalContract {
                contract_digest,
                schema_hash,
                precondition_quin_span: pre_span,
                postcondition_quin_span: post_span,
                invariant_quin_span: inv_span,
            },
        }
    }
}

impl ProviderDescriptor {
    /// Compile-time constructor for a provider descriptor using `ContractVersionAxes::v0_3_default`.
    #[inline]
    pub const fn const_new(
        canonical_iri_str: &'static str,
        short_alias: &'static str,
        provider_version: SemVer64,
        ontology_version: SemVer64,
        supported_targets: TargetProfileMask,
        sub_namespaces: &'static [SubNamespaceDescriptor],
        max_ontology_quins: u32,
    ) -> Self {
        Self {
            canonical_iri: CanonicalIri::new(canonical_iri_str),
            short_alias,
            versions: ContractVersionAxes::v0_3_default(provider_version, ontology_version),
            supported_targets,
            sub_namespaces,
            max_ontology_quins,
        }
    }
}

impl NQuin48 {
    /// Constructs a 5-field parity-sealed precondition `NQuin48` in the public ontology context.
    #[inline]
    pub const fn precondition(subject_iri: &str, object_iri: &str, metadata: u64) -> Self {
        Self::from_parts(
            q_hash_str(subject_iri),
            q_hash_str(VIBE_HAS_PRECONDITION_IRI),
            q_hash_str(object_iri),
            q_hash_str(VIBE_PUBLIC_CONTEXT_IRI),
            metadata,
        )
    }

    /// Constructs a 5-field parity-sealed postcondition `NQuin48` in the public ontology context.
    #[inline]
    pub const fn postcondition(subject_iri: &str, object_iri: &str, metadata: u64) -> Self {
        Self::from_parts(
            q_hash_str(subject_iri),
            q_hash_str(VIBE_HAS_POSTCONDITION_IRI),
            q_hash_str(object_iri),
            q_hash_str(VIBE_PUBLIC_CONTEXT_IRI),
            metadata,
        )
    }

    /// Constructs a 5-field parity-sealed invariant `NQuin48` in the public ontology context.
    #[inline]
    pub const fn invariant(subject_iri: &str, object_iri: &str, metadata: u64) -> Self {
        Self::from_parts(
            q_hash_str(subject_iri),
            q_hash_str(VIBE_PRESERVES_INVARIANT_IRI),
            q_hash_str(object_iri),
            q_hash_str(VIBE_PUBLIC_CONTEXT_IRI),
            metadata,
        )
    }
}

impl<'a> BorrowedSliceView<'a> {
    /// Reads the `index`-th `f64` and traps with `VibeTrap::NumericalDomainError` if non-finite.
    #[inline]
    pub fn read_finite_f64(&self, index: usize) -> Result<f64, VibeTrap> {
        let v = self.read_f64_le(index)?;
        if !v.is_finite() {
            return Err(VibeTrap::NumericalDomainError);
        }
        Ok(v)
    }

    /// Reads the `index`-th `f32` and traps with `VibeTrap::NumericalDomainError` if non-finite.
    #[inline]
    pub fn read_finite_f32(&self, index: usize) -> Result<f32, VibeTrap> {
        let v = self.read_f32_le(index)?;
        if !v.is_finite() {
            return Err(VibeTrap::NumericalDomainError);
        }
        Ok(v)
    }

    /// Reads `N` contiguous finite `f64` values starting at `start_index`.
    #[inline]
    pub fn read_finite_f64_array<const N: usize>(
        &self,
        start_index: usize,
    ) -> Result<[f64; N], VibeTrap> {
        if self.kind() != PrimitiveKind::F64 {
            return Err(VibeTrap::TypeMismatch);
        }
        let mut out = [0.0f64; N];
        let mut i = 0;
        while i < N {
            let idx = start_index
                .checked_add(i)
                .ok_or(VibeTrap::SliceBoundsOverflow)?;
            out[i] = self.read_finite_f64(idx)?;
            i += 1;
        }
        Ok(out)
    }

    /// Reads `N` contiguous finite `f32` values starting at `start_index`.
    #[inline]
    pub fn read_finite_f32_array<const N: usize>(
        &self,
        start_index: usize,
    ) -> Result<[f32; N], VibeTrap> {
        if self.kind() != PrimitiveKind::F32 {
            return Err(VibeTrap::TypeMismatch);
        }
        let mut out = [0.0f32; N];
        let mut i = 0;
        while i < N {
            let idx = start_index
                .checked_add(i)
                .ok_or(VibeTrap::SliceBoundsOverflow)?;
            out[i] = self.read_finite_f32(idx)?;
            i += 1;
        }
        Ok(out)
    }
}

impl<'a> CallerOutputSlice<'a> {
    /// Atomically appends a slice of `f64` values (little-endian) into the output buffer.
    #[inline]
    pub fn push_f64_slice(&mut self, values: &[f64]) -> Result<(), VibeTrap> {
        if self.kind() != PrimitiveKind::F64 {
            return Err(VibeTrap::TypeMismatch);
        }
        let needed = self
            .written_elems()
            .checked_add(values.len())
            .ok_or(VibeTrap::SliceBoundsOverflow)?;
        self.ensure_elem_capacity(needed)?;
        let mut i = 0;
        while i < values.len() {
            self.push_f64_le(values[i])?;
            i += 1;
        }
        Ok(())
    }

    /// Atomically appends a slice of `f32` values (little-endian) into the output buffer.
    #[inline]
    pub fn push_f32_slice(&mut self, values: &[f32]) -> Result<(), VibeTrap> {
        if self.kind() != PrimitiveKind::F32 {
            return Err(VibeTrap::TypeMismatch);
        }
        let needed = self
            .written_elems()
            .checked_add(values.len())
            .ok_or(VibeTrap::SliceBoundsOverflow)?;
        self.ensure_elem_capacity(needed)?;
        let mut i = 0;
        while i < values.len() {
            self.push_f32_le(values[i])?;
            i += 1;
        }
        Ok(())
    }

    /// Alias for [`Self::push_f64_slice`].
    #[inline]
    pub fn write_f64_slice(&mut self, values: &[f64]) -> Result<(), VibeTrap> {
        self.push_f64_slice(values)
    }

    /// Alias for [`Self::push_f32_slice`].
    #[inline]
    pub fn write_f32_slice(&mut self, values: &[f32]) -> Result<(), VibeTrap> {
        self.push_f32_slice(values)
    }
}
