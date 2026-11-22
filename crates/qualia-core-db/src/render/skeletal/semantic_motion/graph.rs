//! Tier B and C motion authored as materialized N3 consequences in the semantic graph.

use crate::{
    container_10d::AffordanceBinaryRecord,
    q42::m64_matter::{M64DecodedRecord, M64Reader},
    q42_volume::{Q42RangeQueryCursor, Q42RangeQueryPattern, Q42RangeQueryPlan, WasmSliceQ42Reader},
    query::resolver::{classify_inline_literal, InlineLiteral},
    simulation::environment::{AeroelasticBranch, BoundaryLayerProfile},
    q_hash, NQuin,
};

use super::{fitted_rotation, FittedRotation, SemanticMotionError as Error, SemanticRotation};

pub(super) const KEY: u64 = q_hash("https://qualiadb.org/ns/kinematics#keyframe");
const CHANNEL: u64 = q_hash("https://qualiadb.org/ns/kinematics#channel");
const TARGET: u64 = q_hash("https://qualiadb.org/ns/kinematics#target");
const SECONDS: u64 = q_hash("https://qualiadb.org/ns/kinematics#timeSeconds");
const VALUE: u64 = q_hash("https://qualiadb.org/ns/kinematics#value");
const ROTATION: u64 = q_hash("https://qualiadb.org/ns/kinematics#RotationRadians");
const BLEND_X: u64 = q_hash("https://qualiadb.org/ns/kinematics#BlendX");
const BLEND_Y: u64 = q_hash("https://qualiadb.org/ns/kinematics#BlendY");
const MUSCLE: u64 = q_hash("https://qualiadb.org/ns/kinematics#MuscleRecruitment");
const MORPH: u64 = q_hash("https://qualiadb.org/ns/kinematics#MorphWeight");
const MATERIAL_CHUNK: u64 = q_hash("https://qualiadb.org/ns/kinematics#materialChunk");
const MATERIAL_RECORD: u64 = q_hash("https://qualiadb.org/ns/kinematics#materialRecord");
const MATERIAL_RESOURCE: u64 = q_hash("https://qualiadb.org/ns/kinematics#materialResource");
const VOLUME: u64 = q_hash("https://qualiadb.org/ns/kinematics#volumeM3");
const GYRATION: u64 = q_hash("https://qualiadb.org/ns/kinematics#radiusOfGyrationM");
const TORSION_J: u64 = q_hash("https://qualiadb.org/ns/kinematics#torsionConstantM4");
const SPAN: u64 = q_hash("https://qualiadb.org/ns/kinematics#spanM");
const DAMPING: u64 = q_hash("https://qualiadb.org/ns/kinematics#dampingRatio");
const TORQUE: u64 = q_hash("https://qualiadb.org/ns/kinematics#maxTorqueNm");
const WIND_HEIGHT: u64 = q_hash("https://qualiadb.org/ns/kinematics#heightMetres");
const WIND_ROUGHNESS: u64 = q_hash("https://qualiadb.org/ns/kinematics#roughnessLengthMetres");
const WIND_DISPLACEMENT: u64 = q_hash("https://qualiadb.org/ns/kinematics#zeroPlaneDisplacementMetres");
const WIND_FRICTION: u64 = q_hash("https://qualiadb.org/ns/kinematics#frictionVelocityMetresPerSecond");
const AIR_DENSITY: u64 = q_hash("https://qualiadb.org/ns/kinematics#airDensityKgPerCubicMetre");
const VOGEL_EXPONENT: u64 = q_hash("https://qualiadb.org/ns/kinematics#vogelExponent");
const FIT_RECEIPT: u64 = q_hash("https://qualiadb.org/ns/kinematics#fitReceipt");
const ENGINEERING_STATUS: u64 = q_hash("https://qualiadb.org/ns/kinematics#engineeringGeometryStatus");
const VERIFIED: &str = "https://qualiadb.org/ns/kinematics#EngineeringVerified";
const AXIS: [u64; 3] = [
    q_hash("https://qualiadb.org/ns/kinematics#axisX"),
    q_hash("https://qualiadb.org/ns/kinematics#axisY"),
    q_hash("https://qualiadb.org/ns/kinematics#axisZ"),
];
const PIVOT: [u64; 3] = [
    q_hash("https://qualiadb.org/ns/kinematics#pivotXMetres"),
    q_hash("https://qualiadb.org/ns/kinematics#pivotYMetres"),
    q_hash("https://qualiadb.org/ns/kinematics#pivotZMetres"),
];
const ROM: [u64; 2] = [
    q_hash("https://qualiadb.org/ns/kinematics#romMinRadians"),
    q_hash("https://qualiadb.org/ns/kinematics#romMaxRadians"),
];
const CONTACT: [u64; 2] = [
    q_hash("https://qualiadb.org/ns/kinematics#contactMinRadians"),
    q_hash("https://qualiadb.org/ns/kinematics#contactMaxRadians"),
];
const MAX_SCRATCH_BYTES: usize = 42 * 1024 * 1024;

/// Caller-owned query scratch and a budget for every scan over the admitted graph.
pub struct MotionGraph<'model, 'bytes, 'scratch> {
    pub model: &'model WasmSliceQ42Reader<'bytes>,
    pub context: u64,
    pub max_model_blocks: u64,
    pub queries_left: usize,
    pub compressed: &'scratch mut [u8],
    pub decoded: &'scratch mut [u8],
    pub lex_page: &'scratch mut [u8],
    pub lex_value: &'scratch mut [u8],
}

impl MotionGraph<'_, '_, '_> {
    pub fn verify_context_iri(&mut self, context_iri: &str) -> Result<(), Error> {
        if self.iri(self.context)? != context_iri { return Err(Error::UnknownIdentity); }
        Ok(())
    }

    pub(super) fn check_budget(&self, extra_bytes: usize) -> Result<(), Error> {
        if !self.model.is_unified_volume() || self.context == 0 {
            return Err(Error::InvalidFact);
        }
        let total = [
            self.compressed.len(), self.decoded.len(), self.lex_page.len(),
            self.lex_value.len(), extra_bytes,
        ].into_iter().try_fold(0usize, usize::checked_add);
        if self.model.block_count() > self.max_model_blocks
            || total.is_none_or(|bytes| bytes > MAX_SCRATCH_BYTES)
        {
            return Err(Error::BudgetExceeded);
        }
        Ok(())
    }

    pub(super) fn iri(&mut self, hash: u64) -> Result<&str, Error> {
        let len = self.model.lookup_lexicon_hash_into(hash, self.lex_page, self.lex_value)
            .map_err(Error::Model)?.ok_or(Error::UnknownIdentity)?;
        let value = self.lex_value.get(..len).ok_or(Error::UnknownIdentity)?;
        let iri = std::str::from_utf8(value).map_err(|_| Error::UnknownIdentity)?;
        if hash == 0 || !crate::bundle::semantic_model::valid_iri(iri) || q_hash(iri) != hash {
            return Err(Error::UnknownIdentity);
        }
        Ok(iri)
    }

    pub(super) fn page(
        &mut self, subject: u64, predicate: u64, cursor: Q42RangeQueryCursor, out: &mut [NQuin],
    ) -> Result<crate::q42_volume::Q42RangeQueryPage, Error> {
        self.query(Q42RangeQueryPattern {
            subject: Some(subject), predicate: Some(predicate), context: Some(self.context),
            ..Default::default()
        }, cursor, out)
    }

    pub(super) fn contains(&mut self, subject: u64, predicate: u64, object: u64, context: u64) -> Result<bool, Error> {
        let mut rows = [NQuin::default(); 1];
        let page = self.query(Q42RangeQueryPattern {
            subject: Some(subject), predicate: Some(predicate), object: Some(object), context: Some(context),
            ..Default::default()
        }, Q42RangeQueryCursor::default(), &mut rows)?;
        Ok(page.returned != 0)
    }

    fn query(
        &mut self, pattern: Q42RangeQueryPattern, cursor: Q42RangeQueryCursor, out: &mut [NQuin],
    ) -> Result<crate::q42_volume::Q42RangeQueryPage, Error> {
        self.queries_left = self.queries_left.checked_sub(1).ok_or(Error::BudgetExceeded)?;
        let plan = Q42RangeQueryPlan::for_pattern(pattern);
        let page = self.model.execute_query_page_into(plan, cursor, self.compressed, self.decoded, out)
            .map_err(Error::Model)?;
        if out[..page.returned].iter().any(|quin| !quin.verify_ecc_parity()) {
            return Err(Error::InvalidFact);
        }
        Ok(page)
    }

    pub(super) fn one(&mut self, subject: u64, predicate: u64) -> Result<u64, Error> {
        let mut rows = [NQuin::default(); 2];
        let page = self.page(subject, predicate, Q42RangeQueryCursor::default(), &mut rows)?;
        match (page.returned, page.next_cursor) {
            (0, None) => Err(Error::MissingFact),
            (1, None) => Ok(rows[0].object),
            _ => Err(Error::ConflictingFact),
        }
    }

    pub(super) fn fit(&mut self, target: u64) -> Result<FittedRotation, Error> {
        let receipt = self.one(target, FIT_RECEIPT)?;
        self.iri(receipt)?;
        let status = self.one(receipt, ENGINEERING_STATUS)?;
        if self.iri(status)? != VERIFIED || self.one(receipt, TARGET)? != target {
            return Err(Error::InvalidFit);
        }
        let fit = FittedRotation {
            semantic_id: target,
            local_axis: self.numeric_lanes(receipt, AXIS)?,
            local_pivot_m: self.numeric_lanes(receipt, PIVOT)?,
            rom_radians: self.numeric_lanes(receipt, ROM)?,
            contact_safe_radians: self.numeric_lanes(receipt, CONTACT)?,
        };
        fitted_rotation(&fit, 0.0)?;
        Ok(fit)
    }

    pub(super) fn numeric_lanes<const N: usize>(&mut self, subject: u64, predicates: [u64; N]) -> Result<[f64; N], Error> {
        let mut values = [0.0; N];
        for (dst, predicate) in values.iter_mut().zip(predicates) {
            *dst = number(self.one(subject, predicate)?)?;
        }
        Ok(values)
    }
}

pub(super) fn number(object: u64) -> Result<f64, Error> {
    let value = match classify_inline_literal(object) {
        Some(InlineLiteral::Integer(value)) => value as f64,
        Some(InlineLiteral::Decimal(value)) => value as f64 / 1_000_000.0,
        Some(InlineLiteral::Float(value)) => f64::from(value),
        _ => return Err(Error::InvalidFact),
    };
    value.is_finite().then_some(value).ok_or(Error::InvalidFact)
}

pub(super) fn index(object: u64) -> Result<usize, Error> {
    match classify_inline_literal(object) {
        Some(InlineLiteral::Integer(value)) => usize::try_from(value).map_err(|_| Error::InvalidFact),
        _ => Err(Error::InvalidFact),
    }
}

fn fit_for(fits: &[FittedRotation], semantic_id: u64) -> Result<&FittedRotation, Error> {
    let mut matches = fits.iter().filter(|fit| fit.semantic_id == semantic_id);
    let fit = matches.next().ok_or(Error::MissingFit)?;
    if matches.next().is_some() {
        return Err(Error::AmbiguousFit);
    }
    Ok(fit)
}

/// Runtime state used to authorize the EONT action before planning any motion.
pub struct SemanticAction<'a> {
    pub iri: &'a str,
    pub affordances: &'a [AffordanceBinaryRecord],
    pub capability_ids: &'a [u64],
    pub observer_status: u8,
    pub energy_permill: u16,
    pub damage_permill: u16,
    pub hydration_permill: u16,
    pub wear_permill: u16,
}

impl SemanticAction<'_> {
    pub(super) fn authorize(&self, graph: &mut MotionGraph<'_, '_, '_>) -> Result<u64, Error> {
        let hash = q_hash(self.iri);
        if graph.iri(hash)? != self.iri {
            return Err(Error::UnknownIdentity);
        }
        let mut records = self.affordances.iter().filter(|record| record.action_id == hash);
        let record = records.next().ok_or(Error::InvalidCapability)?;
        if records.next().is_some()
            || (record.required_capability_hash != 0
                && !self.capability_ids.contains(&record.required_capability_hash))
            || self.observer_status < record.min_observer_status
            || self.energy_permill < record.min_energy_permill
            || self.damage_permill > record.max_damage_permill
            || self.hydration_permill < record.min_hydration_permill
            || self.wear_permill > record.max_wear_permill
            || [self.energy_permill, self.damage_permill, self.hydration_permill, self.wear_permill]
                .into_iter().any(|value| value > 1000)
        {
            return Err(Error::InvalidCapability);
        }
        Ok(hash)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum MacroChannel {
    #[default]
    Rotation,
    BlendX,
    BlendY,
    MuscleRecruitment,
    MorphWeight,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MacroKeyframe {
    target: u64,
    channel: MacroChannel,
    seconds: f64,
    value: f64,
    fit: Option<FittedRotation>,
}

#[derive(Clone, Copy)]
pub struct MacroValue {
    pub semantic_id: u64,
    pub channel: MacroChannel,
    pub value: f64,
    pub rotation: Option<SemanticRotation>,
}

/// Sample a materialized macro at one time. Each keyframe has one target,
/// channel, time and value in the same named graph as its action. A complete
/// validation pass precedes writes to the published output slice.
pub fn stage_semantic_macro(
    graph: &mut MotionGraph<'_, '_, '_>,
    action: &SemanticAction<'_>,
    fits: &[FittedRotation],
    seconds: f64,
    keys: &mut [MacroKeyframe],
    staged: &mut [Option<MacroValue>],
    out: &mut [Option<MacroValue>],
) -> Result<usize, Error> {
    let extra = std::mem::size_of_val(keys)
        .saturating_add(std::mem::size_of_val(staged))
        .saturating_add(std::mem::size_of_val(out))
        .saturating_add(std::mem::size_of_val(fits))
        .saturating_add(std::mem::size_of::<NQuin>());
    graph.check_budget(extra)?;
    let action_id = action.authorize(graph)?;
    let count = collect_keys(graph, action_id, keys)?;
    sample_keys(&keys[..count], fits, seconds, staged, out)
}

/// Resolve the macro and its verified articulation receipts during admission.
/// `sample_prepared_macro` subsequently needs only these caller-owned keys.
pub fn prepare_q42_semantic_macro(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, keys: &mut [MacroKeyframe],
) -> Result<usize, Error> {
    graph.check_budget(std::mem::size_of_val(keys).saturating_add(2 * std::mem::size_of::<NQuin>()))?;
    let action_id = action.authorize(graph)?;
    let count = collect_keys(graph, action_id, keys)?;
    prepare_fits(graph, &mut keys[..count])?;
    Ok(count)
}

/// Compile and execute a selected N3 rule already sealed in the action scope.
/// Existing EONT authorization, fitted rotation and keyframe checks also apply.
pub fn prepare_q42_rule_macro(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, source: &str, keys: &mut [MacroKeyframe],
) -> Result<usize, Error> {
    graph.check_budget(std::mem::size_of_val(keys).saturating_add(super::rule::RULE_SCRATCH_BYTES))?;
    let action_id = action.authorize(graph)?;
    let rule = super::rule::CompiledMacroRule::admit(graph, source)?;
    let count = rule.collect(graph, action_id, keys)?;
    order_keys(&mut keys[..count])?;
    prepare_fits(graph, &mut keys[..count])?;
    Ok(count)
}

fn prepare_fits(graph: &mut MotionGraph<'_, '_, '_>, keys: &mut [MacroKeyframe]) -> Result<(), Error> {
    for frames in keys.chunk_by_mut(|a, b| (a.target, a.channel) == (b.target, b.channel)) {
        if frames[0].channel == MacroChannel::Rotation {
            let fit = graph.fit(frames[0].target)?;
            for key in frames { key.fit = Some(fit); }
        }
    }
    Ok(())
}

/// Evaluate a previously admitted macro at one time without graph I/O.
pub fn sample_prepared_macro(
    keys: &[MacroKeyframe], seconds: f64, staged: &mut [Option<MacroValue>], out: &mut [Option<MacroValue>],
) -> Result<usize, Error> {
    sample_keys(keys, &[], seconds, staged, out)
}

fn collect_keys(graph: &mut MotionGraph<'_, '_, '_>, action_id: u64, keys: &mut [MacroKeyframe]) -> Result<usize, Error> {
    let mut count = 0;
    let mut cursor = Q42RangeQueryCursor::default();
    loop {
        let mut row = [NQuin::default()];
        let page = graph.page(action_id, KEY, cursor, &mut row)?;
        if page.returned != 0 {
            let entry = keys.get_mut(count).ok_or(Error::OutputTooSmall)?;
            *entry = read_key(graph, row[0].object)?;
            count += 1;
        }
        match page.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }
    order_keys(&mut keys[..count])?;
    Ok(count)
}

pub(super) fn read_key(graph: &mut MotionGraph<'_, '_, '_>, frame: u64) -> Result<MacroKeyframe, Error> {
    graph.iri(frame)?;
    let target = graph.one(frame, TARGET)?;
    graph.iri(target)?;
    let channel = match graph.one(frame, CHANNEL)? {
        ROTATION => MacroChannel::Rotation,
        BLEND_X => MacroChannel::BlendX,
        BLEND_Y => MacroChannel::BlendY,
        MUSCLE => MacroChannel::MuscleRecruitment,
        MORPH => MacroChannel::MorphWeight,
        _ => return Err(Error::InvalidFact),
    };
    let time = number(graph.one(frame, SECONDS)?)?;
    let value = number(graph.one(frame, VALUE)?)?;
    if time < 0.0
        || (matches!(channel, MacroChannel::MuscleRecruitment | MacroChannel::MorphWeight)
            && !(0.0..=1.0).contains(&value))
    {
        return Err(Error::InvalidFact);
    }
    Ok(MacroKeyframe { target, channel, seconds: time, value, fit: None })
}

fn order_keys(keys: &mut [MacroKeyframe]) -> Result<(), Error> {
    if keys.is_empty() {
        return Err(Error::MissingFact);
    }
    keys.sort_unstable_by(|a, b| (a.target, a.channel).cmp(&(b.target, b.channel))
        .then(a.seconds.total_cmp(&b.seconds)));
    if keys.windows(2).any(|pair| (pair[0].target, pair[0].channel, pair[0].seconds)
        == (pair[1].target, pair[1].channel, pair[1].seconds))
    {
        return Err(Error::ConflictingFact);
    }
    Ok(())
}

fn sample_keys(
    keys: &[MacroKeyframe], fits: &[FittedRotation], seconds: f64,
    staged: &mut [Option<MacroValue>], out: &mut [Option<MacroValue>],
) -> Result<usize, Error> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(Error::InvalidCommand);
    }
    if keys.is_empty() {
        return Err(Error::MissingFact);
    }
    let mut written = 0;
    for frames in keys.chunk_by(|a, b| (a.target, a.channel) == (b.target, b.channel)) {
        let end = frames.partition_point(|frame| frame.seconds <= seconds);
        let value = match (end, frames.get(end)) {
            (0, _) => frames[0].value,
            (_, None) => frames[end - 1].value,
            (_, Some(right)) => {
                let left = frames[end - 1];
                let time = (seconds - left.seconds) / (right.seconds - left.seconds);
                left.value * (1.0 - time) + right.value * time
            }
        };
        if !value.is_finite() {
            return Err(Error::InvalidFact);
        }
        let frame = frames[0];
        let rotation = if frame.channel == MacroChannel::Rotation {
            let fit = match &frame.fit {
                Some(fit) => fit,
                None => fit_for(fits, frame.target)?,
            };
            Some(fitted_rotation(fit, value)?)
        } else {
            None
        };
        if written >= out.len() {
            return Err(Error::OutputTooSmall);
        }
        *staged.get_mut(written).ok_or(Error::OutputTooSmall)? = Some(MacroValue {
            semantic_id: frame.target,
            channel: frame.channel,
            value: rotation.map_or(value, |rotation| rotation.angle_radians),
            rotation,
        });
        written += 1;
    }
    out[..written].copy_from_slice(&staged[..written]);
    Ok(written)
}

pub struct ControllerFeedback {
    pub angle_rad: f64,
    pub angular_velocity_rad_s: f64,
    pub target_rad: f64,
    pub dt_s: f64,
}

pub struct FeedbackRotation {
    pub rotation: SemanticRotation,
    pub angular_velocity_rad_s: f64,
    pub applied_torque_nm: f64,
}

/// Admitted material and articulation scalars for one resident feedback rotor.
pub struct PreparedFeedbackRotation {
    fit: FittedRotation,
    inertia: f64,
    stiffness: f64,
    damping_ratio: f64,
    max_torque_nm: f64,
}

impl PreparedFeedbackRotation {
    pub fn semantic_id(&self) -> u64 { self.fit.semantic_id }

    pub fn resting_angle(&self) -> Result<f64, Error> {
        Ok(fitted_rotation(&self.fit, 0.0)?.angle_radians)
    }

    /// Integrate one deterministic frame using the admitted mechanical constants.
    pub fn step(&self, feedback: ControllerFeedback) -> Result<FeedbackRotation, Error> {
        feedback_step(&self.fit, feedback, self.inertia, self.stiffness, self.damping_ratio, self.max_torque_nm)
    }
}

/// Admit a Q42 material selector and fitted axis once for repeated feedback steps.
pub fn prepare_feedback_rotation(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, material: &M64Reader<'_>,
) -> Result<PreparedFeedbackRotation, Error> {
    graph.check_budget(2 * std::mem::size_of::<NQuin>())?;
    let controller = action.authorize(graph)?;
    let target = graph.one(controller, TARGET)?;
    graph.iri(target)?;
    let fit = graph.fit(target)?;
    prepare_material(graph, controller, fit, material)
}

/// Also require the selected material entry to match its authored HMC IRI.
pub fn prepare_feedback_rotation_for_resource(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, material_iri: &str,
    material: &M64Reader<'_>,
) -> Result<PreparedFeedbackRotation, Error> {
    graph.check_budget(2 * std::mem::size_of::<NQuin>())?;
    let controller = action.authorize(graph)?;
    let source = graph.one(controller, MATERIAL_RESOURCE)?;
    if graph.iri(source)? != material_iri { return Err(Error::InvalidMaterial); }
    let target = graph.one(controller, TARGET)?;
    graph.iri(target)?;
    let fit = graph.fit(target)?;
    prepare_material(graph, controller, fit, material)
}

/// Step a torsional feedback controller. Controller parameters and the material
/// selector are Q42 facts. Density and shear modulus come from a checked M64
/// tissue record. Return a staged motor only after checking articulation limits.
pub fn stage_feedback_rotation(
    graph: &mut MotionGraph<'_, '_, '_>,
    action: &SemanticAction<'_>,
    material: &M64Reader<'_>,
    fits: &[FittedRotation],
    feedback: ControllerFeedback,
) -> Result<FeedbackRotation, Error> {
    graph.check_budget(std::mem::size_of_val(fits).saturating_add(2 * std::mem::size_of::<NQuin>()))?;
    let controller = action.authorize(graph)?;
    let target = graph.one(controller, TARGET)?;
    graph.iri(target)?;
    let fit = *fit_for(fits, target)?;
    prepare_material(graph, controller, fit, material)?.step(feedback)
}

fn prepare_material(
    graph: &mut MotionGraph<'_, '_, '_>, controller: u64, fit: FittedRotation, material: &M64Reader<'_>,
) -> Result<PreparedFeedbackRotation, Error> {
    let (inertia, stiffness, damping_ratio) = rotor_constants(graph, controller, material)?;
    let max_torque_nm = number(graph.one(controller, TORQUE)?)?;
    if max_torque_nm <= 0.0 { return Err(Error::InvalidFact); }
    Ok(PreparedFeedbackRotation { fit, inertia, stiffness, damping_ratio, max_torque_nm })
}

fn rotor_constants(
    graph: &mut MotionGraph<'_, '_, '_>, controller: u64, material: &M64Reader<'_>,
) -> Result<(f64, f64, f64), Error> {
    let chunk = index(graph.one(controller, MATERIAL_CHUNK)?)?;
    let index = index(graph.one(controller, MATERIAL_RECORD)?)?;
    let record = material.chunk(chunk).ok_or(Error::InvalidMaterial)?
        .decode_record(index).map_err(|_| Error::InvalidMaterial)?;
    let Some(M64DecodedRecord::BioTissue(tissue)) = record else {
        return Err(Error::InvalidMaterial);
    };
    let volume_m3 = number(graph.one(controller, VOLUME)?)?;
    let gyration_m = number(graph.one(controller, GYRATION)?)?;
    let torsion_j = number(graph.one(controller, TORSION_J)?)?;
    let span_m = number(graph.one(controller, SPAN)?)?;
    let damping_ratio = number(graph.one(controller, DAMPING)?)?;
    if ![volume_m3, gyration_m, torsion_j, span_m].into_iter().all(|v| v > 0.0)
        || damping_ratio < 0.0
    {
        return Err(Error::InvalidFact);
    }
    let inertia = f64::from(tissue.apparent_density_kg_m3) * volume_m3 * gyration_m.powi(2);
    let stiffness = f64::from(tissue.shear_g12_mpa()) * 1.0e6 * torsion_j / span_m;
    if !inertia.is_finite() || !stiffness.is_finite() || inertia <= 0.0 || stiffness <= 0.0 {
        return Err(Error::InvalidMaterial);
    }
    Ok((inertia, stiffness, damping_ratio))
}

/// One resident aeroelastic branch. Geometry and wind-profile scalars come
/// from the mounted model, while density and torsional stiffness are checked
/// against the referenced M64 tissue record at setup.
pub struct PreparedAeroelasticRotation {
    fit: FittedRotation,
    profile: BoundaryLayerProfile,
    branch: AeroelasticBranch,
    pressure: f32,
    maximum_step_s: f32,
}

impl PreparedAeroelasticRotation {
    pub fn semantic_id(&self) -> u64 { self.fit.semantic_id }

    pub fn wind_speed_m_s(&self, height_m: f32) -> f32 { self.profile.sample_wind_speed(height_m) }

    /// Step the actual Etalon Vogel oscillator, then enforce the fitted contact
    /// and ROM envelope before publishing either the state or its motor.
    pub fn advance(&mut self, dt_s: f64) -> Result<SemanticRotation, Error> {
        if !dt_s.is_finite() || !(0.0..=1.0).contains(&dt_s) { return Err(Error::InvalidCommand); }
        if dt_s == 0.0 { return fitted_rotation(&self.fit, f64::from(self.branch.deflection_rad)); }
        let count = (dt_s / f64::from(self.maximum_step_s)).ceil();
        if !count.is_finite() || count > 1024.0 { return Err(Error::BudgetExceeded); }
        let dt = (dt_s / count) as f32;
        if dt <= 0.0 { return Err(Error::InvalidCommand); }
        let mut branch = self.branch;
        for _ in 0..count as u32 {
            branch.step(self.pressure, dt);
            if !branch.deflection_rad.is_finite() || !branch.angular_velocity.is_finite() { return Err(Error::InvalidCommand); }
            let angle = fitted_rotation(&self.fit, f64::from(branch.deflection_rad))?.angle_radians as f32;
            if branch.deflection_rad != angle { branch.angular_velocity = 0.0; }
            branch.deflection_rad = angle;
        }
        let rotation = fitted_rotation(&self.fit, f64::from(branch.deflection_rad))?;
        self.branch = branch;
        Ok(rotation)
    }
}

pub fn prepare_aeroelastic_rotation(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, material_iri: &str,
    material: &M64Reader<'_>,
) -> Result<PreparedAeroelasticRotation, Error> {
    graph.check_budget(2 * std::mem::size_of::<NQuin>())?;
    let controller = action.authorize(graph)?;
    let source = graph.one(controller, MATERIAL_RESOURCE)?;
    if graph.iri(source)? != material_iri { return Err(Error::InvalidMaterial); }
    let target = graph.one(controller, TARGET)?;
    graph.iri(target)?;
    let fit = graph.fit(target)?;
    let (inertia, stiffness, damping_ratio) = rotor_constants(graph, controller, material)?;
    let [height, roughness, displacement, friction, density, vogel, length] = graph.numeric_lanes(controller, [
        WIND_HEIGHT, WIND_ROUGHNESS, WIND_DISPLACEMENT, WIND_FRICTION, AIR_DENSITY, VOGEL_EXPONENT, SPAN,
    ])?.map(|value| value as f32);
    if ![height, roughness, displacement, friction, density, vogel, length].iter().all(|v| v.is_finite())
        || height < 0.0 || roughness <= 0.0 || displacement < 0.0 || friction < 0.0 || density <= 0.0
        || !(-0.8..=-0.4).contains(&vogel) || length <= 0.0
    {
        return Err(Error::InvalidFact);
    }
    let profile = BoundaryLayerProfile {
        roughness_length_z0: roughness, zero_plane_displacement_d: displacement, friction_velocity_u_star: friction,
    };
    let wind_speed = profile.sample_wind_speed(height);
    let pressure = profile.dynamic_pressure(density, wind_speed);
    let omega = (stiffness / inertia).sqrt();
    let frequency = (omega / std::f64::consts::TAU) as f32;
    let step = (0.25 / omega) as f32;
    let damping = damping_ratio as f32;
    if !pressure.is_finite() || !frequency.is_finite() || frequency <= 0.0
        || !damping.is_finite() || !step.is_finite() || step <= 0.0
    {
        return Err(Error::InvalidMaterial);
    }
    let branch = AeroelasticBranch {
        length_m: length, natural_frequency_hz: frequency, damping_ratio: damping,
        vogel_exponent: vogel, deflection_rad: fitted_rotation(&fit, 0.0)?.angle_radians as f32, angular_velocity: 0.0,
    };
    Ok(PreparedAeroelasticRotation { fit, profile, branch, pressure, maximum_step_s: 0.02f32.min(step) })
}

fn feedback_step(
    fit: &FittedRotation, feedback: ControllerFeedback, inertia: f64, stiffness: f64,
    damping_ratio: f64, max_torque: f64,
) -> Result<FeedbackRotation, Error> {
    let ControllerFeedback { angle_rad, angular_velocity_rad_s: velocity, target_rad, dt_s: dt } = feedback;
    if ![angle_rad, velocity, target_rad, dt, inertia, stiffness, damping_ratio, max_torque]
        .into_iter().all(f64::is_finite)
        || dt <= 0.0 || inertia <= 0.0 || stiffness <= 0.0 || damping_ratio < 0.0 || max_torque <= 0.0
    {
        return Err(Error::InvalidCommand);
    }
    let current = fitted_rotation(fit, angle_rad)?;
    if current.angle_radians != angle_rad {
        return Err(Error::InvalidCommand);
    }
    let goal = fitted_rotation(fit, target_rad)?.angle_radians;
    let damping = 2.0 * damping_ratio * (stiffness * inertia).sqrt();
    // Backward Euler for the spring and dashpot, with an actuator torque limit.
    let denom = 1.0 + dt / inertia * (damping + stiffness * dt);
    let implicit_velocity = (velocity + dt * stiffness / inertia * (goal - angle_rad)) / denom;
    let unconstrained_torque = (implicit_velocity - velocity) * inertia / dt;
    if !denom.is_finite() || !unconstrained_torque.is_finite() {
        return Err(Error::InvalidCommand);
    }
    let torque = unconstrained_torque.clamp(-max_torque, max_torque);
    let next_velocity = velocity + dt * torque / inertia;
    let next_angle = angle_rad + dt * next_velocity;
    if ![next_angle, next_velocity, torque].into_iter().all(f64::is_finite) {
        return Err(Error::InvalidCommand);
    }
    let rotation = fitted_rotation(fit, next_angle)?;
    Ok(FeedbackRotation {
        angular_velocity_rad_s: if next_angle == rotation.angle_radians { next_velocity } else { 0.0 },
        applied_torque_nm: torque,
        rotation,
    })
}

#[cfg(test)]
mod tests;
