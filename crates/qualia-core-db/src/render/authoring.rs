//! Phase 5 — the authoring vocabulary (`ns/ui`) + render **planner** *(the qapps upgrade; §6/§7)*.
//!
//! Upgrades the qapps model ([`docs/manuals/qapps_specification.md`]) from 2D-pane CSS-grid layouts
//! to **manifold worlds**: a qapp declares *views over one manifold* (a 3D scene **and** a 2D pane
//! from the SAME source), each carrying **governance** + **budget** annotations the engine
//! **enforces at plan time** — before anything is drawn.
//!
//! This module is the **in-memory vocabulary + planner**. The wire-form (`yaml-ld-q42` →
//! RDF → CBOR-LD → NQuin `@context` expansion) is task #8; **ShEx *describes*** the contract and
//! **SHACL *enforces*** the shape (ADR 0009) — one source. Those are deliberately *not* duplicated
//! here; this is the runtime the parsed document drives.
//!
//! ## The rails (RENDERER_DEFINITION §8) — enforced, not reinvented
//! * **Governance primitives are the §8 substrate *surfaced*.** Rights-bounded refusal reuses the
//!   inherited `logic::deontic` gate (the same one Phases 3–4 use); this module authors **no** new
//!   normative rules — it only *applies* caller-supplied norms.
//! * **Wisdom-out-of-band (attestation gates).** A view marked `requires_attestation` is
//!   **withheld** until an attestation is present — the human ratifies by attesting (a DID-signature
//!   trigger). Signature *verification* is the identity/key-vault layer's job (it fails closed
//!   there); this gate enforces the *presence* of an attestation, the out-of-band hook.
//! * **Affordability at authoring time.** Budget is declared, not hoped-for: on a constrained
//!   device tier a `Scene3D` view **degrades to 2D** ([`ViewDisposition::Collapsed2D`]) rather than
//!   failing — graceful 3D→2D, so the qapp still works on hardware people own.
//! * **Fail closed.** Rights-bounded render in a shared/civic standpoint defaults to **refuse**.

use crate::gpu_context::OperationalMode;
use crate::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID, OP_PERMIT,
};
use crate::{q_hash, NQuin};

/// Max views planned in one [`plan_qapp`] pass (stack-bounded, zero-heap).
pub const MAX_QAPP_VIEWS: usize = 16;
/// Max governance norms evaluated per rights-bounded check (stack-bounded, zero-heap).
pub const MAX_GOV_NORMS: usize = 32;

/// Property-path for a view-render action governed by a deontic norm.
pub const P_VIEW_RENDER: u64 = q_hash("urn:qualia:authoring:render");
/// Predicate stamp for an attestation `(attester) attests (manifold)`.
pub const P_ATTESTS: u64 = q_hash("urn:qualia:authoring:attests");

/// A view onto the manifold within a qapp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    /// The 3D scene (the GPU `Volume3D` projection).
    Scene3D,
    /// A 2D pane (the `Plane2D` projection — the manifold's flat shadow).
    Pane2D,
}

/// Sensitivity class — surfaces the §8 *rights-bounded context* primitive (not new doctrine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensitivity {
    /// Renders anywhere.
    Public,
    /// Sensitive: a container **refuses** to render it in a shared/civic standpoint without consent.
    RightsBounded,
}

/// One declared view in a qapp: a projection of `manifold`, with governance + budget annotations.
#[derive(Debug, Clone, Copy)]
pub struct QappView {
    /// The manifold/tensor source id. The **same** id across views ⇒ "one manifold, many views".
    pub manifold: u64,
    pub kind: ViewKind,
    pub sensitivity: Sensitivity,
    /// If `true`, the view is withheld until an attestation for `manifold` is present.
    pub requires_attestation: bool,
}

impl QappView {
    /// A public, ungated view.
    pub fn public(manifold: u64, kind: ViewKind) -> Self {
        QappView { manifold, kind, sensitivity: Sensitivity::Public, requires_attestation: false }
    }
}

/// The standpoint a qapp is being rendered into.
#[derive(Debug, Clone, Copy)]
pub struct RenderStandpoint {
    pub id: u64,
    /// A shared/civic view (multiple parties) vs the owner's private view. Sensitive content is
    /// refused in a shared/civic view absent consent.
    pub shared_civic: bool,
}

/// What the planner decides for a view (before any drawing happens).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewDisposition {
    /// Render this view as the given kind.
    Render(ViewKind),
    /// A `Scene3D` view degraded to 2D under a constrained device budget (graceful, not a failure).
    Collapsed2D,
    /// Attestation-gated and not yet attested — withheld (wisdom-out-of-band).
    WithheldUnattested,
    /// Sensitive content in a shared/civic standpoint without consent — refused.
    RefusedRightsBounded,
}

// ── budget (device tier → 3D capability) ─────────────────────────────────────────────────────────

/// Whether the device tier supports the full 3D scene — delegates to the single-source budget rule
/// [`OperationalMode::supports_3d`] (shared with the portal facade). `Eco`/`Reserve` **degrade
/// 3D → 2D** — the affordability rail.
#[inline]
pub fn supports_3d(mode: OperationalMode) -> bool {
    mode.supports_3d()
}

// ── attestation (wisdom-out-of-band) ─────────────────────────────────────────────────────────────

/// Build an attestation quin `(attester) attests (manifold)` in a `frame`. Real DID-signature
/// verification belongs to the identity/key-vault layer; this is the presence hook the gate checks.
pub fn attestation_quin(attester: u64, manifold: u64, frame: u64) -> NQuin {
    let mut q = NQuin {
        subject: attester,
        predicate: P_ATTESTS,
        object: manifold,
        context: frame,
        metadata: 0,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
    q
}

/// Whether some attestation in `attestations` ratifies this view's `manifold`.
pub fn has_attestation(view: &QappView, attestations: &[NQuin]) -> bool {
    attestations
        .iter()
        .any(|a| a.predicate == P_ATTESTS && a.object == view.manifold)
}

// ── rights-bounded context (deontic; fail-closed) ───────────────────────────────────────────────

/// Build a civic-render consent norm `(standpoint) OPCODE render(manifold)` in a `frame`.
pub fn view_render_norm(
    standpoint: u64,
    opcode: u8,
    manifold: u64,
    frame: u64,
    expiry_unix32: u32,
) -> NQuin {
    compile_norm_quin(standpoint, opcode, P_VIEW_RENDER, manifold, frame, expiry_unix32, false)
}

/// Whether sensitive content for `manifold` may render into `standpoint`.
///
/// * The owner's **private** view (`shared_civic == false`) ⇒ always permitted.
/// * A **shared/civic** view ⇒ permitted only with an Active `PERMIT` for `(standpoint, manifold)`
///   and no Active `FORBID`. **Fails closed** (no consent ⇒ refuse).
pub fn rights_render_permitted(
    standpoint: &RenderStandpoint,
    manifold: u64,
    gov_norms: &[NQuin],
    now_unix: u32,
) -> bool {
    if !standpoint.shared_civic {
        return true; // the owner's own private view
    }
    if gov_norms.len() > MAX_GOV_NORMS {
        return false; // fail closed
    }
    let mut out = [DeonticVerdict::default(); MAX_GOV_NORMS];
    let n = match evaluate_deontic_contract(gov_norms, now_unix, &mut out) {
        Ok(n) => n,
        Err(_) => return false, // fail closed
    };
    let mut permitted = false;
    for v in &out[..n] {
        if v.status != DeonticStatus::Active || v.norm.subject != standpoint.id || v.norm.object != manifold
        {
            continue;
        }
        match v.opcode {
            OP_FORBID => return false, // an active prohibition always wins
            OP_PERMIT => permitted = true,
            _ => {}
        }
    }
    permitted
}

// ── the planner ──────────────────────────────────────────────────────────────────────────────────

/// Resolve one view's disposition, applying the gates in order: **attestation → rights-bounded →
/// budget**. (Governance refusals take precedence over budget degradation — a refused view is not
/// "degraded", it is withheld/refused.)
pub fn plan_view(
    view: &QappView,
    standpoint: &RenderStandpoint,
    mode: OperationalMode,
    attestations: &[NQuin],
    gov_norms: &[NQuin],
    now_unix: u32,
) -> ViewDisposition {
    // 1) Attestation gate (wisdom-out-of-band): withhold until attested.
    if view.requires_attestation && !has_attestation(view, attestations) {
        return ViewDisposition::WithheldUnattested;
    }
    // 2) Rights-bounded context: refuse sensitive render in a shared/civic standpoint w/o consent.
    if matches!(view.sensitivity, Sensitivity::RightsBounded)
        && !rights_render_permitted(standpoint, view.manifold, gov_norms, now_unix)
    {
        return ViewDisposition::RefusedRightsBounded;
    }
    // 3) Budget: a 3D scene degrades to 2D on a constrained tier (affordability).
    match view.kind {
        ViewKind::Scene3D if !supports_3d(mode) => ViewDisposition::Collapsed2D,
        k => ViewDisposition::Render(k),
    }
}

/// Plan a whole qapp: write each view's disposition into `out`. Zero-heap (caller slices). Returns
/// the number of dispositions written (`min(views.len(), out.len())`).
#[allow(clippy::too_many_arguments)]
pub fn plan_qapp(
    views: &[QappView],
    standpoint: &RenderStandpoint,
    mode: OperationalMode,
    attestations: &[NQuin],
    gov_norms: &[NQuin],
    now_unix: u32,
    out: &mut [ViewDisposition],
) -> usize {
    let n = views.len().min(out.len());
    for i in 0..n {
        out[i] = plan_view(&views[i], standpoint, mode, attestations, gov_norms, now_unix);
    }
    n
}

/// A sample qapp: a `Scene3D` + a `Pane2D` over the **same** manifold — "one manifold, two views".
pub fn sample_world_qapp(manifold: u64) -> [QappView; 2] {
    [
        QappView::public(manifold, ViewKind::Scene3D),
        QappView::public(manifold, ViewKind::Pane2D),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifold() -> u64 {
        q_hash("urn:qualia:manifold:demo")
    }
    fn owner() -> u64 {
        q_hash("urn:qualia:standpoint:owner")
    }
    fn civic() -> RenderStandpoint {
        RenderStandpoint { id: q_hash("urn:qualia:standpoint:civic"), shared_civic: true }
    }
    fn private() -> RenderStandpoint {
        RenderStandpoint { id: owner(), shared_civic: false }
    }
    fn frame() -> u64 {
        q_hash("urn:qualia:frame:app")
    }

    /// ACCEPTANCE (part 1): a 3D scene AND a 2D pane from ONE manifold, on a capable tier.
    #[test]
    fn one_manifold_two_views() {
        let m = manifold();
        let views = sample_world_qapp(m);
        // both views point at the same manifold
        assert_eq!(views[0].manifold, m);
        assert_eq!(views[1].manifold, m);

        let mut out = [ViewDisposition::Collapsed2D; MAX_QAPP_VIEWS];
        let n = plan_qapp(&views, &private(), OperationalMode::Full, &[], &[], 100, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0], ViewDisposition::Render(ViewKind::Scene3D));
        assert_eq!(out[1], ViewDisposition::Render(ViewKind::Pane2D));
    }

    /// ACCEPTANCE (part 2): on a constrained device tier the 3D scene collapses to 2D; the 2D pane
    /// is unaffected.
    #[test]
    fn budget_collapses_3d_to_2d() {
        let views = sample_world_qapp(manifold());
        for mode in [OperationalMode::Eco, OperationalMode::Reserve] {
            let mut out = [ViewDisposition::Collapsed2D; MAX_QAPP_VIEWS];
            plan_qapp(&views, &private(), mode, &[], &[], 100, &mut out);
            assert_eq!(out[0], ViewDisposition::Collapsed2D, "Scene3D should degrade under {mode:?}");
            assert_eq!(out[1], ViewDisposition::Render(ViewKind::Pane2D));
        }
        // Full tier keeps the 3D scene.
        let mut out = [ViewDisposition::Collapsed2D; MAX_QAPP_VIEWS];
        plan_qapp(&views, &private(), OperationalMode::Full, &[], &[], 100, &mut out);
        assert_eq!(out[0], ViewDisposition::Render(ViewKind::Scene3D));
    }

    /// ACCEPTANCE (part 3): a rights-bounded view is refused in a shared/civic standpoint without
    /// consent, permitted with an Active PERMIT, and always shown in the owner's private view.
    #[test]
    fn rights_bounded_context_enforced() {
        let m = manifold();
        let sensitive = QappView {
            manifold: m,
            kind: ViewKind::Pane2D,
            sensitivity: Sensitivity::RightsBounded,
            requires_attestation: false,
        };

        // Civic, no consent → refused (fail closed).
        let civ = civic();
        assert_eq!(
            plan_view(&sensitive, &civ, OperationalMode::Full, &[], &[], 100),
            ViewDisposition::RefusedRightsBounded
        );

        // Civic, with an Active PERMIT for (civic standpoint, manifold) → rendered.
        let permit = view_render_norm(civ.id, OP_PERMIT, m, frame(), 0);
        assert_eq!(
            plan_view(&sensitive, &civ, OperationalMode::Full, &[], &[permit], 100),
            ViewDisposition::Render(ViewKind::Pane2D)
        );

        // Civic, with an Active FORBID → refused even if a permit is also present.
        let forbid = view_render_norm(civ.id, OP_FORBID, m, frame(), 0);
        assert_eq!(
            plan_view(&sensitive, &civ, OperationalMode::Full, &[], &[permit, forbid], 100),
            ViewDisposition::RefusedRightsBounded
        );

        // Owner's private view → always rendered (no consent needed for one's own view).
        assert_eq!(
            plan_view(&sensitive, &private(), OperationalMode::Full, &[], &[], 100),
            ViewDisposition::Render(ViewKind::Pane2D)
        );
    }

    /// ACCEPTANCE (part 4): an attestation-gated view is withheld until attested (wisdom-out-of-band).
    #[test]
    fn attestation_gate_withholds_then_admits() {
        let m = manifold();
        let gated = QappView {
            manifold: m,
            kind: ViewKind::Scene3D,
            sensitivity: Sensitivity::Public,
            requires_attestation: true,
        };

        // No attestation → withheld.
        assert_eq!(
            plan_view(&gated, &private(), OperationalMode::Full, &[], &[], 100),
            ViewDisposition::WithheldUnattested
        );

        // A matching attestation → rendered.
        let attester = q_hash("did:example:auditor");
        let att = attestation_quin(attester, m, frame());
        assert!(has_attestation(&gated, &[att]));
        assert_eq!(
            plan_view(&gated, &private(), OperationalMode::Full, &[att], &[], 100),
            ViewDisposition::Render(ViewKind::Scene3D)
        );

        // An attestation for a DIFFERENT manifold does not satisfy the gate.
        let other = attestation_quin(attester, q_hash("urn:qualia:manifold:other"), frame());
        assert_eq!(
            plan_view(&gated, &private(), OperationalMode::Full, &[other], &[], 100),
            ViewDisposition::WithheldUnattested
        );
    }

    /// Governance precedence: an attestation-gated, rights-bounded 3D view on a low tier resolves
    /// the governance refusals *before* budget — withheld first, then (once attested) refused in
    /// civic, then (once permitted) collapsed under budget.
    #[test]
    fn gates_compose_in_order() {
        let m = manifold();
        let v = QappView {
            manifold: m,
            kind: ViewKind::Scene3D,
            sensitivity: Sensitivity::RightsBounded,
            requires_attestation: true,
        };
        let civ = civic();
        let att = attestation_quin(q_hash("did:example:auditor"), m, frame());
        let permit = view_render_norm(civ.id, OP_PERMIT, m, frame(), 0);

        // Unattested → withheld (attestation wins over everything).
        assert_eq!(
            plan_view(&v, &civ, OperationalMode::Eco, &[], &[permit], 100),
            ViewDisposition::WithheldUnattested
        );
        // Attested but no consent in civic → refused.
        assert_eq!(
            plan_view(&v, &civ, OperationalMode::Eco, &[att], &[], 100),
            ViewDisposition::RefusedRightsBounded
        );
        // Attested + consent, but low tier → collapsed to 2D.
        assert_eq!(
            plan_view(&v, &civ, OperationalMode::Eco, &[att], &[permit], 100),
            ViewDisposition::Collapsed2D
        );
        // Attested + consent + full tier → the 3D scene renders.
        assert_eq!(
            plan_view(&v, &civ, OperationalMode::Full, &[att], &[permit], 100),
            ViewDisposition::Render(ViewKind::Scene3D)
        );
    }
}
