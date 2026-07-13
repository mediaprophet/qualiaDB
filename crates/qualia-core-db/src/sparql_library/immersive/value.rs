//! QISP typed value model — the typed function descriptor, execution/exactness
//! classification, and the stable QISP error codes.
//!
//! This module is the *typed signature* foundation for the Immersive SPARQL (QISP)
//! profile (plan §4.2 "No untyped `u64 -> u64` public function contract", §4.3
//! "Error semantics", §4.4 "exactness profiles"). Every type here is a fixed-size,
//! `#[repr(C)]`, `Copy` record with **no `String`/`Box`/`Vec` inside** — it belongs
//! on the zero-heap evaluation tier, matching the house idiom in `sparql_ast.rs`.
//!
//! Provisional, Editor's-Draft IRIs (no compatibility promise) — see the namespace
//! constants in the parent [`mod`](super).

/// The closed set of typed value kinds a QISP function may consume or produce.
///
/// Reproduced verbatim from plan §4.2. `AssetRef`/`GeometryRef`/`TensorRef` are
/// *validated, process-local* handles resolved from an absolute RDF IRI — never an
/// opaque numeric pointer in the public term (plan §3.6, §2.2 item 5).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImmersiveValueKind {
    /// `xsd:boolean` — predicate result.
    Boolean = 0,
    /// A dimensionless numeric literal (`xsd:double`/`xsd:decimal`/…).
    Scalar = 1,
    /// A numeric measurement that MUST carry a unit (QUDT-described).
    Quantity = 2,
    /// A validated reference to a content-addressed dense asset.
    AssetRef = 3,
    /// A validated reference to a (possibly query-scoped) geometry.
    GeometryRef = 4,
    /// A validated reference to a Tensor10D buffer/section.
    TensorRef = 5,
    /// An `xsd:dateTime`/OWL-Time instant.
    Instant = 6,
    /// An OWL-Time interval.
    Interval = 7,
}

/// How a function is allowed to execute (plan §4.2 / §6.1).
///
/// `HotZeroHeap` and `ColdBoundedSync` may run inside a synchronous SPARQL
/// expression (FILTER/BIND); `AsyncRequired` must go through the job API and is
/// therefore illegal in an inline expression.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionClass {
    /// Zero allocation, runs in the hot predicate path.
    HotZeroHeap = 0,
    /// Bounded, synchronous; may use a caller-owned/byte-budgeted cold workspace.
    ColdBoundedSync = 1,
    /// Cannot run synchronously; must be submitted as a QISP job.
    AsyncRequired = 2,
}

/// Exactness profile (plan §4.4). Exactness is an explicit contract, never an
/// invisible server preference.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExactnessClass {
    /// `qisp:Exact` — robust/exact predicates and constructions where supported.
    Exact = 0,
    /// `qisp:DeterministicApproximate` — reproducible approximation with declared
    /// absolute/relative error bounds.
    DeterministicApproximate = 1,
    /// `qisp:InteractiveApproximate` — renderer/GPU-oriented result; never accepted
    /// for a rights-affecting policy without a separate exact verification.
    InteractiveApproximate = 2,
}

impl ExactnessClass {
    /// The provisional `qisp:` IRI for this exactness profile.
    ///
    /// The returned literal is `QISP_NS` + the profile local name; a unit test
    /// asserts that invariant so the two stay in lockstep.
    pub const fn iri(&self) -> &'static str {
        match self {
            ExactnessClass::Exact => "https://webizen.org/immersive/0.1#Exact",
            ExactnessClass::DeterministicApproximate => {
                "https://webizen.org/immersive/0.1#DeterministicApproximate"
            }
            ExactnessClass::InteractiveApproximate => {
                "https://webizen.org/immersive/0.1#InteractiveApproximate"
            }
        }
    }
}

/// Typed function descriptor for a QISP extension function (plan §4.2, verbatim
/// field set plus the `ExactnessClass` referenced there).
///
/// Fixed-size, `#[repr(C)]`, `Copy` — connects a function IRI (as a compile-time
/// `q_hash`) to its typed signature, execution class, determinism, exactness, and
/// I/O byte budgets. It carries no owned buffers; the resource limits reference the
/// canonical geometry capability manifests rather than duplicating them.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImmersiveFunctionDescriptor {
    /// `q_hash` of the function's absolute IRI (e.g. `qispf:intersects`).
    pub iri_hash: u64,
    /// Positional argument kinds (only the first `arg_count` are meaningful).
    pub args: [ImmersiveValueKind; 4],
    /// Number of populated entries in `args` (0..=4).
    pub arg_count: u8,
    /// Result value kind.
    pub result: ImmersiveValueKind,
    /// Execution class (hot / cold-sync / async).
    pub execution: ExecutionClass,
    /// Whether repeated evaluation with the same key yields the same term
    /// (referential transparency within a query snapshot — plan §4.4).
    pub deterministic: bool,
    /// Exactness profile the descriptor is registered under.
    pub exactness: ExactnessClass,
    /// Maximum accepted input size in bytes (admission control).
    pub max_input_bytes: u32,
    /// Maximum producible output size in bytes (admission control).
    pub max_output_bytes: u32,
}

impl ImmersiveFunctionDescriptor {
    /// Const-friendly constructor so registries can be built in `const`/`static`
    /// tables. `arg_count` is clamped to the 4-slot capacity.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        iri_hash: u64,
        args: [ImmersiveValueKind; 4],
        arg_count: u8,
        result: ImmersiveValueKind,
        execution: ExecutionClass,
        deterministic: bool,
        exactness: ExactnessClass,
        max_input_bytes: u32,
        max_output_bytes: u32,
    ) -> Self {
        let arg_count = if arg_count > 4 { 4 } else { arg_count };
        Self {
            iri_hash,
            args,
            arg_count,
            result,
            execution,
            deterministic,
            exactness,
            max_input_bytes,
            max_output_bytes,
        }
    }

    /// The meaningful (populated) argument kinds.
    pub fn arg_kinds(&self) -> &[ImmersiveValueKind] {
        &self.args[..(self.arg_count as usize)]
    }

    /// Whether this function must be submitted as a job (cannot run inline).
    pub const fn is_async(&self) -> bool {
        matches!(self.execution, ExecutionClass::AsyncRequired)
    }

    /// Whether this function is legal inside a SPARQL `FILTER`.
    ///
    /// A `HotZeroHeap` or `ColdBoundedSync` **deterministic** function is legal;
    /// an `AsyncRequired` or non-deterministic function is not (plan §4.2, §4.4).
    pub const fn legal_in_filter(&self) -> bool {
        self.deterministic && !self.is_async()
    }

    /// Whether this function is legal inside a SPARQL `BIND`. Same rule as
    /// `FILTER`: snapshot-pure, synchronous, deterministic (plan §4.4).
    pub const fn legal_in_bind(&self) -> bool {
        self.legal_in_filter()
    }
}

/// Stable QISP error codes (plan §4.3). SPARQL expression errors stay expression
/// errors — they MUST NOT silently become `false`. Each variant has a stable
/// kebab-case `code()` for machine-readable diagnostics and Problem-Details type
/// IRIs.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QispError {
    /// The asset reference does not resolve to a known/valid record.
    UnknownAsset = 0,
    /// The asset reference is syntactically known but its generation is stale.
    StaleAsset = 1,
    /// Unsupported CRS or profile conversion.
    UnsupportedCrsOrProfile = 2,
    /// Invalid or non-manifold geometry.
    InvalidGeometry = 3,
    /// Dimension/profile mismatch (e.g. wrong Tensor10D arity/profile).
    ProfileMismatch = 4,
    /// Output or workspace byte/work budget exceeded.
    BudgetExceeded = 5,
    /// The requested exactness profile is unavailable for this operation.
    ExactnessUnavailable = 6,
    /// Authorization denied.
    AuthorizationDenied = 7,
    /// The computation was cancelled or its lease expired.
    CancelledOrExpired = 8,
    /// A non-deterministic backend was requested where it is disallowed.
    NonDeterministicDisallowed = 9,
}

impl QispError {
    /// Stable, machine-readable kebab-case error code. These strings are part of
    /// the profile's external contract and must not be renamed without a namespace
    /// version bump (plan §2.4 "Terms are never silently repurposed").
    pub const fn code(&self) -> &'static str {
        match self {
            QispError::UnknownAsset => "unknown-asset",
            QispError::StaleAsset => "stale-asset",
            QispError::UnsupportedCrsOrProfile => "unsupported-crs-or-profile",
            QispError::InvalidGeometry => "invalid-geometry",
            QispError::ProfileMismatch => "profile-mismatch",
            QispError::BudgetExceeded => "budget-exceeded",
            QispError::ExactnessUnavailable => "exactness-unavailable",
            QispError::AuthorizationDenied => "authorization-denied",
            QispError::CancelledOrExpired => "cancelled-or-expired",
            QispError::NonDeterministicDisallowed => "non-deterministic-disallowed",
        }
    }

    /// A short human-readable description (never leaks internal paths/addresses,
    /// per plan §7.2 / §10.1 QISP-R04).
    pub const fn message(&self) -> &'static str {
        match self {
            QispError::UnknownAsset => "unknown or invalid dense asset reference",
            QispError::StaleAsset => "stale dense asset reference (generation mismatch)",
            QispError::UnsupportedCrsOrProfile => "unsupported CRS or profile conversion",
            QispError::InvalidGeometry => "invalid or non-manifold geometry",
            QispError::ProfileMismatch => "dimension or profile mismatch",
            QispError::BudgetExceeded => "output or workspace budget exceeded",
            QispError::ExactnessUnavailable => "requested exactness profile is unavailable",
            QispError::AuthorizationDenied => "authorization denied",
            QispError::CancelledOrExpired => "computation cancelled or expired",
            QispError::NonDeterministicDisallowed => "non-deterministic backend disallowed here",
        }
    }
}

impl core::fmt::Display for QispError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "qisp[{}]: {}", self.code(), self.message())
    }
}

impl std::error::Error for QispError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn descriptor_is_a_small_fixed_repr_c_record() {
        // Fixed, deterministic `#[repr(C)]` layout: u64 + [u8;4] + u8*4 + pad + u32*2.
        // If this changes, the ABI changed — update deliberately, don't paper over it.
        assert_eq!(size_of::<ImmersiveFunctionDescriptor>(), 32);
        assert_eq!(size_of::<ImmersiveValueKind>(), 1);
        assert_eq!(size_of::<ExecutionClass>(), 1);
        assert_eq!(size_of::<ExactnessClass>(), 1);
        // No owned buffers: the descriptor is Copy.
        fn assert_copy<T: Copy>() {}
        assert_copy::<ImmersiveFunctionDescriptor>();
    }

    #[test]
    fn descriptor_round_trips_by_value() {
        let d = ImmersiveFunctionDescriptor::new(
            crate::q_hash("https://webizen.org/immersive/function/0.1#intersects"),
            [
                ImmersiveValueKind::AssetRef,
                ImmersiveValueKind::AssetRef,
                ImmersiveValueKind::Boolean,
                ImmersiveValueKind::Boolean,
            ],
            2,
            ImmersiveValueKind::Boolean,
            ExecutionClass::HotZeroHeap,
            true,
            ExactnessClass::Exact,
            1 << 20,
            0,
        );
        let copy = d; // Copy
        assert_eq!(d, copy);
        assert_eq!(d.arg_count, 2);
        assert_eq!(d.arg_kinds(), &[ImmersiveValueKind::AssetRef, ImmersiveValueKind::AssetRef]);
        assert_eq!(d.result, ImmersiveValueKind::Boolean);
    }

    #[test]
    fn arg_count_is_clamped() {
        let d = ImmersiveFunctionDescriptor::new(
            0,
            [ImmersiveValueKind::Scalar; 4],
            9,
            ImmersiveValueKind::Scalar,
            ExecutionClass::HotZeroHeap,
            true,
            ExactnessClass::Exact,
            0,
            0,
        );
        assert_eq!(d.arg_count, 4);
        assert_eq!(d.arg_kinds().len(), 4);
    }

    #[test]
    fn filter_bind_legality() {
        let hot = ImmersiveFunctionDescriptor::new(
            0,
            [ImmersiveValueKind::AssetRef; 4],
            1,
            ImmersiveValueKind::Boolean,
            ExecutionClass::HotZeroHeap,
            true,
            ExactnessClass::Exact,
            0,
            0,
        );
        assert!(hot.legal_in_filter());
        assert!(hot.legal_in_bind());

        let cold = ImmersiveFunctionDescriptor {
            execution: ExecutionClass::ColdBoundedSync,
            ..hot
        };
        assert!(cold.legal_in_filter());

        // AsyncRequired is never legal inline.
        let async_fn = ImmersiveFunctionDescriptor {
            execution: ExecutionClass::AsyncRequired,
            ..hot
        };
        assert!(!async_fn.legal_in_filter());
        assert!(!async_fn.legal_in_bind());
        assert!(async_fn.is_async());

        // Non-deterministic is never legal inline (optimizer may reorder/repeat).
        let nondet = ImmersiveFunctionDescriptor {
            deterministic: false,
            ..hot
        };
        assert!(!nondet.legal_in_filter());
    }

    #[test]
    fn exactness_iris_are_stable_and_namespaced() {
        use super::super::QISP_NS;
        assert!(ExactnessClass::Exact.iri().starts_with(QISP_NS));
        assert!(ExactnessClass::DeterministicApproximate.iri().starts_with(QISP_NS));
        assert!(ExactnessClass::InteractiveApproximate.iri().starts_with(QISP_NS));
        assert_eq!(
            ExactnessClass::Exact.iri(),
            "https://webizen.org/immersive/0.1#Exact"
        );
        assert_eq!(
            ExactnessClass::DeterministicApproximate.iri(),
            "https://webizen.org/immersive/0.1#DeterministicApproximate"
        );
        assert_eq!(
            ExactnessClass::InteractiveApproximate.iri(),
            "https://webizen.org/immersive/0.1#InteractiveApproximate"
        );
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(QispError::UnknownAsset.code(), "unknown-asset");
        assert_eq!(QispError::StaleAsset.code(), "stale-asset");
        assert_eq!(QispError::UnsupportedCrsOrProfile.code(), "unsupported-crs-or-profile");
        assert_eq!(QispError::InvalidGeometry.code(), "invalid-geometry");
        assert_eq!(QispError::ProfileMismatch.code(), "profile-mismatch");
        assert_eq!(QispError::BudgetExceeded.code(), "budget-exceeded");
        assert_eq!(QispError::ExactnessUnavailable.code(), "exactness-unavailable");
        assert_eq!(QispError::AuthorizationDenied.code(), "authorization-denied");
        assert_eq!(QispError::CancelledOrExpired.code(), "cancelled-or-expired");
        assert_eq!(
            QispError::NonDeterministicDisallowed.code(),
            "non-deterministic-disallowed"
        );
    }

    #[test]
    fn error_display_and_std_error() {
        let e = QispError::StaleAsset;
        let s = format!("{e}");
        assert!(s.contains("stale-asset"));
        // Usable as a std::error::Error trait object.
        let _boxed: Box<dyn std::error::Error> = Box::new(e);
    }
}
