//! Zero-heap Provider SDK helpers for domain package authors (`const fn` descriptor
//! constructors, canonical ontology `NQuin48` builders, and checked finite slice I/O).

use crate::descriptor::{
    FuncDescriptor, OntologicalContract, ParamSpec, ProviderDescriptor, SubNamespaceDescriptor,
};
use crate::effects::{
    AsyncExecutionClass, DeterminismGuarantee, EffectSet, ResourceEnvelope, ReversibilityKind,
    TargetProfileMask,
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

    /// Compile-time constructor that additionally encodes [`AsyncExecutionClass`] into `envelope.flags`.
    #[inline]
    pub const fn const_new_with_async(
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
        async_class: AsyncExecutionClass,
        envelope: ResourceEnvelope,
        pre_span: (u32, u32),
        post_span: (u32, u32),
        inv_span: (u32, u32),
    ) -> Self {
        Self::const_new(
            name,
            canonical_iri_str,
            stable_id,
            schema_sig_str,
            params,
            output_kind,
            required_output_elems,
            effects,
            determinism,
            reversibility,
            envelope.with_async_class(async_class),
            pre_span,
            post_span,
            inv_span,
        )
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

#[cfg(test)]
mod wp_abi_001_tests {
    use super::*;
    use crate::memory::ResourceHandle;
    use crate::version::ABI_CRATE_VERSION;

    #[test]
    fn canonical_digest_has_no_lossy_field_collisions() {
        let base_env = ResourceEnvelope::hot_zero_heap(100, 64);
        let d0 = OntologicalContract::canonical_digest(
            0x11,
            StableFuncId::new(1, 2),
            0x22,
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::PureNoState,
            base_env,
            (0, 1),
            (1, 1),
            (2, 1),
        );
        // Varying flags, max_spawned_tasks, or high span bits must alter the SHA-256 digest.
        let mut env_async = base_env.with_async_class(AsyncExecutionClass::AsyncBoundedTask);
        let d1 = OntologicalContract::canonical_digest(
            0x11,
            StableFuncId::new(1, 2),
            0x22,
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::PureNoState,
            env_async,
            (0, 1),
            (1, 1),
            (2, 1),
        );
        assert_ne!(d0, d1);

        env_async.max_spawned_tasks = 4;
        let d2 = OntologicalContract::canonical_digest(
            0x11,
            StableFuncId::new(1, 2),
            0x22,
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::PureNoState,
            env_async,
            (0, 1),
            (1, 1),
            (2, 1),
        );
        assert_ne!(d1, d2);

        let d3 = OntologicalContract::canonical_digest(
            0x11,
            StableFuncId::new(1, 2),
            0x22,
            EffectSet::PURE_COMPUTE,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::PureNoState,
            base_env,
            (0x0100_0000, 1),
            (1, 1),
            (2, 1),
        );
        assert_ne!(d0, d3);
        assert_eq!(ABI_CRATE_VERSION, SemVer64::release(0, 0, 41));
    }

    #[test]
    fn memory_views_roundtrip_all_added_primitive_kinds_and_check_parity() {
        // u32 / i32
        let mut buf32 = [0u8; 8];
        let mut out_u32 = CallerOutputSlice::new(&mut buf32, PrimitiveKind::U32).unwrap();
        out_u32.push_u32_le(42).unwrap();
        out_u32.push_u32_le(u32::MAX).unwrap();
        let v_u32 =
            BorrowedSliceView::from_raw_bytes(out_u32.written_byte_slice(), PrimitiveKind::U32, 2)
                .unwrap();
        assert_eq!(v_u32.read_u32_le(0).unwrap(), 42);
        assert_eq!(v_u32.read_u32_le(1).unwrap(), u32::MAX);

        let mut buf_i32 = [0u8; 4];
        let mut out_i32 = CallerOutputSlice::new(&mut buf_i32, PrimitiveKind::I32).unwrap();
        out_i32.push_i32_le(-77).unwrap();
        let v_i32 =
            BorrowedSliceView::from_raw_bytes(out_i32.written_byte_slice(), PrimitiveKind::I32, 1)
                .unwrap();
        assert_eq!(v_i32.read_i32_le(0).unwrap(), -77);

        // NQuin48 with parity validation in from_raw_bytes and read_nquin
        let q = NQuin48::from_parts(1, 2, 3, 4, 5);
        let mut qbuf = [0u8; 48];
        let mut qout = CallerOutputSlice::new(&mut qbuf, PrimitiveKind::NQuin48).unwrap();
        qout.push_nquin(&q).unwrap();
        let qview =
            BorrowedSliceView::from_raw_bytes(qout.written_byte_slice(), PrimitiveKind::NQuin48, 1)
                .unwrap();
        assert_eq!(qview.read_nquin(0).unwrap(), q);

        qbuf[0] ^= 0xFF;
        assert_eq!(
            BorrowedSliceView::from_raw_bytes(&qbuf, PrimitiveKind::NQuin48, 1).unwrap_err(),
            VibeTrap::InvalidQuinParity
        );

        // ResourceHandle
        let h = ResourceHandle::new(0x1234, 7, 3, 9, 11);
        let mut hbuf = [0u8; 24];
        let mut hout = CallerOutputSlice::new(&mut hbuf, PrimitiveKind::ResourceHandle).unwrap();
        hout.push_resource_handle(&h).unwrap();
        let hview = BorrowedSliceView::from_raw_bytes(
            hout.written_byte_slice(),
            PrimitiveKind::ResourceHandle,
            1,
        )
        .unwrap();
        assert_eq!(hview.read_resource_handle(0).unwrap(), h);

        // Utf8Bytes
        let mut sbuf = [0u8; 16];
        let mut sout = CallerOutputSlice::new(&mut sbuf, PrimitiveKind::Utf8Bytes).unwrap();
        sout.push_utf8_str("vibescript").unwrap();
        let sview = BorrowedSliceView::from_raw_bytes(
            sout.written_byte_slice(),
            PrimitiveKind::Utf8Bytes,
            10,
        )
        .unwrap();
        assert_eq!(sview.read_utf8_str().unwrap(), "vibescript");
    }
}
