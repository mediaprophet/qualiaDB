//! Deterministic, caller-buffered planning for texture mip residency.
//!
//! The planner never uploads or evicts resources. It computes a complete plan before writing
//! `out`, so malformed input, arithmetic overflow, insufficient capacity, or budget failure
//! leaves both resident state and the caller's output slice unchanged. Apply a successful plan
//! separately, then feed the resulting residency snapshot into the next invocation.

/// One texture's immutable identity, current residency, and current-frame demand.
///
/// `resident_finest_mip` follows the usual mip numbering: zero is full resolution and larger
/// values are progressively coarser. If it is `Some(r)`, every mip `r..mip_count` is resident;
/// if it is `None`, no mip is resident. `desired_finest_mip` requests every mip from that level
/// through the coarsest level. Set it to `mip_count - 1` for a coarse presentation mip only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureStreamDemand {
    /// Stable semantic/resource identity used for deterministic tie-breaking.
    pub semantic_id: u64,
    pub width: u32,
    pub height: u32,
    pub mip_count: u8,
    /// Compressed block width in texels (use 1 for uncompressed formats).
    pub block_width: u8,
    /// Compressed block height in texels (use 1 for uncompressed formats).
    pub block_height: u8,
    /// Bytes in one compression block (or bytes per texel for 1x1 formats).
    pub bytes_per_block: u8,
    pub desired_finest_mip: u8,
    pub resident_finest_mip: Option<u8>,
    /// Bytes currently resident for this texture, including all resident mips.
    pub resident_bytes: u64,
    /// This texture is in the requested working set and should gain/retain desired mips.
    pub requested: bool,
    pub pinned: bool,
    pub visible: bool,
    /// Authored importance; larger values are serviced first.
    pub importance: u16,
    /// Monotonic fixed-point distance key; smaller values are serviced first.
    pub distance_key: u32,
    /// Frame in which this texture was last used; newer use wins request priority.
    pub last_used_frame: u64,
}

/// Hard bounds for one planning pass. These budgets are renderer/texture budgets and are
/// independent of the semantic 42 MiB Sentinel limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureStreamBudget {
    pub max_resident_bytes: u64,
    pub max_upload_bytes: u64,
}

/// The action to perform after a plan has been accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureStreamActionKind {
    /// Drop every currently resident mip for this texture before applying requests.
    EvictTexture,
    /// Upload this mip. Mips with larger indices are coarser and are ordered first.
    RequestMip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureStreamAction {
    pub semantic_id: u64,
    pub kind: TextureStreamActionKind,
    /// Meaningful for `RequestMip`; zero for `EvictTexture`.
    pub mip_level: u8,
    /// Exact tightly packed payload size for a request, zero for eviction.
    pub bytes: u64,
}

/// Accounting for an additive, best-effort residency plan.
///
/// The admitted requests are a single global priority prefix. Deferred requests retain their
/// place in that order and can be retried with the next frame's budget. Evictions are reported
/// first in least-protected-first order, followed by the admitted mip uploads in request order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureStreamPartialPlan {
    pub action_count: usize,
    pub admitted_mip_count: usize,
    pub deferred_mip_count: usize,
    pub admitted_upload_bytes: u64,
    pub deferred_upload_bytes: u64,
    /// Resident bytes after the listed evictions and admitted uploads.
    pub projected_resident_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureStreamPlanError {
    InvalidDimensions,
    InvalidBlockLayout,
    InvalidMipRange,
    InvalidDemandState,
    InconsistentResidency,
    DuplicateSemanticId,
    ArithmeticOverflow,
    UploadBudgetExceeded { required: u64, budget: u64 },
    ResidentBudgetExceeded { required: u64, budget: u64 },
    OutputCapacity { required: usize, capacity: usize },
}

/// One physical resource transition in the bounded resident-texture lifecycle.
///
/// Unlike [`TextureStreamAction`], which describes an additive mip prefix, this action models
/// the actual backend operation used by the renderer: one texture view is evicted and, when
/// requested, one replacement view is uploaded at `mip_level` with its coarser tail generated
/// by the backend. Keeping that distinction here prevents a refinement from charging the old
/// allocation and every generated mip twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureStreamLifecycleActionKind {
    EvictTexture,
    UploadTexture,
    ReplaceTexture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureStreamLifecycleAction {
    pub semantic_id: u64,
    pub kind: TextureStreamLifecycleActionKind,
    /// Source mip used as the physical texture base. Zero for a pure eviction.
    pub mip_level: u8,
    /// Bytes submitted for the physical base upload, not generated tail bytes.
    pub bytes: u64,
    /// Existing physical allocation released before an upload, if any.
    pub previous_resident_bytes: u64,
    /// Physical allocation held after the transition. Zero for an eviction.
    pub target_resident_bytes: u64,
}

/// Accounting for one generation of a physical resident-texture transition plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureStreamLifecyclePlan {
    pub generation: u64,
    pub action_count: usize,
    pub admitted_count: usize,
    pub deferred_count: usize,
    pub admitted_upload_bytes: u64,
    pub deferred_upload_bytes: u64,
    pub projected_resident_bytes: u64,
}

/// Plan physical texture replacements, uploads, and eviction of every unrequested resource.
///
/// This is the lifecycle counterpart to [`plan_texture_residency_partial`]. It emits at most one
/// upload action per texture because the GPU resident texture owns a generated coarser tail. The
/// old allocation is debited before the replacement allocation is credited, so callers can apply
/// the action without double-debiting a residency budget. Validation, priority admission, and
/// capacity checks finish before `out` is modified.
pub fn plan_texture_residency_lifecycle(
    demands: &[TextureStreamDemand],
    budget: TextureStreamBudget,
    generation: u64,
    out: &mut [TextureStreamLifecycleAction],
) -> Result<TextureStreamLifecyclePlan, TextureStreamPlanError> {
    validate_lifecycle_demands(demands)?;

    let mut retained_resident = 0u64;
    let mut eviction_count = 0usize;
    for demand in demands {
        if !demand.requested {
            if demand.resident_finest_mip.is_some() {
                eviction_count = eviction_count
                    .checked_add(1)
                    .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
            }
            continue;
        }
        retained_resident = retained_resident
            .checked_add(demand.resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    }
    if retained_resident > budget.max_resident_bytes {
        return Err(TextureStreamPlanError::ResidentBudgetExceeded {
            required: retained_resident,
            budget: budget.max_resident_bytes,
        });
    }

    let mut projected_resident = retained_resident;
    let mut admitted_count = 0usize;
    let mut admitted_upload_bytes = 0u64;
    let mut previous_request = None;
    while let Some((index, target_mip)) = find_next_lifecycle_request(demands, previous_request) {
        let demand = &demands[index];
        let target_resident_bytes = texture_mip_chain_bytes(demand, target_mip)?;
        let upload_bytes = mip_level_bytes(demand, target_mip)?;
        let next_upload = admitted_upload_bytes
            .checked_add(upload_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        if next_upload > budget.max_upload_bytes {
            break;
        }

        let next_resident = projected_resident
            .checked_sub(demand.resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?
            .checked_add(target_resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        if next_resident > budget.max_resident_bytes {
            break;
        }

        projected_resident = next_resident;
        admitted_upload_bytes = next_upload;
        admitted_count = admitted_count
            .checked_add(1)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        previous_request = Some((index, target_mip));
    }

    // Include all lower-priority lifecycle requests in deferred upload accounting.
    let mut total_deferred_upload_bytes = 0u64;
    let mut deferred_count = 0usize;
    let mut deferred_previous = previous_request;
    while let Some((index, target_mip)) =
        find_next_lifecycle_request(demands, deferred_previous)
    {
        total_deferred_upload_bytes = total_deferred_upload_bytes
            .checked_add(mip_level_bytes(&demands[index], target_mip)?)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        deferred_count = deferred_count
            .checked_add(1)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        deferred_previous = Some((index, target_mip));
    }

    let action_count = eviction_count
        .checked_add(admitted_count)
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    if action_count > out.len() {
        return Err(TextureStreamPlanError::OutputCapacity {
            required: action_count,
            capacity: out.len(),
        });
    }

    let mut written = 0usize;
    let mut previous_eviction = None;
    for _ in 0..eviction_count {
        let index = find_next_eviction(demands, previous_eviction)
            .expect("preflight selected the same lifecycle evictions")
            .expect("preflight counted every unrequested resident texture");
        let demand = &demands[index];
        out[written] = TextureStreamLifecycleAction {
                semantic_id: demand.semantic_id,
                kind: TextureStreamLifecycleActionKind::EvictTexture,
                mip_level: 0,
                bytes: 0,
                previous_resident_bytes: demand.resident_bytes,
                target_resident_bytes: 0,
        };
        written += 1;
        previous_eviction = Some(index);
    }
    let mut admitted_previous = None;
    for _ in 0..admitted_count {
        let (index, target_mip) = find_next_lifecycle_request(demands, admitted_previous)
            .expect("preflight selected the same lifecycle prefix");
        let demand = &demands[index];
        let target_resident_bytes = texture_mip_chain_bytes(demand, target_mip)
            .expect("preflight checked target resident bytes");
        let bytes = mip_level_bytes(demand, target_mip).expect("preflight checked upload bytes");
        out[written] = TextureStreamLifecycleAction {
            semantic_id: demand.semantic_id,
            kind: if demand.resident_finest_mip.is_some() {
                TextureStreamLifecycleActionKind::ReplaceTexture
            } else {
                TextureStreamLifecycleActionKind::UploadTexture
            },
            mip_level: target_mip,
            bytes,
            previous_resident_bytes: demand.resident_bytes,
            target_resident_bytes,
        };
        written += 1;
        admitted_previous = Some((index, target_mip));
    }

    Ok(TextureStreamLifecyclePlan {
        generation,
        action_count,
        admitted_count,
        deferred_count,
        admitted_upload_bytes,
        deferred_upload_bytes: total_deferred_upload_bytes,
        projected_resident_bytes: projected_resident,
    })
}

fn validate_lifecycle_demands(demands: &[TextureStreamDemand]) -> Result<(), TextureStreamPlanError> {
    validate_demands(demands)?;
    for demand in demands {
        if let Some(first_mip) = demand.resident_finest_mip {
            if demand.resident_bytes != texture_mip_chain_bytes(demand, first_mip)? {
                return Err(TextureStreamPlanError::InconsistentResidency);
            }
        }
    }
    Ok(())
}

fn texture_mip_chain_bytes(
    demand: &TextureStreamDemand,
    first_mip: u8,
) -> Result<u64, TextureStreamPlanError> {
    let mut total = 0u64;
    for mip in first_mip..demand.mip_count {
        total = total
            .checked_add(mip_level_bytes(demand, mip)?)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    }
    Ok(total)
}

fn find_next_lifecycle_request(
    demands: &[TextureStreamDemand],
    previous: Option<(usize, u8)>,
) -> Option<(usize, u8)> {
    let mut selected = None;
    for (index, demand) in demands.iter().enumerate() {
        if !demand.requested || demand.resident_finest_mip == Some(demand.desired_finest_mip) {
            continue;
        }
        if previous.is_some_and(|previous| {
            request_candidate_order(demand, demand.desired_finest_mip, &demands[previous.0], previous.1)
                != core::cmp::Ordering::Greater
        }) {
            continue;
        }
        if selected.is_none_or(|(selected_index, selected_mip)| {
            request_candidate_order(
                demand,
                demand.desired_finest_mip,
                &demands[selected_index],
                selected_mip,
            )
            .is_lt()
        }) {
            selected = Some((index, demand.desired_finest_mip));
        }
    }
    selected
}

/// Compute an all-or-nothing texture streaming plan without heap allocation.
///
/// Priority for requests is pinned, visible, authored importance, distance, recent use, then
/// semantic ID. For a texture, coarse mips precede finer mips. Eviction reverses protection:
/// unpinned and invisible resources with low importance, greater distance, and older use leave
/// first. Semantic ID provides a stable final tie-break in both directions.
///
/// Complexity is bounded by the supplied slices, with repeated scans instead of an allocator- or
/// stack-sized scratch buffer. No field in `demands` is modified. `out` is untouched on every
/// error, including capacity and budget errors.
pub fn plan_texture_residency(
    demands: &[TextureStreamDemand],
    budget: TextureStreamBudget,
    out: &mut [TextureStreamAction],
) -> Result<usize, TextureStreamPlanError> {
    validate_demands(demands)?;

    let mut resident_total = 0u64;
    let mut upload_total = 0u64;
    let mut request_count = 0usize;
    for demand in demands {
        resident_total = resident_total
            .checked_add(demand.resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        if !demand.requested {
            continue;
        }
        let first_resident = demand.resident_finest_mip.unwrap_or(demand.mip_count);
        for mip in (demand.desired_finest_mip..first_resident).rev() {
            upload_total = upload_total
                .checked_add(mip_level_bytes(demand, mip)?)
                .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
            request_count = request_count
                .checked_add(1)
                .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        }
    }
    if upload_total > budget.max_upload_bytes {
        return Err(TextureStreamPlanError::UploadBudgetExceeded {
            required: upload_total,
            budget: budget.max_upload_bytes,
        });
    }

    let mut projected_resident = resident_total
        .checked_add(upload_total)
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    let mut eviction_count = 0usize;
    let mut previous_eviction: Option<usize> = None;
    while projected_resident > budget.max_resident_bytes {
        let next = find_next_eviction(demands, previous_eviction)?;
        let Some(index) = next else {
            return Err(TextureStreamPlanError::ResidentBudgetExceeded {
                required: projected_resident,
                budget: budget.max_resident_bytes,
            });
        };
        projected_resident = projected_resident
            .checked_sub(demands[index].resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        previous_eviction = Some(index);
        eviction_count = eviction_count
            .checked_add(1)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    }

    let required = eviction_count
        .checked_add(request_count)
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    if required > out.len() {
        return Err(TextureStreamPlanError::OutputCapacity {
            required,
            capacity: out.len(),
        });
    }

    // All fallible validation and budget checks have completed. From here to return, the plan is
    // written atomically with respect to errors (no further checked arithmetic is needed).
    let mut written = 0usize;
    previous_eviction = None;
    for _ in 0..eviction_count {
        let index = find_next_eviction(demands, previous_eviction)
            .expect("preflight selected the same eviction set")
            .expect("preflight proved sufficient evictions");
        out[written] = TextureStreamAction {
            semantic_id: demands[index].semantic_id,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 0,
            bytes: 0,
        };
        written += 1;
        previous_eviction = Some(index);
    }

    let request_start = written;
    for demand in demands {
        if !demand.requested {
            continue;
        }
        let first = demand.desired_finest_mip;
        let stop = demand.resident_finest_mip.unwrap_or(demand.mip_count);
        for mip in (first..stop).rev() {
            out[written] = TextureStreamAction {
                semantic_id: demand.semantic_id,
                kind: TextureStreamActionKind::RequestMip,
                mip_level: mip,
                bytes: mip_level_bytes(demand, mip).expect("preflight checked each mip byte count"),
            };
            written += 1;
        }
    }
    sort_requests(demands, &mut out[request_start..written]);
    Ok(written)
}

/// Plan the highest-priority mip-request prefix that fits both budgets.
///
/// Unlike [`plan_texture_residency`], this API defers lower-priority requests instead of failing
/// when upload or incremental residency capacity is exhausted. The input is validated in full;
/// invalid input, arithmetic overflow, an impossible baseline resident budget, or insufficient
/// output capacity leaves `out` untouched. Existing, eligible evictions are selected in the
/// planner's normal least-protected-first order. A request that cannot fit causes it and every
/// lower-priority request to be deferred, even if a later smaller request could fit.
pub fn plan_texture_residency_partial(
    demands: &[TextureStreamDemand],
    budget: TextureStreamBudget,
    out: &mut [TextureStreamAction],
) -> Result<TextureStreamPartialPlan, TextureStreamPlanError> {
    validate_demands(demands)?;

    let mut resident_total = 0u64;
    let mut requested_bytes = 0u64;
    let mut requested_count = 0usize;
    for demand in demands {
        resident_total = resident_total
            .checked_add(demand.resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        if !demand.requested {
            continue;
        }
        let stop = demand.resident_finest_mip.unwrap_or(demand.mip_count);
        for mip in (demand.desired_finest_mip..stop).rev() {
            requested_bytes = requested_bytes
                .checked_add(mip_level_bytes(demand, mip)?)
                .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
            requested_count = requested_count
                .checked_add(1)
                .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        }
    }

    // First make the retained baseline fit. If protected resources alone exceed the budget,
    // there is no valid plan and the caller's output remains untouched.
    let mut projected_resident = resident_total;
    let mut eviction_count = 0usize;
    let mut previous_eviction = None;
    while projected_resident > budget.max_resident_bytes {
        let Some(index) = find_next_eviction(demands, previous_eviction)? else {
            return Err(TextureStreamPlanError::ResidentBudgetExceeded {
                required: projected_resident,
                budget: budget.max_resident_bytes,
            });
        };
        projected_resident = projected_resident
            .checked_sub(demands[index].resident_bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        previous_eviction = Some(index);
        eviction_count = eviction_count
            .checked_add(1)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    }

    let mut admitted_mip_count = 0usize;
    let mut admitted_upload_bytes = 0u64;
    let mut previous_request: Option<(usize, u8)> = None;
    loop {
        let Some((index, mip)) = find_next_request(demands, previous_request) else {
            break;
        };
        let demand = &demands[index];
        let bytes = mip_level_bytes(demand, mip)?;
        let Some(next_upload) = admitted_upload_bytes.checked_add(bytes) else {
            return Err(TextureStreamPlanError::ArithmeticOverflow);
        };
        if next_upload > budget.max_upload_bytes {
            break;
        }

        // Try the required evictions speculatively. Commit them only if this request can then be
        // admitted, so a deferred request never causes a needless eviction.
        let mut trial_resident = projected_resident;
        let mut trial_evictions = eviction_count;
        let mut trial_previous = previous_eviction;
        while trial_resident
            .checked_add(bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?
            > budget.max_resident_bytes
        {
            let Some(eviction_index) = find_next_eviction(demands, trial_previous)? else {
                break;
            };
            trial_resident = trial_resident
                .checked_sub(demands[eviction_index].resident_bytes)
                .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
            trial_previous = Some(eviction_index);
            trial_evictions = trial_evictions
                .checked_add(1)
                .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        }
        if trial_resident
            .checked_add(bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?
            > budget.max_resident_bytes
        {
            break;
        }
        projected_resident = trial_resident
            .checked_add(bytes)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        eviction_count = trial_evictions;
        previous_eviction = trial_previous;
        admitted_upload_bytes = next_upload;
        admitted_mip_count = admitted_mip_count
            .checked_add(1)
            .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
        previous_request = Some((index, mip));
    }

    let action_count = eviction_count
        .checked_add(admitted_mip_count)
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)?;
    if action_count > out.len() {
        return Err(TextureStreamPlanError::OutputCapacity {
            required: action_count,
            capacity: out.len(),
        });
    }

    // All possible errors have been checked; output mutation begins only here.
    let mut written = 0usize;
    previous_eviction = None;
    for _ in 0..eviction_count {
        let index = find_next_eviction(demands, previous_eviction)
            .expect("preflight selected the same eviction set")
            .expect("preflight proved sufficient evictions");
        out[written] = TextureStreamAction {
            semantic_id: demands[index].semantic_id,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 0,
            bytes: 0,
        };
        written += 1;
        previous_eviction = Some(index);
    }
    let mut previous_request = None;
    for _ in 0..admitted_mip_count {
        let (index, mip) = find_next_request(demands, previous_request)
            .expect("preflight selected the same request prefix");
        let demand = &demands[index];
        let bytes = mip_level_bytes(demand, mip).expect("preflight checked every request size");
        out[written] = TextureStreamAction {
            semantic_id: demand.semantic_id,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: mip,
            bytes,
        };
        written += 1;
        previous_request = Some((index, mip));
    }

    Ok(TextureStreamPartialPlan {
        action_count,
        admitted_mip_count,
        deferred_mip_count: requested_count - admitted_mip_count,
        admitted_upload_bytes,
        deferred_upload_bytes: requested_bytes - admitted_upload_bytes,
        projected_resident_bytes: projected_resident,
    })
}

fn find_next_request(
    demands: &[TextureStreamDemand],
    previous: Option<(usize, u8)>,
) -> Option<(usize, u8)> {
    let mut selected: Option<(usize, u8)> = None;
    for (index, demand) in demands.iter().enumerate() {
        if !demand.requested {
            continue;
        }
        let stop = demand.resident_finest_mip.unwrap_or(demand.mip_count);
        for mip in (demand.desired_finest_mip..stop).rev() {
            if previous.is_some_and(|previous| {
                request_candidate_order(demand, mip, &demands[previous.0], previous.1)
                    != core::cmp::Ordering::Greater
            }) {
                continue;
            }
            if selected.is_none_or(|(selected_index, selected_mip)| {
                request_candidate_order(demand, mip, &demands[selected_index], selected_mip).is_lt()
            }) {
                selected = Some((index, mip));
            }
        }
    }
    selected
}

fn validate_demands(demands: &[TextureStreamDemand]) -> Result<(), TextureStreamPlanError> {
    for (index, demand) in demands.iter().enumerate() {
        if demand.width == 0 || demand.height == 0 {
            return Err(TextureStreamPlanError::InvalidDimensions);
        }
        if demand.block_width == 0 || demand.block_height == 0 || demand.bytes_per_block == 0 {
            return Err(TextureStreamPlanError::InvalidBlockLayout);
        }
        let max_dim = demand.width.max(demand.height);
        let max_mips = (u32::BITS - max_dim.leading_zeros()) as u8;
        if demand.mip_count == 0
            || demand.mip_count > max_mips
            || demand.desired_finest_mip >= demand.mip_count
            || demand
                .resident_finest_mip
                .is_some_and(|mip| mip >= demand.mip_count)
        {
            return Err(TextureStreamPlanError::InvalidMipRange);
        }
        if demand.visible && !demand.requested {
            return Err(TextureStreamPlanError::InvalidDemandState);
        }
        match demand.resident_finest_mip {
            None if demand.resident_bytes != 0 => {
                return Err(TextureStreamPlanError::InconsistentResidency);
            }
            Some(_) if demand.resident_bytes == 0 => {
                return Err(TextureStreamPlanError::InconsistentResidency);
            }
            _ => {}
        }
        for other in &demands[..index] {
            if other.semantic_id == demand.semantic_id {
                return Err(TextureStreamPlanError::DuplicateSemanticId);
            }
        }
    }
    Ok(())
}

fn mip_level_bytes(demand: &TextureStreamDemand, mip: u8) -> Result<u64, TextureStreamPlanError> {
    let width = (demand.width >> mip).max(1) as u64;
    let height = (demand.height >> mip).max(1) as u64;
    let block_width = demand.block_width as u64;
    let block_height = demand.block_height as u64;
    let blocks_x = width
        .checked_add(block_width - 1)
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)?
        / block_width;
    let blocks_y = height
        .checked_add(block_height - 1)
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)?
        / block_height;
    blocks_x
        .checked_mul(blocks_y)
        .and_then(|blocks| blocks.checked_mul(demand.bytes_per_block as u64))
        .ok_or(TextureStreamPlanError::ArithmeticOverflow)
}

/// Select in least-protected-first order, after `previous` when supplied.
fn find_next_eviction(
    demands: &[TextureStreamDemand],
    previous: Option<usize>,
) -> Result<Option<usize>, TextureStreamPlanError> {
    let mut selected: Option<usize> = None;
    for (index, demand) in demands.iter().enumerate() {
        if demand.resident_bytes == 0
            || demand.pinned
            || demand.visible
            || demand.requested
            || needs_upload(demand)
        {
            continue;
        }
        if let Some(previous_index) = previous {
            if eviction_order(demand, index, &demands[previous_index], previous_index)
                != core::cmp::Ordering::Greater
            {
                continue;
            }
        }
        if selected.is_none_or(|current| {
            eviction_order(demand, index, &demands[current], current) == core::cmp::Ordering::Less
        }) {
            selected = Some(index);
        }
    }
    Ok(selected)
}

fn needs_upload(demand: &TextureStreamDemand) -> bool {
    demand.requested
        && demand
            .resident_finest_mip
            .is_none_or(|resident| demand.desired_finest_mip < resident)
}

/// Ordering from most valuable to least valuable, with stable identity as final tie-break.
fn request_order(
    demands: &[TextureStreamDemand],
    a: &TextureStreamAction,
    b: &TextureStreamAction,
) -> core::cmp::Ordering {
    let a_demand = demands
        .iter()
        .find(|demand| demand.semantic_id == a.semantic_id)
        .expect("every request references a validated demand");
    let b_demand = demands
        .iter()
        .find(|demand| demand.semantic_id == b.semantic_id)
        .expect("every request references a validated demand");
    request_candidate_order(a_demand, a.mip_level, b_demand, b.mip_level)
}

fn request_candidate_order(
    a: &TextureStreamDemand,
    a_mip: u8,
    b: &TextureStreamDemand,
    b_mip: u8,
) -> core::cmp::Ordering {
    b.pinned
        .cmp(&a.pinned)
        .then_with(|| b.visible.cmp(&a.visible))
        .then_with(|| b.importance.cmp(&a.importance))
        .then_with(|| a.distance_key.cmp(&b.distance_key))
        .then_with(|| b.last_used_frame.cmp(&a.last_used_frame))
        .then_with(|| a.semantic_id.cmp(&b.semantic_id))
        .then_with(|| b_mip.cmp(&a_mip))
}

fn eviction_order(
    a: &TextureStreamDemand,
    a_index: usize,
    b: &TextureStreamDemand,
    b_index: usize,
) -> core::cmp::Ordering {
    a.pinned
        .cmp(&b.pinned)
        .then_with(|| a.visible.cmp(&b.visible))
        .then_with(|| a.importance.cmp(&b.importance))
        .then_with(|| b.distance_key.cmp(&a.distance_key))
        .then_with(|| a.last_used_frame.cmp(&b.last_used_frame))
        .then_with(|| b.semantic_id.cmp(&a.semantic_id))
        .then_with(|| a_index.cmp(&b_index))
}

fn sort_requests(demands: &[TextureStreamDemand], actions: &mut [TextureStreamAction]) {
    // Stable insertion sort is allocation-free and the request set is bounded by caller inputs.
    for index in 1..actions.len() {
        let value = actions[index];
        let mut destination = index;
        while destination > 0 && request_order(demands, &value, &actions[destination - 1]).is_lt() {
            actions[destination] = actions[destination - 1];
            destination -= 1;
        }
        actions[destination] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demand(semantic_id: u64) -> TextureStreamDemand {
        TextureStreamDemand {
            semantic_id,
            width: 8,
            height: 8,
            mip_count: 4,
            block_width: 1,
            block_height: 1,
            bytes_per_block: 4,
            desired_finest_mip: 1,
            resident_finest_mip: Some(3),
            resident_bytes: 16,
            requested: true,
            pinned: false,
            visible: false,
            importance: 1,
            distance_key: 100,
            last_used_frame: 1,
        }
    }

    fn plan<'a>(
        demands: &[TextureStreamDemand],
        budget: TextureStreamBudget,
        out: &'a mut [TextureStreamAction],
    ) -> &'a [TextureStreamAction] {
        let len = plan_texture_residency(demands, budget, out).unwrap();
        &out[..len]
    }

    #[test]
    fn request_order_is_deterministic_for_priority_ties() {
        let mut demands = [demand(9), demand(3)];
        let budget = TextureStreamBudget {
            max_resident_bytes: 192,
            max_upload_bytes: 4096,
        };
        let mut first = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 0,
            bytes: 0,
        }; 8];
        let mut second = first;
        let first_plan = plan(&demands, budget, &mut first).to_vec();
        demands.reverse();
        let second_plan = plan(&demands, budget, &mut second).to_vec();
        assert_eq!(first_plan, second_plan);
        assert_eq!(first_plan[0].semantic_id, 3);
        assert_eq!(first_plan[0].mip_level, 2);
        assert_eq!(first_plan[1].mip_level, 1);
    }

    #[test]
    fn visible_and_pinned_requests_precede_importance_distance_and_age() {
        let mut low = demand(1);
        low.distance_key = 0;
        low.importance = u16::MAX;
        let mut visible = demand(2);
        visible.visible = true;
        let mut pinned = demand(3);
        pinned.pinned = true;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 6];
        let result = plan(
            &[low, visible, pinned],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 4096,
            },
            &mut out,
        );
        assert_eq!(result[0].semantic_id, 3);
        assert_eq!(result[2].semantic_id, 2);
        assert_eq!(result[4].semantic_id, 1);
    }

    #[test]
    fn mip_bytes_respect_compressed_blocks_and_exact_budget_boundary() {
        let mut compressed = demand(1);
        compressed.width = 7;
        compressed.height = 5;
        compressed.block_width = 4;
        compressed.block_height = 4;
        compressed.bytes_per_block = 16;
        compressed.mip_count = 3;
        compressed.desired_finest_mip = 0;
        compressed.resident_finest_mip = Some(1);
        let exact = mip_level_bytes(&compressed, 0).unwrap();
        assert_eq!(exact, 64);
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 4];
        let got = plan_texture_residency(
            &[compressed],
            TextureStreamBudget {
                max_resident_bytes: 80,
                max_upload_bytes: exact,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(got, 1);
        assert_eq!(out[0].bytes, exact);
    }

    #[test]
    fn upload_budget_overrun_reports_required_bytes_without_partial_plan() {
        let sentinel = TextureStreamAction {
            semantic_id: 77,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 5,
            bytes: 99,
        };
        let mut out = [sentinel; 8];
        let result = plan_texture_residency(
            &[demand(1)],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 63,
            },
            &mut out,
        );
        assert_eq!(
            result,
            Err(TextureStreamPlanError::UploadBudgetExceeded {
                required: 80,
                budget: 63,
            })
        );
        assert!(out.iter().all(|action| *action == sentinel));
    }

    #[test]
    fn insufficient_output_capacity_leaves_output_unchanged() {
        let sentinel = TextureStreamAction {
            semantic_id: 77,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 5,
            bytes: 99,
        };
        let mut out = [sentinel; 1];
        let result = plan_texture_residency(
            &[demand(1)],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 4096,
            },
            &mut out,
        );
        assert_eq!(
            result,
            Err(TextureStreamPlanError::OutputCapacity {
                required: 2,
                capacity: 1
            })
        );
        assert_eq!(out[0], sentinel);
    }

    #[test]
    fn malformed_layout_and_byte_overflow_fail_closed() {
        let mut invalid = demand(1);
        invalid.block_width = 0;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 8];
        assert_eq!(
            plan_texture_residency(
                &[invalid],
                TextureStreamBudget {
                    max_resident_bytes: u64::MAX,
                    max_upload_bytes: u64::MAX,
                },
                &mut out
            ),
            Err(TextureStreamPlanError::InvalidBlockLayout)
        );
        let mut overflow = demand(2);
        overflow.width = u32::MAX;
        overflow.height = u32::MAX;
        overflow.block_width = 1;
        overflow.block_height = 1;
        overflow.bytes_per_block = u8::MAX;
        overflow.mip_count = 1;
        overflow.desired_finest_mip = 0;
        overflow.resident_finest_mip = None;
        overflow.resident_bytes = 0;
        assert_eq!(
            plan_texture_residency(
                &[overflow],
                TextureStreamBudget {
                    max_resident_bytes: u64::MAX,
                    max_upload_bytes: u64::MAX,
                },
                &mut out
            ),
            Err(TextureStreamPlanError::ArithmeticOverflow)
        );
    }

    #[test]
    fn resident_budget_evicts_unprotected_farthest_oldest_first() {
        let mut near = demand(1);
        near.requested = false;
        near.resident_finest_mip = Some(1);
        near.desired_finest_mip = 1;
        near.resident_bytes = 64;
        near.distance_key = 1;
        let mut far = near;
        far.semantic_id = 2;
        far.distance_key = 99;
        let mut pinned = near;
        pinned.semantic_id = 3;
        pinned.pinned = true;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 0,
            bytes: 0,
        }; 2];
        let got = plan(
            &[near, far, pinned],
            TextureStreamBudget {
                max_resident_bytes: 128,
                max_upload_bytes: 0,
            },
            &mut out,
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].semantic_id, 2);
    }

    #[test]
    fn partial_plan_admits_only_global_priority_prefix_under_upload_budget() {
        let mut high = demand(2);
        high.importance = 10;
        let mut low = demand(1);
        low.importance = 1;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 8];
        let plan = plan_texture_residency_partial(
            &[low, high],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 16,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(plan.admitted_mip_count, 1);
        assert_eq!(plan.deferred_mip_count, 3);
        assert_eq!(plan.admitted_upload_bytes, 16);
        assert_eq!(plan.deferred_upload_bytes, 144);
        assert_eq!(out[0].semantic_id, 2);
        assert_eq!(out[0].mip_level, 2);
    }

    #[test]
    fn partial_plan_is_deterministic_for_ties_and_input_permutations() {
        let demands = [demand(9), demand(3)];
        let mut reversed = demands;
        reversed.reverse();
        let budget = TextureStreamBudget {
            max_resident_bytes: 1024,
            max_upload_bytes: 16,
        };
        let sentinel = TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        };
        let mut first = [sentinel; 4];
        let mut second = [sentinel; 4];
        let a = plan_texture_residency_partial(&demands, budget, &mut first).unwrap();
        let b = plan_texture_residency_partial(&reversed, budget, &mut second).unwrap();
        assert_eq!(a, b);
        assert_eq!(&first[..a.action_count], &second[..b.action_count]);
        assert_eq!(first[0].semantic_id, 3);
    }

    #[test]
    fn partial_plan_pinned_then_visible_requests_win_constrained_budget() {
        let mut ordinary = demand(1);
        ordinary.importance = u16::MAX;
        ordinary.distance_key = 0;
        let mut visible = demand(2);
        visible.visible = true;
        let mut pinned = demand(3);
        pinned.pinned = true;
        pinned.desired_finest_mip = 2;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 8];
        let plan = plan_texture_residency_partial(
            &[ordinary, visible, pinned],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 32,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(plan.admitted_mip_count, 2);
        assert_eq!(plan.deferred_mip_count, 3);
        assert_eq!(out[0].semantic_id, 3);
        assert_eq!(out[1].semantic_id, 2);
    }

    #[test]
    fn partial_plan_resident_budget_defers_after_admitting_fitting_prefix() {
        let mut high = demand(2);
        high.importance = 10;
        let mut low = demand(1);
        low.importance = 1;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 8];
        let plan = plan_texture_residency_partial(
            &[low, high],
            TextureStreamBudget {
                max_resident_bytes: 48,
                max_upload_bytes: 4096,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(plan.admitted_mip_count, 1);
        assert_eq!(plan.deferred_mip_count, 3);
        assert_eq!(plan.projected_resident_bytes, 48);
        assert_eq!(out[0].semantic_id, 2);
        assert_eq!(out[0].mip_level, 2);
    }

    #[test]
    fn partial_plan_evicts_unrequested_residency_only_as_needed() {
        let mut stale = demand(9);
        stale.requested = false;
        stale.resident_finest_mip = Some(1);
        stale.desired_finest_mip = 1;
        stale.resident_bytes = 64;
        stale.importance = 0;
        let mut wanted = demand(1);
        wanted.resident_bytes = 16;
        let mut out = [TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        }; 8];
        let plan = plan_texture_residency_partial(
            &[wanted, stale],
            TextureStreamBudget {
                max_resident_bytes: 32,
                max_upload_bytes: 16,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(out[0].kind, TextureStreamActionKind::EvictTexture);
        assert_eq!(out[0].semantic_id, 9);
        assert_eq!(plan.admitted_mip_count, 1);
        assert_eq!(plan.projected_resident_bytes, 32);
    }

    #[test]
    fn partial_plan_errors_leave_output_unchanged() {
        let sentinel = TextureStreamAction {
            semantic_id: 77,
            kind: TextureStreamActionKind::EvictTexture,
            mip_level: 5,
            bytes: 99,
        };
        let mut too_small = [sentinel; 1];
        assert_eq!(
            plan_texture_residency_partial(
                &[demand(1)],
                TextureStreamBudget {
                    max_resident_bytes: 1024,
                    max_upload_bytes: 4096,
                },
                &mut too_small,
            ),
            Err(TextureStreamPlanError::OutputCapacity {
                required: 2,
                capacity: 1,
            })
        );
        assert_eq!(too_small, [sentinel]);

        let mut protected = demand(2);
        protected.pinned = true;
        protected.requested = false;
        protected.resident_finest_mip = Some(1);
        protected.desired_finest_mip = 1;
        protected.resident_bytes = 64;
        let mut impossible = [sentinel; 8];
        assert_eq!(
            plan_texture_residency_partial(
                &[protected],
                TextureStreamBudget {
                    max_resident_bytes: 32,
                    max_upload_bytes: 0,
                },
                &mut impossible,
            ),
            Err(TextureStreamPlanError::ResidentBudgetExceeded {
                required: 64,
                budget: 32,
            })
        );
        assert!(impossible.iter().all(|action| *action == sentinel));
    }

    #[test]
    fn lifecycle_refinement_replaces_once_and_evicts_unrequested_resources() {
        let mut wanted = demand(1);
        wanted.resident_finest_mip = Some(2);
        wanted.resident_bytes = 20; // 2x2, 1x1 tail for an 8x8 RGBA8 chain
        wanted.desired_finest_mip = 0;
        let mut stale = demand(2);
        stale.requested = false;
        stale.visible = false;
        stale.resident_finest_mip = Some(1);
        stale.desired_finest_mip = 1;
        stale.resident_bytes = 84;
        let sentinel = TextureStreamLifecycleAction {
            semantic_id: 99,
            kind: TextureStreamLifecycleActionKind::EvictTexture,
            mip_level: 7,
            bytes: 9,
            previous_resident_bytes: 8,
            target_resident_bytes: 7,
        };
        let mut out = [sentinel; 2];
        let plan = plan_texture_residency_lifecycle(
            &[wanted, stale],
            TextureStreamBudget {
                max_resident_bytes: 400,
                max_upload_bytes: 256,
            },
            4,
            &mut out,
        )
        .unwrap();
        assert_eq!(plan.generation, 4);
        assert_eq!(plan.action_count, 2);
        assert_eq!(plan.projected_resident_bytes, 340);
        assert_eq!(out[0].kind, TextureStreamLifecycleActionKind::EvictTexture);
        assert_eq!(out[0].semantic_id, 2);
        assert_eq!(out[1].kind, TextureStreamLifecycleActionKind::ReplaceTexture);
        assert_eq!(out[1].mip_level, 0);
        assert_eq!(out[1].bytes, 256);
        assert_eq!(out[1].previous_resident_bytes, 20);
        assert_eq!(out[1].target_resident_bytes, 340);
    }

    #[test]
    fn lifecycle_plan_is_atomic_and_generation_stamped() {
        let sentinel = TextureStreamLifecycleAction {
            semantic_id: 77,
            kind: TextureStreamLifecycleActionKind::UploadTexture,
            mip_level: 3,
            bytes: 1,
            previous_resident_bytes: 2,
            target_resident_bytes: 3,
        };
        let mut out = [sentinel; 1];
        let mut malformed = demand(1);
        malformed.resident_bytes = 0;
        let error = plan_texture_residency_lifecycle(
            &[malformed],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 4096,
            },
            9,
            &mut out,
        );
        assert_eq!(
            error,
            Err(TextureStreamPlanError::InconsistentResidency)
        );
        assert_eq!(out[0], sentinel);

        let mut valid = demand(1);
        valid.resident_bytes = 4;
        valid.desired_finest_mip = 1;
        let mut valid_out = [sentinel; 1];
        let plan = plan_texture_residency_lifecycle(
            &[valid],
            TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 4096,
            },
            9,
            &mut valid_out,
        )
        .unwrap();
        assert_eq!(plan.generation, 9);
    }
}
