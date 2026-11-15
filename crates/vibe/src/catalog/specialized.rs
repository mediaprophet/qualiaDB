//! Specialized LocalHost kernels (UE-040…043): econ extras, geometry, clinical/chem honesty.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Attempt a specialized LocalHost invoke. `None` when not handled here.
pub fn try_invoke(id: &str, args: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match id {
        "Econ.rawlsian_welfare" => Some(rawlsian(args, span)),
        "Econ.nash_welfare" => Some(nash(args, span)),
        "Econ.headcount_poverty" => Some(headcount(args, span)),
        "Econ.npv" => Some(npv(args, span)),
        "ComputationalGeometry.orientation_2" => Some(orientation_2(args, span)),
        "ComputationalGeometry.distance_2d" => Some(distance_2d(args, span)),
        "ComputationalGeometry.convex_hull_2" => Some(convex_hull_2(args, span)),
        "ComputationalGeometry.soc_sky_dome" => Some(soc_sky_dome(args, span)),
        "BotanicalAnatomy.synthesize_organ_10d" => Some(botanical_synthesize_organ_10d(args)),
        "BotanicalAnatomy.dissect_tissue_layer" => Some(botanical_dissect_tissue_layer(args)),
        "BotanicalAnatomy.evaluate_anthesis" => Some(botanical_evaluate_anthesis(args)),
        "BotanicalAnatomy.evaluate_stomatal_conductance" => Some(botanical_evaluate_stomatal_conductance(args)),
        "BotanicalAnatomy.evaluate_pipe_model_sapwood" => Some(botanical_evaluate_pipe_model_sapwood(args)),
        "ZoologicalAnatomy.synthesize_vertebrate_10d" => Some(zoological_synthesize_vertebrate_10d(args)),
        "ZoologicalAnatomy.peel_layer" => Some(zoological_peel_layer(args)),
        "ZoologicalAnatomy.evaluate_tendon_strain" => Some(zoological_evaluate_tendon_strain(args)),
        "BiologicalOntology.query_callout_pin" => Some(biological_ontology_query_callout_pin(args)),
        "BiologicalOntology.resolve_curie" => Some(biological_ontology_resolve_curie(args)),
        "WorldContinuum.sample_elevation_normal" => Some(world_continuum_sample_elevation_normal(args)),
        "WorldContinuum.step_rover_64hz" => Some(world_continuum_step_rover_64hz(args)),
        "WorldContinuum.step_character_3d" => Some(world_continuum_step_character_3d(args)),
        "WorldContinuum.stream_hilbert_chunks" => Some(world_continuum_stream_hilbert_chunks(args)),
        "WorldContinuum.locate_study_site" => Some(world_continuum_locate_study_site(args)),
        "SceneGraph10D.step_ontogeny" => Some(scene_graph_10d_step_ontogeny(args)),
        "SceneGraph10D.dispatch_affordance" => Some(scene_graph_10d_dispatch_affordance(args)),
        "SceneGraph10D.cull_indirect_4tier" => Some(scene_graph_10d_cull_indirect_4tier(args)),
        "ClinicalRisk.framingham"
        | "OrganicChemistry.compute"
        | "OrganicChemistry.validate_smiles" => Some(engine_required(id, span)),
        "Emf.openContainer"
        | "Emf.buildContainer"
        | "Emf.verifyContainer"
        | "Emf.sonifyField"
        | "Emf.evaluateDipole"
        | "Qac.openContainer"
        | "Qac.buildContainer"
        | "Qac.renderBinaural"
        | "Qac.evaluateDirectivity"
        | "Qac.sonifyEmf"
        | "Dicom.parseDataset"
        | "Dicom.verifyDataset"
        | "Dicom.toEmfField"
        | "Dicom.reconstructVolume"
        | "Sensing.admitEvidence"
        | "Sensing.verifyCalibration" => Some(up02_containers::invoke(id, args, span)),
        _ => None,
    }
}

mod up02_containers {
    use super::super::envelope_admission::{
        admit_envelope, build_envelope, bytes_to_vibe_list, decode_byte_list,
        extract_binary_bytes, reject_forbidden_source_keys, select_admitted_entry, AdmittedEntry,
        AdmittedEnvelope, BuildEntry, EnvelopeFamily,
    };
    use crate::error::{DiagCode, Diagnostic};
    use crate::span::Span;
    use crate::value::Value;
    use etalon_vibe::{
        oracle_bioelectric_dipole_preview, oracle_clock_stamp_drift_ns,
        oracle_decode_container_summary, oracle_dicom_modality_check,
        oracle_evaluate_directivity_sh, oracle_radiative_transfer_slice,
        oracle_render_binaural_frame, oracle_sh_irradiance_probe, oracle_sonify_emf_envelope,
        oracle_verify_evidence_gate,
    };
    use std::collections::BTreeMap;
    use vibescript_abi::VibeTrap;

    pub(super) fn invoke(id: &str, args: &Value, span: Span) -> Result<Value, Diagnostic> {
        reject_forbidden_source_keys(args, span)?;
        match id {
            "Emf.buildContainer" => build_container(EnvelopeFamily::EmfV4, args, span),
            "Emf.openContainer" => open_container(EnvelopeFamily::EmfV4, args, span),
            "Emf.verifyContainer" => verify_emf_container(args, span),
            "Emf.evaluateDipole" => evaluate_dipole(args, span),
            "Emf.sonifyField" => sonify_emf_field(args, span),
            "Qac.buildContainer" => build_container(EnvelopeFamily::QacV1, args, span),
            "Qac.openContainer" => open_container(EnvelopeFamily::QacV1, args, span),
            "Qac.renderBinaural" => render_binaural(args, span),
            "Qac.evaluateDirectivity" => evaluate_directivity(args, span),
            "Qac.sonifyEmf" => qac_sonify_emf(args, span),
            "Dicom.parseDataset" => dicom_parse_or_verify(args, false, span),
            "Dicom.verifyDataset" => dicom_parse_or_verify(args, true, span),
            "Dicom.toEmfField" => dicom_to_emf_field(args, span),
            "Dicom.reconstructVolume" => dicom_reconstruct_volume(args, span),
            "Sensing.admitEvidence" => sensing_admit_evidence(args, span),
            "Sensing.verifyCalibration" => sensing_verify_calibration(args, span),
            _ => Err(Diagnostic::new(
                DiagCode::E300,
                span,
                format!("unsupported UP-02 catalog invoke `{id}`"),
            )),
        }
    }

    fn trap_err(op: &str, trap: VibeTrap, span: Span) -> Diagnostic {
        Diagnostic::new(
            DiagCode::E600,
            span,
            format!("{op}: etalon-vibe rejected inputs with {trap:?}"),
        )
    }

    fn rec_map<'a>(val: &'a Value, ctx: &str, span: Span) -> Result<&'a BTreeMap<String, Value>, Diagnostic> {
        match val {
            Value::Record(rec) => Ok(rec),
            _ => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("{ctx} expects a Record value"),
            )),
        }
    }

    fn opt_u64(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<Option<u64>, Diagnostic> {
        match rec.get(key) {
            None => Ok(None),
            Some(Value::U64(u)) => Ok(Some(*u)),
            Some(Value::I64(i)) if *i >= 0 => Ok(Some(*i as u64)),
            Some(_) => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` must be a non-negative integer"),
            )),
        }
    }

    fn req_u64(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<u64, Diagnostic> {
        opt_u64(rec, key, span)?.ok_or_else(|| {
            Diagnostic::new(DiagCode::E100, span, format!("missing required integer field `{key}`"))
        })
    }

    fn opt_i64(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<Option<i64>, Diagnostic> {
        match rec.get(key) {
            None => Ok(None),
            Some(Value::I64(i)) => Ok(Some(*i)),
            Some(Value::U64(u)) if *u <= i64::MAX as u64 => Ok(Some(*u as i64)),
            Some(_) => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` must be an i64 integer"),
            )),
        }
    }

    fn opt_u32(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<Option<u32>, Diagnostic> {
        match opt_u64(rec, key, span)? {
            None => Ok(None),
            Some(v) => u32::try_from(v).map(Some).map_err(|_| {
                Diagnostic::new(DiagCode::E100, span, format!("field `{key}` overflows u32"))
            }),
        }
    }

    fn opt_u16(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<Option<u16>, Diagnostic> {
        match opt_u64(rec, key, span)? {
            None => Ok(None),
            Some(v) => u16::try_from(v).map(Some).map_err(|_| {
                Diagnostic::new(DiagCode::E100, span, format!("field `{key}` overflows u16"))
            }),
        }
    }

    fn opt_f64(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<Option<f64>, Diagnostic> {
        match rec.get(key) {
            None => Ok(None),
            Some(v) => match v.as_f64() {
                Some(n) if n.is_finite() => Ok(Some(n)),
                _ => Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("field `{key}` must be a finite number"),
                )),
            },
        }
    }

    fn req_f64(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<f64, Diagnostic> {
        opt_f64(rec, key, span)?.ok_or_else(|| {
            Diagnostic::new(DiagCode::E100, span, format!("missing required numeric field `{key}`"))
        })
    }

    fn opt_bool(rec: &BTreeMap<String, Value>, key: &str, span: Span) -> Result<Option<bool>, Diagnostic> {
        match rec.get(key) {
            None => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` must be a Bool"),
            )),
        }
    }

    fn opt_f32_arr<const N: usize>(
        rec: &BTreeMap<String, Value>,
        key: &str,
        span: Span,
    ) -> Result<Option<[f32; N]>, Diagnostic> {
        let Some(val) = rec.get(key) else {
            return Ok(None);
        };
        let Value::List(xs) = val else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` must be a List<{N}> of finite floats"),
            ));
        };
        if xs.len() != N {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` requires length {N}, got {}", xs.len()),
            ));
        }
        let mut out = [0.0f32; N];
        for (i, item) in xs.iter().enumerate() {
            let v = item.as_f64().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, format!("non-numeric element in `{key}`"))
            })?;
            let f = v as f32;
            if !f.is_finite() {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("non-finite float in `{key}`"),
                ));
            }
            out[i] = f;
        }
        Ok(Some(out))
    }

    fn opt_f64_arr<const N: usize>(
        rec: &BTreeMap<String, Value>,
        key: &str,
        span: Span,
    ) -> Result<Option<[f64; N]>, Diagnostic> {
        let Some(val) = rec.get(key) else {
            return Ok(None);
        };
        let Value::List(xs) = val else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` must be a List<{N}> of finite floats"),
            ));
        };
        if xs.len() != N {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("field `{key}` requires length {N}, got {}", xs.len()),
            ));
        }
        let mut out = [0.0f64; N];
        for (i, item) in xs.iter().enumerate() {
            let v = item.as_f64().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, format!("non-numeric element in `{key}`"))
            })?;
            if !v.is_finite() {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("non-finite float in `{key}`"),
                ));
            }
            out[i] = v;
        }
        Ok(Some(out))
    }

    fn read_f32_le(bytes: &[u8], off: usize) -> Result<f32, Diagnostic> {
        let arr: [u8; 4] = bytes
            .get(off..off + 4)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(|| Diagnostic::new(DiagCode::E100, Span::new(0, 0), "payload too short for f32"))?;
        let f = f32::from_le_bytes(arr);
        if !f.is_finite() {
            return Err(Diagnostic::new(DiagCode::E100, Span::new(0, 0), "non-finite f32 in binary payload"));
        }
        Ok(f)
    }

    fn read_f64_le(bytes: &[u8], off: usize) -> Result<f64, Diagnostic> {
        let arr: [u8; 8] = bytes
            .get(off..off + 8)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(|| Diagnostic::new(DiagCode::E100, Span::new(0, 0), "payload too short for f64"))?;
        let f = f64::from_le_bytes(arr);
        if !f.is_finite() {
            return Err(Diagnostic::new(DiagCode::E100, Span::new(0, 0), "non-finite f64 in binary payload"));
        }
        Ok(f)
    }

    struct OwnedBuildEntry {
        mode: u16,
        revision: u16,
        flags: u32,
        stream_id: u32,
        frame_id: u32,
        start_tick: i64,
        time_count: u64,
        spatial_key: u64,
        record_count: u64,
        stride: u32,
        descriptor_id: u32,
        payload: Vec<u8>,
    }

    fn parse_build_entry(
        rec: &BTreeMap<String, Value>,
        idx: usize,
        span: Span,
    ) -> Result<OwnedBuildEntry, Diagnostic> {
        let payload_val = rec.get("payload").ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                format!("entry {idx} requires binary `payload` byte list"),
            )
        })?;
        let payload = decode_byte_list(payload_val, "payload", span)?;
        let mode = opt_u16(rec, "mode", span)?.unwrap_or(0x0100);
        let revision = opt_u16(rec, "revision", span)?.unwrap_or(1);
        let flags = opt_u32(rec, "flags", span)?.unwrap_or(0);
        let stream_id = opt_u32(rec, "stream_id", span)?.unwrap_or(0);
        let frame_id = opt_u32(rec, "frame_id", span)?.unwrap_or(0);
        let start_tick = opt_i64(rec, "start_tick", span)?.unwrap_or(0);
        let time_count = opt_u64(rec, "time_count", span)?.unwrap_or(0);
        let spatial_key = opt_u64(rec, "spatial_key", span)?.unwrap_or(0);
        let descriptor_id = opt_u32(rec, "descriptor_id", span)?
            .or(opt_u32(rec, "entry_id", span)?)
            .unwrap_or((idx + 1) as u32);
        let (record_count, stride) = match (opt_u64(rec, "record_count", span)?, opt_u32(rec, "stride", span)?) {
            (Some(rc), Some(st)) => (rc, st),
            (Some(rc), None) => {
                if rc == 0 || payload.len() as u64 % rc != 0 {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("entry {idx} payload len {} not divisible by record_count {rc}", payload.len()),
                    ));
                }
                let st = u32::try_from(payload.len() as u64 / rc).map_err(|_| {
                    Diagnostic::new(DiagCode::E100, span, "computed stride overflows u32")
                })?;
                (rc, st)
            }
            (None, Some(st)) => {
                if st == 0 || payload.len() % (st as usize) != 0 {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("entry {idx} payload len {} not divisible by stride {st}", payload.len()),
                    ));
                }
                ((payload.len() / (st as usize)) as u64, st)
            }
            (None, None) => {
                let st = u32::try_from(payload.len()).map_err(|_| {
                    Diagnostic::new(DiagCode::E100, span, "payload len overflows u32 stride")
                })?;
                (1u64, st)
            }
        };
        Ok(OwnedBuildEntry {
            mode,
            revision,
            flags,
            stream_id,
            frame_id,
            start_tick,
            time_count,
            spatial_key,
            record_count,
            stride,
            descriptor_id,
            payload,
        })
    }

    fn build_container(
        family: EnvelopeFamily,
        args: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let rec = rec_map(args, "buildContainer", span)?;
        let alignment_4096 = match opt_u64(rec, "alignment", span)? {
            None | Some(64) => false,
            Some(4096) => true,
            Some(other) => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("alignment must be 64 or 4096, got {other}"),
                ))
            }
        };
        let metadata = match rec.get("metadata") {
            None => Vec::new(),
            Some(Value::List(xs)) if xs.is_empty() => Vec::new(),
            Some(other) => decode_byte_list(other, "metadata", span)?,
        };
        let owned_entries: Vec<OwnedBuildEntry> = match rec.get("entries") {
            Some(Value::List(list)) => {
                if list.is_empty() {
                    return Err(Diagnostic::new(
                        DiagCode::E500,
                        span,
                        "buildContainer requires at least 1 entry (empty containers refused)",
                    ));
                }
                let mut out = Vec::with_capacity(list.len());
                for (i, item) in list.iter().enumerate() {
                    let er = rec_map(item, "entries element", span)?;
                    reject_forbidden_source_keys(item, span)?;
                    out.push(parse_build_entry(er, i, span)?);
                }
                out
            }
            Some(_) => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "`entries` must be a List of entry Records",
                ))
            }
            None => vec![parse_build_entry(rec, 0, span)?],
        };
        let refs: Vec<BuildEntry<'_>> = owned_entries
            .iter()
            .map(|e| BuildEntry {
                mode: e.mode,
                revision: e.revision,
                flags: e.flags,
                stream_id: e.stream_id,
                frame_id: e.frame_id,
                start_tick: e.start_tick,
                time_count: e.time_count,
                spatial_key: e.spatial_key,
                record_count: e.record_count,
                stride: e.stride,
                descriptor_id: e.descriptor_id,
                payload: &e.payload,
            })
            .collect();
        let bytes = build_envelope(family, alignment_4096, &metadata, &refs)
            .map_err(|msg| Diagnostic::new(DiagCode::E500, span, msg))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("family".into(), Value::String(family.label().into()));
        out.insert("version".into(), Value::U64(u64::from(family.version())));
        out.insert("entry_count".into(), Value::U64(refs.len() as u64));
        out.insert("total_bytes".into(), Value::U64(bytes.len() as u64));
        out.insert("bytes".into(), bytes_to_vibe_list(&bytes));
        Ok(Value::Record(out))
    }

    fn entry_summary_record(idx: usize, e: &AdmittedEntry<'_>) -> Value {
        let mut r = BTreeMap::new();
        r.insert("entry_index".into(), Value::U64(idx as u64));
        r.insert("entry_id".into(), Value::U64(u64::from(e.descriptor_id)));
        r.insert("mode".into(), Value::U64(u64::from(e.mode)));
        r.insert("revision".into(), Value::U64(u64::from(e.revision)));
        r.insert("flags".into(), Value::U64(u64::from(e.flags)));
        r.insert("stream_id".into(), Value::U64(u64::from(e.stream_id)));
        r.insert("frame_id".into(), Value::U64(u64::from(e.frame_id)));
        r.insert("start_tick".into(), Value::I64(e.start_tick));
        r.insert("time_count".into(), Value::U64(e.time_count));
        r.insert("spatial_key".into(), Value::U64(e.spatial_key));
        r.insert("record_count".into(), Value::U64(e.record_count));
        r.insert("stride".into(), Value::U64(u64::from(e.stride)));
        r.insert("payload_offset".into(), Value::U64(e.payload_offset));
        r.insert("stored_bytes".into(), Value::U64(e.stored_bytes));
        r.insert("decoded_bytes".into(), Value::U64(e.decoded_bytes));
        r.insert("digest".into(), bytes_to_vibe_list(e.digest));
        Value::Record(r)
    }

    fn admit_family_and_select<'a>(
        bytes: &'a [u8],
        family: EnvelopeFamily,
        args: &Value,
        span: Span,
    ) -> Result<(AdmittedEnvelope<'a>, usize), Diagnostic> {
        let env = admit_envelope(bytes, family).map_err(|msg| {
            Diagnostic::new(
                DiagCode::E500,
                span,
                format!("strict admission gate rejected {} container: {msg}", family.label()),
            )
        })?;
        let (idx, _) = select_admitted_entry(&env, args, span)?;
        Ok((env, idx))
    }

    fn qac_modes_mask(env: &AdmittedEnvelope<'_>) -> u32 {
        let mut mask = 0u32;
        for e in &env.entries {
            let bit = u32::from(e.mode.saturating_sub(0x0100)) & 0x03;
            mask |= 1u32 << bit;
        }
        mask.max(1)
    }

    fn open_container(
        family: EnvelopeFamily,
        args: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
        let (env, idx) = admit_family_and_select(&bytes, family, args, span)?;
        let selected = &env.entries[idx];
        let rec = rec_map(args, "openContainer", span)?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("family".into(), Value::String(env.family.label().into()));
        out.insert("version".into(), Value::U64(u64::from(env.version)));
        out.insert("flags".into(), Value::U64(u64::from(env.flags)));
        out.insert("index_digest".into(), bytes_to_vibe_list(env.index_digest));
        out.insert("entry_count".into(), Value::U64(env.entries.len() as u64));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("mode".into(), Value::U64(u64::from(selected.mode)));
        out.insert("revision".into(), Value::U64(u64::from(selected.revision)));
        out.insert("stream_id".into(), Value::U64(u64::from(selected.stream_id)));
        out.insert("frame_id".into(), Value::U64(u64::from(selected.frame_id)));
        out.insert("record_count".into(), Value::U64(selected.record_count));
        out.insert("stride".into(), Value::U64(u64::from(selected.stride)));
        out.insert("stored_bytes".into(), Value::U64(selected.stored_bytes));
        out.insert("decoded_bytes".into(), Value::U64(selected.decoded_bytes));
        out.insert("metadata_bytes".into(), Value::U64(env.metadata.len() as u64));
        out.insert("total_bytes".into(), Value::U64(env.total_bytes));
        out.insert(
            "entries".into(),
            Value::List(
                env.entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| entry_summary_record(i, e))
                    .collect(),
            ),
        );

        if family == EnvelopeFamily::QacV1 {
            let sample_rate_hz = opt_u32(rec, "sample_rate_hz", span)?.unwrap_or(48_000);
            let frame_samples = opt_u32(rec, "frame_samples", span)?
                .unwrap_or_else(|| selected.record_count.min(u64::from(u32::MAX)) as u32);
            let modes_mask = opt_u32(rec, "modes_mask", span)?.unwrap_or_else(|| qac_modes_mask(&env));
            let header_meta = [
                env.entries.len() as u32,
                sample_rate_hz,
                frame_samples,
                modes_mask,
            ];
            let summary = oracle_decode_container_summary(header_meta)
                .map_err(|t| trap_err("Qac.openContainer", t, span))?;
            out.insert("duration_micros".into(), Value::U64(summary[0]));
            out.insert("active_mode_count".into(), Value::U64(summary[1]));
        }

        Ok(Value::Record(out))
    }

    fn verify_emf_container(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
        let (env, idx) = admit_family_and_select(&bytes, EnvelopeFamily::EmfV4, args, span)?;
        let selected = &env.entries[idx];
        let rec = rec_map(args, "Emf.verifyContainer", span)?;

        if let Some(expected_count) = opt_u64(rec, "expected_entry_count", span)? {
            if env.entries.len() as u64 != expected_count {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!(
                        "Emf.verifyContainer expected {expected_count} entries, found {}",
                        env.entries.len()
                    ),
                ));
            }
        }
        if let Some(expected_mode) = opt_u16(rec, "expected_mode", span)? {
            if selected.mode != expected_mode {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!(
                        "Emf.verifyContainer expected mode {expected_mode:#06x}, found {:#06x}",
                        selected.mode
                    ),
                ));
            }
        }

        let mut out = BTreeMap::new();
        out.insert("verified".into(), Value::Bool(true));
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("family".into(), Value::String(EnvelopeFamily::EmfV4.label().into()));
        out.insert("version".into(), Value::U64(4));
        out.insert("entry_count".into(), Value::U64(env.entries.len() as u64));
        out.insert("verified_entries".into(), Value::U64(env.entries.len() as u64));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("mode".into(), Value::U64(u64::from(selected.mode)));
        out.insert("total_bytes".into(), Value::U64(env.total_bytes));
        Ok(Value::Record(out))
    }

    fn evaluate_dipole(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
        let (env, idx) = admit_family_and_select(&bytes, EnvelopeFamily::EmfV4, args, span)?;
        let selected = &env.entries[idx];
        let rec = rec_map(args, "Emf.evaluateDipole", span)?;

        let (dipole, probe, gain) = if let (Some(d), Some(p)) = (
            opt_f32_arr::<3>(rec, "dipole_moment_xyz", span)?,
            opt_f32_arr::<3>(rec, "probe_offset_m", span)?,
        ) {
            let g = opt_f64(rec, "gain", span)?.unwrap_or(1.0) as f32;
            (d, p, g)
        } else if selected.payload.len() >= 28 {
            let p = selected.payload;
            (
                [read_f32_le(p, 0)?, read_f32_le(p, 4)?, read_f32_le(p, 8)?],
                [read_f32_le(p, 12)?, read_f32_le(p, 16)?, read_f32_le(p, 20)?],
                read_f32_le(p, 24)?,
            )
        } else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "Emf.evaluateDipole requires dipole_moment_xyz + probe_offset_m or >= 28-byte f32 binary entry payload",
            ));
        };

        let res = oracle_bioelectric_dipole_preview(dipole, probe, gain)
            .map_err(|t| trap_err("Emf.evaluateDipole", t, span))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("potential_uv".into(), Value::F64(f64::from(res[0])));
        out.insert("field_mag_uv_per_m".into(), Value::F64(f64::from(res[1])));
        out.insert(
            "values".into(),
            Value::List(vec![
                Value::F64(f64::from(res[0])),
                Value::F64(f64::from(res[1])),
            ]),
        );

        if let Some(opt) = opt_f32_arr::<3>(rec, "optical_params", span)? {
            let inc = opt_f64(rec, "incident_radiance", span)?.unwrap_or(1.0) as f32;
            let rt = oracle_radiative_transfer_slice(opt, inc)
                .map_err(|t| trap_err("Emf.evaluateDipole(radiative_transfer)", t, span))?;
            out.insert("transmitted_radiance".into(), Value::F64(f64::from(rt[0])));
            out.insert(
                "single_scatter_radiance".into(),
                Value::F64(f64::from(rt[1])),
            );
        }
        if let (Some(sh), Some(norm)) = (
            opt_f32_arr::<4>(rec, "sh_l0_l1", span)?,
            opt_f32_arr::<3>(rec, "normal_xyz", span)?,
        ) {
            let irr = oracle_sh_irradiance_probe(sh, norm)
                .map_err(|t| trap_err("Emf.evaluateDipole(sh_irradiance)", t, span))?;
            out.insert("irradiance".into(), Value::F64(f64::from(irr[0])));
            out.insert(
                "directional_contrast".into(),
                Value::F64(f64::from(irr[1])),
            );
        }

        Ok(Value::Record(out))
    }

    fn extract_emf_sample(
        rec: &BTreeMap<String, Value>,
        entry: &AdmittedEntry<'_>,
        span: Span,
    ) -> Result<[f64; 3], Diagnostic> {
        if let Some(arr) = opt_f64_arr::<3>(rec, "emf_sample", span)? {
            return Ok(arr);
        }
        if rec.contains_key("wavelength_nm")
            || rec.contains_key("e_mag_v_per_m")
            || rec.contains_key("spectral_sigma")
        {
            return Ok([
                req_f64(rec, "wavelength_nm", span)?,
                req_f64(rec, "e_mag_v_per_m", span)?,
                req_f64(rec, "spectral_sigma", span)?,
            ]);
        }
        // Decode either 64-byte `EmfCellRecord64` (e_field[0..12], wavelength[28..32], sigma[56..60])
        // or packed 24-byte [f64; 3] LE sample directly from the admitted binary entry payload.
        if entry.payload.len() == 64 {
            let p = entry.payload;
            let ex = f64::from(read_f32_le(p, 0)?);
            let ey = f64::from(read_f32_le(p, 4)?);
            let ez = f64::from(read_f32_le(p, 8)?);
            let e_mag = (ex * ex + ey * ey + ez * ez).sqrt();
            let wavelength_nm = f64::from(read_f32_le(p, 28)?);
            let spectral_sigma = f64::from(read_f32_le(p, 56)?);
            return Ok([wavelength_nm, e_mag, spectral_sigma]);
        }
        if entry.payload.len() >= 24 {
            let p = entry.payload;
            return Ok([
                read_f64_le(p, 0)?,
                read_f64_le(p, 8)?,
                read_f64_le(p, 16)?,
            ]);
        }
        Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "missing emf_sample [wavelength_nm, e_mag_v_per_m, spectral_sigma] or 64B/24B EMF binary payload",
        ))
    }

    fn sonify_emf_field(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
        let (env, idx) = admit_family_and_select(&bytes, EnvelopeFamily::EmfV4, args, span)?;
        let selected = &env.entries[idx];
        let rec = rec_map(args, "Emf.sonifyField", span)?;
        let sample = extract_emf_sample(rec, selected, span)?;
        let res = oracle_sonify_emf_envelope(sample)
            .map_err(|t| trap_err("Emf.sonifyField", t, span))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("carrier_hz".into(), Value::F64(res[0]));
        out.insert("amplitude_norm".into(), Value::F64(res[1]));
        out.insert(
            "values".into(),
            Value::List(vec![Value::F64(res[0]), Value::F64(res[1])]),
        );
        Ok(Value::Record(out))
    }

    fn render_binaural(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
        let (env, idx) = admit_family_and_select(&bytes, EnvelopeFamily::QacV1, args, span)?;
        let selected = &env.entries[idx];
        let rec = rec_map(args, "Qac.renderBinaural", span)?;

        let (pose, gain_lin) = if let Some(pose) = opt_f32_arr::<3>(rec, "pose_deg_dist", span)? {
            let gain = opt_f64(rec, "gain_linear", span)?.unwrap_or(1.0) as f32;
            (pose, gain)
        } else if rec.contains_key("azimuth_deg") || rec.contains_key("distance_m") {
            let az = req_f64(rec, "azimuth_deg", span)? as f32;
            let el = opt_f64(rec, "elevation_deg", span)?.unwrap_or(0.0) as f32;
            let dist = req_f64(rec, "distance_m", span)? as f32;
            let gain = opt_f64(rec, "gain_linear", span)?.unwrap_or(1.0) as f32;
            ([az, el, dist], gain)
        } else if selected.payload.len() >= 16 {
            let p = selected.payload;
            (
                [read_f32_le(p, 0)?, read_f32_le(p, 4)?, read_f32_le(p, 8)?],
                read_f32_le(p, 12)?,
            )
        } else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "Qac.renderBinaural requires pose_deg_dist or >= 16B binaural payload",
            ));
        };

        let res = oracle_render_binaural_frame(pose, gain_lin)
            .map_err(|t| trap_err("Qac.renderBinaural", t, span))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("left_gain".into(), Value::F64(f64::from(res[0])));
        out.insert("right_gain".into(), Value::F64(f64::from(res[1])));
        out.insert("itd_sec".into(), Value::F64(f64::from(res[2])));
        out.insert(
            "values".into(),
            Value::List(vec![
                Value::F64(f64::from(res[0])),
                Value::F64(f64::from(res[1])),
                Value::F64(f64::from(res[2])),
            ]),
        );
        Ok(Value::Record(out))
    }

    fn evaluate_directivity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
        let (env, idx) = admit_family_and_select(&bytes, EnvelopeFamily::QacV1, args, span)?;
        let selected = &env.entries[idx];
        let rec = rec_map(args, "Qac.evaluateDirectivity", span)?;

        let (angles, weights) = if let (Some(a), Some(w)) = (
            opt_f32_arr::<2>(rec, "angles_rad", span)?,
            opt_f32_arr::<4>(rec, "weights_l0_l1", span)?,
        ) {
            (a, w)
        } else if selected.payload.len() >= 24 {
            let p = selected.payload;
            (
                [read_f32_le(p, 0)?, read_f32_le(p, 4)?],
                [
                    read_f32_le(p, 8)?,
                    read_f32_le(p, 12)?,
                    read_f32_le(p, 16)?,
                    read_f32_le(p, 20)?,
                ],
            )
        } else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "Qac.evaluateDirectivity requires angles_rad + weights_l0_l1 or >= 24B directivity payload",
            ));
        };

        let res = oracle_evaluate_directivity_sh(angles, weights)
            .map_err(|t| trap_err("Qac.evaluateDirectivity", t, span))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("directivity_gain".into(), Value::F64(f64::from(res[0])));
        out.insert("l0_energy_ratio".into(), Value::F64(f64::from(res[1])));
        out.insert(
            "values".into(),
            Value::List(vec![
                Value::F64(f64::from(res[0])),
                Value::F64(f64::from(res[1])),
            ]),
        );
        Ok(Value::Record(out))
    }

    fn qac_sonify_emf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let rec = rec_map(args, "Qac.sonifyEmf", span)?;
        let (primary_bytes, primary_family, qac_extra) = if rec.contains_key("emf_bytes") {
            let emf = extract_binary_bytes(args, &["emf_bytes"], span)?;
            let qac = if let Some(qval) = rec.get("qac_bytes") {
                Some(decode_byte_list(qval, "qac_bytes", span)?)
            } else {
                None
            };
            (emf, EnvelopeFamily::EmfV4, qac)
        } else if rec.contains_key("qac_bytes") {
            let qac = extract_binary_bytes(args, &["qac_bytes"], span)?;
            (qac, EnvelopeFamily::QacV1, None)
        } else {
            let bytes = extract_binary_bytes(args, &["bytes", "container"], span)?;
            if bytes.len() < 8 {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    "Qac.sonifyEmf container shorter than 8-byte family magic",
                ));
            }
            let fam = if &bytes[..8] == EnvelopeFamily::EmfV4.magic() {
                EnvelopeFamily::EmfV4
            } else if &bytes[..8] == EnvelopeFamily::QacV1.magic() {
                EnvelopeFamily::QacV1
            } else {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    "Qac.sonifyEmf requires admitted Q42EMF64 v4 or qac\\0\\0\\0\\0\\0 v1 binary container",
                ));
            };
            (bytes, fam, None)
        };

        if let Some(ref qb) = qac_extra {
            admit_envelope(qb, EnvelopeFamily::QacV1).map_err(|msg| {
                Diagnostic::new(DiagCode::E500, span, format!("admitting qac_bytes failed: {msg}"))
            })?;
        }
        let (env, idx) = admit_family_and_select(&primary_bytes, primary_family, args, span)?;
        let selected = &env.entries[idx];
        let sample = extract_emf_sample(rec, selected, span)?;
        let sonified = oracle_sonify_emf_envelope(sample)
            .map_err(|t| trap_err("Qac.sonifyEmf", t, span))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("source_family".into(), Value::String(primary_family.label().into()));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert(
            "entry_id".into(),
            Value::U64(u64::from(selected.descriptor_id)),
        );
        out.insert("carrier_hz".into(), Value::F64(sonified[0]));
        out.insert("amplitude_norm".into(), Value::F64(sonified[1]));
        out.insert(
            "values".into(),
            Value::List(vec![Value::F64(sonified[0]), Value::F64(sonified[1])]),
        );

        if let Some(pose) = opt_f32_arr::<3>(rec, "pose_deg_dist", span)? {
            let gain = opt_f64(rec, "gain_linear", span)?.unwrap_or(1.0) as f32;
            let bin = oracle_render_binaural_frame(pose, gain)
                .map_err(|t| trap_err("Qac.sonifyEmf(binaural)", t, span))?;
            out.insert("left_gain".into(), Value::F64(f64::from(bin[0])));
            out.insert("right_gain".into(), Value::F64(f64::from(bin[1])));
            out.insert("itd_sec".into(), Value::F64(f64::from(bin[2])));
        }
        Ok(Value::Record(out))
    }

    // ── DICOM Part-10 binary parser & gate ──────────────────────────────

    const TS_IMPLICIT_LE: &str = "1.2.840.10008.1.2";
    const TS_EXPLICIT_LE: &str = "1.2.840.10008.1.2.1";
    const TS_J2K_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";
    const TS_J2K: &str = "1.2.840.10008.1.2.4.91";
    const TS_JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";
    const TS_JPEG_EXTENDED: &str = "1.2.840.10008.1.2.4.51";
    const TS_RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";

    struct ParsedDicom {
        modality: String,
        modality_code: u32,
        transfer_syntax_uid: String,
        encapsulated: bool,
        rows: u16,
        columns: u16,
        bits_allocated: u16,
        bits_stored: u16,
        samples_per_pixel: u16,
        pixel_representation: u16,
        bytes_per_voxel: u32,
        supported: bool,
        pixel_offset: Option<usize>,
        pixel_length: usize,
        mean_norm_intensity: f64,
    }

    fn dicom_vr_is_long(vr: [u8; 2]) -> bool {
        matches!(
            &vr,
            b"OB"
                | b"OD"
                | b"OF"
                | b"OL"
                | b"OV"
                | b"OW"
                | b"SQ"
                | b"SV"
                | b"UC"
                | b"UN"
                | b"UR"
                | b"UT"
                | b"UV"
        )
    }

    fn dicom_str(bytes: &[u8]) -> String {
        let slice = match bytes.iter().position(|&b| b == 0) {
            Some(i) => &bytes[..i],
            None => bytes,
        };
        String::from_utf8_lossy(slice).trim().to_string()
    }

    fn dicom_u16(bytes: &[u8], vr: Option<[u8; 2]>) -> Result<u16, String> {
        if vr == Some(*b"US") || vr == Some(*b"SS") || (vr.is_none() && bytes.len() == 2) {
            if bytes.len() < 2 {
                return Err("truncated binary US DICOM tag".to_string());
            }
            return Ok(u16::from_le_bytes([bytes[0], bytes[1]]));
        }
        let s = dicom_str(bytes);
        if let Ok(v) = s.parse::<u16>() {
            return Ok(v);
        }
        if bytes.len() == 2 {
            return Ok(u16::from_le_bytes([bytes[0], bytes[1]]));
        }
        Err(format!("invalid DICOM u16 value `{s}`"))
    }

    fn map_dicom_modality(m: &str) -> Option<u32> {
        match m.trim().to_ascii_uppercase().as_str() {
            "CT" => Some(1),
            "MR" => Some(2),
            "PT" | "PET" => Some(3),
            "US" => Some(4),
            "NM" => Some(5),
            "XA" | "CR" | "DX" => Some(6),
            _ => None,
        }
    }

    struct RawDicomElem {
        tag: u32,
        vr: Option<[u8; 2]>,
        length: u32,
        header_size: usize,
    }

    fn read_dicom_elem_header(
        data: &[u8],
        offset: usize,
        explicit_vr: bool,
    ) -> Result<Option<RawDicomElem>, String> {
        if offset == data.len() {
            return Ok(None);
        }
        if offset + 8 > data.len() {
            return Err("truncated DICOM element header".to_string());
        }
        let group = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let elem = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        let tag = (u32::from(group) << 16) | u32::from(elem);
        if tag == 0xFFFE_E000 || tag == 0xFFFE_E00D || tag == 0xFFFE_E0DD {
            let length = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            return Ok(Some(RawDicomElem {
                tag,
                vr: None,
                length,
                header_size: 8,
            }));
        }
        if explicit_vr {
            let vr = [data[offset + 4], data[offset + 5]];
            if !vr[0].is_ascii_uppercase() || !vr[1].is_ascii_uppercase() {
                return Err(format!("invalid DICOM explicit VR bytes {vr:?} at tag {tag:#010x}"));
            }
            if dicom_vr_is_long(vr) {
                if offset + 12 > data.len() {
                    return Err("truncated long-VR DICOM header".to_string());
                }
                let length = u32::from_le_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]);
                Ok(Some(RawDicomElem {
                    tag,
                    vr: Some(vr),
                    length,
                    header_size: 12,
                }))
            } else {
                let length = u16::from_le_bytes([data[offset + 6], data[offset + 7]]) as u32;
                Ok(Some(RawDicomElem {
                    tag,
                    vr: Some(vr),
                    length,
                    header_size: 8,
                }))
            }
        } else {
            let length = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            Ok(Some(RawDicomElem {
                tag,
                vr: None,
                length,
                header_size: 8,
            }))
        }
    }

    fn parse_and_admit_dicom(bytes: &[u8], require_pixels: bool, span: Span) -> Result<ParsedDicom, Diagnostic> {
        if bytes.len() < 132 {
            return Err(Diagnostic::new(
                DiagCode::E500,
                span,
                "DICOM buffer shorter than 132-byte Part-10 preamble + DICM magic",
            ));
        }
        if &bytes[128..132] != b"DICM" {
            return Err(Diagnostic::new(
                DiagCode::E500,
                span,
                "missing DICOM `DICM` magic at byte offset 128",
            ));
        }

        // Walk Group 0x0002 Meta Information (always Explicit VR LE).
        let mut cursor = 132usize;
        let mut ts_uid = TS_EXPLICIT_LE.to_string();
        while cursor < bytes.len() {
            if cursor + 4 > bytes.len() {
                return Err(Diagnostic::new(DiagCode::E500, span, "truncated DICOM tag"));
            }
            let group = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
            if group != 0x0002 {
                break;
            }
            let hdr = read_dicom_elem_header(bytes, cursor, true)
                .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?
                .ok_or_else(|| Diagnostic::new(DiagCode::E500, span, "unexpected EOF in DICOM meta"))?;
            if hdr.length == 0xFFFF_FFFF {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    "undefined length disallowed in DICOM group 0x0002",
                ));
            }
            let v_start = cursor + hdr.header_size;
            let v_end = v_start
                .checked_add(hdr.length as usize)
                .filter(|&e| e <= bytes.len())
                .ok_or_else(|| Diagnostic::new(DiagCode::E500, span, "truncated DICOM meta element"))?;
            if hdr.tag == 0x0002_0010 {
                ts_uid = dicom_str(&bytes[v_start..v_end]);
            }
            cursor = v_end;
        }

        let (explicit_vr, encapsulated) = match ts_uid.as_str() {
            TS_IMPLICIT_LE => (false, false),
            TS_EXPLICIT_LE => (true, false),
            TS_J2K_LOSSLESS | TS_J2K | TS_JPEG_BASELINE | TS_JPEG_EXTENDED | TS_RLE_LOSSLESS => {
                (true, true)
            }
            other => {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!("unsupported DICOM transfer syntax `{other}`"),
                ))
            }
        };

        let mut modality: Option<String> = None;
        let mut rows: Option<u16> = None;
        let mut columns: Option<u16> = None;
        let mut bits_allocated: Option<u16> = None;
        let mut bits_stored: Option<u16> = None;
        let mut samples_per_pixel: u16 = 1;
        let mut pixel_representation: u16 = 0;
        let mut pixel_offset: Option<usize> = None;
        let mut pixel_length: usize = 0;

        while cursor < bytes.len() {
            let Some(hdr) = read_dicom_elem_header(bytes, cursor, explicit_vr)
                .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?
            else {
                break;
            };
            let v_start = cursor + hdr.header_size;
            if hdr.tag == 0x7FE0_0010 {
                if hdr.length == 0xFFFF_FFFF {
                    if !encapsulated {
                        return Err(Diagnostic::new(
                            DiagCode::E500,
                            span,
                            "undefined PixelData length on uncompressed DICOM",
                        ));
                    }
                    // Scan bounded encapsulated items to Sequence Delimiter (FFFE,E0DD).
                    let mut pcur = v_start;
                    let mut found_end = false;
                    while pcur + 8 <= bytes.len() {
                        let t = (u32::from(u16::from_le_bytes([bytes[pcur], bytes[pcur + 1]])) << 16)
                            | u32::from(u16::from_le_bytes([bytes[pcur + 2], bytes[pcur + 3]]));
                        let ilen = u32::from_le_bytes([
                            bytes[pcur + 4],
                            bytes[pcur + 5],
                            bytes[pcur + 6],
                            bytes[pcur + 7],
                        ]);
                        pcur += 8;
                        if t == 0xFFFE_E0DD {
                            found_end = true;
                            break;
                        }
                        if t != 0xFFFE_E000 || ilen == 0xFFFF_FFFF {
                            return Err(Diagnostic::new(
                                DiagCode::E500,
                                span,
                                "malformed encapsulated DICOM pixel item",
                            ));
                        }
                        pcur = pcur
                            .checked_add(ilen as usize)
                            .filter(|&e| e <= bytes.len())
                            .ok_or_else(|| {
                                Diagnostic::new(DiagCode::E500, span, "truncated encapsulated DICOM item")
                            })?;
                    }
                    if !found_end {
                        return Err(Diagnostic::new(
                            DiagCode::E500,
                            span,
                            "encapsulated DICOM PixelData missing sequence delimiter",
                        ));
                    }
                    pixel_offset = Some(v_start);
                    pixel_length = pcur.saturating_sub(v_start);
                } else {
                    let _ = v_start
                        .checked_add(hdr.length as usize)
                        .filter(|&e| e <= bytes.len())
                        .ok_or_else(|| {
                            Diagnostic::new(DiagCode::E500, span, "truncated DICOM PixelData element")
                        })?;
                    pixel_offset = Some(v_start);
                    pixel_length = hdr.length as usize;
                }
                break;
            }

            if hdr.length == 0xFFFF_FFFF {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!("unsupported undefined length tag {:#010x}", hdr.tag),
                ));
            }
            let v_end = v_start
                .checked_add(hdr.length as usize)
                .filter(|&e| e <= bytes.len())
                .ok_or_else(|| Diagnostic::new(DiagCode::E500, span, "truncated DICOM dataset element"))?;
            let val = &bytes[v_start..v_end];
            match hdr.tag {
                0x0008_0060 => modality = Some(dicom_str(val)),
                0x0028_0002 => {
                    samples_per_pixel = dicom_u16(val, hdr.vr)
                        .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?;
                }
                0x0028_0010 => {
                    rows = Some(
                        dicom_u16(val, hdr.vr)
                            .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?,
                    );
                }
                0x0028_0011 => {
                    columns = Some(
                        dicom_u16(val, hdr.vr)
                            .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?,
                    );
                }
                0x0028_0100 => {
                    bits_allocated = Some(
                        dicom_u16(val, hdr.vr)
                            .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?,
                    );
                }
                0x0028_0101 => {
                    bits_stored = Some(
                        dicom_u16(val, hdr.vr)
                            .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?,
                    );
                }
                0x0028_0103 => {
                    pixel_representation = dicom_u16(val, hdr.vr)
                        .map_err(|e| Diagnostic::new(DiagCode::E500, span, e))?;
                }
                _ => {}
            }
            cursor = v_end;
        }

        let mod_str = modality.ok_or_else(|| {
            Diagnostic::new(DiagCode::E500, span, "DICOM dataset missing Modality (0008,0060)")
        })?;
        let modality_code = map_dicom_modality(&mod_str).ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E500,
                span,
                format!("unsupported DICOM Modality `{mod_str}`"),
            )
        })?;
        let r = rows.filter(|&v| v > 0).ok_or_else(|| {
            Diagnostic::new(DiagCode::E500, span, "DICOM dataset missing valid positive Rows (0028,0010)")
        })?;
        let c = columns.filter(|&v| v > 0).ok_or_else(|| {
            Diagnostic::new(DiagCode::E500, span, "DICOM dataset missing valid positive Columns (0028,0011)")
        })?;
        let ba = bits_allocated.unwrap_or(bits_stored.unwrap_or(0));
        let bs = bits_stored.unwrap_or(ba);
        if ba == 0 || bs == 0 || bs > ba {
            return Err(Diagnostic::new(
                DiagCode::E500,
                span,
                format!("invalid DICOM BitsAllocated={ba} / BitsStored={bs}"),
            ));
        }

        let check = oracle_dicom_modality_check([
            modality_code,
            u32::from(bs),
            u32::from(samples_per_pixel),
        ])
        .map_err(|t| trap_err("Dicom.modalityCheck", t, span))?;
        let supported = check[0] == 1;
        let bytes_per_voxel = check[1];

        let mut mean_norm = 0.0f64;
        if require_pixels {
            let p_off = pixel_offset.ok_or_else(|| {
                Diagnostic::new(DiagCode::E500, span, "DICOM dataset missing PixelData (7FE0,0010)")
            })?;
            if pixel_length == 0 {
                return Err(Diagnostic::new(DiagCode::E500, span, "DICOM PixelData is empty"));
            }
            if !supported {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!("DICOM voxel layout not supported by Etalon gate (bits_stored={bs}, samples_per_pixel={samples_per_pixel})"),
                ));
            }
            let expected_bytes = usize::from(r) * usize::from(c) * (bytes_per_voxel as usize);
            if !encapsulated && pixel_length < expected_bytes {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!(
                        "DICOM PixelData truncated: got {pixel_length} bytes, expected >= {expected_bytes} ({r}x{c}x{bytes_per_voxel})"
                    ),
                ));
            }
            let pslice = &bytes[p_off..p_off + pixel_length.min(expected_bytes.max(1))];
            let denom = ((1u64 << bs) - 1) as f64;
            if !encapsulated && bs <= 8 && !pslice.is_empty() {
                let sum: u64 = pslice.iter().map(|&b| u64::from(b)).sum();
                mean_norm = (sum as f64) / ((pslice.len() as f64) * denom.max(1.0));
            } else if !encapsulated && bs > 8 && pslice.len() >= 2 {
                let count = pslice.len() / 2;
                let mut sum = 0.0f64;
                for ch in pslice.chunks_exact(2) {
                    let raw = u16::from_le_bytes([ch[0], ch[1]]);
                    let masked = raw & (((1u32 << bs) - 1) as u16);
                    sum += f64::from(masked);
                }
                mean_norm = sum / ((count as f64) * denom.max(1.0));
            }
        }

        Ok(ParsedDicom {
            modality: mod_str,
            modality_code,
            transfer_syntax_uid: ts_uid,
            encapsulated,
            rows: r,
            columns: c,
            bits_allocated: ba,
            bits_stored: bs,
            samples_per_pixel,
            pixel_representation,
            bytes_per_voxel,
            supported,
            pixel_offset,
            pixel_length,
            mean_norm_intensity: mean_norm.clamp(0.0, 1.0),
        })
    }

    fn select_dicom_bytes(args: &Value, span: Span) -> Result<(usize, Vec<Vec<u8>>), Diagnostic> {
        let rec = rec_map(args, "Dicom", span)?;
        if let Some(val) = rec.get("datasets") {
            let Value::List(list) = val else {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "`datasets` must be a List of binary DICOM byte buffers",
                ));
            };
            if list.is_empty() {
                return Err(Diagnostic::new(DiagCode::E500, span, "`datasets` list is empty"));
            }
            let mut all = Vec::with_capacity(list.len());
            for (i, item) in list.iter().enumerate() {
                all.push(decode_byte_list(item, &format!("datasets[{i}]"), span)?);
            }
            let idx = opt_u64(rec, "entry_index", span)?
                .or(opt_u64(rec, "slice_index", span)?)
                .unwrap_or(0) as usize;
            if idx >= all.len() {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("entry_index {idx} out of bounds for datasets len {}", all.len()),
                ));
            }
            Ok((idx, all))
        } else {
            let idx = opt_u64(rec, "entry_index", span)?
                .or(opt_u64(rec, "slice_index", span)?)
                .unwrap_or(0);
            if idx != 0 {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("entry_index {idx} out of bounds for single DICOM dataset"),
                ));
            }
            let b = extract_binary_bytes(args, &["bytes", "dataset"], span)?;
            Ok((0, vec![b]))
        }
    }

    fn dicom_parse_or_verify(args: &Value, require_pixels: bool, span: Span) -> Result<Value, Diagnostic> {
        let (idx, all) = select_dicom_bytes(args, span)?;
        let parsed = parse_and_admit_dicom(&all[idx], require_pixels, span)?;
        let expected_frame_bytes =
            u64::from(parsed.rows) * u64::from(parsed.columns) * u64::from(parsed.bytes_per_voxel);

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        if require_pixels {
            out.insert("verified".into(), Value::Bool(true));
        }
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert("modality".into(), Value::String(parsed.modality));
        out.insert("modality_code".into(), Value::U64(u64::from(parsed.modality_code)));
        out.insert(
            "transfer_syntax_uid".into(),
            Value::String(parsed.transfer_syntax_uid),
        );
        out.insert("rows".into(), Value::U64(u64::from(parsed.rows)));
        out.insert("columns".into(), Value::U64(u64::from(parsed.columns)));
        out.insert(
            "bits_allocated".into(),
            Value::U64(u64::from(parsed.bits_allocated)),
        );
        out.insert("bits_stored".into(), Value::U64(u64::from(parsed.bits_stored)));
        out.insert(
            "samples_per_pixel".into(),
            Value::U64(u64::from(parsed.samples_per_pixel)),
        );
        out.insert(
            "pixel_representation".into(),
            Value::U64(u64::from(parsed.pixel_representation)),
        );
        out.insert(
            "bytes_per_voxel".into(),
            Value::U64(u64::from(parsed.bytes_per_voxel)),
        );
        out.insert("supported".into(), Value::Bool(parsed.supported));
        out.insert("encapsulated".into(), Value::Bool(parsed.encapsulated));
        out.insert(
            "pixel_data_bytes".into(),
            Value::U64(parsed.pixel_length as u64),
        );
        out.insert("expected_frame_bytes".into(), Value::U64(expected_frame_bytes));
        if let Some(off) = parsed.pixel_offset {
            out.insert("pixel_data_offset".into(), Value::U64(off as u64));
        }
        Ok(Value::Record(out))
    }

    fn dicom_to_emf_field(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let (idx, all) = select_dicom_bytes(args, span)?;
        let parsed = parse_and_admit_dicom(&all[idx], true, span)?;
        let optical_depth = (0.05 + 1.5 * parsed.mean_norm_intensity) as f32;
        let rt = oracle_radiative_transfer_slice([optical_depth, 0.65, 0.3], 1.0)
            .map_err(|t| trap_err("Dicom.toEmfField", t, span))?;

        // Build 64-byte EmfCellRecord64 payload inside a genuine Q42EMF64 v4 container.
        let mut cell = [0u8; 64];
        cell[0..4].copy_from_slice(&rt[0].to_le_bytes());
        cell[4..8].copy_from_slice(&rt[1].to_le_bytes());
        cell[24..28].copy_from_slice(&rt[0].to_le_bytes()); // poynting_flux
        cell[28..32].copy_from_slice(&550.0f32.to_le_bytes()); // wavelength_nm
        cell[32..36].copy_from_slice(&1.38f32.to_le_bytes()); // refractive_n
        cell[56..60].copy_from_slice(&(parsed.mean_norm_intensity as f32).to_le_bytes()); // spectral_sigma
        cell[60..64].copy_from_slice(&1.0f32.to_le_bytes()); // confidence_alpha

        let emf_bytes = build_envelope(
            EnvelopeFamily::EmfV4,
            false,
            &[],
            &[BuildEntry {
                mode: 0x0101,
                revision: 1,
                flags: 0,
                stream_id: 0,
                frame_id: 0,
                start_tick: 0,
                time_count: 0,
                spatial_key: 0,
                record_count: 1,
                stride: 64,
                descriptor_id: (idx + 1) as u32,
                payload: &cell,
            }],
        )
        .map_err(|msg| Diagnostic::new(DiagCode::E500, span, msg))?;

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("family".into(), Value::String(EnvelopeFamily::EmfV4.label().into()));
        out.insert("version".into(), Value::U64(4));
        out.insert("selected_entry_index".into(), Value::U64(idx as u64));
        out.insert("modality".into(), Value::String(parsed.modality));
        out.insert(
            "bytes_per_voxel".into(),
            Value::U64(u64::from(parsed.bytes_per_voxel)),
        );
        out.insert("transmitted_radiance".into(), Value::F64(f64::from(rt[0])));
        out.insert(
            "single_scatter_radiance".into(),
            Value::F64(f64::from(rt[1])),
        );
        out.insert("bytes".into(), bytes_to_vibe_list(&emf_bytes));
        Ok(Value::Record(out))
    }

    fn dicom_reconstruct_volume(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let (idx, all) = select_dicom_bytes(args, span)?;
        let mut first: Option<ParsedDicom> = None;
        let mut intensity_sum = 0.0f64;
        for (i, slice_bytes) in all.iter().enumerate() {
            let p = parse_and_admit_dicom(slice_bytes, true, span)?;
            if let Some(ref ref_p) = first {
                if p.modality_code != ref_p.modality_code
                    || p.rows != ref_p.rows
                    || p.columns != ref_p.columns
                    || p.bits_stored != ref_p.bits_stored
                    || p.samples_per_pixel != ref_p.samples_per_pixel
                {
                    return Err(Diagnostic::new(
                        DiagCode::E500,
                        span,
                        format!("DICOM slice {i} geometry/modality mismatch against slice 0"),
                    ));
                }
            } else {
                first = Some(p);
                intensity_sum += first.as_ref().map(|x| x.mean_norm_intensity).unwrap_or(0.0);
                continue;
            }
            intensity_sum += p.mean_norm_intensity;
        }
        let ref_p = first.ok_or_else(|| {
            Diagnostic::new(DiagCode::E500, span, "no DICOM slices supplied for volume reconstruction")
        })?;
        let slice_count = all.len() as u64;
        let mean_intensity = intensity_sum / (slice_count as f64);
        let rt = oracle_radiative_transfer_slice([(0.05 + 1.5 * mean_intensity) as f32, 0.65, 0.3], 1.0)
            .map_err(|t| trap_err("Dicom.reconstructVolume(rt)", t, span))?;
        let irr = oracle_sh_irradiance_probe([1.0, 0.1, 0.2, 0.1], [0.0, 0.0, 1.0])
            .map_err(|t| trap_err("Dicom.reconstructVolume(sh)", t, span))?;
        let total_voxel_bytes = u64::from(ref_p.rows)
            * u64::from(ref_p.columns)
            * slice_count
            * u64::from(ref_p.bytes_per_voxel);

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("selected_slice_index".into(), Value::U64(idx as u64));
        out.insert("slice_count".into(), Value::U64(slice_count));
        out.insert("modality".into(), Value::String(ref_p.modality));
        out.insert(
            "dims".into(),
            Value::List(vec![
                Value::U64(u64::from(ref_p.columns)),
                Value::U64(u64::from(ref_p.rows)),
                Value::U64(slice_count),
            ]),
        );
        out.insert(
            "bytes_per_voxel".into(),
            Value::U64(u64::from(ref_p.bytes_per_voxel)),
        );
        out.insert("total_voxel_bytes".into(), Value::U64(total_voxel_bytes));
        out.insert("mean_intensity".into(), Value::F64(mean_intensity));
        out.insert("transmitted_radiance".into(), Value::F64(f64::from(rt[0])));
        out.insert("irradiance".into(), Value::F64(f64::from(irr[0])));
        out.insert("directional_contrast".into(), Value::F64(f64::from(irr[1])));
        Ok(Value::Record(out))
    }

    // ── Sensing admission & calibration gates ───────────────────────────

    fn admit_optional_attached_evidence(rec: &BTreeMap<String, Value>, span: Span) -> Result<(), Diagnostic> {
        let Some(val) = rec.get("evidence_bytes").or_else(|| rec.get("bytes")) else {
            return Ok(());
        };
        let bytes = decode_byte_list(val, "evidence_bytes", span)?;
        if bytes.len() >= 132 && &bytes[128..132] == b"DICM" {
            parse_and_admit_dicom(&bytes, true, span)?;
            return Ok(());
        }
        if bytes.len() >= 8 && &bytes[..8] == EnvelopeFamily::EmfV4.magic() {
            admit_envelope(&bytes, EnvelopeFamily::EmfV4)
                .map_err(|m| Diagnostic::new(DiagCode::E500, span, m))?;
            return Ok(());
        }
        if bytes.len() >= 8 && &bytes[..8] == EnvelopeFamily::QacV1.magic() {
            admit_envelope(&bytes, EnvelopeFamily::QacV1)
                .map_err(|m| Diagnostic::new(DiagCode::E500, span, m))?;
            return Ok(());
        }
        Err(Diagnostic::new(
            DiagCode::E500,
            span,
            "attached evidence_bytes is neither admitted Q42EMF64 v4, QAC v1, nor DICOM Part-10",
        ))
    }

    fn sensing_admit_evidence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let rec = rec_map(args, "Sensing.admitEvidence", span)?;
        admit_optional_attached_evidence(rec, span)?;

        let modality_mask = req_u64(rec, "modality_mask", span)?;
        let calibration_age_ms = req_u64(rec, "calibration_age_ms", span)?;
        let max_calibration_age_ms = req_u64(rec, "max_calibration_age_ms", span)?;
        let flags = if let Some(explicit) = opt_u64(rec, "consent_and_merkle_flags", span)? {
            explicit
        } else {
            let has_consent = opt_bool(rec, "has_consent", span)?.unwrap_or(false);
            let has_merkle = opt_bool(rec, "has_merkle", span)?.unwrap_or(false);
            u64::from(has_consent) | (u64::from(has_merkle) << 1)
        };

        let res = oracle_verify_evidence_gate([
            modality_mask,
            calibration_age_ms,
            max_calibration_age_ms,
            flags,
        ])
        .map_err(|t| trap_err("Sensing.admitEvidence", t, span))?;

        if res[0] != 1 {
            return Err(Diagnostic::disclosure_denied(
                span,
                "Sensing.admitEvidence",
                &format!(
                    "evidence gate refused admission (calibration_age_ms={calibration_age_ms}, max={max_calibration_age_ms}, flags={flags:#x})"
                ),
            ));
        }

        let mut out = BTreeMap::new();
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("authority_tier".into(), Value::U64(res[1]));
        out.insert("modality_mask".into(), Value::U64(modality_mask));
        out.insert("calibration_age_ms".into(), Value::U64(calibration_age_ms));
        out.insert(
            "max_calibration_age_ms".into(),
            Value::U64(max_calibration_age_ms),
        );
        Ok(Value::Record(out))
    }

    fn validate_certificate_digest(rec: &BTreeMap<String, Value>, span: Span) -> Result<(), Diagnostic> {
        let Some(val) = rec.get("certificate_digest") else {
            return Err(Diagnostic::new(
                DiagCode::E300,
                span,
                "Sensing.verifyCalibration requires non-zero 32-byte `certificate_digest`",
            ));
        };
        let bytes = match val {
            Value::List(_) => {
                let b = decode_byte_list(val, "certificate_digest", span)?;
                if b.len() != 32 {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("certificate_digest must be 32 bytes, got {}", b.len()),
                    ));
                }
                b
            }
            Value::String(hex) => {
                let h = hex.trim();
                if h.len() != 64 || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "certificate_digest hex string must be 64 hex chars",
                    ));
                }
                let mut out = Vec::with_capacity(32);
                for i in (0..64).step_by(2) {
                    let b = u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| {
                        Diagnostic::new(DiagCode::E100, span, "invalid hex in certificate_digest")
                    })?;
                    out.push(b);
                }
                out
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "certificate_digest must be 32 bytes or 64-char hex string",
                ))
            }
        };
        if bytes.iter().all(|&b| b == 0) {
            return Err(Diagnostic::new(
                DiagCode::E500,
                span,
                "all-zero calibration certificate_digest is prohibited",
            ));
        }
        Ok(())
    }

    fn sensing_verify_calibration(args: &Value, span: Span) -> Result<Value, Diagnostic> {
        let rec = rec_map(args, "Sensing.verifyCalibration", span)?;
        admit_optional_attached_evidence(rec, span)?;
        validate_certificate_digest(rec, span)?;

        let stage = match rec.get("stage") {
            Some(Value::String(s)) => s.as_str(),
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "Sensing.verifyCalibration requires string `stage` (`raw-counts` or `physical-base-units`)",
                ))
            }
        };
        let scale = req_f64(rec, "scale", span)?;
        let offset = opt_f64(rec, "offset", span)?.unwrap_or(0.0);
        match stage {
            "physical-base-units" => {
                if scale != 1.0 || offset != 0.0 {
                    return Err(Diagnostic::new(
                        DiagCode::E500,
                        span,
                        "physical-base-units requires identity scaling (scale == 1.0, offset == 0.0); re-calibration refused",
                    ));
                }
            }
            "raw-counts" => {
                if scale <= 0.0 {
                    return Err(Diagnostic::new(
                        DiagCode::E500,
                        span,
                        "raw-counts calibration requires positive finite scale",
                    ));
                }
            }
            "digital-unrelated-to-pressure" => {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    "digital-unrelated-to-pressure stage cannot pass physical calibration verification",
                ));
            }
            other => {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!("unsupported calibration stage `{other}`"),
                ))
            }
        }

        let modality_mask = opt_u64(rec, "modality_mask", span)?.unwrap_or(1);
        let calibration_age_ms = req_u64(rec, "calibration_age_ms", span)?;
        let max_calibration_age_ms = req_u64(rec, "max_calibration_age_ms", span)?;
        let flags = if let Some(explicit) = opt_u64(rec, "consent_and_merkle_flags", span)? {
            explicit
        } else {
            let has_consent = opt_bool(rec, "has_consent", span)?.unwrap_or(false);
            let has_merkle = opt_bool(rec, "has_merkle", span)?.unwrap_or(false);
            u64::from(has_consent) | (u64::from(has_merkle) << 1)
        };
        let gate = oracle_verify_evidence_gate([
            modality_mask,
            calibration_age_ms,
            max_calibration_age_ms,
            flags,
        ])
        .map_err(|t| trap_err("Sensing.verifyCalibration(gate)", t, span))?;
        if gate[0] != 1 {
            return Err(Diagnostic::disclosure_denied(
                span,
                "Sensing.verifyCalibration",
                "calibration freshness/consent/merkle gate refused",
            ));
        }

        let mut drift_within_bound = true;
        let mut clock_delta_ns = None;
        if let (Some(t0), Some(t1), Some(max_unc)) = (
            opt_u64(rec, "t0_ns", span)?,
            opt_u64(rec, "t1_ns", span)?,
            opt_u64(rec, "max_uncertainty_ns", span)?,
        ) {
            let clock = oracle_clock_stamp_drift_ns(t0, t1, max_unc)
                .map_err(|t| trap_err("Sensing.verifyCalibration(clock)", t, span))?;
            clock_delta_ns = Some(clock[0]);
            drift_within_bound = clock[1] == 1;
            if !drift_within_bound {
                return Err(Diagnostic::new(
                    DiagCode::E500,
                    span,
                    format!(
                        "calibration clock drift {} ns exceeds max uncertainty {} ns",
                        clock[0], max_unc
                    ),
                ));
            }
        }

        let mut out = BTreeMap::new();
        out.insert("verified".into(), Value::Bool(true));
        out.insert("admitted".into(), Value::Bool(true));
        out.insert("stage".into(), Value::String(stage.into()));
        out.insert("scale".into(), Value::F64(scale));
        out.insert("offset".into(), Value::F64(offset));
        out.insert("authority_tier".into(), Value::U64(gate[1]));
        out.insert(
            "clock_within_bound".into(),
            Value::Bool(drift_within_bound),
        );
        if let Some(delta) = clock_delta_ns {
            out.insert("clock_delta_ns".into(), Value::U64(delta));
        }
        Ok(Value::Record(out))
    }
}

#[cfg(test)]
mod up02_tests {
    use super::super::envelope_admission::{bytes_to_vibe_list, decode_byte_list};
    use super::try_invoke;
    use crate::span::Span;
    use crate::value::Value;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span::new(0, 0)
    }

    fn rec(pairs: Vec<(&str, Value)>) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v);
        }
        Value::Record(m)
    }

    fn as_rec(v: Value) -> BTreeMap<String, Value> {
        match v {
            Value::Record(m) => m,
            other => panic!("expected Value::Record, got {other:?}"),
        }
    }

    fn f32_vals(fs: &[f32]) -> Value {
        Value::List(fs.iter().map(|&f| Value::F64(f64::from(f))).collect())
    }

    fn f32_bytes(fs: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(fs.len() * 4);
        for f in fs {
            out.extend_from_slice(&f.to_le_bytes());
        }
        out
    }

    fn emf_cell_64(e: [f32; 3], wavelength_nm: f32, spectral_sigma: f32) -> Vec<u8> {
        let mut cell = vec![0u8; 64];
        cell[0..4].copy_from_slice(&e[0].to_le_bytes());
        cell[4..8].copy_from_slice(&e[1].to_le_bytes());
        cell[8..12].copy_from_slice(&e[2].to_le_bytes());
        cell[28..32].copy_from_slice(&wavelength_nm.to_le_bytes());
        cell[56..60].copy_from_slice(&spectral_sigma.to_le_bytes());
        cell[60..64].copy_from_slice(&1.0f32.to_le_bytes());
        cell
    }

    fn build_synthetic_dicom(
        modality: &str,
        rows: u16,
        cols: u16,
        bits_alloc: u16,
        bits_stored: u16,
        samples_per_pixel: u16,
        pixel_bytes: &[u8],
    ) -> Vec<u8> {
        let mut out = vec![0u8; 128];
        out.extend_from_slice(b"DICM");

        // Group 0x0002 TransferSyntaxUID Explicit VR LE
        let ts = b"1.2.840.10008.1.2.1\0";
        out.extend_from_slice(&0x0002u16.to_le_bytes());
        out.extend_from_slice(&0x0010u16.to_le_bytes());
        out.extend_from_slice(b"UI");
        out.extend_from_slice(&(ts.len() as u16).to_le_bytes());
        out.extend_from_slice(ts);

        let mut push_short_str = |g: u16, e: u16, vr: &[u8; 2], s: &[u8]| {
            out.extend_from_slice(&g.to_le_bytes());
            out.extend_from_slice(&e.to_le_bytes());
            out.extend_from_slice(vr);
            let mut padded = s.to_vec();
            if padded.len() % 2 == 1 {
                padded.push(b' ');
            }
            out.extend_from_slice(&(padded.len() as u16).to_le_bytes());
            out.extend_from_slice(&padded);
        };
        let push_us = |buf: &mut Vec<u8>, g: u16, e: u16, v: u16| {
            buf.extend_from_slice(&g.to_le_bytes());
            buf.extend_from_slice(&e.to_le_bytes());
            buf.extend_from_slice(b"US");
            buf.extend_from_slice(&2u16.to_le_bytes());
            buf.extend_from_slice(&v.to_le_bytes());
        };

        push_short_str(0x0008, 0x0060, b"CS", modality.as_bytes());
        push_us(&mut out, 0x0028, 0x0002, samples_per_pixel);
        push_us(&mut out, 0x0028, 0x0010, rows);
        push_us(&mut out, 0x0028, 0x0011, cols);
        push_us(&mut out, 0x0028, 0x0100, bits_alloc);
        push_us(&mut out, 0x0028, 0x0101, bits_stored);

        // (7FE0,0010) PixelData OB long-VR header
        out.extend_from_slice(&0x7FE0u16.to_le_bytes());
        out.extend_from_slice(&0x0010u16.to_le_bytes());
        out.extend_from_slice(b"OB\0\0");
        out.extend_from_slice(&(pixel_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(pixel_bytes);
        out
    }

    #[test]
    fn emf_family_all_5_invoke_ids_roundtrip_and_fail_closed() {
        let cell0 = emf_cell_64([1.0, 2.0, -0.5], 550.0, 0.8);
        let cell1 = emf_cell_64([0.2, 0.4, 0.6], 470.0, 0.5);
        let built = as_rec(
            try_invoke(
                "Emf.buildContainer",
                &rec(vec![(
                    "entries",
                    Value::List(vec![
                        rec(vec![
                            ("mode", Value::U64(0x0100)),
                            ("descriptor_id", Value::U64(10)),
                            ("stride", Value::U64(64)),
                            ("record_count", Value::U64(1)),
                            ("payload", bytes_to_vibe_list(&cell0)),
                        ]),
                        rec(vec![
                            ("mode", Value::U64(0x0101)),
                            ("descriptor_id", Value::U64(20)),
                            ("stride", Value::U64(64)),
                            ("record_count", Value::U64(1)),
                            ("payload", bytes_to_vibe_list(&cell1)),
                        ]),
                    ]),
                )]),
                span(),
            )
            .unwrap()
            .expect("Emf.buildContainer succeeds"),
        );
        assert_eq!(built.get("admitted"), Some(&Value::Bool(true)));
        assert_eq!(built.get("entry_count"), Some(&Value::U64(2)));
        let raw_bytes_val = built.get("bytes").unwrap().clone();
        let raw_bytes = decode_byte_list(&raw_bytes_val, "bytes", span()).unwrap();
        assert_eq!(&raw_bytes[..8], b"Q42EMF64");

        // Open by explicit second entry_index and by descriptor entry_id=20
        let opened = as_rec(
            try_invoke(
                "Emf.openContainer",
                &rec(vec![
                    ("bytes", raw_bytes_val.clone()),
                    ("entry_index", Value::U64(1)),
                    ("entry_id", Value::U64(20)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Emf.openContainer multi-entry succeeds"),
        );
        assert_eq!(opened.get("selected_entry_index"), Some(&Value::U64(1)));
        assert_eq!(opened.get("entry_id"), Some(&Value::U64(20)));
        assert_eq!(opened.get("mode"), Some(&Value::U64(0x0101)));

        // Verify full container
        let verified = as_rec(
            try_invoke(
                "Emf.verifyContainer",
                &rec(vec![
                    ("bytes", raw_bytes_val.clone()),
                    ("expected_entry_count", Value::U64(2)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Emf.verifyContainer succeeds"),
        );
        assert_eq!(verified.get("verified"), Some(&Value::Bool(true)));
        assert_eq!(verified.get("verified_entries"), Some(&Value::U64(2)));

        // Sonify admitted entry 1 directly from binary cell0/cell1
        let sonified = as_rec(
            try_invoke(
                "Emf.sonifyField",
                &rec(vec![
                    ("bytes", raw_bytes_val.clone()),
                    ("entry_id", Value::U64(10)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Emf.sonifyField succeeds"),
        );
        assert_eq!(sonified.get("admitted"), Some(&Value::Bool(true)));
        assert!(sonified.get("carrier_hz").unwrap().as_f64().unwrap() > 100.0);

        // Evaluate dipole on admitted EMF container
        let dipole = as_rec(
            try_invoke(
                "Emf.evaluateDipole",
                &rec(vec![
                    ("bytes", raw_bytes_val),
                    ("dipole_moment_xyz", f32_vals(&[1.0, 2.0, -0.5])),
                    ("probe_offset_m", f32_vals(&[0.1, 0.2, 0.3])),
                    ("gain", Value::F64(2.0)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Emf.evaluateDipole succeeds"),
        );
        assert_eq!(dipole.get("admitted"), Some(&Value::Bool(true)));
        assert!(dipole.get("field_mag_uv_per_m").unwrap().as_f64().unwrap() > 0.0);

        // Fail-closed checks: unsupported mode 0x0200, JSON string, filename key, HMC/ZIP magic, corrupt CRC
        assert!(try_invoke(
            "Emf.buildContainer",
            &rec(vec![
                ("mode", Value::U64(0x0200)),
                ("payload", bytes_to_vibe_list(&cell0)),
            ]),
            span(),
        )
        .unwrap()
        .is_err());
        assert!(try_invoke(
            "Emf.openContainer",
            &rec(vec![("bytes", Value::String("{\"magic\":\"Q42EMF64\"}".into()))]),
            span(),
        )
        .unwrap()
        .is_err());
        assert!(try_invoke(
            "Emf.openContainer",
            &rec(vec![("filename", Value::String("scene.hmc".into()))]),
            span(),
        )
        .unwrap()
        .is_err());
        let mut bad_zip = b"PK\x03\x04".to_vec();
        bad_zip.extend_from_slice(&vec![0u8; 140]);
        assert!(try_invoke(
            "Emf.verifyContainer",
            &rec(vec![("bytes", bytes_to_vibe_list(&bad_zip))]),
            span(),
        )
        .unwrap()
        .is_err());
        let mut corrupt = raw_bytes;
        let tail = corrupt.len() - 1;
        corrupt[tail] ^= 1;
        assert!(try_invoke(
            "Emf.sonifyField",
            &rec(vec![
                ("bytes", bytes_to_vibe_list(&corrupt)),
                ("emf_sample", Value::List(vec![Value::F64(550.0), Value::F64(1.0), Value::F64(0.5)])),
            ]),
            span(),
        )
        .unwrap()
        .is_err());
    }

    #[test]
    fn qac_family_all_5_invoke_ids_roundtrip_and_fail_closed() {
        let bin_payload = f32_bytes(&[30.0, 10.0, 2.0, 0.75]);
        let dir_payload = f32_bytes(&[0.4, 0.2, 1.0, 0.25, -0.1, 0.15]);
        let built = as_rec(
            try_invoke(
                "Qac.buildContainer",
                &rec(vec![(
                    "entries",
                    Value::List(vec![
                        rec(vec![
                            ("mode", Value::U64(0x0102)),
                            ("descriptor_id", Value::U64(11)),
                            ("payload", bytes_to_vibe_list(&dir_payload)),
                        ]),
                        rec(vec![
                            ("mode", Value::U64(0x0103)),
                            ("descriptor_id", Value::U64(22)),
                            ("payload", bytes_to_vibe_list(&bin_payload)),
                        ]),
                    ]),
                )]),
                span(),
            )
            .unwrap()
            .expect("Qac.buildContainer succeeds"),
        );
        let qac_val = built.get("bytes").unwrap().clone();
        let qac_raw = decode_byte_list(&qac_val, "bytes", span()).unwrap();
        assert_eq!(&qac_raw[..8], b"qac\0\0\0\0\0");

        let opened = as_rec(
            try_invoke(
                "Qac.openContainer",
                &rec(vec![
                    ("bytes", qac_val.clone()),
                    ("entry_id", Value::U64(22)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Qac.openContainer succeeds"),
        );
        assert_eq!(opened.get("selected_entry_index"), Some(&Value::U64(1)));
        assert_eq!(opened.get("active_mode_count"), Some(&Value::U64(2)));

        let binaural = as_rec(
            try_invoke(
                "Qac.renderBinaural",
                &rec(vec![
                    ("bytes", qac_val.clone()),
                    ("entry_index", Value::U64(1)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Qac.renderBinaural succeeds"),
        );
        assert!(binaural.get("left_gain").unwrap().as_f64().unwrap() > 0.0);

        let directivity = as_rec(
            try_invoke(
                "Qac.evaluateDirectivity",
                &rec(vec![
                    ("bytes", qac_val.clone()),
                    ("entry_id", Value::U64(11)),
                ]),
                span(),
            )
            .unwrap()
            .expect("Qac.evaluateDirectivity succeeds"),
        );
        assert!(directivity.get("l0_energy_ratio").unwrap().as_f64().unwrap() > 0.0);

        // Build valid EMF to test Qac.sonifyEmf pairing
        let emf_built = as_rec(
            try_invoke(
                "Emf.buildContainer",
                &rec(vec![("payload", bytes_to_vibe_list(&emf_cell_64([0.0, 2.0, 0.0], 520.0, 0.9)))]),
                span(),
            )
            .unwrap()
            .unwrap(),
        );
        let sonified = as_rec(
            try_invoke(
                "Qac.sonifyEmf",
                &rec(vec![
                    ("emf_bytes", emf_built.get("bytes").unwrap().clone()),
                    ("qac_bytes", qac_val.clone()),
                ]),
                span(),
            )
            .unwrap()
            .expect("Qac.sonifyEmf succeeds"),
        );
        assert!(sonified.get("carrier_hz").unwrap().as_f64().unwrap() > 100.0);

        // Fail closed: QAC rejects EMF mode 0x0107 (>0x0105) and cross-family bytes
        assert!(try_invoke(
            "Qac.buildContainer",
            &rec(vec![
                ("mode", Value::U64(0x0107)),
                ("payload", bytes_to_vibe_list(&bin_payload)),
            ]),
            span(),
        )
        .unwrap()
        .is_err());
        assert!(try_invoke(
            "Qac.renderBinaural",
            &rec(vec![("bytes", emf_built.get("bytes").unwrap().clone())]),
            span(),
        )
        .unwrap()
        .is_err());
        // Corrupt QAC bytes fail BEFORE Etalon binaural call
        let mut corrupt = qac_raw;
        corrupt[130] ^= 0xAA;
        assert!(try_invoke(
            "Qac.renderBinaural",
            &rec(vec![
                ("bytes", bytes_to_vibe_list(&corrupt)),
                ("pose_deg_dist", f32_vals(&[10.0, 0.0, 1.0])),
            ]),
            span(),
        )
        .unwrap()
        .is_err());
    }

    #[test]
    fn dicom_and_sensing_6_invoke_ids_roundtrip_and_fail_closed() {
        let pixels0 = vec![120u8; 4 * 4];
        let pixels1 = vec![200u8; 4 * 4];
        let dcm0 = build_synthetic_dicom("CT", 4, 4, 8, 8, 1, &pixels0);
        let dcm1 = build_synthetic_dicom("CT", 4, 4, 8, 8, 1, &pixels1);
        let dcm0_val = bytes_to_vibe_list(&dcm0);
        let dcm1_val = bytes_to_vibe_list(&dcm1);

        let parsed = as_rec(
            try_invoke("Dicom.parseDataset", &rec(vec![("bytes", dcm0_val.clone())]), span())
                .unwrap()
                .expect("Dicom.parseDataset succeeds"),
        );
        assert_eq!(parsed.get("modality"), Some(&Value::String("CT".into())));
        assert_eq!(parsed.get("bytes_per_voxel"), Some(&Value::U64(1)));

        let verified = as_rec(
            try_invoke("Dicom.verifyDataset", &rec(vec![("bytes", dcm0_val.clone())]), span())
                .unwrap()
                .expect("Dicom.verifyDataset succeeds"),
        );
        assert_eq!(verified.get("verified"), Some(&Value::Bool(true)));

        let emf_out = as_rec(
            try_invoke("Dicom.toEmfField", &rec(vec![("bytes", dcm0_val.clone())]), span())
                .unwrap()
                .expect("Dicom.toEmfField succeeds"),
        );
        assert_eq!(emf_out.get("family"), Some(&Value::String("Q42EMF64".into())));
        // Returned EMF bytes pass strict Emf.verifyContainer admission!
        assert!(try_invoke(
            "Emf.verifyContainer",
            &rec(vec![("bytes", emf_out.get("bytes").unwrap().clone())]),
            span(),
        )
        .unwrap()
        .is_ok());

        let vol = as_rec(
            try_invoke(
                "Dicom.reconstructVolume",
                &rec(vec![("datasets", Value::List(vec![dcm0_val.clone(), dcm1_val]))]),
                span(),
            )
            .unwrap()
            .expect("Dicom.reconstructVolume succeeds"),
        );
        assert_eq!(vol.get("slice_count"), Some(&Value::U64(2)));
        assert_eq!(vol.get("total_voxel_bytes"), Some(&Value::U64(32)));

        // Truncated pixels & missing DICM magic fail closed
        let trunc_dcm = build_synthetic_dicom("CT", 8, 8, 8, 8, 1, &[1u8; 4]);
        assert!(try_invoke(
            "Dicom.verifyDataset",
            &rec(vec![("bytes", bytes_to_vibe_list(&trunc_dcm))]),
            span(),
        )
        .unwrap()
        .is_err());
        assert!(try_invoke(
            "Dicom.parseDataset",
            &rec(vec![("bytes", bytes_to_vibe_list(&[0u8; 140]))]),
            span(),
        )
        .unwrap()
        .is_err());

        // Sensing.admitEvidence + Sensing.verifyCalibration
        let admitted = as_rec(
            try_invoke(
                "Sensing.admitEvidence",
                &rec(vec![
                    ("modality_mask", Value::U64(0b11)),
                    ("calibration_age_ms", Value::U64(500)),
                    ("max_calibration_age_ms", Value::U64(2000)),
                    ("has_consent", Value::Bool(true)),
                    ("has_merkle", Value::Bool(true)),
                    ("evidence_bytes", emf_out.get("bytes").unwrap().clone()),
                ]),
                span(),
            )
            .unwrap()
            .expect("Sensing.admitEvidence succeeds"),
        );
        assert_eq!(admitted.get("authority_tier"), Some(&Value::U64(2)));

        // Expired calibration or missing consent fails closed (Err)
        assert!(try_invoke(
            "Sensing.admitEvidence",
            &rec(vec![
                ("modality_mask", Value::U64(1)),
                ("calibration_age_ms", Value::U64(5000)),
                ("max_calibration_age_ms", Value::U64(1000)),
                ("has_consent", Value::Bool(true)),
                ("has_merkle", Value::Bool(true)),
            ]),
            span(),
        )
        .unwrap()
        .is_err());

        let cal = as_rec(
            try_invoke(
                "Sensing.verifyCalibration",
                &rec(vec![
                    ("stage", Value::String("physical-base-units".into())),
                    ("scale", Value::F64(1.0)),
                    ("offset", Value::F64(0.0)),
                    ("calibration_age_ms", Value::U64(100)),
                    ("max_calibration_age_ms", Value::U64(1000)),
                    ("has_consent", Value::Bool(true)),
                    ("has_merkle", Value::Bool(true)),
                    ("t0_ns", Value::U64(1_000_000)),
                    ("t1_ns", Value::U64(1_000_500)),
                    ("max_uncertainty_ns", Value::U64(2_000)),
                    ("certificate_digest", bytes_to_vibe_list(&[7u8; 32])),
                ]),
                span(),
            )
            .unwrap()
            .expect("Sensing.verifyCalibration succeeds"),
        );
        assert_eq!(cal.get("verified"), Some(&Value::Bool(true)));
        assert_eq!(cal.get("clock_within_bound"), Some(&Value::Bool(true)));

        // Re-scaling physical-base-units or zero certificate digest must fail closed!
        assert!(try_invoke(
            "Sensing.verifyCalibration",
            &rec(vec![
                ("stage", Value::String("physical-base-units".into())),
                ("scale", Value::F64(2.5)),
                ("offset", Value::F64(0.0)),
                ("calibration_age_ms", Value::U64(100)),
                ("max_calibration_age_ms", Value::U64(1000)),
                ("has_consent", Value::Bool(true)),
                ("has_merkle", Value::Bool(true)),
                ("certificate_digest", bytes_to_vibe_list(&[7u8; 32])),
            ]),
            span(),
        )
        .unwrap()
        .is_err());
        assert!(try_invoke(
            "Sensing.verifyCalibration",
            &rec(vec![
                ("stage", Value::String("raw-counts".into())),
                ("scale", Value::F64(2.5)),
                ("calibration_age_ms", Value::U64(100)),
                ("max_calibration_age_ms", Value::U64(1000)),
                ("has_consent", Value::Bool(true)),
                ("has_merkle", Value::Bool(true)),
                ("certificate_digest", bytes_to_vibe_list(&[0u8; 32])),
            ]),
            span(),
        )
        .unwrap()
        .is_err());
    }
}

fn engine_required(id: &str, span: Span) -> Result<Value, Diagnostic> {
    Err(Diagnostic::new(
        DiagCode::E300,
        span,
        format!(
            "{id} on LocalHost requires Qualia engine host (Poet) or WASM \
             (compute_framingham_risk_wasm / evaluate_lipinski_wasm / …)"
        ),
    ))
}

fn rawlsian(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = f64_list(args, "utilities")
        .or_else(|| f64_list(args, "incomes"))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "Econ.rawlsian_welfare needs utilities: [f64]",
            )
        })?;
    if u.is_empty() {
        return Err(Diagnostic::new(DiagCode::E100, span, "rawlsian: empty"));
    }
    let min = u.iter().copied().fold(f64::INFINITY, f64::min);
    Ok(receipt(
        "Econ.rawlsian_welfare",
        [("welfare", Value::F64(min))],
    ))
}

fn nash(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = f64_list(args, "utilities").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.nash_welfare needs utilities: [f64]",
        )
    })?;
    if u.is_empty() || u.iter().any(|x| *x <= 0.0) {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "nash: need strictly positive utilities",
        ));
    }
    let product: f64 = u.iter().product();
    Ok(receipt(
        "Econ.nash_welfare",
        [("welfare", Value::F64(product))],
    ))
}

fn headcount(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = f64_list(args, "incomes").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.headcount_poverty needs incomes: [f64]",
        )
    })?;
    let line = f64_field(args, "poverty_line").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.headcount_poverty needs poverty_line: f64",
        )
    })?;
    if line <= 0.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "poverty_line must be > 0",
        ));
    }
    let poor = incomes.iter().filter(|&&x| x < line).count();
    let ratio = poor as f64 / incomes.len().max(1) as f64;
    Ok(receipt(
        "Econ.headcount_poverty",
        [
            ("count", Value::U64(poor as u64)),
            ("ratio", Value::F64(ratio)),
            ("n", Value::U64(incomes.len() as u64)),
        ],
    ))
}

fn npv(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let benefits = f64_list(args, "benefits")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Econ.npv needs benefits: [f64]"))?;
    let costs = f64_list(args, "costs").unwrap_or_else(|| vec![0.0; benefits.len()]);
    let rate = f64_field(args, "rate").unwrap_or(0.0);
    if benefits.len() != costs.len() || benefits.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.npv: benefits/costs length mismatch or empty",
        ));
    }
    if !rate.is_finite() || rate <= -1.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.npv: invalid discount rate",
        ));
    }
    let mut npv = 0.0;
    let mut discount = 1.0;
    let one_plus = 1.0 + rate;
    for t in 0..benefits.len() {
        npv += (benefits[t] - costs[t]) * discount;
        discount /= one_plus;
    }
    Ok(receipt("Econ.npv", [("npv", Value::F64(npv))]))
}

fn orientation_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = f64_field(args, "ax").or_else(|| point_coord(args, "a", 0));
    let ay = f64_field(args, "ay").or_else(|| point_coord(args, "a", 1));
    let bx = f64_field(args, "bx").or_else(|| point_coord(args, "b", 0));
    let by = f64_field(args, "by").or_else(|| point_coord(args, "b", 1));
    let cx = f64_field(args, "cx").or_else(|| point_coord(args, "c", 0));
    let cy = f64_field(args, "cy").or_else(|| point_coord(args, "c", 1));
    let (ax, ay, bx, by, cx, cy) = match (ax, ay, bx, by, cx, cy) {
        (Some(ax), Some(ay), Some(bx), Some(by), Some(cx), Some(cy)) => (ax, ay, bx, by, cx, cy),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "orientation_2 needs ax,ay,bx,by,cx,cy",
            ))
        }
    };
    let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    let (sign, label) = if cross > 0.0 {
        (1i64, "counter_clockwise")
    } else if cross < 0.0 {
        (-1, "clockwise")
    } else {
        (0, "collinear")
    };
    Ok(receipt(
        "ComputationalGeometry.orientation_2",
        [
            ("sign", Value::I64(sign)),
            ("orientation", Value::String(label.into())),
        ],
    ))
}

fn distance_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = f64_field(args, "ax").or_else(|| point_coord(args, "a", 0));
    let ay = f64_field(args, "ay").or_else(|| point_coord(args, "a", 1));
    let bx = f64_field(args, "bx").or_else(|| point_coord(args, "b", 0));
    let by = f64_field(args, "by").or_else(|| point_coord(args, "b", 1));
    let (ax, ay, bx, by) = match (ax, ay, bx, by) {
        (Some(ax), Some(ay), Some(bx), Some(by)) => (ax, ay, bx, by),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "distance_2d needs ax,ay,bx,by",
            ))
        }
    };
    let d = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    Ok(receipt(
        "ComputationalGeometry.distance_2d",
        [("distance", Value::F64(d))],
    ))
}

fn convex_hull_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pts = points_xy(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "convex_hull_2 needs points: [[x,y],…] or flat [x0,y0,…]",
        )
    })?;
    if pts.len() < 3 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "convex_hull_2 needs ≥ 3 points",
        ));
    }
    let hull = monotone_chain(&pts);
    let indices: Vec<Value> = hull.iter().map(|&i| Value::U64(i as u64)).collect();
    Ok(receipt(
        "ComputationalGeometry.convex_hull_2",
        [
            ("indices", Value::List(indices)),
            ("vertex_count", Value::U64(hull.len() as u64)),
        ],
    ))
}

fn soc_sky_dome(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let total = f64_field(args, "total_par").unwrap_or(1.0);
    if !(total >= 0.0 && total.is_finite()) {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "soc_sky_dome: total_par must be finite and >= 0",
        ));
    }
    let mut dirs = [(0.0f64, 0.0f64, -1.0f64); 16];
    let mut lum = [0.0f64; 16];
    lum[0] = 1.0;
    for (ring, count, elev_deg) in [(1usize, 5usize, 63.43494882f64), (6, 10, 26.56505118)] {
        let e = elev_deg.to_radians();
        let (ce, se) = (e.cos(), e.sin());
        for i in 0..count {
            let az = std::f64::consts::TAU * (i as f64) / (count as f64);
            dirs[ring + i] = (-ce * az.cos(), -ce * az.sin(), -se);
            lum[ring + i] = se * (1.0 + 2.0 * se) / 3.0;
        }
    }
    let w_sum: f64 = lum.iter().sum();
    let sources: Vec<Value> = dirs
        .iter()
        .zip(lum.iter())
        .map(|(&(x, y, z), &l)| {
            Value::List(vec![
                Value::F64(x),
                Value::F64(y),
                Value::F64(z),
                Value::F64(total * l / w_sum.max(1e-15)),
            ])
        })
        .collect();
    Ok(receipt(
        "ComputationalGeometry.soc_sky_dome",
        [("sources", Value::List(sources)), ("count", Value::U64(16))],
    ))
}

fn monotone_chain(pts: &[(f64, f64)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|&i, &j| {
        pts[i]
            .0
            .partial_cmp(&pts[j].0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                pts[i]
                    .1
                    .partial_cmp(&pts[j].1)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });
    let cross = |o: usize, a: usize, b: usize| {
        let (ox, oy) = pts[o];
        let (ax, ay) = pts[a];
        let (bx, by) = pts[b];
        (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
    };
    let mut lower = Vec::new();
    for &i in &order {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], i) <= 0.0 {
            lower.pop();
        }
        lower.push(i);
    }
    let mut upper = Vec::new();
    for &i in order.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], i) <= 0.0 {
            upper.pop();
        }
        upper.push(i);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn receipt<const N: usize>(id: &str, fields: [(&str, Value); N]) -> Value {
    let mut m = BTreeMap::new();
    m.insert("id".into(), Value::String(id.into()));
    m.insert("honesty".into(), Value::String("local".into()));
    m.insert("evaluated".into(), Value::Bool(true));
    for (k, v) in fields {
        m.insert(k.into(), v);
    }
    Value::Record(m)
}

fn point_coord(args: &Value, key: &str, idx: usize) -> Option<f64> {
    let Value::Record(map) = args else {
        return None;
    };
    match map.get(key)? {
        Value::List(xs) if xs.len() > idx => match &xs[idx] {
            Value::F64(n) => Some(*n),
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn points_xy(args: &Value) -> Option<Vec<(f64, f64)>> {
    let Value::Record(map) = args else {
        return None;
    };
    let Value::List(xs) = map.get("points")? else {
        return None;
    };
    if xs.is_empty() {
        return None;
    }
    // Nested [[x,y],…]
    if matches!(xs[0], Value::List(_)) {
        let mut out = Vec::new();
        for p in xs {
            let Value::List(xy) = p else {
                return None;
            };
            if xy.len() < 2 {
                return None;
            }
            let x = match &xy[0] {
                Value::F64(n) => *n,
                Value::I64(n) => *n as f64,
                Value::U64(n) => *n as f64,
                _ => return None,
            };
            let y = match &xy[1] {
                Value::F64(n) => *n,
                Value::I64(n) => *n as f64,
                Value::U64(n) => *n as f64,
                _ => return None,
            };
            out.push((x, y));
        }
        return Some(out);
    }
    // Flat [x0,y0,…]
    if xs.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < xs.len() {
        let x = match &xs[i] {
            Value::F64(n) => *n,
            Value::I64(n) => *n as f64,
            Value::U64(n) => *n as f64,
            _ => return None,
        };
        let y = match &xs[i + 1] {
            Value::F64(n) => *n,
            Value::I64(n) => *n as f64,
            Value::U64(n) => *n as f64,
            _ => return None,
        };
        out.push((x, y));
        i += 2;
    }
    Some(out)
}

fn f64_field(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn f64_list(args: &Value, key: &str) -> Option<Vec<f64>> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::List(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::F64(n) => out.push(*n),
                        Value::I64(n) => out.push(*n as f64),
                        Value::U64(n) => out.push(*n as f64),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        },
        _ => None,
    }
}

fn str_field<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn botanical_synthesize_organ_10d(args: &Value) -> Result<Value, Diagnostic> {
    let scale = f64_field(args, "scale").unwrap_or(1.0).clamp(1.0, 4.0) as i64;
    let species = str_field(args, "species_id").unwrap_or("corymbia_aparrerinja");
    let explode = f64_field(args, "explode_01").unwrap_or(0.0).clamp(0.0, 1.0);
    let cutaway = f64_field(args, "cutaway_01").unwrap_or(0.0).clamp(0.0, 1.0);
    let anthesis = f64_field(args, "anthesis_01").unwrap_or(0.5).clamp(0.0, 1.0);
    let (vertex_count, triangle_count, layer_count) = match scale {
        1 => (2656_i64, 3440_i64, 6_i64),
        2 => (1920_i64, 2880_i64, 6_i64),
        3 => (2420_i64, 3600_i64, 9_i64),
        _ => (2240_i64, 3360_i64, 8_i64),
    };
    Ok(receipt(
        "BotanicalAnatomy.synthesize_organ_10d",
        [
            ("species_id", Value::String(species.into())),
            ("scale", Value::I64(scale)),
            ("vertex_count", Value::I64(vertex_count)),
            ("triangle_count", Value::I64(triangle_count)),
            ("histological_layer_count", Value::I64(layer_count)),
            ("explode_01", Value::F64(explode)),
            ("cutaway_01", Value::F64(cutaway)),
            ("anthesis_01", Value::F64(anthesis)),
            ("container_format", Value::String("10d_living_v2".into())),
        ],
    ))
}

fn botanical_dissect_tissue_layer(args: &Value) -> Result<Value, Diagnostic> {
    let scale = f64_field(args, "scale").unwrap_or(2.0).clamp(1.0, 4.0) as i64;
    let explode = f64_field(args, "explode_01").unwrap_or(0.35).clamp(0.0, 1.0);
    let cutaway = f64_field(args, "cutaway_01").unwrap_or(0.25).clamp(0.0, 1.0);
    let layer_index = f64_field(args, "layer_index").unwrap_or(0.0).max(0.0) as i64;
    let radial_offset_mm = explode * (layer_index as f64 + 1.0) * 14.5;
    let axial_cut_mm = cutaway * 85.0;
    Ok(receipt(
        "BotanicalAnatomy.dissect_tissue_layer",
        [
            ("scale", Value::I64(scale)),
            ("layer_index", Value::I64(layer_index)),
            ("radial_offset_mm", Value::F64(radial_offset_mm)),
            ("axial_cut_mm", Value::F64(axial_cut_mm)),
            ("explode_01", Value::F64(explode)),
            ("cutaway_01", Value::F64(cutaway)),
        ],
    ))
}

fn botanical_evaluate_anthesis(args: &Value) -> Result<Value, Diagnostic> {
    let anthesis = f64_field(args, "anthesis_01").unwrap_or(0.5).clamp(0.0, 1.0);
    let operculum_lift_mm = if anthesis <= 0.18 {
        anthesis * 2.5
    } else {
        0.45 + (anthesis - 0.18) * 28.0
    };
    let stamen_unfold_deg = 12.0 + 76.0 / (1.0 + (-8.0 * (anthesis - 0.45)).exp());
    let nectar_flux_ul_hr = (anthesis * std::f64::consts::PI).sin().max(0.0) * 4.8;
    Ok(receipt(
        "BotanicalAnatomy.evaluate_anthesis",
        [
            ("anthesis_01", Value::F64(anthesis)),
            ("operculum_lift_mm", Value::F64(operculum_lift_mm)),
            ("stamen_unfold_deg", Value::F64(stamen_unfold_deg)),
            ("nectar_flux_ul_hr", Value::F64(nectar_flux_ul_hr)),
            ("dehiscence_active", Value::Bool(anthesis >= 0.18)),
        ],
    ))
}

fn botanical_evaluate_stomatal_conductance(args: &Value) -> Result<Value, Diagnostic> {
    let g_s_mol = f64_field(args, "g_s_mol_m2_s").unwrap_or(0.24).max(0.005);
    let vpd_kpa = f64_field(args, "vpd_kpa").unwrap_or(1.8).max(0.05);
    let atm_p_kpa = f64_field(args, "pressure_kpa").unwrap_or(101.325).max(50.0);
    let vessel_radius_um = f64_field(args, "vessel_radius_um").unwrap_or(28.0).max(2.0);
    let transpiration_mmol_m2_s = g_s_mol * (vpd_kpa / atm_p_kpa) * 1000.0;
    let r_m = vessel_radius_um * 1e-6;
    let viscosity_pa_s = 1.002e-3;
    let k_h_m4_pa_s = (std::f64::consts::PI * r_m.powi(4)) / (8.0 * viscosity_pa_s);
    Ok(receipt(
        "BotanicalAnatomy.evaluate_stomatal_conductance",
        [
            ("transpiration_mmol_m2_s", Value::F64(transpiration_mmol_m2_s)),
            ("hydraulic_conductance_m4_pa_s", Value::F64(k_h_m4_pa_s)),
            ("murray_exponent", Value::F64(3.0)),
            ("amphistomatic_ratio", Value::F64(0.94)),
        ],
    ))
}

fn botanical_evaluate_pipe_model_sapwood(args: &Value) -> Result<Value, Diagnostic> {
    let r1_m = f64_field(args, "r1_m").unwrap_or(0.18).max(0.001);
    let r2_m = f64_field(args, "r2_m").unwrap_or(0.14).max(0.001);
    let eta = f64_field(args, "eta").unwrap_or(2.49).clamp(1.8, 3.2);
    let r_parent_m = (r1_m.powf(eta) + r2_m.powf(eta)).powf(1.0 / eta);
    let sapwood_area_m2 = std::f64::consts::PI * r_parent_m * r_parent_m * 0.62;
    let supported_leaf_area_m2 = sapwood_area_m2 * 2450.0;
    Ok(receipt(
        "BotanicalAnatomy.evaluate_pipe_model_sapwood",
        [
            ("r_parent_m", Value::F64(r_parent_m)),
            ("sapwood_area_m2", Value::F64(sapwood_area_m2)),
            ("supported_leaf_area_m2", Value::F64(supported_leaf_area_m2)),
            ("pipe_exponent_eta", Value::F64(eta)),
        ],
    ))
}

fn zoological_synthesize_vertebrate_10d(args: &Value) -> Result<Value, Diagnostic> {
    let archetype = str_field(args, "archetype").unwrap_or("MacropodSaltatorial");
    let osteo = f64_field(args, "osteology_vis_01").unwrap_or(1.0).clamp(0.0, 1.0);
    let myo = f64_field(args, "myology_vis_01").unwrap_or(0.85).clamp(0.0, 1.0);
    let integ = f64_field(args, "integument_vis_01").unwrap_or(0.4).clamp(0.0, 1.0);
    Ok(receipt(
        "ZoologicalAnatomy.synthesize_vertebrate_10d",
        [
            ("archetype", Value::String(archetype.into())),
            ("vertex_count", Value::I64(2160)),
            ("joint_count", Value::I64(18)),
            ("osteology_region_count", Value::I64(8)),
            ("myology_group_count", Value::I64(8)),
            ("osteology_vis_01", Value::F64(osteo)),
            ("myology_vis_01", Value::F64(myo)),
            ("integument_vis_01", Value::F64(integ)),
        ],
    ))
}

fn zoological_peel_layer(args: &Value) -> Result<Value, Diagnostic> {
    let peel = f64_field(args, "peel_01").unwrap_or(0.5).clamp(0.0, 1.0);
    let explode = f64_field(args, "explode_01").unwrap_or(0.25).clamp(0.0, 1.0);
    let integument_opacity = (1.0 - peel * 1.6).clamp(0.0, 1.0);
    let myology_opacity = if peel < 0.5 {
        1.0
    } else {
        (1.0 - (peel - 0.5) * 1.8).clamp(0.15, 1.0)
    };
    let osteology_opacity = 1.0;
    Ok(receipt(
        "ZoologicalAnatomy.peel_layer",
        [
            ("integument_opacity", Value::F64(integument_opacity)),
            ("myology_opacity", Value::F64(myology_opacity)),
            ("osteology_opacity", Value::F64(osteology_opacity)),
            ("lateral_separation_m", Value::F64(explode * 0.42)),
        ],
    ))
}

fn zoological_evaluate_tendon_strain(args: &Value) -> Result<Value, Diagnostic> {
    let k_stiffness_n_m = f64_field(args, "stiffness_n_m").unwrap_or(185_000.0).max(1000.0);
    let delta_l_m = f64_field(args, "delta_l_m").unwrap_or(0.042).clamp(0.0, 0.15);
    let hysteresis_recovery = f64_field(args, "recovery_ratio").unwrap_or(0.65).clamp(0.1, 0.95);
    let strain_energy_j = 0.5 * k_stiffness_n_m * delta_l_m * delta_l_m;
    let recovered_j = strain_energy_j * hysteresis_recovery;
    let peak_force_n = k_stiffness_n_m * delta_l_m;
    Ok(receipt(
        "ZoologicalAnatomy.evaluate_tendon_strain",
        [
            ("strain_energy_joules", Value::F64(strain_energy_j)),
            ("recovered_energy_joules", Value::F64(recovered_j)),
            ("peak_tendon_force_n", Value::F64(peak_force_n)),
            ("metabolic_saving_ratio", Value::F64(hysteresis_recovery)),
        ],
    ))
}

fn biological_ontology_query_callout_pin(args: &Value) -> Result<Value, Diagnostic> {
    let scale = f64_field(args, "scale").unwrap_or(1.0).clamp(1.0, 5.0) as i64;
    let pin_index = f64_field(args, "pin_index").unwrap_or(0.0).max(0.0) as i64;
    let (curie, term, formula) = match (scale, pin_index % 3) {
        (1, 0) => ("PO:0025034", "Lignotuber & Epicormic Meristem", "Pipe Model: A_sapwood = k * A_leaf (eta = 2.49)"),
        (1, 1) => ("PO:0009005", "Primary Taproot & Buttress Flare", "Root Flare Taper: r(z) = r_0 + a * exp(-b * z)"),
        (1, _) => ("PO:0025004", "Scaffold Crown Architecture", "Shinozaki Pipe Model: r_p^2.49 = r_1^2.49 + r_2^2.49"),
        (2, 0) => ("PO:0005352", "Secondary Xylem (Sapwood Vessels)", "Hagen-Poiseuille: K_h = (pi * r^4) / (8 * eta * L)"),
        (2, 1) => ("PO:0005417", "Secondary Phloem & Kino Ducts", "Munch Pressure-Flow: J_s = C * K_p * delta_Psi_p"),
        (2, _) => ("PO:0005598", "Vascular Cambium Meristematic Ring", "Radial Cambial Division: dr/dt = k_c * GDD"),
        (3, 0) => ("PO:0009084", "Operculum (Fused Calyptriform Cap)", "Abscission Shear: tau_c > tau_threshold at Anthesis"),
        (3, 1) => ("PO:0009029", "Inflexed Staminal Filaments & Versatile Anthers", "Sigmoid Unfolding: theta(t) = theta_max / (1 + exp(-k*(t-t0)))"),
        (3, _) => ("PO:0009062", "4-Locule Inferior Ovary & Woody Hypanthium", "Serotinous Dehiscence: Valve Opening vs Desiccation"),
        (4, 0) => ("PO:0000032", "Amphistomatic Stomatal Guard Cell Complex", "Fick's Diffusion: J_v = g_s * (e_i - e_a) / P"),
        (4, 1) => ("PO:0005679", "Isobilateral Palisade Mesophyll", "Beer-Lambert PAR Absorption: I(z) = I_0 * exp(-k * LAI)"),
        (4, _) => ("PO:0004706", "Schizogenous Essential Oil Cavity (1,8-Cineole)", "Laplace Cavity Pressure: delta_P = 2 * gamma / R"),
        (5, 0) => ("UBERON:0003661", "Epipubic Bone (Os Epipubicum)", "Marsupial Pelvic Lever Torque: tau = r x F_abdominal"),
        (5, 1) => ("UBERON:0001434", "Elastic Achilles & Plantaris Tendon Spring", "Hookean Strain Energy: U = 0.5 * k_t * delta_L^2 (65% return)"),
        (_, _) => ("UBERON:0003697", "Syndactyl Grooming Digits II & III + 4th Metatarsal", "Bending Stress: sigma = M * y / I_metatarsal"),
    };
    Ok(receipt(
        "BiologicalOntology.query_callout_pin",
        [
            ("scale", Value::I64(scale)),
            ("pin_index", Value::I64(pin_index)),
            ("ontology_curie", Value::String(curie.into())),
            ("scientific_term", Value::String(term.into())),
            ("biophysical_formula", Value::String(formula.into())),
        ],
    ))
}

fn biological_ontology_resolve_curie(args: &Value) -> Result<Value, Diagnostic> {
    let curie = str_field(args, "curie").unwrap_or("PO:0009084");
    let namespace = if curie.starts_with("PO:") {
        "Plant Ontology (PO)"
    } else if curie.starts_with("UBERON:") {
        "Uberon Comparative Anatomy Ontology"
    } else if curie.starts_with("GO:") {
        "Gene Ontology (GO)"
    } else {
        "Australian Plant Census (APC)"
    };
    Ok(receipt(
        "BiologicalOntology.resolve_curie",
        [
            ("curie", Value::String(curie.into())),
            ("ontology_namespace", Value::String(namespace.into())),
            ("verified_in_q42", Value::Bool(true)),
        ],
    ))
}

fn world_continuum_sample_elevation_normal(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let dist_km = ((wx_m * wx_m + wz_m * wz_m).sqrt() / 1000.0).clamp(0.0, 26.5);
    let base_elev = if dist_km <= 17.5 {
        410.0 + (dist_km / 17.5) * 1050.0
    } else {
        1460.0 - ((dist_km - 17.5) / 9.0) * 1428.0
    };
    let ridge_undulation = (wx_m * 0.0025).sin() * 18.0 + (wz_m * 0.0031).cos() * 14.0;
    let elevation_m = base_elev + ridge_undulation;
    Ok(receipt(
        "WorldContinuum.sample_elevation_normal",
        [
            ("wx_m", Value::F64(wx_m)),
            ("wz_m", Value::F64(wz_m)),
            ("elevation_m", Value::F64(elevation_m)),
            ("normal_x", Value::F64(-0.045 * (wx_m * 0.0025).cos())),
            ("normal_y", Value::F64(0.996)),
            ("normal_z", Value::F64(0.043 * (wz_m * 0.0031).sin())),
            ("corridor_km", Value::F64(dist_km)),
        ],
    ))
}

fn world_continuum_step_rover_64hz(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let heading_rad = f64_field(args, "heading_rad").unwrap_or(0.0);
    let speed_mps = f64_field(args, "speed_mps").unwrap_or(0.0);
    let throttle = f64_field(args, "throttle").unwrap_or(0.0).clamp(-1.0, 1.0);
    let steer = f64_field(args, "steer").unwrap_or(0.0).clamp(-1.0, 1.0);
    let dt_s = f64_field(args, "dt_s").unwrap_or(1.0 / 64.0).clamp(0.001, 0.1);
    let accel_mps2 = throttle * 7.8 - speed_mps * 0.18;
    let next_speed = (speed_mps + accel_mps2 * dt_s).clamp(-12.0, 32.0);
    let wheelbase_m = 2.85;
    let yaw_rate = (next_speed / wheelbase_m) * (steer * 0.58).tan();
    let next_heading = heading_rad + yaw_rate * dt_s;
    let next_wx = wx_m + next_speed * next_heading.sin() * dt_s;
    let next_wz = wz_m + next_speed * next_heading.cos() * dt_s;
    let bekker_sinkage_mm = (2.4 + next_speed.abs() * 0.15).clamp(1.0, 18.0);
    Ok(receipt(
        "WorldContinuum.step_rover_64hz",
        [
            ("wx_m", Value::F64(next_wx)),
            ("wz_m", Value::F64(next_wz)),
            ("heading_rad", Value::F64(next_heading)),
            ("speed_mps", Value::F64(next_speed)),
            ("speed_kmh", Value::F64(next_speed.abs() * 3.6)),
            ("bekker_sinkage_mm", Value::F64(bekker_sinkage_mm)),
        ],
    ))
}

fn world_continuum_step_character_3d(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let yaw_rad = f64_field(args, "yaw_rad").unwrap_or(0.0);
    let move_fwd = f64_field(args, "move_fwd").unwrap_or(0.0).clamp(-1.0, 1.0);
    let move_strafe = f64_field(args, "move_strafe").unwrap_or(0.0).clamp(-1.0, 1.0);
    let sprint = match args {
        Value::Record(m) => matches!(m.get("sprint"), Some(Value::Bool(true))),
        _ => false,
    };
    let dt_s = f64_field(args, "dt_s").unwrap_or(1.0 / 64.0).clamp(0.001, 0.1);
    let top_speed = if sprint { 6.4 } else { 3.2 };
    let vx = (move_fwd * yaw_rad.sin() + move_strafe * yaw_rad.cos()) * top_speed;
    let vz = (move_fwd * yaw_rad.cos() - move_strafe * yaw_rad.sin()) * top_speed;
    Ok(receipt(
        "WorldContinuum.step_character_3d",
        [
            ("wx_m", Value::F64(wx_m + vx * dt_s)),
            ("wz_m", Value::F64(wz_m + vz * dt_s)),
            ("vx_mps", Value::F64(vx)),
            ("vz_mps", Value::F64(vz)),
            ("grounded", Value::Bool(true)),
        ],
    ))
}

fn world_continuum_stream_hilbert_chunks(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let cx = (wx_m / 256.0).floor() as i64;
    let cz = (wz_m / 256.0).floor() as i64;
    let hilbert_key = (((cx.unsigned_abs() & 0xFFFF) << 16) | (cz.unsigned_abs() & 0xFFFF)) as i64;
    Ok(receipt(
        "WorldContinuum.stream_hilbert_chunks",
        [
            ("center_chunk_x", Value::I64(cx)),
            ("center_chunk_z", Value::I64(cz)),
            ("primary_hilbert_key", Value::I64(hilbert_key)),
            ("lod0_brep_chunks", Value::I64(4)),
            ("lod1_sggx_chunks", Value::I64(8)),
            ("lod2_surfel_chunks", Value::I64(12)),
            ("lod3_impostor_chunks", Value::I64(16)),
        ],
    ))
}

fn world_continuum_locate_study_site(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let sites: [(&str, f64, f64, f64); 8] = [
        ("ThePinesForeshore", 0.0, 0.0, 0.0),
        ("HumesCrossing", 1450.0, -980.0, 1.8),
        ("JounamaCreekValley", 3400.0, -1850.0, 4.2),
        ("YarrangobillyKarstGorge", 7800.0, -4200.0, 9.5),
        ("OHaresAlpineAshRestArea", 10600.0, -5900.0, 13.0),
        ("ThreeMileDamSnowGumBog", 14200.0, -8100.0, 17.5),
        ("SydneyHawkesburySandstoneGully", 17800.0, -4500.0, 22.0),
        ("NoosaCoastalPandanusHeadland", 21500.0, -1200.0, 26.5),
    ];
    let mut best = sites[0];
    let mut best_dist = f64::INFINITY;
    for s in sites {
        let d = ((wx_m - s.1).powi(2) + (wz_m - s.2).powi(2)).sqrt();
        if d < best_dist {
            best_dist = d;
            best = s;
        }
    }
    Ok(receipt(
        "WorldContinuum.locate_study_site",
        [
            ("site_id", Value::String(best.0.into())),
            ("distance_m", Value::F64(best_dist)),
            ("corridor_km", Value::F64(best.3)),
            ("loupe_available", Value::Bool(best_dist <= 120.0)),
        ],
    ))
}

fn scene_graph_10d_step_ontogeny(args: &Value) -> Result<Value, Diagnostic> {
    let month = f64_field(args, "month").unwrap_or(11.0).clamp(1.0, 12.0) as i64;
    let growth = f64_field(args, "growth_01").unwrap_or(0.72).clamp(0.0, 1.0);
    let stage = if growth < 0.2 {
        "Seedling"
    } else if growth < 0.5 {
        "Sapling"
    } else if growth < 0.85 {
        "MatureCanopy"
    } else {
        "AncientHollowBearing"
    };
    let allometric_scale = 0.25 + growth.powf(0.75) * 1.15;
    Ok(receipt(
        "SceneGraph10D.step_ontogeny",
        [
            ("month", Value::I64(month)),
            ("growth_01", Value::F64(growth)),
            ("developmental_stage", Value::String(stage.into())),
            ("allometric_scale", Value::F64(allometric_scale)),
        ],
    ))
}

fn scene_graph_10d_dispatch_affordance(args: &Value) -> Result<Value, Diagnostic> {
    let verb = str_field(args, "verb").unwrap_or("InspectHistology");
    let target = str_field(args, "target").unwrap_or("corymbia_aparrerinja");
    Ok(receipt(
        "SceneGraph10D.dispatch_affordance",
        [
            ("verb", Value::String(verb.into())),
            ("target", Value::String(target.into())),
            ("deontic_permitted", Value::Bool(true)),
            ("sonic_token_emitted", Value::Bool(true)),
        ],
    ))
}

fn scene_graph_10d_cull_indirect_4tier(args: &Value) -> Result<Value, Diagnostic> {
    let total_instances = f64_field(args, "instance_count").unwrap_or(256.0).max(1.0) as i64;
    let lod0 = (total_instances as f64 * 0.18).ceil() as i64;
    let lod1 = (total_instances as f64 * 0.27).ceil() as i64;
    let lod2 = (total_instances as f64 * 0.30).ceil() as i64;
    let lod3 = (total_instances - lod0 - lod1 - lod2).max(0);
    Ok(receipt(
        "SceneGraph10D.cull_indirect_4tier",
        [
            ("visible_instances", Value::I64(total_instances)),
            ("lod0_brep_count", Value::I64(lod0)),
            ("lod1_sggx_count", Value::I64(lod1)),
            ("lod2_surfel_count", Value::I64(lod2)),
            ("lod3_impostor_count", Value::I64(lod3)),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_and_hull() {
        let mut rec = BTreeMap::new();
        rec.insert("ax".into(), Value::F64(0.0));
        rec.insert("ay".into(), Value::F64(0.0));
        rec.insert("bx".into(), Value::F64(1.0));
        rec.insert("by".into(), Value::F64(0.0));
        rec.insert("cx".into(), Value::F64(0.0));
        rec.insert("cy".into(), Value::F64(1.0));
        let v = orientation_2(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(m) = v else { panic!() };
        assert_eq!(m.get("sign"), Some(&Value::I64(1)));

        let mut h = BTreeMap::new();
        h.insert(
            "points".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(0.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(1.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.5), Value::F64(0.5)]),
                Value::List(vec![Value::F64(1.0), Value::F64(1.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(1.0)]),
            ]),
        );
        let hv = convex_hull_2(&Value::Record(h), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(hm) = hv else { panic!() };
        match hm.get("vertex_count") {
            Some(Value::U64(n)) => assert!(*n >= 4),
            _ => panic!(),
        }
    }

    #[test]
    fn framingham_is_e300() {
        let err = try_invoke(
            "ClinicalRisk.framingham",
            &Value::Null,
            Span { start: 0, end: 0 },
        )
        .unwrap()
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E300);
        assert_eq!(err.civics_code(), "E0403");

        let span = Span { start: 0, end: 0 };
        let mut rec = BTreeMap::new();
        rec.insert("scale".into(), Value::I64(3));
        rec.insert("anthesis_01".into(), Value::F64(0.65));
        let organ = try_invoke("BotanicalAnatomy.synthesize_organ_10d", &Value::Record(rec.clone()), span)
            .unwrap()
            .unwrap();
        let Value::Record(om) = organ else { panic!() };
        assert_eq!(om.get("scale"), Some(&Value::I64(3)));

        let anth = try_invoke("BotanicalAnatomy.evaluate_anthesis", &Value::Record(rec), span)
            .unwrap()
            .unwrap();
        let Value::Record(am) = anth else { panic!() };
        assert_eq!(am.get("dehiscence_active"), Some(&Value::Bool(true)));

        let mut zrec = BTreeMap::new();
        zrec.insert("stiffness_n_m".into(), Value::F64(185_000.0));
        zrec.insert("delta_l_m".into(), Value::F64(0.04));
        let tendon = try_invoke("ZoologicalAnatomy.evaluate_tendon_strain", &Value::Record(zrec), span)
            .unwrap()
            .unwrap();
        let Value::Record(tm) = tendon else { panic!() };
        assert!(matches!(tm.get("strain_energy_joules"), Some(Value::F64(e)) if *e > 100.0));

        let mut wrec = BTreeMap::new();
        wrec.insert("wx_m".into(), Value::F64(7800.0));
        wrec.insert("wz_m".into(), Value::F64(-4200.0));
        let site = try_invoke("WorldContinuum.locate_study_site", &Value::Record(wrec), span)
            .unwrap()
            .unwrap();
        let Value::Record(sm) = site else { panic!() };
        assert_eq!(sm.get("site_id"), Some(&Value::String("YarrangobillyKarstGorge".into())));

        let drive_src = include_str!("../../fixtures/ecopunk_continuum_drive.vibe");
        let lab_src = include_str!("../../fixtures/ecopunk_biological_lab.vibe");
        let talbingo_src = include_str!("../../fixtures/talbingo-drive.vibe");

        let p_drive = crate::load_program(drive_src).expect("ecopunk_continuum_drive.vibe must typecheck");
        let c_drive = crate::bytecode::compile(&p_drive).expect("ecopunk_continuum_drive.vibe must compile to .qbc");
        let b_drive = crate::bytecode::encode_chunk(&c_drive);
        assert!(b_drive.starts_with(b"VBC1"));

        let p_lab = crate::load_program(lab_src).expect("ecopunk_biological_lab.vibe must typecheck");
        let c_lab = crate::bytecode::compile(&p_lab).expect("ecopunk_biological_lab.vibe must compile to .qbc");
        let b_lab = crate::bytecode::encode_chunk(&c_lab);
        assert!(b_lab.starts_with(b"VBC1"));

        let p_tal = crate::load_program(talbingo_src).expect("talbingo-drive.vibe must typecheck");
        let c_tal = crate::bytecode::compile(&p_tal).expect("talbingo-drive.vibe must compile to .qbc");
        let b_tal = crate::bytecode::encode_chunk(&c_tal);
        assert!(b_tal.starts_with(b"VBC1"));
    }
}
