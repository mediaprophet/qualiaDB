//! Self-describing ontological contract (`OntologicalContract`), function/provider
//! descriptors (`FuncDescriptor`, `ProviderDescriptor`), and `VibeNamespaceProvider`.

use crate::effects::{
    AsyncExecutionClass, DeterminismGuarantee, EffectSet, ResourceEnvelope, ReversibilityKind,
    TargetProfileMask,
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

    /// Computes a deterministic 32-byte SHA-256 canonical contract digest at compile time
    /// from all fields of the function's identity, schema hash, effects, envelope, and ontology spans.
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
        let mut msg = [0u8; 82];
        // Domain tag "VS03CNTR"
        let domain = *b"VS03CNTR";
        let f_iri = func_iri_hash.to_le_bytes();
        let s_id = stable_id.to_u32().to_le_bytes();
        let sch = schema_hash.to_le_bytes();
        let eff = effects.0.to_le_bytes();
        let det = determinism as u8;
        let rev = reversibility as u8;
        let max_ticks = envelope.max_ticks.to_le_bytes();
        let max_scratch = envelope.max_scratch_bytes.to_le_bytes();
        let max_vram = envelope.max_vram_bytes.to_le_bytes();
        let max_out = envelope.max_output_bytes.to_le_bytes();
        let max_tasks = envelope.max_spawned_tasks.to_le_bytes();
        let flags = envelope.flags.to_le_bytes();
        let pre0 = pre_span.0.to_le_bytes();
        let pre1 = pre_span.1.to_le_bytes();
        let post0 = post_span.0.to_le_bytes();
        let post1 = post_span.1.to_le_bytes();
        let inv0 = inv_span.0.to_le_bytes();
        let inv1 = inv_span.1.to_le_bytes();

        let mut pos = 0;
        let mut i = 0;
        while i < 8 { msg[pos] = domain[i]; pos += 1; i += 1; }
        i = 0; while i < 8 { msg[pos] = f_iri[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = s_id[i]; pos += 1; i += 1; }
        i = 0; while i < 8 { msg[pos] = sch[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = eff[i]; pos += 1; i += 1; }
        msg[pos] = det; pos += 1;
        msg[pos] = rev; pos += 1;
        i = 0; while i < 8 { msg[pos] = max_ticks[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = max_scratch[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = max_vram[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = max_out[i]; pos += 1; i += 1; }
        i = 0; while i < 2 { msg[pos] = max_tasks[i]; pos += 1; i += 1; }
        i = 0; while i < 2 { msg[pos] = flags[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = pre0[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = pre1[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = post0[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = post1[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = inv0[i]; pos += 1; i += 1; }
        i = 0; while i < 4 { msg[pos] = inv1[i]; pos += 1; i += 1; }
        debug_assert!(pos == 82);

        sha256_const(&msg)
    }
}

const SHA256_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const fn sha256_compress(mut state: [u32; 8], block: &[u8; 64]) -> [u32; 8] {
    let mut w = [0u32; 64];
    let mut i = 0;
    while i < 16 {
        let j = i * 4;
        w[i] = u32::from_be_bytes([block[j], block[j + 1], block[j + 2], block[j + 3]]);
        i += 1;
    }
    while i < 64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        i += 1;
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];

    i = 0;
    while i < 64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
        i += 1;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
    state
}

/// Pure `#![no_std]` compile-time SHA-256 over arbitrary slices.
pub(crate) const fn sha256_const(data: &[u8]) -> [u8; 32] {
    let mut state = SHA256_INIT;
    let mut offset = 0;
    while offset + 64 <= data.len() {
        let mut block = [0u8; 64];
        let mut i = 0;
        while i < 64 {
            block[i] = data[offset + i];
            i += 1;
        }
        state = sha256_compress(state, &block);
        offset += 64;
    }

    let rem = data.len() - offset;
    let mut tail = [0u8; 64];
    let mut i = 0;
    while i < rem {
        tail[i] = data[offset + i];
        i += 1;
    }
    tail[rem] = 0x80;
    if rem >= 56 {
        state = sha256_compress(state, &tail);
        tail = [0u8; 64];
    }
    let bit_len = (data.len() as u64) * 8;
    let len_bytes = bit_len.to_be_bytes();
    i = 0;
    while i < 8 {
        tail[56 + i] = len_bytes[i];
        i += 1;
    }
    state = sha256_compress(state, &tail);

    let mut out = [0u8; 32];
    let mut j = 0;
    while j < 8 {
        let be = state[j].to_be_bytes();
        out[j * 4] = be[0];
        out[j * 4 + 1] = be[1];
        out[j * 4 + 2] = be[2];
        out[j * 4 + 3] = be[3];
        j += 1;
    }
    out
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

/// QUDT unit IRI and optional numeric domain bounds for a parameter.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamQudtBoundsSpec {
    /// Parameter name matching `ParamSpec::name`.
    pub name: &'static str,
    /// Canonical QUDT unit IRI.
    pub qudt_unit_iri: &'static str,
    /// IEEE-754 bits of the inclusive lower bound, or negative infinity.
    pub min_value_f64_bits: u64,
    /// IEEE-754 bits of the inclusive upper bound, or positive infinity.
    pub max_value_f64_bits: u64,
    /// Whether an inclusive numeric range is declared (`1`) or absent (`0`).
    pub has_numeric_bounds: u8,
}

impl ParamQudtBoundsSpec {
    /// Declares a unit without restricting the value's numeric range.
    #[inline]
    pub const fn unit_only(name: &'static str, qudt_unit_iri: &'static str) -> Self {
        Self {
            name,
            qudt_unit_iri,
            min_value_f64_bits: f64::NEG_INFINITY.to_bits(),
            max_value_f64_bits: f64::INFINITY.to_bits(),
            has_numeric_bounds: 0,
        }
    }

    /// Declares a unit and inclusive numeric bounds.
    #[inline]
    pub const fn bounded_f64(
        name: &'static str,
        qudt_unit_iri: &'static str,
        min_val: f64,
        max_val: f64,
    ) -> Self {
        Self {
            name,
            qudt_unit_iri,
            min_value_f64_bits: min_val.to_bits(),
            max_value_f64_bits: max_val.to_bits(),
            has_numeric_bounds: 1,
        }
    }

    #[inline]
    pub fn min_f64(self) -> f64 {
        f64::from_bits(self.min_value_f64_bits)
    }

    #[inline]
    pub fn max_f64(self) -> f64 {
        f64::from_bits(self.max_value_f64_bits)
    }
}

#[cfg(test)]
mod param_qudt_bounds_tests {
    use super::ParamQudtBoundsSpec;

    #[test]
    fn unit_and_inclusive_bounds_preserve_ieee_bits() {
        const METER: &str = "http://qudt.org/vocab/unit/M";
        const BOUNDED: ParamQudtBoundsSpec =
            ParamQudtBoundsSpec::bounded_f64("offset", METER, -0.0, 0.25);
        const UNIT_ONLY: ParamQudtBoundsSpec = ParamQudtBoundsSpec::unit_only("offset", METER);
        assert_eq!(BOUNDED.qudt_unit_iri, METER);
        assert_eq!(BOUNDED.has_numeric_bounds, 1);
        assert_eq!(BOUNDED.min_f64().to_bits(), (-0.0f64).to_bits());
        assert_eq!(BOUNDED.max_f64(), 0.25);
        assert_eq!(UNIT_ONLY.has_numeric_bounds, 0);
        assert_eq!(UNIT_ONLY.min_f64(), f64::NEG_INFINITY);
        assert_eq!(UNIT_ONLY.max_f64(), f64::INFINITY);
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

impl FuncDescriptor {
    /// Returns the synchronous vs bounded-async execution classification encoded in `envelope.flags`.
    #[inline]
    pub const fn async_class(&self) -> AsyncExecutionClass {
        self.envelope.async_class()
    }
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
