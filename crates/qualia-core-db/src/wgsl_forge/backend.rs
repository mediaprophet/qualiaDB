//! Automatic native→fallback backend selection (plan §2).
//!
//! Plan §2 requires: *"If a target-specific native backend (e.g., PTX or MSL)
//! fails to initialize on the host, the Forge must automatically fall back to the
//! next available compilation target (e.g., SPIR-V or WGSL) to ensure the compute
//! pipeline remains operational, albeit at a potentially reduced performance
//! tier."*
//!
//! This module provides that policy as a **pure, deterministic** function so it is
//! unit-testable with a mock availability predicate — no GPU or toolchain is
//! required to exercise it. The execution/validation pipeline calls it to pick a
//! runnable backend; *source emission* (`shader generate --target ptx`) is
//! deliberately NOT routed through here (an explicit emission target must still
//! emit that target's source).

use super::emit::TargetBackend;

/// The ordered native→portable fallback chain for `preferred`, starting with
/// `preferred` itself and ending at the universal WGSL fallback.
///
/// Two families converge on the same universal tail:
/// - NVIDIA native (`Ptx` / `CudaC`) → WGSL. There is no portable SPIR-V tier for
///   the CUDA driver path (PTX is consumed by `cudarc`, not by a wgpu/SPIR-V
///   pipeline), so a failed CUDA init drops straight to the wgpu/WGSL path.
/// - GPU-shading-language native (`Msl` / `Hlsl`) → SPIR-V → WGSL. These compile
///   into a wgpu pipeline, so binary SPIR-V (emitted from the same WGSL via naga's
///   `spv-out`) is a meaningful intermediate tier before the WGSL source fallback.
/// - `Spirv` → WGSL.
/// - `Wgsl` is already the universal fallback; its chain is just `[Wgsl]`.
fn fallback_chain(preferred: TargetBackend) -> &'static [TargetBackend] {
    match preferred {
        TargetBackend::Ptx => &[TargetBackend::Ptx, TargetBackend::Wgsl],
        TargetBackend::CudaC => &[TargetBackend::CudaC, TargetBackend::Wgsl],
        TargetBackend::Msl => &[TargetBackend::Msl, TargetBackend::Spirv, TargetBackend::Wgsl],
        TargetBackend::Hlsl => &[TargetBackend::Hlsl, TargetBackend::Spirv, TargetBackend::Wgsl],
        TargetBackend::Spirv => &[TargetBackend::Spirv, TargetBackend::Wgsl],
        TargetBackend::Wgsl => &[TargetBackend::Wgsl],
    }
}

/// Resolve which backend the *execution/validation* pipeline should actually use,
/// given a `preferred` target and a predicate reporting whether a given backend's
/// native toolchain is available on this host.
///
/// Policy (plan §2):
/// - If `preferred` is available, use it unchanged: returns `(preferred, None)`.
/// - Otherwise walk [`fallback_chain`] to the first available tier and return
///   `(chosen, Some(note))`, where `note` explains the downgrade.
/// - `Wgsl` is **always** considered available — it is the universal fallback and
///   needs no native toolchain (naga compiles it in-process), so this function
///   never fails to resolve a backend. The predicate is therefore not consulted
///   for `Wgsl`; whatever it returns for `Wgsl` is ignored.
///
/// The function is pure and deterministic: identical inputs (including predicate
/// behaviour) yield identical output, with no I/O or global state. This is what
/// makes it unit-testable against a mock predicate.
pub fn resolve_execution_backend(
    preferred: TargetBackend,
    native_available: impl Fn(TargetBackend) -> bool,
) -> (TargetBackend, Option<String>) {
    // The preferred backend wins outright when its toolchain is present (WGSL is
    // always present).
    if preferred == TargetBackend::Wgsl || native_available(preferred) {
        return (preferred, None);
    }

    for &candidate in fallback_chain(preferred) {
        if candidate == preferred {
            continue; // already established as unavailable above
        }
        // WGSL is the universal fallback and is always available; any other tier
        // must pass the availability predicate.
        if candidate == TargetBackend::Wgsl || native_available(candidate) {
            let note = format!(
                "native backend {preferred:?} unavailable on this host; \
                 falling back to {candidate:?} (reduced performance tier, plan §2)"
            );
            return (candidate, Some(note));
        }
    }

    // Unreachable in practice: every chain ends in Wgsl, which is always
    // available. Kept as a total, deterministic fallback rather than a panic.
    (
        TargetBackend::Wgsl,
        Some(format!(
            "native backend {preferred:?} unavailable; falling back to Wgsl (plan §2)"
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a mock predicate that reports the listed backends as available. WGSL
    /// availability is irrelevant (the resolver treats it as always available), so
    /// callers need not include it.
    fn available(set: &'static [TargetBackend]) -> impl Fn(TargetBackend) -> bool {
        move |t| set.contains(&t)
    }

    #[test]
    fn preferred_available_yields_no_downgrade() {
        // PTX present → use PTX, no note.
        let (chosen, note) =
            resolve_execution_backend(TargetBackend::Ptx, available(&[TargetBackend::Ptx]));
        assert_eq!(chosen, TargetBackend::Ptx);
        assert!(note.is_none(), "available preferred must not downgrade");

        // MSL present → use MSL, no note.
        let (chosen, note) =
            resolve_execution_backend(TargetBackend::Msl, available(&[TargetBackend::Msl]));
        assert_eq!(chosen, TargetBackend::Msl);
        assert!(note.is_none());
    }

    #[test]
    fn wgsl_is_always_available_even_if_predicate_denies_it() {
        // Predicate denies everything (including WGSL); WGSL preferred still resolves.
        let (chosen, note) = resolve_execution_backend(TargetBackend::Wgsl, available(&[]));
        assert_eq!(chosen, TargetBackend::Wgsl);
        assert!(note.is_none());
    }

    #[test]
    fn ptx_unavailable_falls_to_wgsl_with_note() {
        // No native toolchains present → PTX drops straight to WGSL (no SPIR-V tier
        // on the CUDA driver path).
        let (chosen, note) = resolve_execution_backend(TargetBackend::Ptx, available(&[]));
        assert_eq!(chosen, TargetBackend::Wgsl);
        let note = note.expect("a downgrade must be reported");
        assert!(note.contains("Ptx"), "note must name the unavailable backend: {note}");
        assert!(note.contains("Wgsl"), "note must name the chosen backend: {note}");
    }

    #[test]
    fn cuda_c_unavailable_falls_to_wgsl_with_note() {
        let (chosen, note) = resolve_execution_backend(TargetBackend::CudaC, available(&[]));
        assert_eq!(chosen, TargetBackend::Wgsl);
        assert!(note.unwrap().contains("CudaC"));
    }

    #[test]
    fn msl_unavailable_prefers_spirv_when_available() {
        // MSL absent but SPIR-V available → choose SPIR-V (the intermediate tier),
        // NOT WGSL.
        let (chosen, note) =
            resolve_execution_backend(TargetBackend::Msl, available(&[TargetBackend::Spirv]));
        assert_eq!(chosen, TargetBackend::Spirv);
        let note = note.expect("a downgrade must be reported");
        assert!(note.contains("Msl"));
        assert!(note.contains("Spirv"));
    }

    #[test]
    fn msl_unavailable_falls_to_wgsl_when_spirv_also_unavailable() {
        // MSL absent and SPIR-V absent → fall all the way to WGSL.
        let (chosen, note) = resolve_execution_backend(TargetBackend::Msl, available(&[]));
        assert_eq!(chosen, TargetBackend::Wgsl);
        assert!(note.unwrap().contains("Wgsl"));
    }

    #[test]
    fn hlsl_unavailable_prefers_spirv_then_wgsl() {
        // HLSL absent, SPIR-V present → SPIR-V.
        let (chosen, _) =
            resolve_execution_backend(TargetBackend::Hlsl, available(&[TargetBackend::Spirv]));
        assert_eq!(chosen, TargetBackend::Spirv);

        // HLSL absent, SPIR-V absent → WGSL.
        let (chosen, _) = resolve_execution_backend(TargetBackend::Hlsl, available(&[]));
        assert_eq!(chosen, TargetBackend::Wgsl);
    }

    #[test]
    fn resolution_is_deterministic() {
        // Identical inputs yield identical outputs across repeated calls.
        let first = resolve_execution_backend(TargetBackend::Hlsl, available(&[]));
        let second = resolve_execution_backend(TargetBackend::Hlsl, available(&[]));
        assert_eq!(first, second);
    }
}
