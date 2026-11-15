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

use super::constitution::DominantSide;
use super::densitometry::{MuscleQualityProfile, PhysealFusionStatus, SkeletalDensityProfile};
use super::factor::Effect;
use super::model::Karyotype;
use super::system_key;
use super::systems::BODY_SYSTEMS;
use super::temporal::EnvironmentModulator;
use crate::record::EpistemicStatus;

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

    /// Plain-language label.
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
    /// Plain-language label.
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

    /// Apply the state modulation to a set of `SystemBurden()`s: the **adverse** load on each system is
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

// ===========================================================================
// BIO-WP-05: Endocrine, Ontogenetic & Laterality Modifiers for Densitometry
// ===========================================================================

/// Endocrine & reproductive-continuum phase for musculoskeletal turnover and
/// ligamentous laxity calibration (Spec §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductiveEndocrinePhase {
    PreMenarche,
    Cycling(CyclePhase),
    Pregnant(Trimester),
    Postpartum,
    Lactating,
    Perimenopause,
    PostMenopause,
    /// Eu-gonadal baseline when no cyclic/gestational/menopausal phase applies.
    AdultBaseline,
    /// Age-related androgen decline (andropause).
    Andropause,
    /// Declared exogenous endocrine modulation (HRT / gender-affirming / medical suppression).
    ExogenousHormoneTherapy,
}

impl From<ReproductiveState> for ReproductiveEndocrinePhase {
    fn from(s: ReproductiveState) -> Self {
        match s {
            ReproductiveState::PreMenarche => Self::PreMenarche,
            ReproductiveState::Cycling(p) => Self::Cycling(p),
            ReproductiveState::Pregnant(t) => Self::Pregnant(t),
            ReproductiveState::Postpartum => Self::Postpartum,
            ReproductiveState::Lactating => Self::Lactating,
            ReproductiveState::Perimenopause => Self::Perimenopause,
            ReproductiveState::PostMenopause => Self::PostMenopause,
        }
    }
}

impl From<PhysiologicalState> for ReproductiveEndocrinePhase {
    fn from(s: PhysiologicalState) -> Self {
        match s {
            PhysiologicalState::Baseline => Self::AdultBaseline,
            PhysiologicalState::Reproductive(r) => Self::from(r),
        }
    }
}

/// Systemic endocrine, renal, and vitamin-D bone/ligament turnover state (Spec §5.3 & §5.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndocrineOsteoState {
    pub phase: ReproductiveEndocrinePhase,
    /// Relative estrogenic activity (`1.0` = young-adult baseline, `< 0.3` = hypoestrogenic).
    pub estrogen_status_ratio: f64,
    /// Relative androgenic activity (`1.0` = young-adult baseline).
    pub androgen_status_ratio: f64,
    /// Circulating relaxin concentration in ng/mL (`0.0` non-pregnant; `0.5..=3.0` pregnant).
    pub relaxin_ng_ml: f64,
    /// Serum 25(OH)D in ng/mL (`< 20.0` indicates deficiency / osteomalacic risk).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vitamin_d_25oh_ng_ml: Option<f64>,
    /// Serum parathyroid hormone in pg/mL (`> 65.0` increases cortical porosity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pth_pg_ml: Option<f64>,
    /// Estimated GFR in mL/min/1.73 m^2 (`< 45.0` flags CKD-MBD mineral-bone shift).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub egfr_ml_min_1_73m2: Option<f64>,
    /// Chronic systemic glucocorticoid exposure (suppresses osteoblast formation & muscle quality).
    pub glucocorticoid_exposure: bool,
}

impl Default for EndocrineOsteoState {
    fn default() -> Self {
        Self::for_phase(ReproductiveEndocrinePhase::AdultBaseline)
    }
}

impl EndocrineOsteoState {
    /// Illustrative baseline endocrine parameters for a given [`ReproductiveEndocrinePhase`].
    pub fn for_phase(phase: ReproductiveEndocrinePhase) -> Self {
        let (estrogen, androgen, relaxin) = match phase {
            ReproductiveEndocrinePhase::PreMenarche => (0.25, 0.25, 0.0),
            ReproductiveEndocrinePhase::Cycling(CyclePhase::Menstrual) => (0.65, 1.0, 0.0),
            ReproductiveEndocrinePhase::Cycling(CyclePhase::Follicular) => (0.95, 1.0, 0.0),
            ReproductiveEndocrinePhase::Cycling(CyclePhase::Ovulatory) => (1.25, 1.0, 0.15),
            ReproductiveEndocrinePhase::Cycling(CyclePhase::Luteal) => (1.05, 1.0, 0.25),
            ReproductiveEndocrinePhase::Pregnant(Trimester::First) => (1.60, 1.0, 1.20),
            ReproductiveEndocrinePhase::Pregnant(Trimester::Second) => (2.00, 1.0, 1.50),
            ReproductiveEndocrinePhase::Pregnant(Trimester::Third) => (2.40, 1.0, 2.20),
            ReproductiveEndocrinePhase::Postpartum => (0.45, 0.90, 0.35),
            ReproductiveEndocrinePhase::Lactating => (0.35, 0.90, 0.0),
            ReproductiveEndocrinePhase::Perimenopause => (0.55, 0.90, 0.0),
            ReproductiveEndocrinePhase::PostMenopause => (0.15, 0.80, 0.0),
            ReproductiveEndocrinePhase::AdultBaseline => (1.00, 1.00, 0.0),
            ReproductiveEndocrinePhase::Andropause => (0.80, 0.55, 0.0),
            ReproductiveEndocrinePhase::ExogenousHormoneTherapy => (1.00, 1.00, 0.0),
        };
        Self {
            phase,
            estrogen_status_ratio: estrogen,
            androgen_status_ratio: androgen,
            relaxin_ng_ml: relaxin,
            vitamin_d_25oh_ng_ml: None,
            pth_pg_ml: None,
            egfr_ml_min_1_73m2: None,
            glucocorticoid_exposure: false,
        }
    }

    /// Validate all endocrine measurements are finite and within physical bounds.
    pub fn validate(&self) -> Result<(), String> {
        if !self.estrogen_status_ratio.is_finite() || self.estrogen_status_ratio < 0.0 {
            return Err("estrogen_status_ratio must be finite and non-negative".into());
        }
        if !self.androgen_status_ratio.is_finite() || self.androgen_status_ratio < 0.0 {
            return Err("androgen_status_ratio must be finite and non-negative".into());
        }
        if !self.relaxin_ng_ml.is_finite() || self.relaxin_ng_ml < 0.0 {
            return Err("relaxin_ng_ml must be finite and non-negative".into());
        }
        if let Some(vd) = self.vitamin_d_25oh_ng_ml {
            if !vd.is_finite() || vd < 0.0 {
                return Err("vitamin_d_25oh_ng_ml must be finite and non-negative".into());
            }
        }
        if let Some(pth) = self.pth_pg_ml {
            if !pth.is_finite() || pth < 0.0 {
                return Err("pth_pg_ml must be finite and non-negative".into());
            }
        }
        if let Some(egfr) = self.egfr_ml_min_1_73m2 {
            if !egfr.is_finite() || egfr < 0.0 {
                return Err("egfr_ml_min_1_73m2 must be finite and non-negative".into());
            }
        }
        Ok(())
    }

    /// Smooth relaxin-mediated ligamentous stiffness multiplier in `[0.65, 1.0]`
    /// (matches `clinical_formulas::relaxin_ligament_stiffness_modifier`).
    pub fn ligament_stiffness_modifier(&self) -> Result<f64, String> {
        self.validate()?;
        Ok(1.0 - 0.35 * (self.relaxin_ng_ml / (self.relaxin_ng_ml + 0.5)))
    }

    /// Joint ROM Swing-Twist cone laxity scale (`>= 1.0`; inverse of stiffness modifier).
    pub fn ligament_laxity_scale(&self) -> Result<f64, String> {
        let stiffness = self.ligament_stiffness_modifier()?;
        Ok((1.0 / stiffness).clamp(1.0, 1.6))
    }

    /// Trabecular bone-turnover multiplier reflecting physiological lactation mobilization
    /// (`~0.94`, a 6% transient calcium drawdown), menopausal estrogen loss, and glucocorticoids.
    pub fn trabecular_turnover_multiplier(&self) -> f64 {
        let phase_factor = match self.phase {
            ReproductiveEndocrinePhase::Lactating => 0.94,
            ReproductiveEndocrinePhase::Pregnant(Trimester::Third) => 0.97,
            ReproductiveEndocrinePhase::Postpartum => 0.96,
            ReproductiveEndocrinePhase::Perimenopause => 0.92,
            ReproductiveEndocrinePhase::PostMenopause => 0.80,
            ReproductiveEndocrinePhase::Andropause => 0.90,
            _ => 1.0,
        };
        let hormone_support = (0.75 + 0.25 * (0.6 * self.estrogen_status_ratio + 0.4 * self.androgen_status_ratio))
            .clamp(0.70, 1.10);
        let gc_penalty = if self.glucocorticoid_exposure { 0.85 } else { 1.0 };
        (phase_factor * hormone_support * gc_penalty).clamp(0.45, 1.15)
    }

    /// Additive intracortical porosity shift from hyperparathyroidism, CKD-MBD, or menopause.
    pub fn cortical_porosity_delta(&self) -> f64 {
        let mut delta = 0.0;
        if matches!(
            self.phase,
            ReproductiveEndocrinePhase::Perimenopause | ReproductiveEndocrinePhase::PostMenopause
        ) {
            delta += 0.03;
        }
        if let Some(pth) = self.pth_pg_ml {
            if pth > 65.0 {
                delta += ((pth - 65.0) / 500.0).min(0.08);
            }
        }
        if let Some(egfr) = self.egfr_ml_min_1_73m2 {
            if egfr < 45.0 {
                delta += ((45.0 - egfr) / 300.0).min(0.08);
            }
        }
        if self.glucocorticoid_exposure {
            delta += 0.02;
        }
        delta
    }

    /// Mineralization degree (`mu_min`) multiplier capturing vitamin-D deficiency / CKD-MBD
    /// osteomalacic hypomineralization (`< 1.0` when 25(OH)D < 20 ng/mL or eGFR < 30).
    pub fn mineralization_mu_modifier(&self) -> f64 {
        let mut mu = 1.0;
        if let Some(vd) = self.vitamin_d_25oh_ng_ml {
            if vd < 20.0 {
                mu *= (0.65 + 0.35 * (vd / 20.0)).clamp(0.65, 1.0);
            }
        }
        if let Some(egfr) = self.egfr_ml_min_1_73m2 {
            if egfr < 30.0 {
                mu *= (0.75 + 0.25 * (egfr / 30.0)).clamp(0.75, 1.0);
            }
        }
        mu
    }
}

/// Coarse ontogenetic life stage derived from chronological or skeletal bone age (Spec §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OntogeneticStage {
    FetalNeonatal,
    Pediatric,
    Adolescent,
    YoungAdultPeak,
    MiddleAdult,
    Senescent,
}

/// Multi-axis ontogenetic age profile separating chronological, skeletal bone, and tissue ages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgeOntogenyProfile {
    pub chronological_age_months: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skeletal_bone_age_months: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tissue_biological_age_months: Option<u16>,
    pub physeal_status: PhysealFusionStatus,
    pub life_stage: OntogeneticStage,
}

impl AgeOntogenyProfile {
    /// Construct from chronological age in months (`0..=1560`).
    pub fn from_age_months(chronological_age_months: u16) -> Result<Self, String> {
        if chronological_age_months > 1560 {
            return Err(format!(
                "chronological_age_months {chronological_age_months} exceeds 1560"
            ));
        }
        let life_stage = match chronological_age_months {
            0..=11 => OntogeneticStage::FetalNeonatal,
            12..=143 => OntogeneticStage::Pediatric,
            144..=239 => OntogeneticStage::Adolescent,
            240..=420 => OntogeneticStage::YoungAdultPeak,
            421..=779 => OntogeneticStage::MiddleAdult,
            _ => OntogeneticStage::Senescent,
        };
        let physeal_status = match chronological_age_months {
            0..=167 => PhysealFusionStatus::OpenCartilaginousPhysis,
            168..=240 => PhysealFusionStatus::PartiallyFused,
            _ => PhysealFusionStatus::ClosedMatureDiaphysis,
        };
        Ok(Self {
            chronological_age_months,
            skeletal_bone_age_months: None,
            tissue_biological_age_months: None,
            physeal_status,
            life_stage,
        })
    }

    /// Override skeletal bone age (Greulich-Pyle / Tanner-Whitehouse) and update physeal status.
    pub fn with_skeletal_bone_age_months(mut self, bone_age_months: u16) -> Result<Self, String> {
        if bone_age_months > 1560 {
            return Err(format!(
                "skeletal_bone_age_months {bone_age_months} exceeds 1560"
            ));
        }
        self.skeletal_bone_age_months = Some(bone_age_months);
        self.physeal_status = match bone_age_months {
            0..=167 => PhysealFusionStatus::OpenCartilaginousPhysis,
            168..=240 => PhysealFusionStatus::PartiallyFused,
            _ => PhysealFusionStatus::ClosedMatureDiaphysis,
        };
        Ok(self)
    }

    /// Age-related trabecular BMD multiplier relative to young-adult Peak Bone Mass (25–35y = 1.0).
    /// Matches `clinical_formulas::age_trabecular_bmd_multiplier` while remaining well-defined when
    /// chromosomal basis is `None` (no invented sex assumption).
    pub fn age_trabecular_bmd_multiplier(
        &self,
        karyotype: Option<Karyotype>,
        postmenopausal: bool,
    ) -> f64 {
        let age_years = (self.skeletal_bone_age_months.unwrap_or(self.chronological_age_months) as f64) / 12.0;
        if age_years < 25.0 {
            0.45 + 0.55 * (age_years / 25.0)
        } else if age_years <= 35.0 {
            1.0
        } else {
            let years_past_peak = age_years - 35.0;
            let base_decline = 0.006 * years_past_peak;
            if matches!(karyotype, Some(Karyotype::Xx)) && postmenopausal {
                let estrogen_withdrawal = 0.18 * (1.0 - (-0.10 * years_past_peak).exp());
                (1.0 - base_decline - estrogen_withdrawal).max(0.35)
            } else {
                (1.0 - base_decline).max(0.40)
            }
        }
    }

    /// Pediatric secondary mineralization factor (`< 1.0` in childhood, `1.0` once mature).
    pub fn pediatric_mineralization_mu(&self) -> f64 {
        let age_m = self.skeletal_bone_age_months.unwrap_or(self.chronological_age_months) as f64;
        if age_m >= 216.0 {
            1.0
        } else {
            (0.72 + 0.28 * (age_m / 216.0)).clamp(0.72, 1.0)
        }
    }

    /// Intervertebral disc / cartilage hydration permill (`0..=1000`), declining in senescence.
    pub fn disc_hydration_permill(&self) -> u16 {
        let bio_years = self
            .tissue_biological_age_months
            .unwrap_or(self.chronological_age_months) as f64
            / 12.0;
        if bio_years <= 25.0 {
            880
        } else {
            (880.0 - 3.2 * (bio_years - 25.0)).clamp(580.0, 880.0).round() as u16
        }
    }

    /// Type-II motor-unit neuromuscular drive / RFD preservation factor in `[0.65, 1.0]`.
    pub fn neural_drive_factor(&self) -> f64 {
        let bio_years = self
            .tissue_biological_age_months
            .unwrap_or(self.chronological_age_months) as f64
            / 12.0;
        if bio_years <= 40.0 {
            1.0
        } else {
            (1.0 - 0.0065 * (bio_years - 40.0)).clamp(0.65, 1.0)
        }
    }
}

/// Handedness classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Left,
    Right,
    Ambidextrous,
    Unspecified,
}

/// Regional limb dominance classification (bridges [`DominantSide`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimbDominance {
    Left,
    Right,
    Symmetric,
    Unspecified,
}

impl From<DominantSide> for LimbDominance {
    fn from(d: DominantSide) -> Self {
        match d {
            DominantSide::Left => Self::Left,
            DominantSide::Right => Self::Right,
            DominantSide::Ambidextrous => Self::Symmetric,
        }
    }
}

/// Visceral / thoracic organ situs classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SitusClassification {
    SitusSolitus,
    SitusInversusTotalis,
    SitusAmbiguusHeterotaxy,
}

/// Bilateral dominance and visceral situs profile (Spec §5.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LateralityProfile {
    pub handedness: Handedness,
    pub upper_limb_dominance: LimbDominance,
    pub lower_limb_dominance: LimbDominance,
    pub situs: SitusClassification,
    /// Physiological cortical/PCSA hypertrophy on the dominant side (`5.0..=15.0` percent typical).
    pub dominant_asymmetry_pct: f64,
}

impl Default for LateralityProfile {
    fn default() -> Self {
        Self {
            handedness: Handedness::Unspecified,
            upper_limb_dominance: LimbDominance::Unspecified,
            lower_limb_dominance: LimbDominance::Unspecified,
            situs: SitusClassification::SitusSolitus,
            dominant_asymmetry_pct: 8.0,
        }
    }
}

impl LateralityProfile {
    /// Multiplicative cortical thickness / PCSA scaling factor (`>= 1.0` when `is_dominant_side`).
    pub fn side_hypertrophy_multiplier(&self, is_dominant_side: bool) -> Result<f64, String> {
        if !self.dominant_asymmetry_pct.is_finite()
            || !(0.0..=30.0).contains(&self.dominant_asymmetry_pct)
        {
            return Err("dominant_asymmetry_pct must be finite and in [0.0, 30.0]".into());
        }
        if !is_dominant_side
            || matches!(
                self.upper_limb_dominance,
                LimbDominance::Symmetric | LimbDominance::Unspecified
            )
        {
            Ok(1.0)
        } else {
            Ok(1.0 + self.dominant_asymmetry_pct / 100.0)
        }
    }
}

/// Summary of physiological modifiers applied to bone and muscle profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysiologicalModifierSummary {
    pub trabecular_bmd_multiplier: f64,
    pub mineralization_multiplier: f64,
    pub cortical_porosity_delta: f64,
    pub ligament_laxity_scale: f64,
    pub side_hypertrophy_multiplier: f64,
    pub disc_hydration_permill: u16,
    pub neural_drive_factor: f64,
    pub epistemic_status: EpistemicStatus,
}

/// Apply age, endocrine/reproductive continuum, and bilateral laterality modifiers to
/// baseline [`SkeletalDensityProfile`] and [`MuscleQualityProfile`] records (Spec §5.5).
pub fn apply_physiological_modifiers_to_density(
    bone: &SkeletalDensityProfile,
    muscle: &MuscleQualityProfile,
    endocrine: &EndocrineOsteoState,
    age: &AgeOntogenyProfile,
    laterality: &LateralityProfile,
    karyotype: Option<Karyotype>,
    is_dominant_side: bool,
) -> Result<(SkeletalDensityProfile, MuscleQualityProfile, PhysiologicalModifierSummary), String> {
    bone.validate()?;
    endocrine.validate()?;
    let ligament_laxity_scale = endocrine.ligament_laxity_scale()?;
    let side_hypertrophy_multiplier = laterality.side_hypertrophy_multiplier(is_dominant_side)?;

    let postmenopausal = matches!(
        endocrine.phase,
        ReproductiveEndocrinePhase::PostMenopause
    );
    let age_trab_mult = age.age_trabecular_bmd_multiplier(karyotype, postmenopausal);
    let endo_trab_mult = endocrine.trabecular_turnover_multiplier();
    let trabecular_bmd_multiplier = (age_trab_mult * endo_trab_mult).clamp(0.25, 1.25);

    let mineralization_multiplier =
        (age.pediatric_mineralization_mu() * endocrine.mineralization_mu_modifier()).clamp(0.45, 1.15);
    let cortical_porosity_delta = endocrine.cortical_porosity_delta();
    let disc_hydration_permill = age.disc_hydration_permill();
    let neural_drive_factor = age.neural_drive_factor();

    let mut out_bone = bone.clone();
    out_bone.trabecular_apparent_density_g_cm3 =
        (bone.trabecular_apparent_density_g_cm3 * trabecular_bmd_multiplier).max(0.01);
    out_bone.trabecular_bv_tv = (bone.trabecular_bv_tv * trabecular_bmd_multiplier).clamp(0.01, 1.0);
    out_bone.cortical_thickness_mm =
        (bone.cortical_thickness_mm * side_hypertrophy_multiplier * age_trab_mult.sqrt()).max(0.2);
    out_bone.cortical_porosity_ratio =
        (bone.cortical_porosity_ratio + cortical_porosity_delta).clamp(0.0, 0.95);
    out_bone.mineralization_degree_mu =
        (bone.mineralization_degree_mu * mineralization_multiplier).clamp(0.2, 1.5);
    out_bone.physeal_fusion = age.physeal_status;
    out_bone.epistemic_status = EpistemicStatus::Hypothesis;

    let mut out_muscle = muscle.clone();
    out_muscle.pcsa_cm2 = muscle.pcsa_cm2 * side_hypertrophy_multiplier;
    let gc_imat_shift = if endocrine.glucocorticoid_exposure { 0.06 } else { 0.0 };
    out_muscle.imat_fraction = (muscle.imat_fraction + gc_imat_shift).clamp(0.0, 1.0);
    out_muscle.neural_activation_factor =
        (muscle.neural_activation_factor * neural_drive_factor).clamp(0.0, 1.0);
    out_muscle.recompute_quality_eta();
    out_muscle.epistemic_status = EpistemicStatus::Hypothesis;

    let summary = PhysiologicalModifierSummary {
        trabecular_bmd_multiplier,
        mineralization_multiplier,
        cortical_porosity_delta,
        ligament_laxity_scale,
        side_hypertrophy_multiplier,
        disc_hydration_permill,
        neural_drive_factor,
        epistemic_status: EpistemicStatus::Hypothesis,
    };

    Ok((out_bone, out_muscle, summary))
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

    #[test]
    fn endocrine_and_ontogeny_modifiers_adjust_density_and_laxity_honestly() {
        use super::super::densitometry::SpeciesDensitometryPreset;

        let baseline_bone =
            SkeletalDensityProfile::from_preset(SpeciesDensitometryPreset::HumanYoungAdultFemale);
        let baseline_muscle =
            MuscleQualityProfile::new("quadriceps", 420.0, 10.0, 0.20, 40.0, 50.0, 0.04, 1.0)
                .unwrap();

        // Third-trimester pregnancy increases relaxin ligament laxity scale > 1.0.
        let preg = EndocrineOsteoState::for_phase(ReproductiveEndocrinePhase::Pregnant(
            Trimester::Third,
        ));
        assert!(preg.ligament_laxity_scale().unwrap() > 1.25);

        // Postmenopausal + 72y vs young adult dominant side.
        let postmeno =
            EndocrineOsteoState::for_phase(ReproductiveEndocrinePhase::PostMenopause);
        let elderly_age = AgeOntogenyProfile::from_age_months(72 * 12).unwrap();
        assert_eq!(elderly_age.life_stage, OntogeneticStage::Senescent);
        assert_eq!(
            elderly_age.physeal_status,
            PhysealFusionStatus::ClosedMatureDiaphysis
        );

        let young_age = AgeOntogenyProfile::from_age_months(28 * 12).unwrap();
        let lat = LateralityProfile {
            handedness: Handedness::Right,
            upper_limb_dominance: LimbDominance::Right,
            lower_limb_dominance: LimbDominance::Right,
            situs: SitusClassification::SitusSolitus,
            dominant_asymmetry_pct: 10.0,
        };

        let (bone_young_dom, muscle_young_dom, _) = apply_physiological_modifiers_to_density(
            &baseline_bone,
            &baseline_muscle,
            &EndocrineOsteoState::default(),
            &young_age,
            &lat,
            Some(Karyotype::Xx),
            true,
        )
        .unwrap();
        let (bone_postmeno, muscle_postmeno, summary_pm) =
            apply_physiological_modifiers_to_density(
                &baseline_bone,
                &baseline_muscle,
                &postmeno,
                &elderly_age,
                &lat,
                Some(Karyotype::Xx),
                false,
            )
            .unwrap();

        assert!(bone_young_dom.cortical_thickness_mm > baseline_bone.cortical_thickness_mm);
        assert!(muscle_young_dom.pcsa_cm2 > baseline_muscle.pcsa_cm2);
        assert!(
            bone_postmeno.trabecular_apparent_density_g_cm3
                < bone_young_dom.trabecular_apparent_density_g_cm3
        );
        assert!(muscle_postmeno.effective_quality_eta < muscle_young_dom.effective_quality_eta);
        assert_eq!(summary_pm.epistemic_status, EpistemicStatus::Hypothesis);

        // Pediatric bone age preserves open physis and lower secondary mineralization.
        let child = AgeOntogenyProfile::from_age_months(10 * 12).unwrap();
        assert_eq!(child.physeal_status, PhysealFusionStatus::OpenCartilaginousPhysis);
        assert!(child.pediatric_mineralization_mu() < 1.0);
    }
}
