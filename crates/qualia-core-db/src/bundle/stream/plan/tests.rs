use super::*;
use crate::bundle::stream::descriptor::CandidateIdentity;
use crate::net::qdnf::types::StrongDigest;
const TEND: u32 = 1;
const Q42: u32 = 2;
const EMF: u32 = 3;
const QAC: u32 = 4;
const M64: u32 = 5;
const OPAQUE: u32 = 6;
const CALIB: u32 = 7;
const ENH: u32 = 8;
fn cand(n: u8, entries: u32) -> CandidateIdentity {
    CandidateIdentity {
        descriptor: StrongDigest([n; 48]),
        context: StrongDigest([n.wrapping_add(1); 48]),
        generation: 0,
        entries,
    }
}
fn entry(
    candidate: CandidateIdentity,
    entry_id: u32,
    family: EntryFamily,
    role: RepresentationClass,
    obligation: Obligation,
    bytes: u64,
) -> PlanEntry {
    PlanEntry::new(PlanEntrySpec {
        candidate,
        entry_id,
        family,
        role,
        interval: None,
        timebase: None,
        estimate: ResourceEstimate::new(bytes, 1, 1),
        obligation,
    })
    .unwrap()
}
fn with_interval(
    mut spec: PlanEntrySpec,
    interval: RationalInterval,
    timebase: Option<TimebaseId>,
) -> PlanEntry {
    spec.interval = Some(interval);
    spec.timebase = timebase;
    PlanEntry::new(spec).unwrap()
}
fn spec_of(e: &PlanEntry) -> PlanEntrySpec {
    PlanEntrySpec {
        candidate: e.candidate(),
        entry_id: e.entry_id(),
        family: e.family(),
        role: e.role(),
        interval: e.interval(),
        timebase: e.timebase(),
        estimate: e.estimate(),
        obligation: e.obligation(),
    }
}
fn mixed(candidate: CandidateIdentity) -> [PlanEntry; 6] {
    [
        entry(
            candidate,
            TEND,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            10,
        ),
        entry(
            candidate,
            Q42,
            EntryFamily::Q42,
            RepresentationClass::Base,
            Obligation::Mandatory,
            20,
        ),
        entry(
            candidate,
            EMF,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            30,
        ),
        entry(
            candidate,
            QAC,
            EntryFamily::Qac,
            RepresentationClass::Base,
            Obligation::Mandatory,
            40,
        ),
        entry(
            candidate,
            M64,
            EntryFamily::M64,
            RepresentationClass::Base,
            Obligation::Optional,
            50,
        ),
        entry(
            candidate,
            OPAQUE,
            EntryFamily::Opaque,
            RepresentationClass::Enhancement,
            Obligation::Optional,
            60,
        ),
    ]
}
fn mixed_edges(entries: &[PlanEntry; 6]) -> [DependencyEdge; 3] {
    [
        DependencyEdge::bind(
            &entries[2],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[3],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[2],
            &entries[1],
            Obligation::Optional,
            DependencyReason::Query,
        )
        .unwrap(),
    ]
}
#[test]
fn mixed_inventory_constructs_10d_q42_emf_qac_m64_opaque() {
    let candidate = cand(1, 6);
    let entries = mixed(candidate);
    let edges = mixed_edges(&entries);
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    assert_eq!(inv.len(), 6);
    assert_eq!(inv.candidate(), candidate);
    assert_eq!(inv.entry(TEND).unwrap().family(), EntryFamily::TenD);
    assert_eq!(inv.entry(Q42).unwrap().family(), EntryFamily::Q42);
    assert_eq!(inv.entry(EMF).unwrap().family(), EntryFamily::Emf);
    assert_eq!(inv.entry(QAC).unwrap().family(), EntryFamily::Qac);
    assert_eq!(inv.entry(M64).unwrap().family(), EntryFamily::M64);
    assert_eq!(inv.entry(OPAQUE).unwrap().family(), EntryFamily::Opaque);
    assert!(inv.entry(TEND).unwrap().interval().is_none());
    let present = [TEND, Q42, EMF, QAC, M64, OPAQUE];
    assert_eq!(
        evaluate(&inv, TEND, Some(EntryFamily::TenD), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, Q42, Some(EntryFamily::Q42), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, EMF, Some(EntryFamily::Emf), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, QAC, Some(EntryFamily::Qac), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, M64, Some(EntryFamily::M64), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, OPAQUE, Some(EntryFamily::Opaque), &present),
        Readiness::Opaque
    );
    assert_eq!(inv.edges().len(), 3);
}
#[test]
fn missing_mandatory_calibration_is_incomplete_not_ready() {
    let candidate = cand(2, 3);
    let entries = [
        entry(
            candidate,
            EMF,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            30,
        ),
        entry(
            candidate,
            CALIB,
            EntryFamily::Q42,
            RepresentationClass::Metadata,
            Obligation::Mandatory,
            7,
        ),
        entry(
            candidate,
            TEND,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            10,
        ),
    ];
    let edges = [
        DependencyEdge::bind(
            &entries[0],
            &entries[1],
            Obligation::Mandatory,
            DependencyReason::Calibration,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[0],
            &entries[2],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
    ];
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    let present = [EMF, TEND];
    let r = evaluate(&inv, EMF, Some(EntryFamily::Emf), &present);
    assert_eq!(
        r,
        Readiness::Incomplete {
            missing_mandatory: CALIB
        }
    );
    assert_ne!(r, Readiness::Ready);
    match r {
        Readiness::Incomplete { missing_mandatory } => assert_eq!(missing_mandatory, CALIB),
        Readiness::Ready | Readiness::Opaque | Readiness::Refused => {
            panic!("missing calibration is not silence, empty field, or a complete answer")
        }
    }
}
#[test]
fn missing_optional_enhancement_is_opaque_base_ready() {
    let candidate = cand(3, 3);
    let entries = [
        entry(
            candidate,
            EMF,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            30,
        ),
        entry(
            candidate,
            ENH,
            EntryFamily::M64,
            RepresentationClass::Enhancement,
            Obligation::Optional,
            8,
        ),
        entry(
            candidate,
            TEND,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            10,
        ),
    ];
    let edges = [
        DependencyEdge::bind(
            &entries[0],
            &entries[1],
            Obligation::Optional,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[0],
            &entries[2],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
    ];
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    let present = [EMF, TEND];
    assert_eq!(
        evaluate(&inv, ENH, Some(EntryFamily::M64), &present),
        Readiness::Opaque
    );
    assert_eq!(
        evaluate(&inv, EMF, Some(EntryFamily::Emf), &present),
        Readiness::Ready
    );
}
#[test]
fn next_delays_m64_while_ready_emf_qac_control_continue() {
    let candidate = cand(4, 6);
    let entries = mixed(candidate);
    let edges = mixed_edges(&entries);
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    let present = [TEND, Q42, EMF, QAC, M64];
    let intents = [
        SelectionIntent::scene(EMF, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(QAC, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(M64, EntryFamily::M64, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut plan = Schedule::compile(&inv, &[sel], &present).unwrap();
    plan.push_control(1, RecordUnit::at(0)).unwrap();
    assert!(plan.is_charged(candidate, TEND));
    assert!(!plan.is_charged(candidate, TEND.wrapping_add(100)));
    assert_eq!(plan.charged_bytes(), 10 + 20 + 30 + 40 + 50);
    let first = plan.next().unwrap();
    assert_eq!(first.class(), WorkClass::Control);
    let mut saw_emf = false;
    let mut saw_qac = false;
    let mut saw_tend = false;
    loop {
        let unit = plan.next().expect("live families remain");
        assert_ne!(unit.family(), Some(EntryFamily::M64));
        assert_ne!(unit.family(), Some(EntryFamily::P64));
        match unit.family() {
            Some(EntryFamily::Emf) => saw_emf = true,
            Some(EntryFamily::Qac) => saw_qac = true,
            Some(EntryFamily::TenD) => saw_tend = true,
            Some(EntryFamily::Q42) => {}
            _ => panic!("unexpected live unit {:?}", unit.family()),
        }
        if saw_emf && saw_qac && saw_tend {
            break;
        }
    }
    let deferred = plan.next().unwrap();
    assert_eq!(deferred.family(), Some(EntryFamily::M64));
    assert_eq!(deferred.class(), WorkClass::BaseRepresentation);
}
#[test]
fn cancel_obsolete_region_skips_cancelled_ready_siblings_schedule() {
    let candidate = cand(5, 4);
    let entries = [
        entry(
            candidate,
            EMF,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            30,
        ),
        entry(
            candidate,
            QAC,
            EntryFamily::Qac,
            RepresentationClass::Base,
            Obligation::Mandatory,
            40,
        ),
        entry(
            candidate,
            TEND,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            10,
        ),
        entry(
            candidate,
            M64,
            EntryFamily::M64,
            RepresentationClass::Base,
            Obligation::Optional,
            50,
        ),
    ];
    let edges = [
        DependencyEdge::bind(
            &entries[0],
            &entries[2],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[1],
            &entries[2],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
    ];
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    let present = [EMF, QAC, TEND];
    let region = Region::declared(10, 20).unwrap();
    let intents = [
        SelectionIntent::scene(EMF, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(QAC, EntryFamily::Qac, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut plan = Schedule::compile(&inv, &[sel], &present).unwrap();
    plan.enqueue_unit(1, &entries[0], RecordUnit::at(10), true)
        .unwrap();
    assert_eq!(plan.cancel(EMF, Some(region)), 1);
    assert!(plan.occupancy_retained(1, Some(EMF), 10));
    let mut saw_emf0 = false;
    let mut saw_qac = false;
    while let Some(unit) = plan.next() {
        assert!(!(unit.entry_id() == Some(EMF) && unit.unit().leaf() == 10));
        if unit.entry_id() == Some(EMF) && unit.unit().leaf() == 0 {
            saw_emf0 = true;
        }
        if unit.entry_id() == Some(QAC) {
            saw_qac = true;
        }
        if unit.family() == Some(EntryFamily::M64) {
            panic!("cancelled sibling path must not promote deferred work");
        }
    }
    assert!(saw_emf0);
    assert!(saw_qac);
    plan.acknowledge_complete(1, Some(EMF), 10).unwrap();
    assert!(!plan.occupancy_retained(1, Some(EMF), 10));
}
#[test]
fn mutate_dependency_identity_refuses() {
    let a = cand(6, 2);
    let b = cand(7, 2);
    let local = entry(
        a,
        EMF,
        EntryFamily::Emf,
        RepresentationClass::Base,
        Obligation::Mandatory,
        30,
    );
    let foreign = entry(
        b,
        CALIB,
        EntryFamily::Q42,
        RepresentationClass::Metadata,
        Obligation::Mandatory,
        7,
    );
    assert_eq!(
        DependencyEdge::bind(
            &local,
            &foreign,
            Obligation::Mandatory,
            DependencyReason::Calibration
        ),
        Err(PlanError::Identity)
    );
    let other = PlanEntry::new({
        let mut s = spec_of(&local);
        s.entry_id = CALIB;
        s.candidate = b;
        s.family = EntryFamily::Q42;
        s
    })
    .unwrap();
    match Inventory::compile(a, &[local, other], &[]) {
        Err(e) => assert_eq!(e, PlanError::Identity),
        Ok(_) => panic!("compiled mixed-candidate inventory"),
    }
    let calib = entry(
        a,
        CALIB,
        EntryFamily::Q42,
        RepresentationClass::Metadata,
        Obligation::Mandatory,
        7,
    );
    let local_calib = [local, calib];
    let inv = Inventory::compile(a, &local_calib, &[]).unwrap();
    assert_eq!(
        evaluate(&inv, EMF, Some(EntryFamily::TenD), &[EMF, CALIB]),
        Readiness::Refused
    );
}
#[test]
fn static_10d_without_time_interval_is_valid() {
    let candidate = cand(8, 1);
    let entries = [entry(
        candidate,
        TEND,
        EntryFamily::TenD,
        RepresentationClass::Base,
        Obligation::Mandatory,
        10,
    )];
    assert!(entries[0].interval().is_none());
    assert!(entries[0].timebase().is_none());
    let inv = Inventory::compile(candidate, &entries, &[]).unwrap();
    assert_eq!(
        evaluate(&inv, TEND, Some(EntryFamily::TenD), &[TEND]),
        Readiness::Ready
    );
    let intents = [SelectionIntent::scene(TEND, EntryFamily::TenD, None, None).unwrap()];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut plan = Schedule::compile(&inv, &[sel], &[TEND]).unwrap();
    let unit = plan.next().unwrap();
    assert_eq!(unit.family(), Some(EntryFamily::TenD));
    assert_eq!(unit.entry_id(), Some(TEND));
}
#[test]
fn emf_qac_equal_timestamps_without_shared_timebase_are_not_one_clock() {
    let candidate = cand(9, 2);
    let ticks = RationalInterval::declared(1, 48_000, 2, 48_000).unwrap();
    let emf = with_interval(
        PlanEntrySpec {
            candidate,
            entry_id: EMF,
            family: EntryFamily::Emf,
            role: RepresentationClass::Base,
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(30, 1, 1),
            obligation: Obligation::Mandatory,
        },
        ticks,
        None,
    );
    let qac = with_interval(
        PlanEntrySpec {
            candidate,
            entry_id: QAC,
            family: EntryFamily::Qac,
            role: RepresentationClass::Base,
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(40, 1, 1),
            obligation: Obligation::Mandatory,
        },
        ticks,
        None,
    );
    assert_eq!(emf.interval(), qac.interval());
    let emf_qac = [emf, qac];
    let inv = Inventory::compile(candidate, &emf_qac, &[]).unwrap();
    assert!(!inv.share_clock(EMF, QAC));
    assert_eq!(
        DependencyEdge::bind(&emf, &qac, Obligation::Optional, DependencyReason::Clock),
        Err(PlanError::Refused)
    );
    let tb = TimebaseId::declared(StrongDigest([3; 48])).unwrap();
    let emf_c = with_interval(spec_of(&emf), ticks, Some(tb));
    let qac_c = with_interval(spec_of(&qac), ticks, Some(tb));
    let shared_entries = [emf_c, qac_c];
    let shared_edges = [DependencyEdge::bind(
        &emf_c,
        &qac_c,
        Obligation::Optional,
        DependencyReason::Clock,
    )
    .unwrap()];
    let shared = Inventory::compile(candidate, &shared_entries, &shared_edges).unwrap();
    assert!(shared.share_clock(EMF, QAC));
}
#[test]
fn two_principals_do_not_starve_control() {
    let candidate = cand(10, 1);
    let entries = [entry(
        candidate,
        M64,
        EntryFamily::M64,
        RepresentationClass::Base,
        Obligation::Mandatory,
        50,
    )];
    let inv = Inventory::compile(candidate, &entries, &[]).unwrap();
    let intents_a = [SelectionIntent::scene(M64, EntryFamily::M64, None, None).unwrap()];
    let intents_b = [SelectionIntent::scene(M64, EntryFamily::M64, None, None).unwrap()];
    let req_a = SelectionRequest::scene(&intents_a).unwrap();
    let req_b = SelectionRequest::scene(&intents_b).unwrap();
    let sel_a = PrincipalSelection::new(1, req_a).unwrap();
    let sel_b = PrincipalSelection::new(2, req_b).unwrap();
    let mut plan = Schedule::compile(&inv, &[sel_a, sel_b], &[M64]).unwrap();
    plan.enqueue_unit(1, &entries[0], RecordUnit::at(1), true)
        .unwrap();
    plan.enqueue_unit(1, &entries[0], RecordUnit::at(2), true)
        .unwrap();
    plan.push_control(1, RecordUnit::at(0)).unwrap();
    plan.push_control(2, RecordUnit::at(0)).unwrap();
    let a = plan.next().unwrap();
    let b = plan.next().unwrap();
    assert_eq!(a.class(), WorkClass::Control);
    assert_eq!(b.class(), WorkClass::Control);
    assert_ne!(a.principal(), b.principal());
    let rest: [ScheduledUnit; 3] = [
        plan.next().unwrap(),
        plan.next().unwrap(),
        plan.next().unwrap(),
    ];
    assert!(rest.iter().all(|u| u.class() != WorkClass::Control));
    assert!(rest.iter().all(|u| u.family() == Some(EntryFamily::M64)));
    assert!(plan.next().is_none());
}
#[test]
fn planner_does_not_mark_bytes_verified() {
    let candidate = cand(11, 1);
    let entries = [entry(
        candidate,
        TEND,
        EntryFamily::TenD,
        RepresentationClass::Base,
        Obligation::Mandatory,
        10,
    )];
    let inv = Inventory::compile(candidate, &entries, &[]).unwrap();
    let present = [TEND];
    let r = evaluate(&inv, TEND, Some(EntryFamily::TenD), &present);
    match r {
        Readiness::Ready
        | Readiness::Incomplete { .. }
        | Readiness::Opaque
        | Readiness::Refused => {}
    }
    assert_eq!(r, Readiness::Ready);
    let intents = [SelectionIntent::scene(TEND, EntryFamily::TenD, None, None).unwrap()];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let plan = Schedule::compile(&inv, &[sel], &present).unwrap();
    assert!(plan.is_charged(candidate, TEND));
    assert!(plan.occupancy_retained(1, Some(TEND), 0));
}
#[test]
fn many_same_family_entries_compile_and_select_by_id() {
    let candidate = cand(12, 10);
    let qac_a: u32 = 40;
    let qac_b: u32 = 41;
    let qac_c: u32 = 42;
    let emf_a: u32 = 30;
    let emf_b: u32 = 31;
    let emf_c: u32 = 32;
    let tend_a: u32 = 10;
    let tend_b: u32 = 11;
    let m64_a: u32 = 50;
    let entries = [
        entry(
            candidate,
            tend_a,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            10,
        ),
        entry(
            candidate,
            tend_b,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            11,
        ),
        entry(
            candidate,
            emf_a,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            30,
        ),
        entry(
            candidate,
            emf_b,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            31,
        ),
        entry(
            candidate,
            emf_c,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Optional,
            32,
        ),
        entry(
            candidate,
            qac_a,
            EntryFamily::Qac,
            RepresentationClass::Base,
            Obligation::Mandatory,
            40,
        ),
        entry(
            candidate,
            qac_b,
            EntryFamily::Qac,
            RepresentationClass::Base,
            Obligation::Mandatory,
            41,
        ),
        entry(
            candidate,
            qac_c,
            EntryFamily::Qac,
            RepresentationClass::Enhancement,
            Obligation::Optional,
            42,
        ),
        entry(
            candidate,
            m64_a,
            EntryFamily::M64,
            RepresentationClass::Base,
            Obligation::Optional,
            50,
        ),
        entry(
            candidate,
            CALIB,
            EntryFamily::Q42,
            RepresentationClass::Metadata,
            Obligation::Mandatory,
            7,
        ),
    ];
    let edges = [
        DependencyEdge::bind(
            &entries[2],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[3],
            &entries[1],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[5],
            &entries[9],
            Obligation::Mandatory,
            DependencyReason::Calibration,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[6],
            &entries[9],
            Obligation::Mandatory,
            DependencyReason::Calibration,
        )
        .unwrap(),
    ];
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    assert_eq!(inv.len(), 10);
    assert_eq!(inv.entry(qac_a).unwrap().family(), EntryFamily::Qac);
    assert_eq!(inv.entry(qac_b).unwrap().family(), EntryFamily::Qac);
    assert_eq!(inv.entry(qac_c).unwrap().family(), EntryFamily::Qac);
    assert_eq!(inv.entry(emf_a).unwrap().family(), EntryFamily::Emf);
    assert_eq!(inv.entry(emf_b).unwrap().family(), EntryFamily::Emf);
    let present = [
        tend_a, tend_b, emf_a, emf_b, emf_c, qac_a, qac_b, qac_c, m64_a, CALIB,
    ];
    assert_eq!(
        evaluate(&inv, qac_a, Some(EntryFamily::Qac), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, qac_b, Some(EntryFamily::Qac), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, qac_c, Some(EntryFamily::Qac), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, emf_a, Some(EntryFamily::Emf), &present),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(&inv, qac_a, Some(EntryFamily::Emf), &present),
        Readiness::Refused
    );
    let mut dup = entries;
    dup[6] = entry(
        candidate,
        qac_a,
        EntryFamily::Qac,
        RepresentationClass::Base,
        Obligation::Mandatory,
        99,
    );
    match Inventory::compile(candidate, &dup, &[]) {
        Err(PlanError::DuplicateId) => {}
        Err(_) => panic!("duplicate entry id must be DuplicateId"),
        Ok(_) => panic!("duplicate entry id must refuse"),
    }
}
#[test]
fn schedule_two_qac_and_two_emf_delays_m64() {
    let candidate = cand(13, 6);
    let qac_a: u32 = 40;
    let qac_b: u32 = 41;
    let emf_a: u32 = 30;
    let emf_b: u32 = 31;
    let tend_a: u32 = 10;
    let m64_a: u32 = 50;
    let entries = [
        entry(
            candidate,
            tend_a,
            EntryFamily::TenD,
            RepresentationClass::Base,
            Obligation::Mandatory,
            10,
        ),
        entry(
            candidate,
            emf_a,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            30,
        ),
        entry(
            candidate,
            emf_b,
            EntryFamily::Emf,
            RepresentationClass::Base,
            Obligation::Mandatory,
            31,
        ),
        entry(
            candidate,
            qac_a,
            EntryFamily::Qac,
            RepresentationClass::Base,
            Obligation::Mandatory,
            40,
        ),
        entry(
            candidate,
            qac_b,
            EntryFamily::Qac,
            RepresentationClass::Base,
            Obligation::Mandatory,
            41,
        ),
        entry(
            candidate,
            m64_a,
            EntryFamily::M64,
            RepresentationClass::Base,
            Obligation::Optional,
            50,
        ),
    ];
    let edges = [
        DependencyEdge::bind(
            &entries[1],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[2],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[3],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
        DependencyEdge::bind(
            &entries[4],
            &entries[0],
            Obligation::Mandatory,
            DependencyReason::Geometry,
        )
        .unwrap(),
    ];
    let inv = Inventory::compile(candidate, &entries, &edges).unwrap();
    let present = [tend_a, emf_a, emf_b, qac_a, qac_b, m64_a];
    let intents = [
        SelectionIntent::scene(emf_a, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(emf_b, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(qac_a, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(qac_b, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(m64_a, EntryFamily::M64, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut plan = Schedule::compile(&inv, &[sel], &present).unwrap();
    plan.push_control(1, RecordUnit::at(0)).unwrap();
    let first = plan.next().unwrap();
    assert_eq!(first.class(), WorkClass::Control);
    let mut saw = [false; 4];
    loop {
        let unit = plan.next().expect("live families remain");
        assert_ne!(unit.family(), Some(EntryFamily::M64));
        assert_ne!(unit.family(), Some(EntryFamily::P64));
        match unit.entry_id() {
            Some(id) if id == emf_a => saw[0] = true,
            Some(id) if id == emf_b => saw[1] = true,
            Some(id) if id == qac_a => saw[2] = true,
            Some(id) if id == qac_b => saw[3] = true,
            Some(id) if id == tend_a => {}
            other => panic!("unexpected live unit {other:?}"),
        }
        if saw.iter().all(|v| *v) {
            break;
        }
    }
    let deferred = plan.next().unwrap();
    assert_eq!(deferred.family(), Some(EntryFamily::M64));
    assert_eq!(deferred.entry_id(), Some(m64_a));
}
