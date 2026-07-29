//! P1 — the **reproductive-continuum state model** (plan of record:
//! `docs/plans/reproductive-continuum-and-maternal-fetal-dyad.md`, §3.1–§3.2, §8).
//!
//! This is the continuum-as-baseline layer the plan's whole argument turns on: reproduction and the
//! maternal–fetal continuum are **first-class baseline biology**, not a `reproductive` subsystem sitting
//! quietly as 1-of-17 (§0, §1.1). So the model gains a **physiological-state axis** — a state machine over
//! the continuum (menarche → cyclical phases → conception → gestation → birth → the fourth trimester →
//! lactation → perimenopause → menopause) — and a **[`StateModulator`]** that re-parameterises *all* the
//! body systems by the current state, the structural sibling of [`EnvironmentModulator`](super::temporal::EnvironmentModulator)
//! but keyed by physiological state rather than the weather.
//!
//! **Baseline, not deviation — and not pathology.** A state is represented as heightened whole-body
//! *engagement* ([`whole_body_profile`]), never as adverse "burden": pregnancy is not a disease, and the
//! modulator does not manufacture load out of a healthy state. What the modulator *does* is change how an
//! *external* load lands — a nephrotoxic medication is a bigger ask of the renal system when filtration is
//! already elevated in pregnancy. The heightened engagement is the state; the adverse thing (if any) is
//! the external factor.
//!
//! **Honesty (§6, §10 — unchanged).** Everything here is **coarse and illustrative**: the per-system
//! scale table is a *seed* of well-established baseline-physiology *directions* (like the existing
//! `seed_knowledge_base`), not precise clinical magnitudes, and it is integer-only (no float health
//! arithmetic). The authoritative milestone content, the dignity-centered framing, and any specifics that
//! go past structural "consider discussing with a clinician / midwife" are **curation-grade — Timothy's /
//! an expert's to supply (§9)**, not agent-inventable. Nothing here is a diagnosis or a fitness-to-do-X
//! claim.

use serde::{Deserialize, Serialize};

use super::factor::Effect;
use super::system_key;
use super::systems::BODY_SYSTEMS;
use super::temporal::EnvironmentModulator;

// ===========================================================================
// The continuum states
// ===========================================================================

/// A phase of the menstrual cycle. The recurring ~monthly timeline (§1.3); ordered as it progresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CyclePhase {
    /// Menstruation — the shedding phase that (by convention) opens the cycle.
    Menstrual,
    /// Follicular — post-menstrual, follicle maturation.
    Follicular,
    /// Ovulatory — around ovulation.
    Ovulatory,
    /// Luteal — post-ovulatory, pre-menstrual.
    Luteal,
}

impl CyclePhase {
    /// The phase the cycle advances to next (wraps `Luteal → Menstrual`).
    pub fn next(self) -> CyclePhase {
        match self {
            CyclePhase::Menstrual => CyclePhase::Follicular,
            CyclePhase::Follicular => CyclePhase::Ovulatory,
            CyclePhase::Ovulatory => CyclePhase::Luteal,
            CyclePhase::Luteal => CyclePhase::Menstrual,
        }
    }

    /// Accessibility-first plain wording (non-diagnostic).
    pub fn plain_label(self) -> &'static str {
        match self {
            CyclePhase::Menstrual => "period (menstrual phase)",
            CyclePhase::Follicular => "follicular phase",
            CyclePhase::Ovulatory => "around ovulation",
            CyclePhase::Luteal => "luteal (premenstrual) phase",
        }
    }
}

/// A trimester of pregnancy — the staged ~40-week timeline (§1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trimester {
    First,
    Second,
    Third,
}

impl Trimester {
    /// The next trimester, or `None` after the third (birth follows, not a fourth trimester of *pregnancy*
    /// — the "fourth trimester" is [`ReproductiveState::Postpartum`], a distinct state of the mother's body).
    pub fn next(self) -> Option<Trimester> {
        match self {
            Trimester::First => Some(Trimester::Second),
            Trimester::Second => Some(Trimester::Third),
            Trimester::Third => None,
        }
    }

    /// The conventional gestational-week span (inclusive start, inclusive end) of this trimester. Coarse
    /// clinical convention, not a per-person claim.
    pub fn week_span(self) -> (u32, u32) {
        match self {
            Trimester::First => (1, 13),
            Trimester::Second => (14, 27),
            Trimester::Third => (28, 40),
        }
    }

    pub fn plain_label(self) -> &'static str {
        match self {
            Trimester::First => "first trimester",
            Trimester::Second => "second trimester",
            Trimester::Third => "third trimester",
        }
    }
}

/// A point on the reproductive continuum — **baseline biology**, modelled as a whole-body physiological
/// state rather than an event on the `reproductive` system (§1.1). This is the "dignity of people born
/// female" spine: the continuum is the model, not the steady state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductiveState {
    /// Before menarche (the first period) — the pre-cyclical baseline.
    PreMenarche,
    /// The recurring cycle, at a given [`CyclePhase`].
    Cycling(CyclePhase),
    /// Pregnancy, at a given [`Trimester`].
    Pregnant(Trimester),
    /// The **fourth trimester** — early postpartum recovery of *the mother's* body (§3.5). A first-class
    /// state, deliberately kept in frame rather than dropped at birth.
    Postpartum,
    /// Lactation.
    Lactating,
    /// Perimenopause — the transition (cycles becoming irregular).
    Perimenopause,
    /// After menopause (cycles have ceased).
    PostMenopause,
}

impl ReproductiveState {
    /// The valid successor states along the continuum. A state machine, not a free jump: it encodes the
    /// biological ordering (menarche opens cycling; conception can happen from any cycle phase; gestation
    /// advances by trimester then to postpartum; postpartum leads to lactation and/or the cycle's return;
    /// perimenopause precedes menopause). `PostMenopause` is terminal.
    ///
    /// Pregnancy can end at any trimester (→ `Postpartum`), which the transitions allow; this models the
    /// state graph, it does not assert anything about a person's course.
    pub fn next_states(self) -> Vec<ReproductiveState> {
        use ReproductiveState::*;
        match self {
            PreMenarche => vec![Cycling(CyclePhase::Menstrual)],
            Cycling(phase) => vec![
                Cycling(phase.next()),
                Pregnant(Trimester::First),
                Perimenopause,
            ],
            Pregnant(tri) => match tri.next() {
                Some(next_tri) => vec![Pregnant(next_tri), Postpartum],
                None => vec![Postpartum],
            },
            Postpartum => vec![Lactating, Cycling(CyclePhase::Menstrual)],
            Lactating => vec![Cycling(CyclePhase::Menstrual), Postpartum],
            Perimenopause => vec![PostMenopause],
            PostMenopause => vec![],
        }
    }

    /// Whether `next` is a valid continuum transition from `self`.
    pub fn can_transition_to(self, next: ReproductiveState) -> bool {
        self.next_states().contains(&next)
    }

    /// Accessibility-first plain wording (non-diagnostic, dignity-centered — the everyday name, not a
    /// clinical label). Richer per-state framing is curation-grade (§9) and deferred.
    pub fn plain_label(self) -> String {
        use ReproductiveState::*;
        match self {
            PreMenarche => "before periods begin".to_string(),
            Cycling(phase) => phase.plain_label().to_string(),
            Pregnant(tri) => format!("pregnant — {}", tri.plain_label()),
            Postpartum => "the fourth trimester (early recovery after birth)".to_string(),
            Lactating => "breastfeeding".to_string(),
            Perimenopause => "the change (perimenopause)".to_string(),
            PostMenopause => "after menopause".to_string(),
        }
    }
}

/// The physiological-state axis. Currently the reproductive continuum plus a neutral [`Baseline`]; the
/// enum leaves room for other whole-body physiological states later without inventing them now.
///
/// [`Baseline`]: PhysiologicalState::Baseline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysiologicalState {
    /// No active continuum modulation. **Not** a "normal/default" value-judgement (that framing is exactly
    /// the male-default bias §0 corrects) — just the absence of an active whole-body state modulation
    /// (e.g. the state has not been declared, or does not apply to this body's biology).
    Baseline,
    /// A point on the reproductive continuum.
    Reproductive(ReproductiveState),
}

impl PhysiologicalState {
    pub fn plain_label(self) -> String {
        match self {
            PhysiologicalState::Baseline => "no reproductive-state modulation".to_string(),
            PhysiologicalState::Reproductive(r) => r.plain_label(),
        }
    }
}

// ===========================================================================
// Whole-body modulation — the StateModulator (§3.2)
// ===========================================================================

/// Per-system whole-body modulation induced by a physiological state — the structural sibling of
/// [`EnvironmentModulator`], but keyed by the current state and applying across the whole body.
///
/// `system_scale_pct` gives, per system id, a scale in percent (`100` = unchanged, `>100` = the system is
/// more heavily engaged in this state, `<100` = less). A system absent from the list is `100`. The scale
/// expresses how much **more of an ask an external load is** on an already-more-engaged system in this
/// state — see [`apply_to_burdens`](StateModulator::apply_to_burdens). It never fabricates burden from the
/// healthy state itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateModulator {
    pub state: PhysiologicalState,
    pub system_scale_pct: Vec<(String, u32)>,
}

impl StateModulator {
    /// The scale (percent) this state applies to `system_id` (default `100` if unlisted).
    pub fn scale_for(&self, system_id: &str) -> u32 {
        self.system_scale_pct
            .iter()
            .find(|(id, _)| system_key(id) == system_key(system_id))
            .map(|(_, s)| *s)
            .unwrap_or(100)
    }

    /// Scale one adverse-load magnitude on a system by this state (integer math; clamped to the model
    /// ceiling 1000).
    pub fn scale_adverse(&self, system_id: &str, adverse_milli: u32) -> u32 {
        ((adverse_milli as u64 * self.scale_for(system_id) as u64) / 100).min(1000) as u32
    }

    /// Apply the state modulation to a set of [`SystemBurden`]s: the **adverse** load on each system is
    /// scaled by that system's state engagement, then `net` is recomputed (`adverse − supportive`, floored
    /// at 0). Supportive load is left as entered. This models "the same external load lands harder on a
    /// system this state already engages more" without pathologising the state.
    pub fn apply_to_burdens(
        &self,
        burdens: &[super::accumulate::SystemBurden],
    ) -> Vec<super::accumulate::SystemBurden> {
        burdens
            .iter()
            .map(|b| {
                let adverse = self.scale_adverse(&b.system_id, b.adverse_milli);
                super::accumulate::SystemBurden {
                    system_id: b.system_id.clone(),
                    adverse_milli: adverse,
                    supportive_milli: b.supportive_milli,
                    net_milli: adverse.saturating_sub(b.supportive_milli),
                    adverse_contributors: b.adverse_contributors.clone(),
                    supportive_contributors: b.supportive_contributors.clone(),
                }
            })
            .collect()
    }

    /// Re-express this modulation as a temporal [`EnvironmentModulator`] over `[from_minute, to_minute]`,
    /// for one system — so a physiological state can be dropped onto a [`Timeline`](super::temporal::Timeline)
    /// and scale that system's adverse contributions the same way the weather does. One modulator per
    /// engaged system (the timeline layer matches a single `target_system`).
    pub fn as_environment_modulators(
        &self,
        from_minute: i64,
        to_minute: i64,
    ) -> Vec<EnvironmentModulator> {
        self.system_scale_pct
            .iter()
            .filter(|(_, scale)| *scale != 100)
            .map(|(system, scale)| EnvironmentModulator {
                label: format!("state:{}", self.state.plain_label()),
                from_minute,
                to_minute,
                target_system: Some(system.clone()),
                target_effect: Some(Effect::Adverse),
                scale_pct: *scale,
            })
            .collect()
    }
}

/// The **illustrative seed** per-system engagement for a physiological state (§8: "illustrative-but-honest
/// seed like the existing knowledge seed"). Encodes only well-established baseline-physiology *directions*
/// — coarse, not precise magnitudes; the authoritative per-state/per-system content + framing is
/// curation-grade and Timothy's/an expert's to supply (§9.2/§9.3). Systems not listed stay at `100`.
fn seed_scale_table(state: PhysiologicalState) -> &'static [(&'static str, u32)] {
    use PhysiologicalState::*;
    use ReproductiveState::*;
    match state {
        Baseline | Reproductive(PreMenarche) => &[],
        // Cyclical phases — mild, phase-dependent shifts.
        Reproductive(Cycling(CyclePhase::Menstrual)) => &[("circulatory", 110), ("endocrine", 110)],
        Reproductive(Cycling(CyclePhase::Follicular)) => &[("endocrine", 105)],
        Reproductive(Cycling(CyclePhase::Ovulatory)) => &[("endocrine", 110)],
        Reproductive(Cycling(CyclePhase::Luteal)) => {
            &[("endocrine", 115), ("nervous", 110), ("circulatory", 105)]
        }
        // Pregnancy — a whole-body reconfiguration deepening by trimester (blood volume/cardiac output,
        // renal filtration, respiratory tidal volume, endocrine, musculoskeletal load).
        Reproductive(Pregnant(Trimester::First)) => &[
            ("circulatory", 115),
            ("urinary", 115),
            ("endocrine", 130),
            ("digestive", 110),
        ],
        Reproductive(Pregnant(Trimester::Second)) => &[
            ("circulatory", 130),
            ("urinary", 125),
            ("respiratory", 115),
            ("endocrine", 135),
            ("skeletal", 110),
            ("muscular", 110),
        ],
        Reproductive(Pregnant(Trimester::Third)) => &[
            ("circulatory", 140),
            ("urinary", 130),
            ("respiratory", 125),
            ("endocrine", 135),
            ("skeletal", 130),
            ("muscular", 125),
            ("digestive", 115),
        ],
        // The fourth trimester — recovery: hormone shift, musculoskeletal recovery, sleep/mood load.
        Reproductive(Postpartum) => &[
            ("endocrine", 130),
            ("nervous", 115),
            ("circulatory", 110),
            ("skeletal", 115),
            ("muscular", 115),
        ],
        // Lactation — endocrine (prolactin/oxytocin) + nutritional/skeletal (calcium) demand.
        Reproductive(Lactating) => &[("endocrine", 125), ("digestive", 115), ("skeletal", 110)],
        // Perimenopause — endocrine variability, bone, vasomotor/mood.
        Reproductive(Perimenopause) => &[
            ("endocrine", 135),
            ("nervous", 115),
            ("skeletal", 115),
            ("circulatory", 110),
        ],
        // Post-menopause — bone-density and cardiovascular-risk shifts.
        Reproductive(PostMenopause) => {
            &[("skeletal", 120), ("circulatory", 115), ("endocrine", 110)]
        }
    }
}

/// The illustrative-seed [`StateModulator`] for a physiological state.
pub fn state_modulator(state: PhysiologicalState) -> StateModulator {
    StateModulator {
        state,
        system_scale_pct: seed_scale_table(state)
            .iter()
            .map(|(id, s)| (id.to_string(), *s))
            .collect(),
    }
}

// ===========================================================================
// The whole-body engagement profile — the dignified, non-pathological view
// ===========================================================================

/// How heavily a state engages a body system — a **descriptive**, non-pathological classification (a state
/// is not "burden"). Derived from the seed scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngagementLevel {
    /// Not notably changed by this state (`< 110%`).
    Baseline,
    /// More engaged (`110–124%`).
    Elevated,
    /// Substantially more engaged (`≥ 125%`).
    High,
}

impl EngagementLevel {
    fn from_scale(scale_pct: u32) -> EngagementLevel {
        if scale_pct >= 125 {
            EngagementLevel::High
        } else if scale_pct >= 110 {
            EngagementLevel::Elevated
        } else {
            EngagementLevel::Baseline
        }
    }
}

/// One system's engagement under a physiological state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemEngagement {
    pub system_id: String,
    /// Clinical label (progressive disclosure).
    pub system_label: String,
    /// Accessibility-first plain label.
    pub plain_label: String,
    /// The illustrative-seed engagement scale (percent).
    pub scale_pct: u32,
    pub level: EngagementLevel,
}

/// The whole-body engagement profile for a physiological state: the systems this state engages **above
/// baseline**, in canonical [`BODY_SYSTEMS`] order, each with a coarse [`EngagementLevel`]. This is the
/// dignified representation of "a state re-parameterises all systems" — heightened *engagement*, not
/// adverse burden, and never a claim about a specific person. Systems at baseline are omitted.
pub fn whole_body_profile(state: PhysiologicalState) -> Vec<SystemEngagement> {
    let modulator = state_modulator(state);
    let mut out = Vec::new();
    for sys in BODY_SYSTEMS {
        let scale = modulator.scale_for(sys.id);
        if scale <= 100 {
            continue;
        }
        out.push(SystemEngagement {
            system_id: sys.id.to_string(),
            system_label: sys.label.to_string(),
            plain_label: sys.plain_label.to_string(),
            scale_pct: scale,
            level: EngagementLevel::from_scale(scale),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::accumulate::{SystemBurden, accumulate};
    use crate::anatomy::factor::{EvidenceTier, Factor, FactorKind};

    #[test]
    fn cycle_phase_progresses_and_wraps() {
        assert_eq!(CyclePhase::Menstrual.next(), CyclePhase::Follicular);
        assert_eq!(CyclePhase::Follicular.next(), CyclePhase::Ovulatory);
        assert_eq!(CyclePhase::Ovulatory.next(), CyclePhase::Luteal);
        assert_eq!(
            CyclePhase::Luteal.next(),
            CyclePhase::Menstrual,
            "wraps to a new cycle"
        );
    }

    #[test]
    fn trimesters_advance_then_end_at_birth_not_a_fourth_pregnancy_trimester() {
        assert_eq!(Trimester::First.next(), Some(Trimester::Second));
        assert_eq!(Trimester::Second.next(), Some(Trimester::Third));
        assert_eq!(
            Trimester::Third.next(),
            None,
            "birth follows the third, not a fourth trimester of pregnancy"
        );
        assert_eq!(Trimester::Third.week_span(), (28, 40));
    }

    #[test]
    fn continuum_transitions_encode_the_biological_ordering() {
        use ReproductiveState::*;
        // Menarche opens the cycle at menstruation.
        assert!(PreMenarche.can_transition_to(Cycling(CyclePhase::Menstrual)));
        assert!(
            !PreMenarche.can_transition_to(Pregnant(Trimester::First)),
            "no jump straight to pregnancy"
        );

        // Conception is reachable from any cycle phase; the cycle also advances.
        assert!(Cycling(CyclePhase::Luteal).can_transition_to(Pregnant(Trimester::First)));
        assert!(Cycling(CyclePhase::Luteal).can_transition_to(Cycling(CyclePhase::Menstrual)));
        assert!(Cycling(CyclePhase::Follicular).can_transition_to(Perimenopause));

        // Gestation advances by trimester, then to the fourth trimester (postpartum).
        assert!(Pregnant(Trimester::First).can_transition_to(Pregnant(Trimester::Second)));
        assert!(Pregnant(Trimester::Third).can_transition_to(Postpartum));
        assert!(!Pregnant(Trimester::Third).can_transition_to(Pregnant(Trimester::First)));

        // Postpartum → lactation and/or the cycle returns.
        assert!(Postpartum.can_transition_to(Lactating));
        assert!(Postpartum.can_transition_to(Cycling(CyclePhase::Menstrual)));

        // Perimenopause precedes menopause; post-menopause is terminal.
        assert!(Perimenopause.can_transition_to(PostMenopause));
        assert!(PostMenopause.next_states().is_empty());
    }

    #[test]
    fn baseline_and_pre_menarche_do_not_modulate() {
        assert!(
            state_modulator(PhysiologicalState::Baseline)
                .system_scale_pct
                .is_empty()
        );
        assert!(whole_body_profile(PhysiologicalState::Baseline).is_empty());
        assert_eq!(
            state_modulator(PhysiologicalState::Reproductive(
                ReproductiveState::PreMenarche
            ))
            .scale_for("circulatory"),
            100
        );
    }

    #[test]
    fn pregnancy_engages_the_whole_body_and_deepens_by_trimester() {
        let t1 = state_modulator(PhysiologicalState::Reproductive(
            ReproductiveState::Pregnant(Trimester::First),
        ));
        let t3 = state_modulator(PhysiologicalState::Reproductive(
            ReproductiveState::Pregnant(Trimester::Third),
        ));

        // Circulatory (blood volume / cardiac output) engagement rises across trimesters.
        assert!(t3.scale_for("circulatory") > t1.scale_for("circulatory"));
        // It is a *whole-body* reconfiguration — many systems engaged, not just "reproductive".
        let profile = whole_body_profile(PhysiologicalState::Reproductive(
            ReproductiveState::Pregnant(Trimester::Third),
        ));
        assert!(
            profile.len() >= 5,
            "third-trimester pregnancy engages many systems: {}",
            profile.len()
        );
        assert!(profile.iter().all(|e| e.scale_pct > 100));
        // The third-trimester circulatory engagement is classified High.
        let circ = profile
            .iter()
            .find(|e| e.system_id == "circulatory")
            .unwrap();
        assert_eq!(circ.level, EngagementLevel::High);
        assert!(!circ.plain_label.is_empty(), "accessibility label present");
    }

    #[test]
    fn modulator_scales_an_external_adverse_load_not_the_state_itself() {
        // A nephrotoxic medication: adverse renal (urinary) load. The state adds NO factors of its own.
        let med = Factor::new(
            "med:nephrotoxic",
            FactorKind::Medication,
            "a kidney-taxing medication",
        )
        .targeting(
            "urinary",
            Effect::Adverse,
            EvidenceTier::ClinicalEvidence,
            400,
        );
        let burdens = accumulate(&[med]);
        let renal_base = burdens
            .iter()
            .find(|b| b.system_id == "urinary")
            .unwrap()
            .net_milli;
        assert_eq!(renal_base, 400);

        // In the third trimester (renal filtration elevated) the SAME med is a bigger ask on the kidneys.
        let preg = state_modulator(PhysiologicalState::Reproductive(
            ReproductiveState::Pregnant(Trimester::Third),
        ));
        let modulated = preg.apply_to_burdens(&burdens);
        let renal_preg = modulated
            .iter()
            .find(|b| b.system_id == "urinary")
            .unwrap()
            .net_milli;
        assert!(
            renal_preg > renal_base,
            "the external load lands harder: {renal_preg} > {renal_base}"
        );
        assert_eq!(
            renal_preg,
            400 * 130 / 100,
            "scaled by the seed renal engagement (130%)"
        );

        // A system the state does not engage is untouched.
        let unrelated = vec![SystemBurden {
            system_id: "sensory".into(),
            adverse_milli: 200,
            supportive_milli: 0,
            net_milli: 200,
            adverse_contributors: vec!["x".into()],
            supportive_contributors: vec![],
        }];
        assert_eq!(preg.apply_to_burdens(&unrelated)[0].net_milli, 200);
    }

    #[test]
    fn support_is_not_scaled_and_net_floors_at_zero() {
        let preg = state_modulator(PhysiologicalState::Reproductive(
            ReproductiveState::Pregnant(Trimester::Third),
        ));
        let b = vec![SystemBurden {
            system_id: "urinary".into(),
            adverse_milli: 100,
            supportive_milli: 500,
            net_milli: 0,
            adverse_contributors: vec!["a".into()],
            supportive_contributors: vec!["b".into()],
        }];
        let out = preg.apply_to_burdens(&b);
        assert_eq!(out[0].adverse_milli, 130, "adverse scaled 100→130");
        assert_eq!(out[0].supportive_milli, 500, "support unchanged");
        assert_eq!(
            out[0].net_milli, 0,
            "still floored at 0 (support exceeds scaled adverse)"
        );
    }

    #[test]
    fn state_can_drive_a_timeline_via_environment_modulators() {
        let preg = state_modulator(PhysiologicalState::Reproductive(
            ReproductiveState::Pregnant(Trimester::Second),
        ));
        let mods = preg.as_environment_modulators(0, 40 * 7 * 24 * 60);
        // One modulator per engaged system, each targeting Adverse on that system.
        assert_eq!(mods.len(), preg.system_scale_pct.len());
        let renal = mods
            .iter()
            .find(|m| m.target_system.as_deref() == Some("urinary"))
            .unwrap();
        assert_eq!(renal.target_effect, Some(Effect::Adverse));
        assert_eq!(renal.scale_pct, 125);
    }

    #[test]
    fn serde_round_trips() {
        let state = PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::Third));
        let json = serde_json::to_string(&state_modulator(state)).unwrap();
        let back: StateModulator = serde_json::from_str(&json).unwrap();
        assert_eq!(state_modulator(state), back);
    }
}
