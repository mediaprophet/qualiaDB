//! Exact-byte QAC browser bridge. Playback requires a separate explicit conversion.
use crate::audio::qac::{
    metadata::Transform,
    signal,
};
use crate::q42::{
    field_envelope::{Family, StructuralView},
    field_semantics::{field, NodeState},
};
use crate::render::portal_qac::{provenance_role_str, Limits as QacLimits, RetainedQac};
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::JsValue;
fn set(o: &Object, key: &str, value: impl Into<JsValue>) -> Result<(), JsValue> {
    Reflect::set(o, &key.into(), &value.into()).map(|_| ())
}
fn entry_epistemic_role(view: &StructuralView<'_>, descriptor_id: u32) -> &'static str {
    let Some(desc) = view.blocks().find(|b| b.id() == descriptor_id) else {
        return "unknown";
    };
    let Some(prov_ref) = field(desc, 3).and_then(|f| f.value().try_into().ok()).map(u32::from_le_bytes) else {
        return "unknown";
    };
    let Some(prov) = view.blocks().find(|b| b.id() == prov_ref) else {
        return "unknown";
    };
    field(prov, 1)
        .and_then(|f| f.value().try_into().ok())
        .map(u32::from_le_bytes)
        .and_then(provenance_role_str)
        .unwrap_or("unknown")
}
fn write_transform(
    o: &Object,
    t: &Transform,
    window_coefficients_le: Option<&[u8]>,
) -> Result<(), JsValue> {
    let transform = Object::new();
    set(&transform, "kind", format!("{:?}", t.kind))?;
    set(&transform, "fft_size", f64::from(t.fft_size))?;
    set(&transform, "hop", f64::from(t.hop))?;
    set(&transform, "bins", f64::from(t.bins))?;
    set(
        &transform,
        "input_rate_numerator",
        t.input_rate.numerator.to_string(),
    )?;
    set(
        &transform,
        "input_rate_denominator",
        t.input_rate.denominator.to_string(),
    )?;
    set(&transform, "origin_samples_decimal", t.origin_samples.to_string())?;
    set(&transform, "window", t.window.as_str())?;
    set(&transform, "window_kind", t.window.as_str())?;
    set(&transform, "origin_basis", t.origin_basis.as_str())?;
    set(o, "window_kind", t.window.as_str())?;
    set(o, "origin_basis", t.origin_basis.as_str())?;
    if let Some(coeffs) = window_coefficients_le {
        set(&transform, "window_coefficients_le", Uint8Array::from(coeffs))?;
    }
    set(
        &transform,
        "bin_order",
        "full FFT order; even Nyquist negative; signed offsets, not a carrier",
    )?;
    set(
        &transform,
        "convention",
        "forward-negative-exponent; forward-unscaled-inverse-1/N; no padding",
    )?;
    set(o, "transform", transform)
}
pub(super) fn metadata(qac: &RetainedQac) -> Result<Object, JsValue> {
    let m = qac.metadata();
    let o = Object::new();
    for (key, value) in [
        ("selected_entry", m.selected_entry),
        ("descriptor_id", m.descriptor_id),
        ("clock_id", m.clock_id),
        ("provenance_id", m.provenance_id),
        ("frame_id", m.frame_id),
        ("stream_id", m.stream_id),
        ("channels", m.channels),
        ("bins", m.bins),
        ("mode", u32::from(m.mode)),
    ] {
        set(&o, key, f64::from(value))?;
    }
    for (key, value) in [
        ("channel_order", m.channel_order.as_ref()),
        ("purpose", m.purpose.as_ref()),
        ("relation", m.relation.as_ref()),
        ("quantity", m.quantity.as_ref()),
        ("units", m.units.as_ref()),
        ("model_uri", m.model_uri.as_ref()),
        ("epistemic", m.epistemic_role),
        ("provenance_role", m.epistemic_role),
    ] {
        set(&o, key, value)?;
    }
    set(&o, "scalar", format!("{:?}", m.scalar))?;
    set(&o, "records_decimal", m.records.to_string())?;
    set(&o, "units_per_stored_sample", m.units_per_stored_sample)?;
    set(&o, "stored_offset", m.stored_offset)?;
    let t = m.timing;
    set(&o, "clock_scale", f64::from(t.scale))?;
    set(&o, "epoch_seconds_decimal", t.epoch_seconds.to_string())?;
    set(&o, "epoch_nanoseconds", f64::from(t.epoch_nanoseconds))?;
    set(&o, "start_tick_decimal", t.start_tick.to_string())?;
    set(
        &o,
        "ticks_per_second_numerator",
        t.ticks.numerator.to_string(),
    )?;
    set(
        &o,
        "ticks_per_second_denominator",
        t.ticks.denominator.to_string(),
    )?;
    set(&o, "sample_rate_numerator", t.samples.numerator.to_string())?;
    set(
        &o,
        "sample_rate_denominator",
        t.samples.denominator.to_string(),
    )?;
    set(&o, "rate_num", t.samples.numerator.to_string())?;
    set(&o, "rate_den", t.samples.denominator.to_string())?;
    if let Some(t) = m.transform {
        write_transform(&o, &t, m.window_coefficients_le.as_deref())?;
    } else {
        set(&o, "window_kind", "none")?;
        set(&o, "origin_basis", "epoch-sample-grid")?;
        set(&o, "transform", JsValue::NULL)?;
    }
    set(
        &o,
        "source_blake3",
        blake3::hash(qac.source_bytes()).to_hex().as_str(),
    )?;
    set(&o, "admission", "complete current typed signal document")?;
    set(
        &o,
        "playback",
        "none; original samples are not implicitly normalized or converted",
    )?;
    Ok(o)
}
fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
    out
}
pub(super) fn inspect_container(bytes: &[u8]) -> Result<Object, JsValue> {
    if bytes.starts_with(b"Q4AU") || bytes.starts_with(b"UA4Q") {
        return Err(JsValue::from_str(
            "legacy Q4AU/UA4Q container refused: current .qac v1 magic qac\\0\\0\\0\\0\\0 required (no silent transcoding or alias inflation)",
        ));
    }
    let mut limits = QacLimits::default();
    limits.envelope.file_bytes = 128 << 20;
    if bytes.len() as u64 > limits.envelope.file_bytes {
        return Err(JsValue::from_str("qac browser source exceeds 128 MiB"));
    }
    let view = StructuralView::open(bytes, Family::QacV1Candidate, limits.envelope)
        .map_err(|e| JsValue::from_str(&format!("qac structural: {e:?}")))?;
    if view.entry_count() > limits.signal.entries {
        return Err(JsValue::from_str("qac entry count exceeds limit"));
    }
    let blocks = view.blocks().count();
    let mut metadata_scratch = vec![NodeState::default(); blocks];
    let mut entry_scratch = vec![NodeState::default(); view.entry_count() as usize];
    let entries = Array::new();
    for index in 1..=view.entry_count() {
        let e = view
            .entry(index)
            .map_err(|err| JsValue::from_str(&format!("qac entry {index}: {err:?}")))?;
        let item = Object::new();
        let epistemic = entry_epistemic_role(&view, e.descriptor_id());
        set(&item, "index", f64::from(index))?;
        set(&item, "mode", f64::from(u32::from(e.mode())))?;
        set(&item, "revision", f64::from(u32::from(e.revision())))?;
        set(&item, "stream_id", f64::from(e.stream_id()))?;
        set(&item, "frame_id", f64::from(e.frame_id()))?;
        set(&item, "start_tick_decimal", e.start_tick().to_string())?;
        set(&item, "time_count_decimal", e.time_count().to_string())?;
        set(&item, "record_count_decimal", e.record_count().to_string())?;
        set(&item, "records_decimal", e.record_count().to_string())?;
        set(&item, "stride", f64::from(e.stride()))?;
        set(&item, "descriptor_id", f64::from(e.descriptor_id()))?;
        set(&item, "stored_bytes_decimal", e.stored_bytes().to_string())?;
        set(&item, "decoded_bytes_decimal", e.decoded_bytes().to_string())?;
        set(&item, "blake3_hex", hex_lower(e.digest()))?;
        set(&item, "epistemic", epistemic)?;
        set(&item, "provenance_role", epistemic)?;
        set(&item, "supported_candidate", matches!(e.mode(), 0x100 | 0x101))?;
        let mut handled_signal = false;
        if matches!(e.mode(), 0x100 | 0x101 | 0x104) {
            metadata_scratch.fill(NodeState::default());
            entry_scratch.fill(NodeState::default());
            match signal::admit(
                &view,
                index,
                &mut crate::q42::field_semantics::Scratch {
                    metadata: &mut metadata_scratch,
                    entries: &mut entry_scratch,
                },
                limits.shared,
                limits.signal,
            ) {
                Ok(sig) => {
                    handled_signal = true;
                    set(&item, "admitted", true)?;
                    set(&item, "profile_only", false)?;
                    set(
                        &item,
                        "profile_kind",
                        match e.mode() {
                            0x100 => "pcm-preservation",
                            0x101 => "stft-preservation",
                            _ => "opaque-source",
                        },
                    )?;
                    set(&item, "scalar", format!("{:?}", sig.scalar()))?;
                    set(&item, "channels", f64::from(sig.channels()))?;
                    set(&item, "bins", f64::from(sig.bins()))?;
                    set(&item, "quantity", sig.quantity())?;
                    set(&item, "units", sig.units())?;
                    set(&item, "purpose", sig.purpose())?;
                    set(&item, "relation", sig.relation())?;
                    if let Some(tm) = sig.timing() {
                        set(&item, "rate_num", tm.samples.numerator.to_string())?;
                        set(&item, "rate_den", tm.samples.denominator.to_string())?;
                        set(&item, "sample_rate_numerator", tm.samples.numerator.to_string())?;
                        set(&item, "sample_rate_denominator", tm.samples.denominator.to_string())?;
                    }
                    if let Some(t) = sig.transform() {
                        write_transform(&item, &t, sig.window_coefficients_le())?;
                    } else {
                        set(&item, "window_kind", "none")?;
                        set(&item, "origin_basis", "epoch-sample-grid")?;
                    }
                }
                Err(err) if matches!(e.mode(), 0x100 | 0x101) => {
                    return Err(JsValue::from_str(&format!("qac admit entry {index}: {err:?}")));
                }
                Err(_) => {}
            }
        }
        if !handled_signal {
            use crate::audio::qac::profiles::{AdmittedEnvelope, ProfileLimits};
            let prof_limits = ProfileLimits {
                envelope: limits.envelope,
                semantics: limits.shared,
                ..ProfileLimits::default()
            };
            let env = AdmittedEnvelope::open_alloc(bytes, prof_limits)
                .map_err(|err| JsValue::from_str(&format!("qac profile admit: {err:?}")))?;
            let summary = env
                .inspect_entry_summary(index)
                .map_err(|err| JsValue::from_str(&format!("qac entry {index} summary: {err:?}")))?;
            set(&item, "admitted", true)?;
            set(&item, "profile_only", true)?;
            set(&item, "profile_kind", summary.kind.as_str())?;
            set(&item, "quantity", summary.quantity)?;
            set(&item, "units", summary.units)?;
            set(&item, "purpose", summary.purpose)?;
            set(&item, "has_validity_mask", summary.has_validity_mask)?;
            set(&item, "window_kind", "none")?;
            set(&item, "origin_basis", "domain-profile")?;
            if let Some(ch) = summary.channels {
                set(&item, "channels", f64::from(ch))?;
            }
            if let Some(uri) = summary.model_uri {
                set(&item, "model_uri", uri)?;
            }
            set(
                &item,
                "playback",
                "none; domain profile inspection only (no implicit waveform synthesis)",
            )?;
        }
        entries.push(&item);
    }
    let out = Object::new();
    set(&out, "entry_count", f64::from(view.entry_count()))?;
    set(&out, "block_count", blocks as f64)?;
    set(&out, "source_blake3", blake3::hash(bytes).to_hex().as_str())?;
    set(&out, "metadata_tlv_le", Uint8Array::from(view.metadata_bytes()))?;
    set(&out, "entries", entries)?;
    Ok(out)
}
pub(super) fn window(
    qac: &RetainedQac,
    start: u64,
    count: u32,
    channel: u32,
) -> Result<Object, JsValue> {
    let fail = |e| JsValue::from_str(&format!("qac window: {e:?}"));
    let m = qac.metadata();
    let byte_count = u64::from(count)
        .checked_mul(m.channels.into())
        .and_then(|n| n.checked_mul(m.bins.into()))
        .and_then(|n| n.checked_mul(m.scalar.bytes().into()))
        .ok_or_else(|| JsValue::from_str("qac window size overflow"))?;
    if byte_count > 16 << 20 {
        return Err(JsValue::from_str("qac browser window exceeds 16 MiB"));
    }
    let bytes = match m.mode {
        0x100 => Uint8Array::from(
            qac.pcm_window(start, count.into(), channel)
                .map_err(fail)?
                .original_interleaved_bytes(),
        ),
        0x101 => Uint8Array::from(
            qac.stft_window(start, count.into(), channel)
                .map_err(fail)?
                .original_interleaved_bytes(),
        ),
        _ => return Err(JsValue::from_str("unsupported qac window profile")),
    };
    let o = Object::new();
    set(&o, "original_interleaved_bytes", bytes)?;
    set(&o, "start_record_decimal", start.to_string())?;
    set(&o, "records", f64::from(count))?;
    set(&o, "selected_channel", f64::from(channel))?;
    set(&o, "channels", f64::from(m.channels))?;
    set(&o, "bins", f64::from(m.bins))?;
    set(&o, "scalar", format!("{:?}", m.scalar))?;
    set(
        &o,
        "layout",
        if m.mode == 0x100 {
            "time-channel"
        } else {
            "frame-channel-bin"
        },
    )?;
    set(
        &o,
        "byte_order",
        "little-endian; bytes include all interleaved channels",
    )?;
    if count > 0 {
        let time = qac.time_at(start).map_err(fail)?;
        set(
            &o,
            "first_time_seconds_numerator",
            time.numerator.to_string(),
        )?;
        set(
            &o,
            "first_time_seconds_denominator",
            time.denominator.to_string(),
        )?;
    }
    Ok(o)
}
