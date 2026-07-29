//! 3D anatomy asset cache + physiological state

use super::*;

impl WebizenHostApi {
    // --- 3D Anatomy asset cache (S5.8 — user-triggered real-mesh acquisition) -------------------
    //
    // The person triggers a download of the CCF/HRA reference-organ GLB set from the live SPARQL
    // endpoint; the host fetches + compiles each to a sealed `.10d` and caches both under
    // `{storage_root}/assets/ccf/{model}/`. Subsequent runs load the cached `.10d` directly — no
    // re-download. The cache is the person's own, generated on demand.

    /// Whether the body assets for a model are cached + complete (manifest exists + every referenced
    /// `.10d` is on disk). `model` is `"male"` / `"female"` (case-insensitive).
    pub fn body_assets_status(
        &self,
        model: &str,
    ) -> Result<super::super::anatomy_assets::BodyAssetsStatus, String> {
        let m = parse_anatomy_model(model)?;
        Ok(super::super::anatomy_assets::status(&self.storage_root, m))
    }

    /// The cached organ keys for a model (empty if not cached).
    pub fn cached_organ_keys(&self, model: &str) -> Result<Vec<String>, String> {
        let m = parse_anatomy_model(model)?;
        Ok(super::super::anatomy_assets::cached_organ_keys(
            &self.storage_root,
            m,
        ))
    }

    /// Load a cached `.10d` for one organ. Returns the raw container bytes (for the browser portal's
    /// `load_10d_colored`).
    pub fn load_cached_organ_10d(&self, model: &str, organ_key: &str) -> Result<Vec<u8>, String> {
        let m = parse_anatomy_model(model)?;
        super::super::anatomy_assets::load_cached_10d(&self.storage_root, m, organ_key)
    }

    /// The per-organ dual-modality percepts for the cached organ set — so the browser portal knows what
    /// colour to paint each organ (σ → RGBA via `paint_organs`). Returns `(painted, unmapped)`.
    pub fn cached_body_organ_percepts(
        &self,
        model: &str,
    ) -> Result<(Vec<super::super::anatomy_view::OrganPercept>, Vec<String>), String> {
        let m = parse_anatomy_model(model)?;
        let organ_keys = super::super::anatomy_assets::cached_organ_keys(&self.storage_root, m);
        if organ_keys.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let report = self.compute_anatomy_view("person", 2)?;
        let key_refs: Vec<&str> = organ_keys.iter().map(|s| s.as_str()).collect();
        Ok(report.paint_organs(&key_refs))
    }

    /// Clear the cache for a model (idempotent). The person can re-acquire later.
    pub fn clear_body_cache(&self, model: &str) -> Result<(), String> {
        let m = parse_anatomy_model(model)?;
        super::super::anatomy_assets::clear_cache(&self.storage_root, m)
    }

    /// The accumulative, traceable **score-card** + investigable hypotheses over the person's own records —
    /// the reading they can act on. Forum-internum / `Sanctuary`-class selfhood content; a set of
    /// **hypotheses** and pathway-starts, never a diagnosis, never a rating. The card is computed at the
    /// person's **declared physiological state** (their point on the reproductive continuum), or
    /// [`PhysiologicalState::Baseline`] if they have not declared one.
    pub fn compute_scorecard(
        &self,
        convergence_threshold: usize,
    ) -> Result<super::super::anatomy_view::WellbeingScorecardReport, String> {
        let conditions = self.list_journal_by_kind("condition", 256)?;
        let medications = self.list_journal_by_kind("medication", 256)?;
        let diet = self.list_journal_by_kind("diet", 256)?;
        // Read the person through **their own** weight model — their authorship of how they're read — falling
        // back to the seed *suggestion* only if they have not authored one.
        let weights = self.get_weight_model();
        // Read the person at **their declared physiological state** — their own statement of where they are
        // on the reproductive continuum — falling back to Baseline if they have not declared one.
        let state = self.get_physiological_state();
        Ok(
            super::super::anatomy_view::build_scorecard_report_from_journal_with_weights(
                &conditions,
                &medications,
                &diet,
                convergence_threshold,
                &weights,
                state,
            ),
        )
    }

    /// The person's own score-card **weight model** — the interpretive lens the card uses — or the seed
    /// *suggestion* if they have not authored one. Theirs to see, edit, or reset; the software offers a
    /// starting point, it does not *define* how they are read.
    pub fn get_weight_model(&self) -> wellfare_core::anatomy::WeightModel {
        super::super::scorecard_prefs::load(&self.storage_root)
            .unwrap_or_else(wellfare_core::anatomy::seed_weight_model)
    }

    /// The seed **suggestion** on its own — so a UI can show "this is the starting point; here's yours" and
    /// let the person compare / adopt / edit.
    pub fn seed_weight_model(&self) -> wellfare_core::anatomy::WeightModel {
        wellfare_core::anatomy::seed_weight_model()
    }

    /// Whether the person has **authored their own** model (vs. still using the seed suggestion).
    pub fn weight_model_is_authored(&self) -> bool {
        super::super::scorecard_prefs::load(&self.storage_root).is_some()
    }

    /// **Set the person's own** weight model — their authorship of how the score-card reads them.
    pub fn set_weight_model(
        &self,
        model: &wellfare_core::anatomy::WeightModel,
    ) -> Result<(), String> {
        super::super::scorecard_prefs::save(&self.storage_root, model)
    }

    /// **Reset** to the seed suggestion (clears the person's authored model — a choice, always reversible by
    /// re-authoring).
    pub fn reset_weight_model(&self) -> Result<(), String> {
        super::super::scorecard_prefs::clear(&self.storage_root)
    }

    // --- Physiological state (P6 — the reproductive-continuum declaration) -----------------------
    //
    // The person's own statement of where they are on the reproductive continuum — their inward knowledge
    // of their own body. Forum-internum / Sanctuary-class. The score-card is computed at this state so it
    // reads them at their current life stage, not a neutral baseline.

    /// The person's **declared** physiological state, or [`PhysiologicalState::Baseline`] if they have not
    /// declared one. Their own statement; the software never assumes.
    pub fn get_physiological_state(&self) -> wellfare_core::anatomy::PhysiologicalState {
        super::super::physiology_prefs::load(&self.storage_root)
            .unwrap_or(wellfare_core::anatomy::PhysiologicalState::Baseline)
    }

    /// Whether the person has **declared** their physiological state (vs. still at the implicit baseline).
    pub fn physiological_state_is_declared(&self) -> bool {
        super::super::physiology_prefs::load(&self.storage_root).is_some()
    }

    /// **Set** the person's declared physiological state — their own statement of where they are on the
    /// reproductive continuum. Forum-internum / Sanctuary-class.
    pub fn set_physiological_state(
        &self,
        state: &wellfare_core::anatomy::PhysiologicalState,
    ) -> Result<(), String> {
        super::super::physiology_prefs::save(&self.storage_root, state)
    }

    /// **Clear** the declared state — revert to the implicit [`PhysiologicalState::Baseline`]. Idempotent.
    pub fn reset_physiological_state(&self) -> Result<(), String> {
        super::super::physiology_prefs::clear(&self.storage_root)
    }
}
