//! Deterministic, allocation-free mip admission for texture streaming.
//!
//! This planner keeps game-asset residency separate from the Prolog Sentinel
//! budget. Callers provide the available mip records and current per-device
//! capacity; the result identifies the best level that can be admitted now.

/// Current free capacity in each independently constrained texture budget.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextureMipBudget {
    /// Free decoded bytes in the CPU-side texture cache.
    pub decoded_cpu_bytes: u64,
    /// Free bytes in the target GPU residency pool.
    pub gpu_resident_bytes: u64,
    /// Free bytes in upload staging and in-flight transfer storage.
    pub upload_staging_bytes: u64,
    /// Maximum bytes that may be submitted during this frame.
    pub per_frame_upload_bytes: u64,
}

/// One independently addressable mip level and its resource costs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureMipCandidate {
    /// KTX2 mip index; level zero is the largest image.
    pub level: u32,
    pub width: u32,
    pub height: u32,
    /// Preserve KTX2 UNORM versus SRGB interpretation through selection.
    pub is_srgb: bool,
    pub decoded_cpu_bytes: u64,
    pub gpu_resident_bytes: u64,
    pub upload_staging_bytes: u64,
    pub upload_bytes: u64,
}

/// Backend costs not inferable from the KTX2 payload alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureMipBackendCost {
    /// Full resident allocation for the selected base level and any generated
    /// lower-resolution GPU mip tail, including backend-specific padding.
    pub gpu_resident_bytes: u64,
    /// Peak staging capacity held concurrently while this upload is in flight.
    pub upload_staging_bytes: u64,
}

/// Fail-closed reasons a KTX2 level cannot become an uncompressed RGBA8 candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ktx2MipCandidateError {
    UnsupportedFormat,
    UnsupportedTextureShape,
    UnsupportedSupercompression,
    InvalidLevel,
    InvalidByteLength,
    SizeOverflow,
}

/// Errors returned by direct decode of a plain RGBA8 mip level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ktx2MipDecodeError {
    Candidate(Ktx2MipCandidateError),
    OutputTooSmall { required: usize, available: usize },
}

/// Metadata for a successfully copied mip level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedKtx2Mip {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub is_srgb: bool,
    pub bytes_written: usize,
}

/// Build a planning record for one plain RGBA8 KTX2 mip level.
///
/// The caller supplies backend allocation costs because driver tiling and
/// staging alignment are not represented by the KTX2 payload size. KTX2 level
/// bytes are borrowed and validated before this metadata-only candidate is
/// returned; decoding remains the responsibility of the texture decoder.
pub fn rgba8_ktx2_mip_candidate(
    document: &super::texture_ktx2::Ktx2Document<'_>,
    level_index: usize,
    backend_cost: TextureMipBackendCost,
) -> Result<TextureMipCandidate, Ktx2MipCandidateError> {
    if document.vk_format != 37 && document.vk_format != 43 {
        return Err(Ktx2MipCandidateError::UnsupportedFormat);
    }
    if document.type_size != 1
        || document.pixel_width == 0
        || document.pixel_height == 0
        || document.pixel_depth != 0
        || document.layer_count() != 1
        || document.face_count != 1
    {
        return Err(Ktx2MipCandidateError::UnsupportedTextureShape);
    }
    if document.supercompression_scheme != 0 {
        return Err(Ktx2MipCandidateError::UnsupportedSupercompression);
    }

    let largest_dimension = document.pixel_width.max(document.pixel_height);
    let maximum_levels = 32 - largest_dimension.leading_zeros();
    if document.level_count() > maximum_levels as usize {
        return Err(Ktx2MipCandidateError::InvalidLevel);
    }

    let level = document
        .level(level_index)
        .ok_or(Ktx2MipCandidateError::InvalidLevel)?;
    let width = mip_extent(document.pixel_width, level.index);
    let height = mip_extent(document.pixel_height, level.index);
    let expected_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(Ktx2MipCandidateError::SizeOverflow)?;
    let expected_usize =
        usize::try_from(expected_bytes).map_err(|_| Ktx2MipCandidateError::SizeOverflow)?;
    if level.bytes.len() != expected_usize || level.uncompressed_byte_length != expected_bytes {
        return Err(Ktx2MipCandidateError::InvalidByteLength);
    }

    Ok(TextureMipCandidate {
        level: level.index,
        width,
        height,
        is_srgb: document.vk_format == 43,
        decoded_cpu_bytes: expected_bytes,
        gpu_resident_bytes: backend_cost.gpu_resident_bytes,
        upload_staging_bytes: backend_cost.upload_staging_bytes,
        upload_bytes: expected_bytes,
    })
}

/// Decode one indexed, uncompressed RGBA8 KTX2 mip into caller-owned memory.
///
/// All format, shape, level, and exact-length checks complete before output is
/// modified. The function performs no allocation and leaves any output suffix
/// beyond `bytes_written` untouched.
pub fn decode_rgba8_ktx2_mip_into(
    document: &super::texture_ktx2::Ktx2Document<'_>,
    level_index: usize,
    output: &mut [u8],
) -> Result<DecodedKtx2Mip, Ktx2MipDecodeError> {
    let candidate = rgba8_ktx2_mip_candidate(
        document,
        level_index,
        TextureMipBackendCost {
            gpu_resident_bytes: 0,
            upload_staging_bytes: 0,
        },
    )
    .map_err(Ktx2MipDecodeError::Candidate)?;
    let required = usize::try_from(candidate.decoded_cpu_bytes)
        .map_err(|_| Ktx2MipDecodeError::Candidate(Ktx2MipCandidateError::SizeOverflow))?;
    if output.len() < required {
        return Err(Ktx2MipDecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }
    let level = document
        .level(level_index)
        .ok_or(Ktx2MipDecodeError::Candidate(
            Ktx2MipCandidateError::InvalidLevel,
        ))?;
    output[..required].copy_from_slice(level.bytes);
    Ok(DecodedKtx2Mip {
        level: candidate.level,
        width: candidate.width,
        height: candidate.height,
        is_srgb: candidate.is_srgb,
        bytes_written: required,
    })
}

/// Selected level plus whether it meets the projected sampling footprint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureMipSelection {
    pub candidate: TextureMipCandidate,
    /// False means this is a budget-driven quality fallback below the requested
    /// projected footprint. The caller can refine it on a later frame.
    pub meets_footprint: bool,
}

/// Select the least expensive adequate level, or the best affordable fallback.
///
/// The input slice is borrowed and never reordered or allocated. Invalid zero
/// dimensions are ignored. When several entries tie, the lower mip index wins,
/// making selection independent of input order. `None` means no valid level
/// fits every budget and the caller should defer this resource.
pub fn select_texture_mip(
    candidates: &[TextureMipCandidate],
    projected_width: u32,
    projected_height: u32,
    budget: TextureMipBudget,
) -> Option<TextureMipSelection> {
    if projected_width == 0 || projected_height == 0 {
        return None;
    }

    let mut best_adequate: Option<TextureMipCandidate> = None;
    let mut best_fallback: Option<TextureMipCandidate> = None;

    for candidate in candidates {
        if candidate.width == 0
            || candidate.height == 0
            || candidate.decoded_cpu_bytes > budget.decoded_cpu_bytes
            || candidate.gpu_resident_bytes > budget.gpu_resident_bytes
            || candidate.upload_staging_bytes > budget.upload_staging_bytes
            || candidate.upload_bytes > budget.per_frame_upload_bytes
        {
            continue;
        }

        if candidate.width >= projected_width && candidate.height >= projected_height {
            if is_better_adequate(*candidate, best_adequate) {
                best_adequate = Some(*candidate);
            }
        }
        if is_better_fallback(*candidate, best_fallback, projected_width, projected_height) {
            best_fallback = Some(*candidate);
        }
    }

    best_adequate
        .map(|candidate| TextureMipSelection {
            candidate,
            meets_footprint: true,
        })
        .or_else(|| {
            best_fallback.map(|candidate| TextureMipSelection {
                candidate,
                meets_footprint: false,
            })
        })
}

fn pixel_area(candidate: TextureMipCandidate) -> u64 {
    u64::from(candidate.width) * u64::from(candidate.height)
}

fn mip_extent(base: u32, level: u32) -> u32 {
    base.checked_shr(level).unwrap_or(0).max(1)
}

fn is_better_adequate(
    candidate: TextureMipCandidate,
    current: Option<TextureMipCandidate>,
) -> bool {
    match current {
        None => true,
        Some(current) => {
            pixel_area(candidate) < pixel_area(current)
                || (pixel_area(candidate) == pixel_area(current)
                    && tie_break_key(candidate) < tie_break_key(current))
        }
    }
}

/// Reserve the selected candidate against a mutable remaining-budget snapshot.
///
/// All four limits are checked before any field is reduced, so failure leaves
/// `remaining` unchanged. Callers can run this in deterministic asset-priority
/// order to prevent independent per-texture plans from overcommitting a frame.
/// Refresh the per-frame credit each frame, and replenish staging credit when
/// the corresponding in-flight upload has completed; CPU/GPU residency credits
/// return only when the decoded or resident resource is evicted.
pub fn reserve_texture_mip(
    candidate: TextureMipCandidate,
    remaining: &mut TextureMipBudget,
) -> bool {
    if candidate.decoded_cpu_bytes > remaining.decoded_cpu_bytes
        || candidate.gpu_resident_bytes > remaining.gpu_resident_bytes
        || candidate.upload_staging_bytes > remaining.upload_staging_bytes
        || candidate.upload_bytes > remaining.per_frame_upload_bytes
    {
        return false;
    }
    remaining.decoded_cpu_bytes -= candidate.decoded_cpu_bytes;
    remaining.gpu_resident_bytes -= candidate.gpu_resident_bytes;
    remaining.upload_staging_bytes -= candidate.upload_staging_bytes;
    remaining.per_frame_upload_bytes -= candidate.upload_bytes;
    true
}

fn is_better_fallback(
    candidate: TextureMipCandidate,
    current: Option<TextureMipCandidate>,
    projected_width: u32,
    projected_height: u32,
) -> bool {
    match current {
        None => true,
        Some(current) => {
            let candidate_coverage = footprint_score(candidate, projected_width, projected_height);
            let current_coverage = footprint_score(current, projected_width, projected_height);
            candidate_coverage > current_coverage
                || (candidate_coverage == current_coverage
                    && (pixel_area(candidate) > pixel_area(current)
                        || (pixel_area(candidate) == pixel_area(current)
                            && tie_break_key(candidate) < tie_break_key(current))))
        }
    }
}

fn footprint_score(
    candidate: TextureMipCandidate,
    projected_width: u32,
    projected_height: u32,
) -> u64 {
    let horizontal = u64::from(candidate.width) * u64::from(projected_height);
    let vertical = u64::from(candidate.height) * u64::from(projected_width);
    horizontal.min(vertical)
}

fn tie_break_key(candidate: TextureMipCandidate) -> (u32, u32, u32, bool, u64, u64, u64, u64) {
    (
        candidate.level,
        candidate.width,
        candidate.height,
        candidate.is_srgb,
        candidate.decoded_cpu_bytes,
        candidate.gpu_resident_bytes,
        candidate.upload_staging_bytes,
        candidate.upload_bytes,
    )
}

#[cfg(test)]
#[path = "texture_streaming_plan_tests.rs"]
mod tests;
