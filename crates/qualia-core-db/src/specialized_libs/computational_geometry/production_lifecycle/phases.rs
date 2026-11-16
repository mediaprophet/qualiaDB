//! Typed per-phase ledger. Statuses stay separate; never a blended score.

use std::marker::PhantomData;
use std::time::Instant;

use crate::container_10d::cluster_dag_section::{
    decode_cluster_dag_section, MeshletClusterNode,
};

use super::super::geometry_workspace::production_budget::{
    BudgetError, BufferClass, CapacityPlan, ProductionBudget,
};
use super::super::production_contracts::{ContentId, TimingSource};
use super::super::production_runtime::{
    generate_indexed_draws, scan_compact, ActivationFault, ActivationStore, DrawGeneration,
    DrawIndexedCommand, PageState, QualifiedSet, ReplacementLink, RuntimeError,
    ValidatedHierarchy, DRAW_INDEXED_STRIDE_BYTES,
};
use super::LifecycleError;

pub const GPU_ABSENT_NOTE: &str =
    "no live GPU device supplied; P22 does not invent wgpu::Device";
pub const GPU_NO_DISPATCH_NOTE: &str =
    "live device supplied; P22 records accounting only and does not dispatch GPU work";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseKind {
    Decode,
    Upload,
    Compaction,
    Draw,
    Activation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStatus {
    Passed,
    Failed,
    Inconclusive,
    Unsupported,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseRecord {
    pub kind: PhaseKind,
    pub work_units: u64,
    pub bytes: u64,
    pub status: PhaseStatus,
    pub timing_source: TimingSource,
    pub duration_ns: Option<u64>,
}

impl PhaseRecord {
    pub const fn unrun(kind: PhaseKind) -> Self {
        Self {
            kind,
            work_units: 0,
            bytes: 0,
            status: PhaseStatus::Inconclusive,
            timing_source: TimingSource::NotMeasured,
            duration_ns: None,
        }
    }

    /// CPU/synthetic accounting. Rejects `GpuTimestamp`.
    pub fn accounted(
        kind: PhaseKind,
        work_units: u64,
        bytes: u64,
        status: PhaseStatus,
        timing_source: TimingSource,
        duration_ns: Option<u64>,
    ) -> Result<Self, LifecycleError> {
        if timing_source == TimingSource::GpuTimestamp {
            return Err(LifecycleError::DishonestTiming);
        }
        Ok(Self {
            kind,
            work_units,
            bytes,
            status,
            timing_source,
            duration_ns,
        })
    }

    pub fn hold(kind: PhaseKind, prior: PhaseStatus) -> Self {
        let status = match prior {
            PhaseStatus::Cancelled => PhaseStatus::Cancelled,
            _ => PhaseStatus::Inconclusive,
        };
        Self {
            kind,
            work_units: 0,
            bytes: 0,
            status,
            timing_source: TimingSource::NotMeasured,
            duration_ns: None,
        }
    }
}

/// Five independent records. There is no aggregate score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseLedger {
    pub decode: PhaseRecord,
    pub upload: PhaseRecord,
    pub compaction: PhaseRecord,
    pub draw: PhaseRecord,
    pub activation: PhaseRecord,
}

impl PhaseLedger {
    pub fn unrun() -> Self {
        Self {
            decode: PhaseRecord::unrun(PhaseKind::Decode),
            upload: PhaseRecord::unrun(PhaseKind::Upload),
            compaction: PhaseRecord::unrun(PhaseKind::Compaction),
            draw: PhaseRecord::unrun(PhaseKind::Draw),
            activation: PhaseRecord::unrun(PhaseKind::Activation),
        }
    }

    pub fn records(self) -> [PhaseRecord; 5] {
        [self.decode, self.upload, self.compaction, self.draw, self.activation]
    }

    pub fn cpu_times_are_not_gpu_timestamps(self) -> bool {
        self.records()
            .iter()
            .all(|record| record.timing_source != TimingSource::GpuTimestamp)
    }
}

/// Caller-owned live device identity. This adapter never constructs wgpu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveGpuDevice<'a> {
    pub generation: u64,
    gpu_timestamp_ns: Option<u64>,
    _borrow: PhantomData<&'a ()>,
}

impl<'a> LiveGpuDevice<'a> {
    /// Bind an already-live device. `gpu_timestamp_ns` must be a real GPU
    /// timestamp-query result; this adapter never synthesizes one.
    pub const fn bind_live(generation: u64, gpu_timestamp_ns: Option<u64>) -> Self {
        Self {
            generation,
            gpu_timestamp_ns,
            _borrow: PhantomData,
        }
    }

    pub const fn gpu_timestamp_ns(self) -> Option<u64> {
        self.gpu_timestamp_ns
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuExecutionRecord {
    pub status: PhaseStatus,
    pub timing_source: TimingSource,
    pub timestamp_ns: Option<u64>,
    pub note: &'static str,
}

pub fn gpu_execution_record(device: Option<LiveGpuDevice<'_>>) -> GpuExecutionRecord {
    match device {
        None => GpuExecutionRecord {
            status: PhaseStatus::Unsupported,
            timing_source: TimingSource::NotMeasured,
            timestamp_ns: None,
            note: GPU_ABSENT_NOTE,
        },
        Some(device) => {
            let ts = device.gpu_timestamp_ns;
            GpuExecutionRecord {
                status: PhaseStatus::Inconclusive,
                timing_source: if ts.is_some() {
                    TimingSource::GpuTimestamp
                } else {
                    TimingSource::NotMeasured
                },
                timestamp_ns: ts,
                note: GPU_NO_DISPATCH_NOTE,
            }
        }
    }
}

pub fn status_from_runtime(error: RuntimeError) -> PhaseStatus {
    match error {
        RuntimeError::Budget(BudgetError::Cancelled) => PhaseStatus::Cancelled,
        RuntimeError::Budget(BudgetError::WorkLimitExceeded { .. }) => PhaseStatus::Inconclusive,
        RuntimeError::Unsupported => PhaseStatus::Unsupported,
        _ => PhaseStatus::Failed,
    }
}

pub fn plan_lifecycle_capacity(
    encoded_bytes: usize,
    node_count: usize,
    link_count: usize,
    children_count: usize,
    root_count: usize,
    primitive_count: usize,
    cut_cap: usize,
    command_cap: usize,
) -> Result<CapacityPlan, BudgetError> {
    let mut plan = CapacityPlan::default();
    plan.add_bytes(BufferClass::Input, encoded_bytes)?;
    plan.add_slice::<MeshletClusterNode>(BufferClass::Output, node_count)?;
    plan.add_slice::<ReplacementLink>(BufferClass::Input, link_count)?;
    plan.add_slice::<u32>(BufferClass::Input, children_count)?;
    plan.add_slice::<u32>(BufferClass::Input, root_count)?;
    plan.add_slice::<u32>(BufferClass::Input, primitive_count)?;
    plan.add_slice::<bool>(BufferClass::Scratch, primitive_count)?;
    plan.add_slice::<PageState>(BufferClass::Scratch, node_count)?;
    plan.add_slice::<u32>(BufferClass::Scratch, node_count)?;
    plan.add_slice::<u32>(BufferClass::Scratch, cut_cap)?;
    plan.add_slice::<u32>(BufferClass::Output, cut_cap)?;
    plan.add_slice::<u32>(BufferClass::Scratch, cut_cap)?;
    plan.add_slice::<u32>(BufferClass::Scratch, cut_cap)?;
    plan.add_slice::<u32>(BufferClass::Scratch, cut_cap)?;
    plan.add_slice::<DrawIndexedCommand>(BufferClass::Output, command_cap)?;
    plan.add_slice::<PhaseLedger>(BufferClass::Output, 1)?;
    Ok(plan)
}

pub(crate) fn cpu_timed<T>(
    kind: PhaseKind,
    bytes: u64,
    budget: &mut ProductionBudget<'_>,
    body: impl FnOnce(&mut ProductionBudget<'_>) -> Result<T, RuntimeError>,
) -> (PhaseRecord, Result<T, RuntimeError>) {
    let before = budget.work_used();
    let start = Instant::now();
    let value = body(budget);
    let ns = start.elapsed().as_nanos();
    let duration_ns = if ns > u64::MAX as u128 {
        u64::MAX
    } else {
        ns as u64
    };
    let work_units = budget.work_used().saturating_sub(before);
    let status = match &value {
        Ok(_) => PhaseStatus::Passed,
        Err(error) => status_from_runtime(*error),
    };
    let record = PhaseRecord {
        kind,
        work_units,
        bytes,
        status,
        timing_source: TimingSource::CpuMonotonic,
        duration_ns: Some(duration_ns),
    };
    debug_assert_ne!(record.timing_source, TimingSource::GpuTimestamp);
    (record, value)
}

pub fn decode_nodes(
    encoded: &[u8],
    nodes: &mut [MeshletClusterNode],
    budget: &mut ProductionBudget<'_>,
) -> (PhaseRecord, Result<usize, RuntimeError>) {
    cpu_timed(PhaseKind::Decode, encoded.len() as u64, budget, |budget| {
        budget.checkpoint(1)?;
        let (_header, count) = decode_cluster_dag_section(encoded, nodes)
            .map_err(|_| RuntimeError::InvalidHierarchy)?;
        budget.checkpoint(count as u64)?;
        Ok(count)
    })
}

pub fn admit_hierarchy<'a>(
    nodes: &'a [MeshletClusterNode],
    links: &'a [ReplacementLink],
    children: &'a [u32],
    roots: &'a [u32],
    source: ContentId,
    source_primitive_order: &[u32],
    seen: &mut [bool],
    budget: &mut ProductionBudget<'_>,
) -> Result<ValidatedHierarchy<'a>, RuntimeError> {
    ValidatedHierarchy::admit(
        nodes,
        links,
        children,
        roots,
        source,
        source_primitive_order,
        seen,
        budget,
    )
}

pub fn compact_mask(
    mask: &[u32],
    scratch_prefix: &mut [u32],
    out_indices: &mut [u32],
    budget: &mut ProductionBudget<'_>,
) -> (PhaseRecord, Result<usize, RuntimeError>) {
    let bytes = (mask.len() + scratch_prefix.len() + out_indices.len()) as u64 * 4;
    cpu_timed(PhaseKind::Compaction, bytes, budget, |budget| {
        scan_compact(mask, scratch_prefix, out_indices, budget)
    })
}

pub fn draw_commands(
    hierarchy: &ValidatedHierarchy<'_>,
    cut: &[u32],
    compact_indices: &[u32],
    out_commands: &mut [DrawIndexedCommand],
    budget: &mut ProductionBudget<'_>,
) -> (PhaseRecord, Result<DrawGeneration, RuntimeError>) {
    let bytes = (out_commands.len() * DRAW_INDEXED_STRIDE_BYTES) as u64;
    cpu_timed(PhaseKind::Draw, bytes, budget, |budget| {
        generate_indexed_draws(hierarchy, cut, compact_indices, out_commands, budget)
    })
}

pub fn activate_qualified(
    store: &mut ActivationStore,
    set: QualifiedSet,
    device_ok: bool,
    injected: Option<ActivationFault>,
    budget: &mut ProductionBudget<'_>,
) -> (PhaseRecord, Result<(), RuntimeError>) {
    let bytes = core::mem::size_of::<QualifiedSet>() as u64;
    cpu_timed(PhaseKind::Activation, bytes, budget, |budget| {
        store.activate(set, device_ok, injected, budget)
    })
}
