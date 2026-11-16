use crate::container_10d::cluster_dag_section::{
    encode_cluster_dag_section, encode_oct16_normal, ClusterDagMiniHeader, MeshletClusterNode,
    FLAG_CDAG_CONE_CULLED, FLAG_CDAG_SEAM_LOCKED,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;
use crate::specialized_libs::computational_geometry::geometry_workspace::Cancellation;
use crate::specialized_libs::computational_geometry::production_contracts::ContentId;
use crate::specialized_libs::computational_geometry::production_runtime::{
    DrawIndexedCommand, PageState, ReplacementLink, RuntimeError, DRAW_INDEXED_STRIDE_BYTES,
};

use super::*;

fn cid(tag: &str) -> ContentId {
    ContentId::sha256(tag.as_bytes())
}

fn cluster(
    sphere: [f32; 4],
    parent_sphere: [f32; 4],
    err_self: f32,
    err_parent: f32,
    lod: u8,
    child_group_first: u16,
    vertex_off: u32,
    tri_off: u32,
) -> MeshletClusterNode {
    MeshletClusterNode {
        bounding_sphere: sphere,
        parent_error_sphere: parent_sphere,
        lod_error_self: err_self,
        lod_error_parent: err_parent,
        cone_axis_oct16: encode_oct16_normal([0.0, 0.0, 1.0]),
        cone_cutoff_snorm16: -16383,
        lod_level: lod,
        vertex_count: 8,
        triangle_count: 4,
        material_slot: 0,
        child_group_first,
        vertex_data_offset: vertex_off,
        triangle_data_offset: tri_off,
        semantic_subpart_id: 1,
    }
}

struct Tree {
    links: [ReplacementLink; 3],
    children: [u32; 2],
    roots: [u32; 1],
    order: [u32; 4],
    source: ContentId,
    encoded: [u8; 256],
    encoded_len: usize,
}

impl Tree {
    fn new() -> Self {
        let border = cid("border");
        let root = [0.0, 0.0, 0.0, 8.0];
        let a = [-2.0, 0.0, 0.0, 3.0];
        let b = [2.0, 0.0, 0.0, 3.0];
        let nodes = [
            cluster(root, root, 0.08, f32::INFINITY, 1, 0, 0, 0),
            cluster(a, root, 0.0, 0.08, 0, 0xFFFF, 8, 12),
            cluster(b, root, 0.0, 0.08, 0, 0xFFFF, 16, 24),
        ];
        let mut encoded = [0u8; 256];
        let header = ClusterDagMiniHeader {
            max_lod_depth: 1,
            cluster_count: 3,
            group_count: 1,
            flags: FLAG_CDAG_SEAM_LOCKED | FLAG_CDAG_CONE_CULLED,
            ..ClusterDagMiniHeader::default()
        };
        let encoded_len = encode_cluster_dag_section(&header, &nodes, &mut encoded).unwrap();
        Self {
            links: [
                ReplacementLink {
                    parent: None,
                    children_start: 0,
                    children_count: 2,
                    coverage_start: 0,
                    coverage_count: 4,
                    border_identity: border,
                },
                ReplacementLink {
                    parent: Some(0),
                    children_start: 0,
                    children_count: 0,
                    coverage_start: 0,
                    coverage_count: 2,
                    border_identity: border,
                },
                ReplacementLink {
                    parent: Some(0),
                    children_start: 0,
                    children_count: 0,
                    coverage_start: 2,
                    coverage_count: 2,
                    border_identity: border,
                },
            ],
            children: [1, 2],
            roots: [0],
            order: [0, 1, 2, 3],
            source: cid("source-tree"),
            encoded,
            encoded_len,
        }
    }
}

fn qualified(mesh: &str, generation: u64) -> QualifiedSet {
    QualifiedSet {
        mesh: cid(mesh),
        shader: cid("shader"),
        transfer: cid("xfer"),
        evidence: cid("ev"),
        mesh_generation: generation,
        shader_generation: generation,
        transfer_generation: generation,
        assets_complete: true,
        evidence_valid: true,
        boundary: default_boundary(),
    }
}

fn uploads() -> [PageTransfer; 2] {
    [
        PageTransfer {
            index: 1,
            transfer_id: 11,
            size: 40,
        },
        PageTransfer {
            index: 2,
            transfer_id: 12,
            size: 40,
        },
    ]
}

struct Scratch {
    nodes: [MeshletClusterNode; 3],
    states: [PageState; 3],
    bytes: [u32; 3],
    stack: [u32; 3],
    cut: [u32; 3],
    mask: [u32; 3],
    prefix: [u32; 3],
    compact: [u32; 3],
    commands: [DrawIndexedCommand; 3],
}

impl Scratch {
    fn new() -> Self {
        Self {
            nodes: [cluster(
                [0.0, 0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0, 1.0],
                0.0,
                f32::INFINITY,
                0,
                0xFFFF,
                0,
                0,
            ); 3],
            states: [PageState::Absent; 3],
            bytes: [100, 40, 40],
            stack: [0; 3],
            cut: [0; 3],
            mask: [0; 3],
            prefix: [0; 3],
            compact: [0; 3],
            commands: [DrawIndexedCommand::EMPTY; 3],
        }
    }

    fn buffers(&mut self) -> LifecycleBuffers<'_> {
        LifecycleBuffers {
            nodes: &mut self.nodes,
            page_states: &mut self.states,
            page_bytes: &mut self.bytes,
            stack: &mut self.stack,
            cut: &mut self.cut,
            mask: &mut self.mask,
            scratch_prefix: &mut self.prefix,
            compact_indices: &mut self.compact,
            commands: &mut self.commands,
        }
    }
}

fn run(
    tree: &Tree,
    scratch: &mut Scratch,
    injected: Option<InjectedFault>,
    baseline: Option<QualifiedSet>,
    rollback: bool,
    device: Option<LiveGpuDevice<'_>>,
) -> LifecycleReport {
    let cancel = Cancellation::new();
    let plan = plan_lifecycle_capacity(tree.encoded_len, 3, 3, 2, 1, 4, 3, 3).unwrap();
    let mut budget = ProductionBudget::admit(plan, usize::MAX, 10_000, &cancel).unwrap();
    let pages = uploads();
    let mut seen = [false; 4];
    let input = LifecycleInput {
        encoded_cdag: &tree.encoded[..tree.encoded_len],
        links: &tree.links,
        children: &tree.children,
        roots: &tree.roots,
        source: tree.source,
        source_primitive_order: &tree.order,
        seen: &mut seen,
        uploads: &pages,
        refine_parent: Some(0),
        residency_cap_bytes: 200,
        visibility: &[],
        baseline,
        qualified: qualified("cand", 2),
        consume_simulation: 3,
        rollback,
        device,
        injected,
    };
    run_lifecycle(input, scratch.buffers(), &mut budget).unwrap()
}

fn assert_cpu_ledger(ledger: PhaseLedger) {
    assert!(ledger.cpu_times_are_not_gpu_timestamps());
    let records = ledger.records();
    assert_eq!(records.len(), 5);
    assert_eq!(
        [
            records[0].kind,
            records[1].kind,
            records[2].kind,
            records[3].kind,
            records[4].kind,
        ],
        [
            PhaseKind::Decode,
            PhaseKind::Upload,
            PhaseKind::Compaction,
            PhaseKind::Draw,
            PhaseKind::Activation
        ]
    );
    for record in records {
        assert!(
            record.timing_source == TimingSource::CpuMonotonic
                || record.timing_source == TimingSource::NotMeasured
        );
        assert_ne!(record.timing_source, TimingSource::GpuTimestamp);
        assert_ne!(record.timing_source, TimingSource::CompletionClock);
    }
}

fn assert_gpu_token(report: &LifecycleReport, present: bool) {
    assert!(!report.gpu_execution.claims_passed());
    assert_ne!(report.gpu_execution.status, PhaseStatus::Passed);
    assert_eq!(report.gpu_execution.timing_source, TimingSource::NotMeasured);
    assert_eq!(report.gpu_execution.timestamp_ns, None);
    if present {
        assert_eq!(report.gpu_execution.status, PhaseStatus::Inconclusive);
        assert_eq!(report.gpu_execution.note, GPU_NO_DISPATCH_NOTE);
    } else {
        assert_eq!(report.gpu_execution.status, PhaseStatus::Unsupported);
        assert_eq!(report.gpu_execution.note, GPU_ABSENT_NOTE);
    }
}

fn assert_migration(report: &LifecycleReport) {
    assert!(!report.rollback_restores_simulation);
    assert!(!report.migration.simulation_restored);
    assert_eq!(report.migration.energy.status, PhaseStatus::Unsupported);
    assert_eq!(report.migration.momentum.status, PhaseStatus::Unsupported);
    assert_eq!(report.migration.energy.note, ENERGY_ABSENT_NOTE);
    assert_eq!(report.migration.momentum.note, MOMENTUM_ABSENT_NOTE);
    assert_eq!(
        report.migration.identities_rolled_back,
        report.stage == FaultStage::RolledBack
    );
}

#[test]
fn happy_cpu_reference_records_decode_compact_draw_without_claiming_gpu() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let report = run(&tree, &mut scratch, None, None, false, None);
    assert_eq!(report.ledger.decode.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.upload.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.compaction.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Passed);
    assert!(report.ledger.decode.work_units > 0);
    assert!(report.ledger.decode.bytes > 0);
    assert_eq!(report.ledger.draw.bytes, (3 * DRAW_INDEXED_STRIDE_BYTES) as u64);
    assert_eq!(report.compacted, 2);
    assert_eq!(report.command_count, 2);
    assert_eq!(scratch.commands[0].index_count, 12);
    assert_eq!(scratch.commands[0].instance_count, 1);
    assert_eq!(report.stage, FaultStage::Activated);
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
    assert!(!report.rollback_restores_simulation);
    assert!(ROLLBACK_DOES_NOT_UNDO_SIMULATION);
}

#[test]
fn device_lost_around_activation_keeps_baseline_and_simulation() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let baseline = qualified("base", 1);
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::DeviceLostAtActivation),
        Some(baseline),
        false,
        None,
    );
    assert_eq!(report.ledger.decode.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::DeviceLost));
    assert!(report.device_lost);
    assert_eq!(report.current, Some(baseline));
    assert_eq!(report.simulation_consumed, 3);
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::DeviceLostAtActivation)
    );
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
    assert!(!report.migration.identities_rolled_back);
}

#[test]
fn incomplete_replacement_fails_upload_without_activating_missing_page() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::IncompleteReplacement),
        None,
        false,
        None,
    );
    assert_eq!(report.ledger.decode.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.upload.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::MissingFallback));
    assert_eq!(report.ledger.compaction.status, PhaseStatus::Inconclusive);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Inconclusive);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Inconclusive);
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::IncompleteReplacement)
    );
    assert_eq!(scratch.states[1], PageState::Resident);
    assert!(matches!(scratch.states[2], PageState::Inflight { .. }));
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn draw_command_overflow_zeros_caller_buffer() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    for command in &mut scratch.commands {
        command.index_count = 9;
        command.instance_count = 9;
        command.first_index = 9;
        command.base_vertex = 9;
        command.first_instance = 9;
    }
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::DrawCommandOverflow),
        None,
        false,
        None,
    );
    assert_eq!(report.ledger.compaction.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::Capacity));
    assert!(scratch.commands.iter().all(|c| *c == DrawIndexedCommand::EMPTY));
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::DrawCommandOverflow)
    );
    assert_eq!(report.ledger.activation.status, PhaseStatus::Inconclusive);
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn cancelled_inflight_transfer_is_cancelled_not_gpu() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::CancelledInflightTransfer),
        None,
        false,
        None,
    );
    assert_eq!(report.ledger.decode.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.upload.status, PhaseStatus::Cancelled);
    assert_eq!(report.ledger.compaction.status, PhaseStatus::Cancelled);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Cancelled);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Cancelled);
    assert_eq!(scratch.states[1], PageState::Cancelled);
    assert_eq!(scratch.states[2], PageState::Absent);
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::CancelledInflightTransfer)
    );
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn mixed_generation_activation_is_stale_identity() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::MixedGenerationActivation),
        None,
        false,
        None,
    );
    assert_eq!(report.ledger.draw.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::StaleIdentity));
    assert!(!report.device_lost);
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::MixedGenerationActivation)
    );
    let mut mixed = qualified("cand", 2);
    mixed.shader_generation = mixed.mesh_generation.wrapping_add(1);
    mixed.transfer_generation = mixed.mesh_generation.wrapping_add(2);
    assert!(!mixed.generations_agree());
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn rollback_does_not_undo_simulation() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let baseline = qualified("base", 1);
    let report = run(&tree, &mut scratch, None, Some(baseline), true, None);
    assert_eq!(report.stage, FaultStage::RolledBack);
    assert_eq!(report.current, Some(baseline));
    assert_eq!(report.simulation_consumed, 3);
    assert!(ROLLBACK_DOES_NOT_UNDO_SIMULATION);
    assert!(!report.rollback_restores_simulation);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Passed);
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
    assert!(report.migration.identities_rolled_back);
}

#[test]
fn gpu_execution_record_never_passes_for_either_token_case() {
    let absent = gpu_execution_record(None);
    assert!(!absent.claims_passed());
    assert_eq!(absent.status, PhaseStatus::Unsupported);
    assert_eq!(absent.timing_source, TimingSource::NotMeasured);
    assert_eq!(absent.timestamp_ns, None);
    assert_eq!(absent.note, GPU_ABSENT_NOTE);
    let present = gpu_execution_record(Some(LiveGpuDevice::bind_live(1)));
    assert!(!present.claims_passed());
    assert_eq!(present.status, PhaseStatus::Inconclusive);
    assert_eq!(present.timing_source, TimingSource::NotMeasured);
    assert_eq!(present.timestamp_ns, None);
    assert_eq!(present.note, GPU_NO_DISPATCH_NOTE);
}

#[test]
fn cpu_phase_record_rejects_gpu_timestamp_label() {
    let err = PhaseRecord::accounted(
        PhaseKind::Draw,
        1,
        32,
        PhaseStatus::Passed,
        TimingSource::GpuTimestamp,
        Some(1),
    )
    .unwrap_err();
    assert_eq!(err, LifecycleError::DishonestTiming);
    let err = PhaseRecord::accounted(
        PhaseKind::Activation,
        1,
        8,
        PhaseStatus::Passed,
        TimingSource::CompletionClock,
        Some(1),
    )
    .unwrap_err();
    assert_eq!(err, LifecycleError::DishonestTiming);
}

#[test]
fn live_device_token_does_not_pass_gpu_execution() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let device = LiveGpuDevice::bind_live(7);
    assert_eq!(device.generation, 7);
    let report = run(&tree, &mut scratch, None, None, false, Some(device));
    assert_gpu_token(&report, true);
    assert_cpu_ledger(report.ledger);
    assert_migration(&report);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Passed);
}

#[test]
fn live_device_token_does_not_rescue_device_lost() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let baseline = qualified("base", 1);
    let device = LiveGpuDevice::bind_live(9);
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::DeviceLostAtActivation),
        Some(baseline),
        false,
        Some(device),
    );
    assert_gpu_token(&report, true);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::DeviceLost));
    assert_eq!(report.current, Some(baseline));
    assert_eq!(report.simulation_consumed, 3);
    assert_cpu_ledger(report.ledger);
    assert_migration(&report);
}

#[test]
fn absent_gpu_token_is_unsupported_not_passed() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let report = run(&tree, &mut scratch, None, None, false, None);
    assert_gpu_token(&report, false);
    assert_cpu_ledger(report.ledger);
}

#[test]
fn incomplete_asset_refuses_activation_and_keeps_baseline() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let baseline = qualified("base", 1);
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::IncompleteAsset),
        Some(baseline),
        false,
        None,
    );
    assert_eq!(report.ledger.decode.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.draw.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::MissingFallback));
    assert_eq!(report.current, Some(baseline));
    assert_eq!(report.simulation_consumed, 3);
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::IncompleteAsset)
    );
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn invalidated_evidence_is_stale_identity() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let baseline = qualified("base", 1);
    let report = run(
        &tree,
        &mut scratch,
        Some(InjectedFault::InvalidatedEvidence),
        Some(baseline),
        false,
        None,
    );
    assert_eq!(report.ledger.draw.status, PhaseStatus::Passed);
    assert_eq!(report.ledger.activation.status, PhaseStatus::Failed);
    assert_eq!(report.runtime, Some(RuntimeError::StaleIdentity));
    assert_eq!(report.current, Some(baseline));
    assert_eq!(report.simulation_consumed, 3);
    assert_eq!(
        report.stage,
        FaultStage::Faulted(InjectedFault::InvalidatedEvidence)
    );
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn five_phases_stay_separate_with_no_blended_score() {
    let tree = Tree::new();
    let mut scratch = Scratch::new();
    let report = run(&tree, &mut scratch, None, None, false, None);
    let records = report.ledger.records();
    assert_eq!(records.len(), 5);
    assert!(records.iter().all(|record| record.status == PhaseStatus::Passed));
    assert_cpu_ledger(report.ledger);
    assert_gpu_token(&report, false);
    assert_migration(&report);
}

#[test]
fn capacity_plan_is_caller_buffered() {
    let plan = plan_lifecycle_capacity(224, 3, 3, 2, 1, 4, 3, 3).unwrap();
    assert!(plan.total_bytes().unwrap() > 0);
    let cancel = Cancellation::new();
    assert!(ProductionBudget::admit(plan, usize::MAX, 8, &cancel).is_ok());
}
