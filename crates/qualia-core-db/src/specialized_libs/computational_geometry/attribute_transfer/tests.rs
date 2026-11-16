use super::*;
use super::super::ddg_operators::SparseEntry;
use super::super::geometry_workspace::production_budget::{CapacityPlan, ProductionBudget};
use super::super::geometry_workspace::Cancellation;
use super::super::primitives::Point3;
use super::super::production_contracts::{
    CollapseNeighborhood, CollapseProposal, CornerDelta, CornerId, EntityDomain, FaceDelta, TransferAdmission,
};
use crate::container_10d::correspondence_section::{VertexTetBinding, BARYCENTRIC_UNORM16_SUM};

fn ids() -> TransferIdentities {
    let id = ContentId::sha256(b"qualia.attribute-transfer/test");
    TransferIdentities { source: id, target: id, source_frame: id, target_frame: id }
}
fn budget<'a>(cancel: &'a Cancellation) -> ProductionBudget<'a> {
    ProductionBudget::admit(CapacityPlan::default(), usize::MAX, u64::MAX, cancel).unwrap()
}
fn domain(tag: u64) -> ProtectedDomain {
    ProtectedDomain { material: tag, subpart: tag, ..ProtectedDomain::default() }
}

#[test]
fn constant_and_rigid_motion_interpolation() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let source_domains = [domain(1), domain(1)];
    let target_domains = [domain(1)];
    let entries = [
        SparseEntry { row: 0, col: 0, value: 0.5 },
        SparseEntry { row: 0, col: 1, value: 0.5 },
    ];
    let mut sums = [0.0; 2];
    let map = SparseTransfer::new(
        ids(),
        ids(),
        EntityDomain::Vertex,
        ScalarRule::Intensive,
        &entries,
        &source_domains,
        &target_domains,
        &mut sums,
        1e-12,
        &mut work,
    )
    .unwrap();
    let mut out = [[0.0; 1]];
    let mut scratch = [[0.0; 1]];
    assert_eq!(map.apply(&[[4.0], [4.0]], &mut out, &mut scratch, &mut work).unwrap(), 1);
    assert!((out[0][0] - 4.0).abs() < 1e-15);

    let source = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let translated = [[1.0, 2.0, 3.0], [3.0, 2.0, 3.0]];
    let rotated = [[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let mut pos = [[0.0; 3]];
    assert!(map.check_affine_positions(&source, &[[1.0, 0.0, 0.0]], 1e-12, &mut pos, &mut work).unwrap() < 1e-15);
    assert!(map.check_affine_positions(&translated, &[[2.0, 2.0, 3.0]], 1e-12, &mut pos, &mut work).unwrap() < 1e-15);
    assert!(map.check_affine_positions(&rotated, &[[0.0, 1.0, 0.0]], 1e-12, &mut pos, &mut work).unwrap() < 1e-15);
}

#[test]
fn domain_crossing_maps_to_unsatisfied_constraint() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let source_domains = [domain(1), domain(2)];
    let target_domains = [domain(1)];
    let entries = [
        SparseEntry { row: 0, col: 0, value: 0.5 },
        SparseEntry { row: 0, col: 1, value: 0.5 },
    ];
    let mut sums = [0.0; 2];
    let err = SparseTransfer::new(
        ids(),
        ids(),
        EntityDomain::Vertex,
        ScalarRule::Intensive,
        &entries,
        &source_domains,
        &target_domains,
        &mut sums,
        1e-12,
        &mut work,
    )
    .unwrap_err();
    assert!(matches!(err, TransferError::DomainCrossing { row: 0, col: 1 }));
    let contract = ContractError::from(err);
    assert_eq!(contract.code, ReasonCode::UnsatisfiedConstraint);
    assert_eq!(err.contract("protected_domain").code, ReasonCode::UnsatisfiedConstraint);

    let positions = [Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)];
    let domains = [domain(1), domain(2), domain(1)];
    let ids = [1, 2, 3];
    let run = ContentId::sha256(b"collapse-run");
    let mut rows = [TransferRow::default(); 3];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    let proposal = CollapseProposal {
        run,
        source: run,
        policy: run,
        sequence: 0,
        topology_generation: 0,
        removed_vertex: 0,
        kept_vertex: 1,
        stable_endpoint_ids: [1, 2],
        endpoint_generations: [0, 0],
        target: Point3::new(0.5, 0.0, 0.0),
        t_from_removed_to_kept: 0.5,
    };
    let faces = [FaceDelta { original_face: 0, before: [0, 1, 2], after: None }];
    let corners = [
        CornerDelta { source: CornerId { face: 0, slot: 0 }, target: None },
        CornerDelta { source: CornerId { face: 0, slot: 1 }, target: None },
        CornerDelta { source: CornerId { face: 0, slot: 2 }, target: None },
    ];
    let mut scratch = [0_u8; 32];
    let err = transfer
        .prepare(&proposal, CollapseNeighborhood { faces: &faces, corners: &corners }, &mut scratch)
        .unwrap_err();
    assert_eq!(err.code, ReasonCode::UnsatisfiedConstraint);
}

#[test]
fn affine_candidate_rejection_is_unsatisfied_constraint() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let positions = [Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)];
    let domains = [domain(1), domain(1), domain(1)];
    let ids = [1, 2, 3];
    let run = ContentId::sha256(b"collapse-run");
    let mut rows = [TransferRow::default(); 3];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    let proposal = CollapseProposal {
        run,
        source: run,
        policy: run,
        sequence: 0,
        topology_generation: 0,
        removed_vertex: 0,
        kept_vertex: 1,
        stable_endpoint_ids: [1, 2],
        endpoint_generations: [0, 0],
        target: Point3::new(8.0, 8.0, 8.0),
        t_from_removed_to_kept: 0.5,
    };
    let faces = [FaceDelta { original_face: 0, before: [0, 1, 2], after: None }];
    let corners = [
        CornerDelta { source: CornerId { face: 0, slot: 0 }, target: None },
        CornerDelta { source: CornerId { face: 0, slot: 1 }, target: None },
        CornerDelta { source: CornerId { face: 0, slot: 2 }, target: None },
    ];
    let mut scratch = [0_u8; 32];
    let err = transfer
        .prepare(&proposal, CollapseNeighborhood { faces: &faces, corners: &corners }, &mut scratch)
        .unwrap_err();
    assert_eq!(err.code, ReasonCode::UnsatisfiedConstraint);
    assert_eq!(
        TransferError::Unsupported("collapse_affine_reproduction").contract("collapse_affine_reproduction").code,
        ReasonCode::UnsatisfiedConstraint
    );
}

#[test]
fn femc_weights_sum_to_65535_and_invalid_weights_are_rejected() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let input = [TetBindingInput { tet: 0, weights: [0.25, 0.25, 0.25, 0.25], normal: [0.0, 0.0, 1.0] }];
    let mut output = [VertexTetBinding { tet_element_index: 0, barycentric_unorm16: [0; 4], normal_rotation_oct16: [0; 2] }];
    let mut scratch = output;
    assert_eq!(quantize_tet_bindings(&input, 1, &mut output, &mut scratch, &mut work).unwrap(), 1);
    assert_eq!(
        output[0].barycentric_unorm16.iter().map(|w| *w as u32).sum::<u32>(),
        BARYCENTRIC_UNORM16_SUM
    );

    let sentinel = output[0];
    for bad in [
        [f32::NAN, 0.0, 0.0, 0.0],
        [-0.1, 0.4, 0.4, 0.3],
        [1.0, 1.0, 0.0, 0.0],
        [0.2, 0.2, 0.2, 0.2],
    ] {
        output[0] = sentinel;
        scratch[0] = sentinel;
        let err = quantize_tet_bindings(
            &[TetBindingInput { tet: 0, weights: bad, normal: [0.0, 0.0, 1.0] }],
            1,
            &mut output,
            &mut scratch,
            &mut work,
        )
        .unwrap_err();
        assert!(matches!(err, TransferError::InvalidInput("femc_constructor_input")));
        assert_eq!(output[0], sentinel);
    }
}

#[test]
fn capacity_exhaustion_is_structured() {
    assert!(matches!(
        capacity(10, 4),
        Err(TransferError::Capacity { required: 10, available: 4 })
    ));
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let source_domains = [domain(1)];
    let target_domains = [domain(1)];
    let entries = [SparseEntry { row: 0, col: 0, value: 1.0 }];
    let mut sums = [0.0; 1];
    let map = SparseTransfer::new(
        ids(),
        ids(),
        EntityDomain::Vertex,
        ScalarRule::Intensive,
        &entries,
        &source_domains,
        &target_domains,
        &mut sums,
        1e-12,
        &mut work,
    )
    .unwrap();
    let mut tiny = [[0.0; 1]; 0];
    let mut scratch = [[0.0; 1]; 0];
    assert!(matches!(
        map.apply(&[[1.0]], &mut tiny, &mut scratch, &mut work),
        Err(TransferError::Capacity { required: 1, available: 0 })
    ));
    assert_eq!(ContractError::from(capacity_overflow()).code, ReasonCode::CapacityExhausted);

    let positions = [Point3::new(0.0, 0.0, 0.0)];
    let domains = [domain(1)];
    let ids = [1];
    let run = ContentId::sha256(b"cap");
    let mut rows = [TransferRow::default(); 1];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 0];
    let err = match CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    ) {
        Err(e) => e,
        Ok(_) => panic!("expected capacity error"),
    };
    assert!(matches!(err, TransferError::Capacity { required: 2, available: 0 }));
}

#[test]
fn additional_storage_does_not_double_count_p06_positions() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let positions = [Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
    let domains = [domain(1), domain(1)];
    let ids = [1, 2];
    let run = ContentId::sha256(b"storage");
    let mut rows = [TransferRow::default(); 2];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 4];
    let transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    let expected = 2 * std::mem::size_of::<TransferRow>() + 4 * std::mem::size_of::<SparseEntry>();
    assert_eq!(transfer.additional_storage_bytes().unwrap(), expected);
}
