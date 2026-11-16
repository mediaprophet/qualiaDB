use super::*;
use super::super::ddg_operators::SparseEntry;
use super::super::geometry_workspace::production_budget::{CapacityPlan, ProductionBudget};
use super::super::geometry_workspace::Cancellation;
use super::super::primitives::Point3;
use super::super::production_contracts::{
    CollapseNeighborhood, CollapseProposal, CornerDelta, CornerId, EntityDomain, FaceDelta, ResourceEncoding,
    ResourceKind, TransferAdmission,
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
fn triangle_positions() -> [Point3; 3] {
    [Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)]
}
fn deleted_face() -> ([FaceDelta; 1], [CornerDelta; 3]) {
    (
        [FaceDelta { original_face: 0, before: [0, 1, 2], after: None }],
        [
            CornerDelta { source: CornerId { face: 0, slot: 0 }, target: None },
            CornerDelta { source: CornerId { face: 0, slot: 1 }, target: None },
            CornerDelta { source: CornerId { face: 0, slot: 2 }, target: None },
        ],
    )
}
fn collapse_01(run: ContentId, ids: [u64; 2], target: Point3, t: f64) -> CollapseProposal {
    CollapseProposal {
        run,
        source: run,
        policy: run,
        sequence: 0,
        topology_generation: 0,
        removed_vertex: 0,
        kept_vertex: 1,
        stable_endpoint_ids: ids,
        endpoint_generations: [0, 0],
        target,
        t_from_removed_to_kept: t,
    }
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

#[test]
fn identity_lineage_snapshot_has_identity_rows() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let positions = triangle_positions();
    let domains = [domain(1), domain(1), domain(1)];
    let ids = [1, 2, 3];
    let run = ContentId::sha256(b"lineage-identity");
    let mut rows = [TransferRow::default(); 3];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    assert_eq!(transfer.committed_collapses(), 0);
    for i in 0..3 {
        let support = transfer.original_support(i).unwrap();
        assert_eq!(support.len(), 1);
        assert_eq!(
            support[0],
            SparseEntry { row: i as u32, col: i as u32, value: 1.0 }
        );
        assert_eq!(transfer.stable_id(i), Some(ids[i]));
        assert_eq!(transfer.protected_domain(i), Some(domains[i]));
    }
    let mut bytes = [0_u8; 512];
    let resource = encode_lineage_snapshot(&transfer, run, &mut bytes, &mut work).unwrap();
    assert_eq!(resource.kind, ResourceKind::Evidence);
    assert_eq!(
        resource.encoding,
        ResourceEncoding::ExternalTyped { schema: lineage_snapshot_schema() }
    );
    assert_eq!(resource.element_count, Some(3));
    let n = lineage_snapshot_len(&transfer).unwrap();
    assert_eq!(resource.byte_len, n as u64);
    resource.verify(&bytes[..n]).unwrap();
    let mut live = [LineageLiveRow::default(); 3];
    let mut support = [LineageSupport::default(); 6];
    let snap = decode_lineage_snapshot(
        &resource,
        &bytes[..n],
        transfer.lineage_identities(run),
        &mut live,
        &mut support,
        &mut work,
    )
    .unwrap();
    assert_eq!(snap.committed_collapses, 0);
    assert_eq!(snap.live.len(), 3);
    for (i, row) in snap.live.iter().enumerate() {
        assert_eq!(row.slot, i as u32);
        assert_eq!(row.stable_id, ids[i]);
        assert_eq!(row.domain, domains[i]);
        let entries = snap.row_support(i).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], LineageSupport { col: i as u32, weight: 1.0 });
    }
    let mut json = [0_u8; 2048];
    let jn = encode_lineage_snapshot_json(&transfer, run, &mut json, &mut work).unwrap();
    let text = core::str::from_utf8(&json[..jn]).unwrap();
    let mut pos = 0;
    for key in ["committed_collapses", "live", "policy", "run", "schema", "source", "target"] {
        let found = text[pos..].find(&format!("\"{key}\"")).expect("lexical key");
        pos += found + 1;
    }
    assert!(!text.contains("\"weight_le\":\"0000000000000000\""));
    assert!(text.contains("\"weight_le\":\"000000000000f03f\""));
}

#[test]
fn lawful_collapse_updates_original_support() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let positions = triangle_positions();
    let domains = [domain(1), domain(1), domain(1)];
    let ids = [1, 2, 3];
    let run = ContentId::sha256(b"lineage-collapse");
    let mut rows = [TransferRow::default(); 3];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let mut transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    let mut identity_bytes = [0_u8; 512];
    let identity = encode_lineage_snapshot(&transfer, run, &mut identity_bytes, &mut work).unwrap();
    let proposal = collapse_01(run, [1, 2], Point3::new(0.5, 0.0, 0.0), 0.5);
    let (faces, corners) = deleted_face();
    let neighborhood = CollapseNeighborhood { faces: &faces, corners: &corners };
    let mut scratch = [0_u8; 32];
    let prepared = transfer.prepare(&proposal, neighborhood, &mut scratch).unwrap();
    transfer.revalidate(&proposal, neighborhood, &prepared, &scratch).unwrap();
    transfer.commit(&proposal, prepared, &mut scratch);
    assert_eq!(transfer.committed_collapses(), 1);
    assert!(transfer.original_support(0).unwrap().is_empty());
    let kept = transfer.original_support(1).unwrap();
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].col, 0);
    assert_eq!(kept[1].col, 1);
    assert!((kept[0].value - 0.5).abs() < 1e-15);
    assert!((kept[1].value - 0.5).abs() < 1e-15);
    let untouched = transfer.original_support(2).unwrap();
    assert_eq!(untouched.len(), 1);
    assert_eq!(untouched[0], SparseEntry { row: 2, col: 2, value: 1.0 });
    let target = ContentId::sha256(b"lineage-target");
    let mut bytes = [0_u8; 512];
    let resource = encode_lineage_snapshot(&transfer, target, &mut bytes, &mut work).unwrap();
    assert_ne!(resource.id, identity.id);
    let n = lineage_snapshot_len(&transfer).unwrap();
    let mut live = [LineageLiveRow::default(); 3];
    let mut support = [LineageSupport::default(); 6];
    let snap = decode_lineage_snapshot(
        &resource,
        &bytes[..n],
        transfer.lineage_identities(target),
        &mut live,
        &mut support,
        &mut work,
    )
    .unwrap();
    assert_eq!(snap.committed_collapses, 1);
    assert_eq!(snap.live.len(), 2);
    assert_eq!(snap.live[0].slot, 1);
    assert_eq!(snap.live[0].stable_id, 2);
    let merged = snap.row_support(0).unwrap();
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].col, 0);
    assert_eq!(merged[1].col, 1);
    assert!((merged[0].weight - 0.5).abs() < 1e-15);
    assert_eq!(snap.live[1].slot, 2);
    assert_eq!(snap.row_support(1).unwrap()[0], LineageSupport { col: 2, weight: 1.0 });
}

#[test]
fn lineage_domain_crossing_is_rejected_without_commit() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let positions = triangle_positions();
    let domains = [domain(1), domain(2), domain(1)];
    let ids = [1, 2, 3];
    let run = ContentId::sha256(b"lineage-crossing");
    let mut rows = [TransferRow::default(); 3];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    let proposal = collapse_01(run, [1, 2], Point3::new(0.5, 0.0, 0.0), 0.5);
    let (faces, corners) = deleted_face();
    let mut scratch = [0_u8; 32];
    let err = transfer
        .prepare(&proposal, CollapseNeighborhood { faces: &faces, corners: &corners }, &mut scratch)
        .unwrap_err();
    assert_eq!(err.code, ReasonCode::UnsatisfiedConstraint);
    assert_eq!(
        TransferError::DomainCrossing { row: 0, col: 1 }.contract("protected_domain").code,
        ReasonCode::UnsatisfiedConstraint
    );
    assert_eq!(transfer.committed_collapses(), 0);
    for i in 0..3 {
        let support = transfer.original_support(i).unwrap();
        assert_eq!(support.len(), 1);
        assert_eq!(support[0].col, i as u32);
        assert_eq!(support[0].value, 1.0);
    }
    let mut bytes = [0_u8; 512];
    let resource = encode_lineage_snapshot(&transfer, run, &mut bytes, &mut work).unwrap();
    let n = lineage_snapshot_len(&transfer).unwrap();
    let mut live = [LineageLiveRow::default(); 3];
    let mut support = [LineageSupport::default(); 6];
    let snap = decode_lineage_snapshot(
        &resource,
        &bytes[..n],
        transfer.lineage_identities(run),
        &mut live,
        &mut support,
        &mut work,
    )
    .unwrap();
    assert_eq!(snap.committed_collapses, 0);
    assert_eq!(snap.live.len(), 3);
    assert_eq!(snap.live[0].domain, domain(1));
    assert_eq!(snap.live[1].domain, domain(2));
}

#[test]
fn lineage_snapshot_hash_tracks_weight_and_identity() {
    let cancel = Cancellation::new();
    let mut work = budget(&cancel);
    let positions = triangle_positions();
    let domains = [domain(1), domain(1), domain(1)];
    let ids = [1, 2, 3];
    let alt_ids = [1, 2, 4];
    let run = ContentId::sha256(b"lineage-hash");
    let mut rows = [TransferRow::default(); 3];
    let mut entries = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let transfer = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows, &mut entries, 1e-9, &mut work,
    )
    .unwrap();
    let mut a = [0_u8; 512];
    let mut b = [0_u8; 512];
    let ra = encode_lineage_snapshot(&transfer, run, &mut a, &mut work).unwrap();
    let rb = encode_lineage_snapshot(&transfer, ContentId::sha256(b"other-target"), &mut b, &mut work).unwrap();
    assert_ne!(ra.id, rb.id);
    let na = lineage_snapshot_len(&transfer).unwrap();
    a[na - 1] ^= 1;
    assert_ne!(ContentId::sha256(&a[..na]), ra.id);
    let mut rows2 = [TransferRow::default(); 3];
    let mut entries2 = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let other = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &alt_ids, 1, 2, &mut rows2, &mut entries2, 1e-9, &mut work,
    )
    .unwrap();
    let mut c = [0_u8; 512];
    let rc = encode_lineage_snapshot(&other, run, &mut c, &mut work).unwrap();
    assert_ne!(rc.id, ra.id);

    let mut rows3 = [TransferRow::default(); 3];
    let mut entries3 = [SparseEntry { row: 0, col: 0, value: 0.0 }; 6];
    let mut weighted = CollapseTransfer::identity(
        run, run, run, &positions, &domains, &ids, 1, 2, &mut rows3, &mut entries3, 1e-9, &mut work,
    )
    .unwrap();
    let proposal = collapse_01(run, [1, 2], Point3::new(0.25, 0.0, 0.0), 0.25);
    let (faces, corners) = deleted_face();
    let neighborhood = CollapseNeighborhood { faces: &faces, corners: &corners };
    let mut scratch = [0_u8; 32];
    let prepared = weighted.prepare(&proposal, neighborhood, &mut scratch).unwrap();
    weighted.revalidate(&proposal, neighborhood, &prepared, &scratch).unwrap();
    weighted.commit(&proposal, prepared, &mut scratch);
    let mut d = [0_u8; 512];
    let rd = encode_lineage_snapshot(&weighted, run, &mut d, &mut work).unwrap();
    assert_ne!(rd.id, ra.id);
    let kept = weighted.original_support(1).unwrap();
    assert!((kept[0].value - 0.75).abs() < 1e-15);
    assert!((kept[1].value - 0.25).abs() < 1e-15);
    let mut json_a = [0_u8; 2048];
    let mut json_d = [0_u8; 2048];
    let ja = encode_lineage_snapshot_json(&transfer, run, &mut json_a, &mut work).unwrap();
    let jd = encode_lineage_snapshot_json(&weighted, run, &mut json_d, &mut work).unwrap();
    assert_ne!(ContentId::sha256(&json_a[..ja]), ContentId::sha256(&json_d[..jd]));
    let mut restored = [0_u8; 512];
    let restored_ref = encode_lineage_snapshot(&transfer, run, &mut restored, &mut work).unwrap();
    assert_eq!(restored_ref.id, ra.id);
    let mut live = [LineageLiveRow::default(); 3];
    let mut support = [LineageSupport::default(); 6];
    let err = decode_lineage_snapshot(
        &ra,
        &restored[..na],
        transfer.lineage_identities(ContentId::sha256(b"wrong-target")),
        &mut live,
        &mut support,
        &mut work,
    )
    .unwrap_err();
    assert_eq!(err, TransferError::IdentityMismatch);
}
