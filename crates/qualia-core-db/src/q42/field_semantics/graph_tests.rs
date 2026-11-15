use super::*;
#[test]
fn bounded_metadata_dag_rejects_cycles_depth_edges_and_missing_targets() {
    let mut b = good();
    let mut frame = b[1].clone();
    frame.id = 6;
    frame.fields.insert(1, n32(2, 8, 2));
    b[1].fields.insert(1, n32(2, 8, 6));
    b.push(frame);
    assert_eq!(result(&b, &[row()]), Err(Error::Cycle));
    let mut b = good();
    let mut frame = b[1].clone();
    frame.id = 6;
    b[1].fields.insert(1, n32(2, 8, 6));
    b.push(frame);
    let encoded = bytes(&b, &[row()]);
    assert_eq!(
        result_with(
            &encoded,
            &SharedOnly,
            Limits {
                depth: 1,
                ..Limits::default()
            }
        ),
        Err(Error::Depth)
    );
    assert_eq!(
        result_with(
            &encoded,
            &SharedOnly,
            Limits {
                edges: 0,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
    let mut b = good();
    let mut unknown = block(6, 777, vec![f(99, 8, 1, 999u32.to_le_bytes().to_vec())]);
    unknown.critical = false;
    unknown.fields[0].critical = false;
    b.push(unknown);
    assert_eq!(result(&b, &[row()]), Err(Error::Reference));
    let encoded = bytes(&good(), &[row()]);
    let view =
        StructuralView::open(&encoded, Family::QacV1Candidate, StorageLimits::default()).unwrap();
    let mut empty = [];
    let mut e = [NodeState::default(); 1];
    assert!(matches!(
        admit_shared(
            &view,
            &SharedOnly,
            &mut Scratch {
                metadata: &mut empty,
                entries: &mut e
            },
            Limits::default()
        ),
        Err(Error::Scratch)
    ));
    assert_eq!(
        result_with(
            &encoded,
            &SharedOnly,
            Limits {
                blocks: 0,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
}
fn ancestry(cycle: bool) -> (Vec<B>, [EntryInput<'static>; 2]) {
    let mut p1 = vec![n32(1, 1, 2)];
    p1.push(f(2, 7, 32, blake3::hash(b"b").as_bytes().to_vec()));
    let mut p2 = vec![n32(1, 1, 4)];
    if cycle {
        p2.push(f(2, 7, 32, blake3::hash(b"a").as_bytes().to_vec()));
    }
    let descriptor = |id, prov, source| {
        let mut fields = vec![
            n32(3, 8, prov),
            string(7, "opaque-original"),
            string(8, "1"),
            string(12, "urn:test:opaque"),
        ];
        if let Some(source) = source {
            fields.push(n32(17, 9, source));
        }
        block(id, 3, fields)
    };
    let b = vec![
        block(1, 4, p1),
        descriptor(2, 1, Some(2)),
        block(3, 4, p2),
        descriptor(4, 3, if cycle { Some(1) } else { None }),
    ];
    (
        b,
        [
            EntryInput {
                flags: 0,
                frame_id: 0,
                spatial_key: 0,
                descriptor_id: 2,
                ..row()
            },
            EntryInput {
                flags: 0,
                frame_id: 0,
                spatial_key: 0,
                descriptor_id: 4,
                payload: b"b",
                ..row()
            },
        ],
    )
}
#[test]
fn provenance_commitments_and_separate_ancestry_graph() {
    let (b, r) = ancestry(false);
    assert_eq!(result(&b, &r), Ok(()));
    let (b, r) = ancestry(true);
    assert_eq!(result(&b, &r), Err(Error::Cycle));
    let (mut b, r) = ancestry(false);
    b[0].fields[1].value[31] ^= 1;
    assert_eq!(result(&b, &r), Err(Error::Provenance));
    let (mut b, r) = ancestry(false);
    b[0].fields.remove(1);
    assert_eq!(result(&b, &r), Err(Error::MissingField));
    let (mut b, r) = ancestry(false);
    b[2].fields.push(f(2, 7, 32, vec![0; 32]));
    assert_eq!(result(&b, &r), Err(Error::Provenance));
}
#[test]
fn work_budget_bounds_full_rescans() {
    let mut blocks = Vec::new();
    for id in 1..=60 {
        let mut b = block(id, 777, vec![]);
        b.critical = false;
        blocks.push(b);
    }
    let encoded = bytes(&blocks, &[]);
    assert_eq!(
        result_with(
            &encoded,
            &SharedOnly,
            Limits {
                work: 1000,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(
        result_with(&encoded, &SharedOnly, Limits::default()),
        Ok(())
    );
    assert_eq!(
        result_with(
            &encoded,
            &SharedOnly,
            Limits {
                work: 0,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
}

#[test]
fn ancestry_depth_is_separate_and_source_commitments_are_critical() {
    let (mut b, r) = ancestry(false);
    b[0].fields[1].critical = false;
    assert_eq!(result(&b, &r), Err(Error::RequiredCritical));
    let (mut b, r) = ancestry(false);
    b[2].fields
        .push(f(2, 7, 32, blake3::hash(b"c").as_bytes().to_vec()));
    b[3].fields.push(n32(17, 9, 3));
    b.push(block(5, 4, vec![n32(1, 1, 4)]));
    b.push(block(
        6,
        3,
        vec![
            n32(3, 8, 5),
            string(7, "opaque-original"),
            string(8, "1"),
            string(12, "urn:test:opaque"),
        ],
    ));
    let rows = [
        r[0],
        r[1],
        EntryInput {
            descriptor_id: 6,
            payload: b"c",
            ..r[1]
        },
    ];
    let encoded = bytes(&b, &rows);
    assert_eq!(
        result_with(
            &encoded,
            &SharedOnly,
            Limits {
                depth: 1,
                ..Limits::default()
            }
        ),
        Err(Error::Depth)
    );
    assert_eq!(
        result_with(&encoded, &SharedOnly, Limits::default()),
        Ok(())
    );
}
