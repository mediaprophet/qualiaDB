//! GPU-less fault injection over residency, draw overflow, and activation.

use crate::container_10d::cluster_dag_section::MeshletClusterNode;

use super::super::geometry_workspace::production_budget::ProductionBudget;
use super::super::production_contracts::{ContentId, TimingSource};
use super::super::production_runtime::{
    ActivationBoundary, ActivationFault, ActivationStore, DrawIndexedCommand, PageState,
    QualifiedSet, ReplacementLink, ResidencyTable, RuntimeError, ValidatedHierarchy,
    ROLLBACK_DOES_NOT_UNDO_SIMULATION,
};
use super::phases::{
    activate_qualified, admit_hierarchy, compact_mask, cpu_timed, decode_nodes, draw_commands,
    gpu_execution_record, status_from_runtime, GpuExecutionRecord, LiveGpuDevice, MigrationEffects,
    PhaseKind, PhaseLedger, PhaseRecord, PhaseStatus,
};
use super::LifecycleError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectedFault {
    DeviceLostAtActivation,
    IncompleteReplacement,
    DrawCommandOverflow,
    CancelledInflightTransfer,
    MixedGenerationActivation,
    IncompleteAsset,
    InvalidatedEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultStage {
    Activated,
    RolledBack,
    Faulted(InjectedFault),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTransfer {
    pub index: u32,
    pub transfer_id: u32,
    pub size: u32,
}

pub struct LifecycleInput<'a> {
    pub encoded_cdag: &'a [u8],
    pub links: &'a [ReplacementLink],
    pub children: &'a [u32],
    pub roots: &'a [u32],
    pub source: ContentId,
    pub source_primitive_order: &'a [u32],
    pub seen: &'a mut [bool],
    pub uploads: &'a [PageTransfer],
    pub refine_parent: Option<usize>,
    pub residency_cap_bytes: u32,
    pub visibility: &'a [u32],
    pub baseline: Option<QualifiedSet>,
    pub qualified: QualifiedSet,
    pub consume_simulation: u64,
    pub rollback: bool,
    pub device: Option<LiveGpuDevice<'a>>,
    pub injected: Option<InjectedFault>,
}

pub struct LifecycleBuffers<'a> {
    pub nodes: &'a mut [MeshletClusterNode],
    pub page_states: &'a mut [PageState],
    pub page_bytes: &'a mut [u32],
    pub stack: &'a mut [u32],
    pub cut: &'a mut [u32],
    pub mask: &'a mut [u32],
    pub scratch_prefix: &'a mut [u32],
    pub compact_indices: &'a mut [u32],
    pub commands: &'a mut [DrawIndexedCommand],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleReport {
    pub ledger: PhaseLedger,
    pub gpu_execution: GpuExecutionRecord,
    pub stage: FaultStage,
    pub cut_len: usize,
    pub compacted: usize,
    pub command_count: usize,
    pub current: Option<QualifiedSet>,
    pub previous: Option<QualifiedSet>,
    pub simulation_consumed: u64,
    pub device_lost: bool,
    pub runtime: Option<RuntimeError>,
    pub injected: Option<InjectedFault>,
    pub rollback_restores_simulation: bool,
    pub migration: MigrationEffects,
}

#[derive(Clone, Copy)]
struct Counts {
    cut: usize,
    compacted: usize,
    commands: usize,
}

const ZERO: Counts = Counts { cut: 0, compacted: 0, commands: 0 };

fn hold_one(record: &mut PhaseRecord, kind: PhaseKind, stop: PhaseStatus) {
    if record.work_units == 0 && record.status == PhaseStatus::Inconclusive {
        *record = PhaseRecord::hold(kind, stop);
    }
}

fn hold_rest(ledger: &mut PhaseLedger, stop: PhaseStatus) {
    hold_one(&mut ledger.upload, PhaseKind::Upload, stop);
    hold_one(&mut ledger.compaction, PhaseKind::Compaction, stop);
    hold_one(&mut ledger.draw, PhaseKind::Draw, stop);
    hold_one(&mut ledger.activation, PhaseKind::Activation, stop);
}

fn out(
    mut ledger: PhaseLedger,
    gpu: GpuExecutionRecord,
    stage: FaultStage,
    n: Counts,
    store: &ActivationStore,
    runtime: Option<RuntimeError>,
    injected: Option<InjectedFault>,
    stop: PhaseStatus,
) -> LifecycleReport {
    hold_rest(&mut ledger, stop);
    LifecycleReport {
        ledger,
        gpu_execution: gpu,
        stage,
        cut_len: n.cut,
        compacted: n.compacted,
        command_count: n.commands,
        current: store.current(),
        previous: store.previous(),
        simulation_consumed: store.simulation_consumed(),
        device_lost: store.device_lost(),
        runtime,
        injected,
        rollback_restores_simulation: !ROLLBACK_DOES_NOT_UNDO_SIMULATION,
        migration: MigrationEffects::simulated(matches!(stage, FaultStage::RolledBack)),
    }
}

fn upload_pages(
    table: &mut ResidencyTable<'_>,
    hierarchy: &ValidatedHierarchy<'_>,
    uploads: &[PageTransfer],
    refine_parent: Option<usize>,
    injected: Option<InjectedFault>,
    stack: &mut [u32],
    cut: &mut [u32],
    budget: &mut ProductionBudget<'_>,
) -> Result<(usize, u64), RuntimeError> {
    let mut bytes = 0u64;
    let cancel = injected == Some(InjectedFault::CancelledInflightTransfer);
    let incomplete = injected == Some(InjectedFault::IncompleteReplacement);
    let last = uploads.len().saturating_sub(1);
    for (i, page) in uploads.iter().enumerate() {
        let index = page.index as usize;
        table.begin_transfer(index, page.transfer_id, page.size, budget)?;
        bytes = bytes.saturating_add(u64::from(page.size));
        if cancel && i == 0 {
            table.cancel_transfer(index, page.transfer_id, budget)?;
            return Err(RuntimeError::Unsupported);
        }
        if incomplete && i == last {
            return Err(match refine_parent {
                Some(parent) => table
                    .activate_replacement(hierarchy, parent, budget)
                    .err()
                    .unwrap_or(RuntimeError::MissingFallback),
                None => RuntimeError::MissingFallback,
            });
        }
        table.complete_transfer(index, page.transfer_id, budget)?;
    }
    if let Some(parent) = refine_parent {
        table.activate_replacement(hierarchy, parent, budget)?;
    }
    Ok((table.renderable_cut(hierarchy, stack, cut, budget)?, bytes))
}

fn activation_fault(
    injected: Option<InjectedFault>,
    mut set: QualifiedSet,
) -> (QualifiedSet, bool, Option<ActivationFault>) {
    match injected {
        Some(InjectedFault::DeviceLostAtActivation) => (set, false, Some(ActivationFault::DeviceLost)),
        Some(InjectedFault::MixedGenerationActivation) => {
            set.shader_generation = set.mesh_generation.wrapping_add(1);
            set.transfer_generation = set.mesh_generation.wrapping_add(2);
            (set, true, Some(ActivationFault::MixedVersion))
        }
        Some(InjectedFault::IncompleteAsset) => {
            set.assets_complete = false;
            (set, true, Some(ActivationFault::IncompleteAsset))
        }
        Some(InjectedFault::InvalidatedEvidence) => {
            set.evidence_valid = false;
            (set, true, Some(ActivationFault::InvalidatedEvidence))
        }
        _ => (set, true, None),
    }
}

/// CPU-reference lifecycle with optional GPU-less fault injection.
pub fn run_lifecycle(
    input: LifecycleInput<'_>,
    buffers: LifecycleBuffers<'_>,
    budget: &mut ProductionBudget<'_>,
) -> Result<LifecycleReport, LifecycleError> {
    let gpu = gpu_execution_record(input.device);
    let injected = input.injected;
    let fault = |fallback| FaultStage::Faulted(injected.unwrap_or(fallback));
    let mut ledger = PhaseLedger::unrun();
    let mut store = ActivationStore::new();
    store.declare_boundary(
        input
            .baseline
            .map(|set| set.boundary)
            .unwrap_or(input.qualified.boundary),
    );
    if let Some(baseline) = input.baseline {
        store.activate(baseline, true, None, budget)?;
        store.consume_simulation(input.consume_simulation)?;
        debug_assert!(ROLLBACK_DOES_NOT_UNDO_SIMULATION);
    }

    let LifecycleBuffers {
        nodes,
        page_states,
        page_bytes,
        stack,
        cut,
        mask,
        scratch_prefix,
        compact_indices,
        commands,
    } = buffers;

    let decode_work = budget.work_used();
    let (decode, decoded) = decode_nodes(input.encoded_cdag, nodes, budget);
    ledger.decode = decode;
    let count = match decoded {
        Ok(count) => count,
        Err(error) => {
            return Ok(out(
                ledger, gpu, fault(InjectedFault::IncompleteReplacement), ZERO, &store,
                Some(error), injected, decode.status,
            ));
        }
    };
    let hierarchy = match admit_hierarchy(
        &nodes[..count],
        input.links,
        input.children,
        input.roots,
        input.source,
        input.source_primitive_order,
        input.seen,
        budget,
    ) {
        Ok(hierarchy) => hierarchy,
        Err(error) => {
            ledger.decode.status = status_from_runtime(error);
            ledger.decode.work_units = budget.work_used().saturating_sub(decode_work);
            return Ok(out(
                ledger, gpu, fault(InjectedFault::IncompleteReplacement), ZERO, &store,
                Some(error), injected, ledger.decode.status,
            ));
        }
    };
    ledger.decode.work_units = budget.work_used().saturating_sub(decode_work);

    let mut table = match ResidencyTable::admit(
        &hierarchy, page_states, page_bytes, input.residency_cap_bytes, budget,
    ) {
        Ok(table) => table,
        Err(error) => {
            ledger.upload = PhaseRecord::accounted(
                PhaseKind::Upload, 0, 0, status_from_runtime(error), TimingSource::NotMeasured, None,
            )?;
            return Ok(out(
                ledger, gpu, fault(InjectedFault::IncompleteReplacement), ZERO, &store,
                Some(error), injected, ledger.upload.status,
            ));
        }
    };

    let (upload, uploaded) = cpu_timed(PhaseKind::Upload, 0, budget, |budget| {
        upload_pages(
            &mut table, &hierarchy, input.uploads, input.refine_parent, injected, stack, cut, budget,
        )
    });
    drop(table);
    let (cut_len, upload_bytes) = match uploaded {
        Ok(pair) => pair,
        Err(_) if injected == Some(InjectedFault::CancelledInflightTransfer) => {
            ledger.upload = PhaseRecord {
                status: PhaseStatus::Cancelled,
                bytes: 0,
                timing_source: TimingSource::CpuMonotonic,
                ..upload
            };
            return Ok(out(
                ledger, gpu, FaultStage::Faulted(InjectedFault::CancelledInflightTransfer), ZERO,
                &store, None, injected, PhaseStatus::Cancelled,
            ));
        }
        Err(error) => {
            ledger.upload = PhaseRecord { bytes: 0, ..upload };
            return Ok(out(
                ledger, gpu, fault(InjectedFault::IncompleteReplacement), ZERO, &store,
                Some(error), injected, upload.status,
            ));
        }
    };
    ledger.upload = PhaseRecord { bytes: upload_bytes, ..upload };

    if mask.len() < cut_len {
        ledger.compaction = PhaseRecord::accounted(
            PhaseKind::Compaction, 0, 0, PhaseStatus::Failed, TimingSource::NotMeasured, None,
        )?;
        return Ok(out(
            ledger, gpu, FaultStage::Faulted(InjectedFault::DrawCommandOverflow),
            Counts { cut: cut_len, compacted: 0, commands: 0 }, &store,
            Some(RuntimeError::Capacity), injected, PhaseStatus::Failed,
        ));
    }
    if input.visibility.is_empty() {
        mask[..cut_len].fill(1);
    } else if input.visibility.len() < cut_len {
        return Err(LifecycleError::Capacity {
            required: cut_len,
            available: input.visibility.len(),
        });
    } else {
        mask[..cut_len].copy_from_slice(&input.visibility[..cut_len]);
    }

    let (compaction, compacted) = compact_mask(&mask[..cut_len], scratch_prefix, compact_indices, budget);
    ledger.compaction = compaction;
    let compacted_n = match compacted {
        Ok(n) => n,
        Err(error) => {
            commands.fill(DrawIndexedCommand::EMPTY);
            return Ok(out(
                ledger, gpu, fault(InjectedFault::DrawCommandOverflow),
                Counts { cut: cut_len, compacted: 0, commands: 0 }, &store,
                Some(error), injected, compaction.status,
            ));
        }
    };

    let limit = if injected == Some(InjectedFault::DrawCommandOverflow) {
        compacted_n.saturating_sub(1).min(commands.len())
    } else {
        commands.len()
    };
    let (draw, drawn) = draw_commands(
        &hierarchy, &cut[..cut_len], &compact_indices[..compacted_n], &mut commands[..limit], budget,
    );
    ledger.draw = draw;
    let generation = match drawn {
        Ok(generation) => generation,
        Err(error) => {
            commands.fill(DrawIndexedCommand::EMPTY);
            return Ok(out(
                ledger, gpu, fault(InjectedFault::DrawCommandOverflow),
                Counts { cut: cut_len, compacted: compacted_n, commands: 0 }, &store,
                Some(error), injected, draw.status,
            ));
        }
    };

    let (set, device_ok, act_fault) = activation_fault(injected, input.qualified);
    if input.baseline.is_none() && input.consume_simulation > 0 {
        store.consume_simulation(input.consume_simulation)?;
    }
    let (activation, activated) = activate_qualified(&mut store, set, device_ok, act_fault, budget);
    ledger.activation = activation;
    let n = Counts {
        cut: cut_len,
        compacted: compacted_n,
        commands: generation.command_count,
    };
    if let Err(error) = activated {
        let injected_fault = match injected {
            Some(
                InjectedFault::DeviceLostAtActivation
                | InjectedFault::MixedGenerationActivation
                | InjectedFault::IncompleteAsset
                | InjectedFault::InvalidatedEvidence,
            ) => injected.unwrap(),
            _ => InjectedFault::DeviceLostAtActivation,
        };
        return Ok(out(
            ledger, gpu, FaultStage::Faulted(injected_fault), n, &store, Some(error), injected,
            activation.status,
        ));
    }

    let mut stage = FaultStage::Activated;
    if input.rollback {
        store.rollback(budget)?;
        stage = FaultStage::RolledBack;
        debug_assert!(ROLLBACK_DOES_NOT_UNDO_SIMULATION);
    }
    Ok(out(ledger, gpu, stage, n, &store, None, injected, PhaseStatus::Passed))
}

pub const fn default_boundary() -> ActivationBoundary {
    ActivationBoundary::Frame(1)
}
