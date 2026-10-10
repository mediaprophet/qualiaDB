//! Generation-stamped HMC texture working-set planning and application.

use crate::volumetric_texture::coarse_texture_level;
use qualia_core_db::bundle::BundleReader;
use qualia_core_db::render::gpu::{TextureColorSpace, TextureMipSemantic};
use qualia_core_db::render::texture_ktx2::Ktx2Document;
use qualia_core_db::render::texture_stream_policy::{
    plan_texture_residency_lifecycle, TextureStreamBudget, TextureStreamDemand,
    TextureStreamLifecycleAction, TextureStreamLifecycleActionKind,
};
use qualia_core_db::render::texture_streaming_plan::{
    decode_rgba8_ktx2_mip_into, rgba8_ktx2_mip_candidate,
};

use super::{is_ktx2_mime, ktx2_backend_cost, TextureUse};

pub(super) const HMC_TEXTURE_STREAM_STATE_MAX_BYTES: usize = 256 * 1024 * 1024;
pub(super) const HMC_TEXTURE_STREAM_MAX_ENTRIES: usize = 1_048_576;
const HMC_TEXTURE_STREAM_PREPARE_MAX_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HmcTextureStreamEntry {
    pub(super) semantic_id: u64,
    pub(super) digest: [u8; 32],
    pub(super) texture_use: TextureUse,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) mip_count: u8,
    pub(super) supported_mips: u32,
    pub(super) resident_finest_mip: Option<u8>,
    pub(super) resident_bytes: u64,
}

pub(crate) struct HmcTextureStreamState {
    pub(super) bundle_bytes: Vec<u8>,
    pub(super) asset_bytes: Vec<u8>,
    pub(super) entries: Vec<HmcTextureStreamEntry>,
    pub(super) generation: u64,
}

/// One member of the next texture working set. The request list is authoritative: entries not
/// present in it are unrequested and are evicted by the apply step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HmcTextureResidencyRequest {
    pub digest: [u8; 32],
    pub color_space: TextureColorSpace,
    pub mip_semantic: TextureMipSemantic,
    pub projected_width: u32,
    pub projected_height: u32,
    pub importance: u16,
    pub distance_key: u32,
    pub last_used_frame: u64,
    pub pinned: bool,
    pub visible: bool,
}

/// A complete working-set request for a previously loaded HMC asset.
pub struct HmcTextureStreamApplyRequest<'a> {
    pub budget: TextureStreamBudget,
    pub requests: &'a [HmcTextureResidencyRequest],
}

/// A generation-stamped, caller-requested texture transition plan. It is intentionally separate
/// from the renderer so a stale plan cannot mutate a newer asset generation.
#[derive(Clone)]
pub struct HmcTextureStreamPlan {
    pub generation: u64,
    pub action_count: usize,
    pub admitted_count: usize,
    pub deferred_count: usize,
    pub admitted_upload_bytes: u64,
    pub deferred_upload_bytes: u64,
    pub projected_resident_bytes: u64,
    state_fingerprint: u64,
    actions: Vec<TextureStreamLifecycleAction>,
}

/// Build a generation-stamped lifecycle plan for the currently loaded HMC asset.
///
/// The selector slice is the complete requested working set. Omitting an entry is intentional and
/// causes it to be evicted. Planning is read-only; GPU resources and material bindings change only
/// in [`apply_texture_stream`].
pub fn request_texture_stream(
    renderer: &super::super::VolumetricRenderer,
    request: HmcTextureStreamApplyRequest<'_>,
) -> Result<HmcTextureStreamPlan, String> {
    let state = renderer
        .texture_stream_state
        .as_ref()
        .ok_or_else(|| "no HMC texture stream state is active".to_string())?;

    for (index, selector) in request.requests.iter().enumerate() {
        if request.requests[..index].iter().any(|previous| {
            previous.digest == selector.digest
                && previous.color_space == selector.color_space
                && previous.mip_semantic == selector.mip_semantic
        }) {
            return Err("duplicate HMC texture stream selector".to_string());
        }
        if !state.entries.iter().any(|entry| {
            entry.digest == selector.digest
                && entry.texture_use.color_space == selector.color_space
                && entry.texture_use.mip_semantic == selector.mip_semantic
        }) {
            return Err(
                "HMC texture stream selector has no capability-admitted resident interpretation; \
                 unsupported compressed/Basis data remains on the typed material fallback"
                    .to_string(),
            );
        }
    }

    let mut demands = Vec::new();
    demands
        .try_reserve_exact(state.entries.len())
        .map_err(|_| "HMC texture stream demand allocation refused".to_string())?;
    for entry in &state.entries {
        let selector = request.requests.iter().find(|selector| {
            selector.digest == entry.digest
                && selector.color_space == entry.texture_use.color_space
                && selector.mip_semantic == entry.texture_use.mip_semantic
        });
        let requested = selector.is_some();
        let (desired_finest_mip, pinned, visible, importance, distance_key, last_used_frame) =
            if let Some(selector) = selector {
                let requested_mip = desired_stream_mip(
                    entry.width,
                    entry.height,
                    entry.mip_count,
                    selector.projected_width,
                    selector.projected_height,
                );
                (
                    select_capability_mip(*entry, requested_mip).ok_or_else(|| {
                        format!(
                            "HMC texture capability refusal for {:?}: no authored RGBA8 mip \
                             can satisfy the requested footprint; typed material fallback remains \
                             active",
                            entry.texture_use
                        )
                    })?,
                    selector.pinned,
                    selector.visible,
                    selector.importance,
                    selector.distance_key,
                    selector.last_used_frame,
                )
            } else {
                (
                    entry.resident_finest_mip.unwrap_or(entry.mip_count.saturating_sub(1)),
                    false,
                    false,
                    0,
                    u32::MAX,
                    0,
                )
            };
        demands.push(TextureStreamDemand {
            semantic_id: entry.semantic_id,
            width: entry.width,
            height: entry.height,
            mip_count: entry.mip_count,
            block_width: 1,
            block_height: 1,
            bytes_per_block: 4,
            desired_finest_mip,
            resident_finest_mip: entry.resident_finest_mip,
            resident_bytes: entry.resident_bytes,
            requested,
            pinned,
            visible: requested && visible,
            importance,
            distance_key,
            last_used_frame,
        });
    }

    let mut actions = Vec::new();
    actions
        .try_reserve_exact(state.entries.len())
        .map_err(|_| "HMC texture stream action allocation refused".to_string())?;
    actions.resize(
        state.entries.len(),
        TextureStreamLifecycleAction {
            semantic_id: 0,
            kind: TextureStreamLifecycleActionKind::EvictTexture,
            mip_level: 0,
            bytes: 0,
            previous_resident_bytes: 0,
            target_resident_bytes: 0,
        },
    );
    let lifecycle = plan_texture_residency_lifecycle(
        &demands,
        request.budget,
        state.generation,
        &mut actions,
    )
    .map_err(|error| format!("HMC texture lifecycle plan: {error:?}"))?;
    actions.truncate(lifecycle.action_count);

    Ok(HmcTextureStreamPlan {
        generation: lifecycle.generation,
        action_count: lifecycle.action_count,
        admitted_count: lifecycle.admitted_count,
        deferred_count: lifecycle.deferred_count,
        admitted_upload_bytes: lifecycle.admitted_upload_bytes,
        deferred_upload_bytes: lifecycle.deferred_upload_bytes,
        projected_resident_bytes: lifecycle.projected_resident_bytes,
        state_fingerprint: stream_state_fingerprint(state),
        actions,
    })
}

fn stream_state_fingerprint(state: &HmcTextureStreamState) -> u64 {
    // This is an identity stamp, not a security digest. It prevents a plan from asset A from
    // being applied to a newly loaded asset B whose lifecycle generation also starts at zero.
    let mut fingerprint = 0xcbf29ce484222325u64;
    for byte in state.bundle_bytes.iter().chain(&state.asset_bytes) {
        fingerprint ^= u64::from(*byte);
        fingerprint = fingerprint.wrapping_mul(0x100000001b3);
    }
    for entry in &state.entries {
        fingerprint ^= entry.semantic_id;
        fingerprint = fingerprint.wrapping_mul(0x100000001b3);
        for byte in entry.digest {
            fingerprint ^= u64::from(byte);
            fingerprint = fingerprint.wrapping_mul(0x100000001b3);
        }
    }
    fingerprint
}

fn mip_is_supported(entry: HmcTextureStreamEntry, mip_level: u8) -> bool {
    mip_level < entry.mip_count
        && mip_level < u32::BITS as u8
        && (entry.supported_mips & (1u32 << u32::from(mip_level))) != 0
}

/// Pick the closest authored capability to the requested footprint. Finer authored levels are
/// preferred when no adequate coarse level exists; otherwise the closest coarser level is used.
/// This makes a sparse authored chain a quality fallback rather than a lifecycle failure.
fn select_capability_mip(entry: HmcTextureStreamEntry, requested_mip: u8) -> Option<u8> {
    if entry.mip_count == 0 || entry.mip_count > u32::BITS as u8 {
        return None;
    }
    let requested_mip = requested_mip.min(entry.mip_count - 1);
    for mip_level in (0..=requested_mip).rev() {
        if mip_is_supported(entry, mip_level) {
            return Some(mip_level);
        }
    }
    for mip_level in requested_mip.saturating_add(1)..entry.mip_count {
        if mip_is_supported(entry, mip_level) {
            return Some(mip_level);
        }
    }
    None
}

fn desired_stream_mip(
    width: u32,
    height: u32,
    mip_count: u8,
    projected_width: u32,
    projected_height: u32,
) -> u8 {
    if projected_width == 0 || projected_height == 0 {
        return 0;
    }
    let mut level = 0u8;
    while level + 1 < mip_count {
        let next_width = (width >> u32::from(level + 1)).max(1);
        let next_height = (height >> u32::from(level + 1)).max(1);
        if next_width >= projected_width && next_height >= projected_height {
            level += 1;
        } else {
            break;
        }
    }
    level
}

fn rgba8_mip_bytes(entry: HmcTextureStreamEntry, mip_level: u8) -> Result<u64, String> {
    if mip_level >= entry.mip_count {
        return Err("HMC texture lifecycle mip is outside the authored range".to_string());
    }
    let width = entry
        .width
        .checked_shr(u32::from(mip_level))
        .unwrap_or(0)
        .max(1);
    let height = entry
        .height
        .checked_shr(u32::from(mip_level))
        .unwrap_or(0)
        .max(1);
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "HMC texture lifecycle mip byte size overflow".to_string())
}

fn rgba8_resident_bytes(entry: HmcTextureStreamEntry, first_mip: u8) -> Result<u64, String> {
    if first_mip >= entry.mip_count {
        return Err("HMC texture lifecycle resident mip is outside the authored range".to_string());
    }
    let mut total = 0u64;
    for mip_level in first_mip..entry.mip_count {
        total = total
            .checked_add(rgba8_mip_bytes(entry, mip_level)?)
            .ok_or_else(|| "HMC texture lifecycle resident byte size overflow".to_string())?;
    }
    Ok(total)
}

fn validate_lifecycle_action(
    action: TextureStreamLifecycleAction,
    entry: HmcTextureStreamEntry,
) -> Result<(), String> {
    match action.kind {
        TextureStreamLifecycleActionKind::EvictTexture => {
            if entry.resident_finest_mip.is_none()
                || action.mip_level != 0
                || action.bytes != 0
                || action.previous_resident_bytes != entry.resident_bytes
                || action.target_resident_bytes != 0
            {
                return Err("invalid HMC texture eviction transition".to_string());
            }
        }
        TextureStreamLifecycleActionKind::UploadTexture
        | TextureStreamLifecycleActionKind::ReplaceTexture => {
            if !mip_is_supported(entry, action.mip_level)
                || action.bytes != rgba8_mip_bytes(entry, action.mip_level)?
                || action.target_resident_bytes != rgba8_resident_bytes(entry, action.mip_level)?
                || action.previous_resident_bytes != entry.resident_bytes
            {
                return Err(
                    "invalid HMC texture upload transition or unsupported authored mip".to_string(),
                );
            }
            let expected_kind = if entry.resident_finest_mip.is_some() {
                TextureStreamLifecycleActionKind::ReplaceTexture
            } else {
                TextureStreamLifecycleActionKind::UploadTexture
            };
            if action.kind != expected_kind {
                return Err(
                    "HMC texture lifecycle transition kind does not match residency".to_string(),
                );
            }
        }
    }
    Ok(())
}

struct PreparedTextureTransition {
    action: TextureStreamLifecycleAction,
    entry: HmcTextureStreamEntry,
    new_bytes: Option<Vec<u8>>,
    old_bytes: Option<Vec<u8>>,
}

fn prepare_texture_upload(
    bundle: &BundleReader<'_>,
    entry: HmcTextureStreamEntry,
    mip_level: u8,
) -> Result<Vec<u8>, String> {
    if !mip_is_supported(entry, mip_level) {
        return Err(
            "HMC texture capability refusal: authored compressed/Basis mip is unsupported; \
             the material remains on the typed fallback"
                .to_string(),
        );
    }
    let resource = qualia_core_db::render::asset_package::resolve_hmc_texture_resource(
        bundle,
        &entry.digest,
    )
    .map_err(|error| format!("HMC texture resolution: {error}"))?;
    if is_ktx2_mime(resource.mime_type) {
        let document = Ktx2Document::parse(resource.bytes)
            .map_err(|_| {
                "HMC texture capability refusal: invalid or Basis-compressed KTX2; \
                 the material remains on the typed fallback"
                    .to_string()
            })?;
        let candidate = rgba8_ktx2_mip_candidate(
            &document,
            usize::from(mip_level),
            ktx2_backend_cost(&document, usize::from(mip_level))
                .map_err(|error| format!("HMC texture capability refusal: {error:?}"))?,
        )
        .map_err(|error| {
            format!(
                "HMC texture capability refusal: {error:?}; unsupported compressed/Basis data \
                 remains on the typed material fallback"
            )
        })?;
        if (entry.texture_use.color_space == TextureColorSpace::Srgb) != candidate.is_srgb {
            return Err("HMC texture capability refusal: colour-space mismatch".to_string());
        }
        let length = usize::try_from(candidate.decoded_cpu_bytes)
            .map_err(|_| "HMC texture upload size exceeds usize".to_string())?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| "HMC texture upload allocation refused".to_string())?;
        bytes.resize(length, 0);
        decode_rgba8_ktx2_mip_into(&document, usize::from(mip_level), &mut bytes)
            .map_err(|error| format!("HMC KTX2 decode: {error:?}"))?;
        return Ok(bytes);
    }

    let (rgba8, _) = qualia_core_db::render::texture_decode::decode_hmc_texture_rgba8(
        &resource,
        qualia_core_db::render::texture_decode::TextureDecodeLimits::default(),
    )
    .map_err(|error| {
        format!(
            "HMC texture capability refusal: {error:?}; material remains on the typed fallback"
        )
    })?;
    if mip_level == 0 {
        return Ok(rgba8);
    }
    let (coarse, width, height) = coarse_texture_level(
        &rgba8,
        entry.width,
        entry.height,
        mip_level,
        entry.texture_use.mip_semantic,
    )?;
    let expected = usize::try_from(
        u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "HMC texture coarse byte size overflow".to_string())?,
    )
    .map_err(|_| "HMC texture coarse byte size exceeds usize".to_string())?;
    if coarse.len() != expected {
        return Err("HMC texture coarse decode length mismatch".to_string());
    }
    Ok(coarse)
}

/// Apply a plan atomically with respect to the renderer's logical state and material bindings.
/// Existing resources are retained as rollback payloads until every replacement and the material
/// rebind succeed. The GPU ledger is charged only by the actual upload calls; the plan itself is
/// accounting, so a refinement cannot debit the same allocation twice.
pub fn apply_texture_stream(
    renderer: &mut super::super::VolumetricRenderer,
    plan: HmcTextureStreamPlan,
) -> Result<(), String> {
    // Detach the state for the transaction. This keeps the retained bundle/asset bytes borrowed
    // throughout preparation and avoids cloning the entire HMC state on every refinement frame.
    let Some(mut state) = renderer.texture_stream_state.take() else {
        return Err("no HMC texture stream state is active".to_string());
    };
    if state.generation != plan.generation {
        let error = format!(
            "stale HMC texture stream plan: {} != {}",
            plan.generation, state.generation
        );
        renderer.texture_stream_state = Some(state);
        return Err(error);
    }
    if stream_state_fingerprint(&state) != plan.state_fingerprint {
        renderer.texture_stream_state = Some(state);
        return Err("stale HMC texture stream plan: asset identity changed".to_string());
    }
    let next_generation = match state.generation.checked_add(1) {
        Some(generation) => generation,
        None => {
            renderer.texture_stream_state = Some(state);
            return Err("HMC texture stream generation overflow".to_string());
        }
    };
    let bundle = match BundleReader::parse(&state.bundle_bytes) {
        Ok(bundle) => bundle,
        Err(error) => {
            renderer.texture_stream_state = Some(state);
            return Err(format!("HMC texture stream bundle: {error}"));
        }
    };

    if plan.action_count != plan.actions.len() {
        renderer.texture_stream_state = Some(state);
        return Err("HMC texture stream plan action count is inconsistent".to_string());
    }

    let mut transitions = Vec::new();
    let mut prepared_bytes = 0usize;
    if transitions
        .try_reserve_exact(plan.actions.len())
        .is_err()
    {
        renderer.texture_stream_state = Some(state);
        return Err("HMC texture transition allocation refused".to_string());
    }
    for action in &plan.actions {
        if plan.actions[..transitions.len()].iter().any(|previous| {
            previous.semantic_id == action.semantic_id
        }) {
            renderer.texture_stream_state = Some(state);
            return Err(
                "HMC texture lifecycle plan contains duplicate resource transitions".to_string(),
            );
        }
        let entry = state
            .entries
            .iter()
            .find(|entry| entry.semantic_id == action.semantic_id)
            .copied()
            .ok_or_else(|| "HMC texture lifecycle action references unknown resource".to_string());
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                renderer.texture_stream_state = Some(state);
                return Err(error);
            }
        };
        if let Err(error) = validate_lifecycle_action(*action, entry) {
            renderer.texture_stream_state = Some(state);
            return Err(error);
        }
        let new_bytes = match action.kind {
            TextureStreamLifecycleActionKind::EvictTexture => None,
            TextureStreamLifecycleActionKind::UploadTexture
            | TextureStreamLifecycleActionKind::ReplaceTexture => Some(prepare_texture_upload(
                &bundle,
                entry,
                action.mip_level,
            )),
        };
        let new_bytes = match new_bytes {
            Some(result) => match result {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    renderer.texture_stream_state = Some(state);
                    return Err(error);
                }
            },
            None => None,
        };
        let old_bytes = if entry.resident_finest_mip.is_some() {
            Some(prepare_texture_upload(
                &bundle,
                entry,
                entry.resident_finest_mip.unwrap_or(0),
            ))
        } else {
            None
        };
        let old_bytes = match old_bytes {
            Some(result) => match result {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    renderer.texture_stream_state = Some(state);
                    return Err(error);
                }
            },
            None => None,
        };
        let Some(next_prepared_bytes) = prepared_bytes
            .checked_add(new_bytes.as_ref().map_or(0, |bytes| bytes.len()))
            .and_then(|bytes| {
                bytes.checked_add(old_bytes.as_ref().map_or(0, |old_bytes| old_bytes.len()))
            })
        else {
            renderer.texture_stream_state = Some(state);
            return Err("HMC texture transition staging size overflow".to_string());
        };
        prepared_bytes = next_prepared_bytes;
        if prepared_bytes > HMC_TEXTURE_STREAM_PREPARE_MAX_BYTES {
            let error = format!(
                "HMC texture transition staging exceeds bounded limit: {} > {} bytes",
                prepared_bytes, HMC_TEXTURE_STREAM_PREPARE_MAX_BYTES
            );
            renderer.texture_stream_state = Some(state);
            return Err(error);
        }
        transitions.push(PreparedTextureTransition {
            action: *action,
            entry,
            new_bytes,
            old_bytes,
        });
    }

    if transitions.is_empty() {
        state.generation = next_generation;
        renderer.texture_stream_state = Some(state);
        return Ok(());
    }

    for transition in &transitions {
        renderer.inner.evict_resident_texture_with_mips(
            &transition.entry.digest,
            transition.entry.texture_use.color_space,
            transition.entry.texture_use.mip_semantic,
        );
    }
    for transition in &transitions {
        let Some(bytes) = transition.new_bytes.as_deref() else {
            continue;
        };
        if let Err(error) = renderer
            .inner
            .upload_resident_texture_rgba8_at_mip_with_mips(
                transition.entry.digest,
                transition.entry.texture_use.color_space,
                transition.entry.texture_use.mip_semantic,
                transition.entry.width,
                transition.entry.height,
                u32::from(transition.action.mip_level),
                bytes,
            )
        {
            let rollback = rollback_texture_transitions(renderer, &transitions);
            let result = match rollback {
                Ok(()) => format!("HMC texture stream upload failed: {error}"),
                Err(rollback) => format!(
                    "HMC texture stream upload failed: {error}; rollback failed: {rollback}"
                ),
            };
            renderer.texture_stream_state = Some(state);
            return Err(result);
        }
    }

    if let Err(error) = renderer.load_10d_asset(&state.asset_bytes) {
        let rollback = rollback_texture_transitions(renderer, &transitions);
        let rebind = renderer.load_10d_asset(&state.asset_bytes);
        let result = match (rollback, rebind) {
            (Ok(()), Ok(_)) => format!("HMC material rebind failed: {error}"),
            (Err(rollback), Ok(_)) => format!(
                "HMC material rebind failed: {error}; rollback failed: {rollback}"
            ),
            (Ok(()), Err(rebind)) => format!(
                "HMC material rebind failed: {error}; restoring previous material bindings failed: \
                 {rebind}"
            ),
            (Err(rollback), Err(rebind)) => format!(
                "HMC material rebind failed: {error}; rollback failed: {rollback}; \
                 restoring previous material bindings failed: {rebind}"
            ),
        };
        renderer.texture_stream_state = Some(state);
        return Err(result);
    }

    for transition in &transitions {
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.semantic_id == transition.action.semantic_id)
        {
            match transition.action.kind {
                TextureStreamLifecycleActionKind::EvictTexture => {
                    entry.resident_finest_mip = None;
                    entry.resident_bytes = 0;
                }
                TextureStreamLifecycleActionKind::UploadTexture
                | TextureStreamLifecycleActionKind::ReplaceTexture => {
                    entry.resident_finest_mip = Some(transition.action.mip_level);
                    entry.resident_bytes = transition.action.target_resident_bytes;
                }
            }
        }
    }
    state.generation = next_generation;
    renderer.texture_stream_state = Some(state);
    Ok(())
}

fn rollback_texture_transitions(
    renderer: &mut super::super::VolumetricRenderer,
    transitions: &[PreparedTextureTransition],
) -> Result<(), String> {
    for transition in transitions {
        renderer.inner.evict_resident_texture_with_mips(
            &transition.entry.digest,
            transition.entry.texture_use.color_space,
            transition.entry.texture_use.mip_semantic,
        );
    }
    for transition in transitions {
        let Some(bytes) = transition.old_bytes.as_deref() else {
            continue;
        };
        renderer
            .inner
            .upload_resident_texture_rgba8_at_mip_with_mips(
                transition.entry.digest,
                transition.entry.texture_use.color_space,
                transition.entry.texture_use.mip_semantic,
                transition.entry.width,
                transition.entry.height,
                u32::from(transition.entry.resident_finest_mip.unwrap_or(0)),
                bytes,
            )
            .map_err(|error| format!("{error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(supported_mips: u32) -> HmcTextureStreamEntry {
        HmcTextureStreamEntry {
            semantic_id: 1,
            digest: [7; 32],
            texture_use: TextureUse {
                color_space: TextureColorSpace::Srgb,
                mip_semantic: TextureMipSemantic::Color,
            },
            width: 16,
            height: 16,
            mip_count: 5,
            supported_mips,
            resident_finest_mip: None,
            resident_bytes: 0,
        }
    }

    #[test]
    fn capability_selection_prefers_nearest_supported_quality() {
        let entry = entry((1 << 0) | (1 << 2));
        assert_eq!(select_capability_mip(entry, 2), Some(2));
        assert_eq!(select_capability_mip(entry, 1), Some(0));
        assert_eq!(select_capability_mip(entry, 3), Some(2));
    }

    #[test]
    fn capability_selection_refuses_an_empty_authored_chain() {
        assert_eq!(select_capability_mip(entry(0), 0), None);
    }

    #[test]
    fn lifecycle_bytes_match_the_physical_generated_tail() {
        let entry = entry(0b1_1111);
        assert_eq!(rgba8_mip_bytes(entry, 1), Ok(256));
        assert_eq!(rgba8_resident_bytes(entry, 1), Ok(340));
    }

    #[test]
    fn asset_identity_stamp_changes_when_a_new_asset_reuses_generation_zero() {
        let entry = entry(0b1_1111);
        let first = HmcTextureStreamState {
            bundle_bytes: vec![1],
            asset_bytes: vec![2],
            entries: vec![entry],
            generation: 0,
        };
        let mut second = HmcTextureStreamState {
            bundle_bytes: first.bundle_bytes.clone(),
            asset_bytes: first.asset_bytes.clone(),
            entries: first.entries.clone(),
            generation: 0,
        };
        second.asset_bytes[0] = 3;
        assert_ne!(stream_state_fingerprint(&first), stream_state_fingerprint(&second));
    }
}
