//! Schema block validators for QAC v1 profiles registry (Schemas 200–210).

use crate::q42::{field_envelope::Block, field_semantics as sem};
use super::*;

pub(super) fn validate_medium(block: Block<'_>, ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
    let model = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    if model.is_empty() {
        return Err(sem::Error::Value);
    }
    let density = field(block, 2).map(f64_val).transpose()?;
    let bulk = field(block, 3).map(f64_val).transpose()?;
    let shear = field(block, 4).map(f64_val).transpose()?;
    if let Some(d) = density {
        if d <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(k) = bulk {
        if k <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(mu) = shear {
        if mu < 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(t) = field(block, 5).map(f64_val).transpose()? {
        if t <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(p0) = field(block, 6).map(f64_val).transpose()? {
        if p0 <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    match model {
        "fluid-linear" => {
            if density.is_none() || bulk.is_none() {
                return Err(sem::Error::MissingField);
            }
        }
        "elastic-isotropic" => {
            if density.is_none() || bulk.is_none() || shear.is_none() {
                return Err(sem::Error::MissingField);
            }
        }
        "vacuum-mask" => {
            let mask_ref = field(block, 13).ok_or(sem::Error::MissingField)?;
            let entry_idx = u32_val(mask_ref)?;
            let mask_entry = ctx.entry(entry_idx)?;
            if mask_entry.mode() != 0x0106 || mask_entry.stride() != 1 {
                return Err(sem::Error::Validity);
            }
        }
        "unknown-medium" => {
            // Explicitly unknown medium never pretends to be a vacuum mask or positive density.
            if field(block, 13).is_some() {
                return Err(sem::Error::Value);
            }
        }
        _ => {
            // External / anisotropic / poroelastic / viscous URI must stay structurally sound;
            // table digests and epochs bind external material mappings symmetrically.
            if field(block, 9).is_some() ^ field(block, 10).is_some() {
                return Err(sem::Error::Value);
            }
        }
    }
    if let Some(ids) = field(block, 11) {
        if ids.element_count() == 0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(ext) = field(block, 12) {
        let target_id = u32_val(ext)?;
        let _ = find_block_by_schema(ctx, target_id, 205)?;
    }
    Ok(())
}

pub(super) fn validate_transform(block: Block<'_>) -> Result<(), sem::Error> {
    let kind = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    match kind {
        "STFT-complex" | "STFT-magnitude" | "CQT-complex" | "CQT-magnitude" | "legacy-unknown" => {}
        _ => return Err(sem::Error::Value),
    }
    let bins = u32_val(field(block, 13).ok_or(sem::Error::MissingField)?)?;
    if bins == 0 {
        return Err(sem::Error::Value);
    }
    if let Some(one_sided) = field(block, 7).map(u32_val).transpose()? {
        if one_sided > 1 {
            return Err(sem::Error::Value);
        }
    }
    if let (Some(n), Some(d)) = (
        field(block, 14).map(u64_val).transpose()?,
        field(block, 15).map(u64_val).transpose()?,
    ) {
        validate_reduced_rate(n, d)?;
    } else if field(block, 14).is_some() ^ field(block, 15).is_some() {
        return Err(sem::Error::Time);
    }
    if let Some(coeffs) = field(block, 16) {
        if coeffs.value().is_empty() || coeffs.value().len() % 4 != 0 {
            return Err(sem::Error::Value);
        }
        for c in coeffs.value().chunks_exact(4) {
            let v = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            if !v.is_finite() {
                return Err(sem::Error::Value);
            }
        }
    }
    if let Some(origin) = field(block, 17).map(text_val).transpose()? {
        match origin {
            "epoch-sample-grid" | "source-entry-first-sample" => {}
            _ => return Err(sem::Error::Value),
        }
    }
    Ok(())
}

pub(super) fn validate_calibration(block: Block<'_>, ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
    let qty = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    let units = text_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
    if qty.is_empty() || units.is_empty() {
        return Err(sem::Error::Value);
    }
    let scale = f64_val(field(block, 3).ok_or(sem::Error::MissingField)?)?;
    if scale == 0.0 {
        return Err(sem::Error::Value);
    }
    let _offset = f64_val(field(block, 4).ok_or(sem::Error::MissingField)?)?;
    if let Some(p_ref) = field(block, 5).map(f64_val).transpose()? {
        if p_ref <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    let from = field(block, 9).map(u32_val).transpose()?;
    let until = field(block, 10).map(u32_val).transpose()?;
    if let Some(id) = from {
        let _ = find_block_by_schema(ctx, id, 5)?;
    }
    if let Some(id) = until {
        let _ = find_block_by_schema(ctx, id, 5)?;
    }
    Ok(())
}

pub(super) fn validate_filterbank(block: Block<'_>) -> Result<(), sem::Error> {
    let uri = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    let ver = text_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
    let bands = u32_val(field(block, 3).ok_or(sem::Error::MissingField)?)?;
    let scale = text_val(field(block, 4).ok_or(sem::Error::MissingField)?)?;
    if uri.is_empty() || ver.is_empty() || bands == 0 {
        return Err(sem::Error::Value);
    }
    match scale {
        "linear-energy"
        | "log-power"
        | "dB-SPL"
        | "mel"
        | "MFCC"
        | "sone"
        | "LUFS"
        | "gammatone-energy"
        | "erb-energy"
        | "bark-energy" => {}
        _ => return Err(sem::Error::Value),
    }
    if let Some(p_ref) = field(block, 8).map(f64_val).transpose()? {
        if p_ref <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(gp) = field(block, 10).map(text_val).transpose()? {
        if gp.is_empty() {
            return Err(sem::Error::Value);
        }
    }
    Ok(())
}

pub(super) fn validate_spatial_hrir(block: Block<'_>, ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
    let repr = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    match repr {
        "HOA" => {
            let order = u32_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
            if order > 3 {
                return Err(sem::Error::Value);
            }
            let ordering = text_val(field(block, 3).ok_or(sem::Error::MissingField)?)?;
            if ordering != "ACN" && ordering != "FuMa" {
                return Err(sem::Error::Value);
            }
            let norm = text_val(field(block, 4).ok_or(sem::Error::MissingField)?)?;
            if norm != "SN3D" && norm != "N3D" && norm != "maxN" {
                return Err(sem::Error::Value);
            }
            if ordering == "FuMa" && norm == "SN3D" {
                return Err(sem::Error::Value);
            }
        }
        "HRIR" | "RIR" => {
            let taps = u32_val(field(block, 8).ok_or(sem::Error::MissingField)?)?;
            let receivers = u32_val(field(block, 9).ok_or(sem::Error::MissingField)?)?;
            let measurements = u32_val(field(block, 10).ok_or(sem::Error::MissingField)?)?;
            if taps == 0 || receivers == 0 || measurements == 0 {
                return Err(sem::Error::Value);
            }
            let num = u64_val(field(block, 17).ok_or(sem::Error::MissingField)?)?;
            let den = u64_val(field(block, 18).ok_or(sem::Error::MissingField)?)?;
            validate_reduced_rate(num, den)?;
            let delay_ref = text_val(field(block, 19).ok_or(sem::Error::MissingField)?)?;
            if delay_ref != "relative-to-excitation-origin" && delay_ref != "legacy-unknown" {
                return Err(sem::Error::Value);
            }
        }
        _ => return Err(sem::Error::Value),
    }
    if let Some(frame_id) = field(block, 13).map(u32_val).transpose()? {
        let _ = find_block_by_schema(ctx, frame_id, 2)?;
    }
    Ok(())
}

pub(super) fn validate_model_grid(block: Block<'_>) -> Result<(), sem::Error> {
    let uri = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    let ver = text_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
    let names = text_val(field(block, 15).ok_or(sem::Error::MissingField)?)?;
    let shape = field(block, 14).ok_or(sem::Error::MissingField)?;
    if uri.is_empty() || ver.is_empty() || names.is_empty() || shape.element_count() == 0 {
        return Err(sem::Error::Value);
    }
    let mut prod: u64 = 1;
    for chunk in shape.value().chunks_exact(8) {
        let dim = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
        if dim == 0 {
            return Err(sem::Error::Value);
        }
        prod = prod.checked_mul(dim).ok_or(sem::Error::Overflow)?;
    }
    let _ = prod;
    if let (Some(tn), Some(td)) = (
        field(block, 10).map(u64_val).transpose()?,
        field(block, 11).map(u64_val).transpose()?,
    ) {
        validate_reduced_rate(tn, td)?;
    } else if field(block, 10).is_some() ^ field(block, 11).is_some() {
        return Err(sem::Error::Time);
    }
    if let (Some(off_n), Some(off_d)) = (field(block, 5), field(block, 16)) {
        if off_n.element_count() != off_d.element_count() {
            return Err(sem::Error::Time);
        }
        for chunk in off_d.value().chunks_exact(8) {
            let d = u64::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            if d == 0 || d > MAX_CLOCK_DEN {
                return Err(sem::Error::Time);
            }
        }
    } else if field(block, 5).is_some() ^ field(block, 16).is_some() {
        return Err(sem::Error::Time);
    }
    if let Some(tol) = field(block, 12).map(f64_val).transpose()? {
        if tol < 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(tol) = field(block, 13).map(f64_val).transpose()? {
        if tol < 0.0 {
            return Err(sem::Error::Value);
        }
    }
    Ok(())
}

pub(super) fn validate_ray_modal(block: Block<'_>, ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
    let kind = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    match kind {
        "geometric_rays" | "modal_bank" | "hybrid_path" => {}
        _ => return Err(sem::Error::Value),
    }
    if let Some(gq) = field(block, 3).map(text_val).transpose()? {
        if gq != "energy-ratio" && gq != "pressure-amplitude-ratio" {
            return Err(sem::Error::Value);
        }
    } else if kind != "modal_bank" {
        return Err(sem::Error::MissingField);
    }
    if let Some(d0) = field(block, 4).map(f64_val).transpose()? {
        if d0 <= 0.0 {
            return Err(sem::Error::Value);
        }
    } else if kind != "modal_bank" {
        return Err(sem::Error::MissingField);
    }
    if let Some(dm) = field(block, 10).map(text_val).transpose()? {
        if dm != "none" && dm != "UTD-wedge" && dm != "BTM-aperture" {
            return Err(sem::Error::Value);
        }
    }
    if let Some(launch_id) = field(block, 5).map(u32_val).transpose()? {
        let _ = find_block_by_schema(ctx, launch_id, 5)?;
    }
    Ok(())
}

pub(super) fn validate_haptic(block: Block<'_>, ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
    let role = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    match role {
        "measurement" | "receptor-model" | "actuator-command" => {}
        _ => return Err(sem::Error::Value),
    }
    let frame_id = u32_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
    let _ = find_block_by_schema(ctx, frame_id, 2)?;
    if let Some(area) = field(block, 3).map(f64_val).transpose()? {
        if area <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let Some(cal_id) = field(block, 8).map(u32_val).transpose()? {
        let _ = find_block_by_schema(ctx, cal_id, 202)?;
    }
    if let Some(exp_id) = field(block, 7).map(u32_val).transpose()? {
        let _ = find_block_by_schema(ctx, exp_id, 5)?;
    }
    if role == "receptor-model" && field(block, 4).is_none() {
        return Err(sem::Error::MissingField);
    }
    if role == "actuator-command"
        && (field(block, 5).is_none() || field(block, 6).is_none() || field(block, 7).is_none())
    {
        // Schema mandates explicit device profile, authority receipt, and expiry instant.
        // Storage remains passive inspection only and does not act as a hardware actuation gate.
        return Err(sem::Error::MissingField);
    }
    Ok(())
}

pub(super) fn validate_ultrasound(block: Block<'_>, ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
    let role = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    match role {
        "raw-RF" | "baseband-IQ" | "beam-summary" | "transmit-command" => {}
        _ => return Err(sem::Error::Value),
    }
    if let Some(fc) = field(block, 2).map(f64_val).transpose()? {
        if fc <= 0.0 {
            return Err(sem::Error::Value);
        }
    }
    if let (Some(pn), Some(pd)) = (
        field(block, 7).map(u64_val).transpose()?,
        field(block, 8).map(u64_val).transpose()?,
    ) {
        validate_reduced_rate(pn, pd)?;
    } else if field(block, 7).is_some() ^ field(block, 8).is_some() {
        return Err(sem::Error::Time);
    }
    if let Some(frame_id) = field(block, 16).map(u32_val).transpose()? {
        let _ = find_block_by_schema(ctx, frame_id, 2)?;
    }
    Ok(())
}

pub(super) fn validate_legacy_retention(
    block: Block<'_>,
    ctx: &sem::Context<'_, '_>,
) -> Result<(), sem::Error> {
    let fam = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    let ver = u32_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
    let entry_idx = u32_val(field(block, 3).ok_or(sem::Error::MissingField)?)?;
    let alg = text_val(field(block, 5).ok_or(sem::Error::MissingField)?)?;
    let alg_ver = text_val(field(block, 6).ok_or(sem::Error::MissingField)?)?;
    if fam != "Q4AU" || !(1..=3).contains(&ver) || alg.is_empty() || alg_ver.is_empty() {
        return Err(sem::Error::Value);
    }
    let orig = ctx.entry(entry_idx)?;
    if orig.mode() != 0x0104 || orig.stride() != 1 {
        return Err(sem::Error::Reference);
    }
    Ok(())
}

pub(super) fn validate_signal_encoding(block: Block<'_>) -> Result<(), sem::Error> {
    let channels = text_val(field(block, 1).ok_or(sem::Error::MissingField)?)?;
    let scale = f64_val(field(block, 2).ok_or(sem::Error::MissingField)?)?;
    let _offset = f64_val(field(block, 3).ok_or(sem::Error::MissingField)?)?;
    let purpose = text_val(field(block, 4).ok_or(sem::Error::MissingField)?)?;
    let rel = text_val(field(block, 5).ok_or(sem::Error::MissingField)?)?;
    if channels.is_empty() || purpose.is_empty() || scale <= 0.0 {
        return Err(sem::Error::Value);
    }
    match rel {
        "digital-unrelated-to-pressure" | "raw-counts" | "physical-base-units" => Ok(()),
        _ => Err(sem::Error::Value),
    }
}
