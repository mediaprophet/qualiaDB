use super::*;
use super::super::{
    default_mixed_entries, MIXED_ID_EMF_ALIGN, MIXED_ID_EMF_OPAQUE, MIXED_ID_M64, MIXED_ID_P64,
    MIXED_ID_Q42, MIXED_ID_QAC_OPAQUE, MIXED_ID_QAC_PCM,
};
use crate::bundle::stream::plan::{EntryFamily, RecordUnit};
use crate::bundle::stream::policy::{adopt_local_whole_source, CLOSURE_PURPOSE};
use crate::net::peer::replication::qbdl::service::{
    Session, SessionOwner, ROLE_CLIENT, ROLE_PROVIDER,
};
use crate::net::qdnf::authority::AuthorityOwner;
use crate::net::qdnf::bearer::ipc_pair;
use crate::net::qdnf::types::{ScopeEpoch, StrongDigest};
const TRANSCRIPT: StrongDigest = StrongDigest([9; 48]);
fn attach_pair<'a>(
    client_lanes: &'a mut SessionOwner,
    provider_lanes: &'a mut SessionOwner,
) -> (Session<'a>, Session<'a>) {
    let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
    let client_peer = provider_ep.locator();
    let provider_peer = client_ep.locator();
    let client_session = Session::attach(
        client_ep,
        client_peer,
        TRANSCRIPT,
        ROLE_CLIENT,
        client_lanes,
    )
    .unwrap();
    let provider_session = Session::attach(
        provider_ep,
        provider_peer,
        TRANSCRIPT,
        ROLE_PROVIDER,
        provider_lanes,
    )
    .unwrap();
    (client_session, provider_session)
}
fn bind_scene(hmc: &MixedHmc) -> (
    crate::crypto::network::authority::PinAuthority,
    crate::bundle::stream::policy::BoundClosure<'_>,
) {
    let (pins, _, pin, publisher) = match hmc.installed(CLOSURE_PURPOSE) {
        Ok(v) => v,
        Err(_) => panic!("installed"),
    };
    let bound = match hmc.bound(&pins, pin, publisher) {
        Ok(v) => v,
        Err(_) => panic!("bound"),
    };
    (pins, bound)
}
fn outboards(
    hmc: &MixedHmc,
    selected: &[SelectedPlainEntry<'_>],
) -> (Vec<Vec<OutboardNode>>, Vec<StrongDigest>) {
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in selected {
        match mixed_outboard(hmc.source(), *sel) {
            Ok((n, r)) => {
                nodes.push(n);
                roots.push(r);
            }
            Err(_) => panic!("outboard"),
        }
    }
    (nodes, roots)
}
#[test]
fn compile_from_mixed_transfers_two_qac_and_two_emf() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let (pins, bound) = bind_scene(&hmc);
    let selected = match select_mixed(&hmc, &bound) {
        Ok(v) => v,
        Err(_) => panic!("select"),
    };
    let (nodes, roots) = outboards(&hmc, &selected);
    let entries = match bind_mixed_entries(hmc.source(), &selected, &nodes, &roots) {
        Ok(v) => v,
        Err(_) => panic!("bind entries"),
    };
    let mut clients = match bind_mixed_clients(&selected) {
        Ok(v) => v,
        Err(_) => panic!("bind clients"),
    };
    let ids = [
        MIXED_ID_QAC_OPAQUE,
        MIXED_ID_QAC_PCM,
        MIXED_ID_EMF_OPAQUE,
        MIXED_ID_EMF_ALIGN,
    ];
    let intents = match mixed_intents(&hmc, &ids) {
        Ok(v) => v,
        Err(_) => panic!("intents"),
    };
    let sel = match mixed_selection(&intents) {
        Ok(v) => v,
        Err(_) => panic!("selection"),
    };
    let (_plan, present, mut mux) = match compile_from_mixed(&hmc, &bound, &[sel]) {
        Ok(v) => v,
        Err(_) => panic!("compile"),
    };
    assert!(present.contains(&MIXED_ID_QAC_OPAQUE));
    assert!(present.contains(&MIXED_ID_EMF_OPAQUE));
    assert!(present.contains(&MIXED_ID_P64));
    mux.push_control(1, RecordUnit::at(0)).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, hmc.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0u8; 16384];
    let report = match transfer_mixed(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Ok(r) => r,
        Err(_) => panic!("four live families must transfer"),
    };
    assert_eq!(report.control, 1);
    assert_eq!(report.transferred, 4);
    let mut saw_qac = 0u16;
    let mut saw_emf = 0u16;
    for i in 0..report.order_len as usize {
        match report.order[i] {
            None => {}
            Some(EntryFamily::Qac) => saw_qac += 1,
            Some(EntryFamily::Emf) => saw_emf += 1,
            Some(_) => panic!("only QAC/EMF/Control in this walk"),
        }
    }
    assert_eq!(saw_qac, 2);
    assert_eq!(saw_emf, 2);
}
#[test]
fn next_delays_m64_and_p64_while_ready_qac_and_emf_continue() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let (pins, bound) = bind_scene(&hmc);
    let selected = match select_mixed(&hmc, &bound) {
        Ok(v) => v,
        Err(_) => panic!("select"),
    };
    let (nodes, roots) = outboards(&hmc, &selected);
    let entries = match bind_mixed_entries(hmc.source(), &selected, &nodes, &roots) {
        Ok(v) => v,
        Err(_) => panic!("bind entries"),
    };
    let mut clients = match bind_mixed_clients(&selected) {
        Ok(v) => v,
        Err(_) => panic!("bind clients"),
    };
    let ids = [
        MIXED_ID_QAC_OPAQUE,
        MIXED_ID_QAC_PCM,
        MIXED_ID_EMF_OPAQUE,
        MIXED_ID_EMF_ALIGN,
        MIXED_ID_M64,
        MIXED_ID_P64,
    ];
    let intents = match mixed_intents(&hmc, &ids) {
        Ok(v) => v,
        Err(_) => panic!("intents"),
    };
    let sel = match mixed_selection(&intents) {
        Ok(v) => v,
        Err(_) => panic!("selection"),
    };
    let (_plan, present, mut mux) = match compile_from_mixed(&hmc, &bound, &[sel]) {
        Ok(v) => v,
        Err(_) => panic!("compile"),
    };
    assert!(present.contains(&MIXED_ID_M64));
    assert!(present.contains(&MIXED_ID_P64));
    mux.push_control(1, RecordUnit::at(0)).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, hmc.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0u8; 16384];
    let report = match transfer_mixed(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Ok(r) => r,
        Err(_) => panic!("live plus delayed M64/P64 must transfer"),
    };
    assert!(report.delayed_deferred >= 1);
    assert_eq!(report.control, 1);
    assert_eq!(report.transferred, 6);
    let mut saw_deferred = false;
    for i in 0..report.order_len as usize {
        match report.order[i] {
            None | Some(EntryFamily::Qac) | Some(EntryFamily::Emf) => {
                assert!(!saw_deferred, "M64/P64 must wait on live families");
            }
            Some(EntryFamily::M64) | Some(EntryFamily::P64) => {
                assert!(i > 0, "Control/QAC/EMF run before deferred families");
                saw_deferred = true;
            }
            Some(_) => panic!("unexpected family in delay scene"),
        }
    }
    assert!(saw_deferred);
}
#[test]
fn missing_mandatory_calibration_refuses_before_leaf() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let (pins, bound) = bind_scene(&hmc);
    let selected = match select_mixed(&hmc, &bound) {
        Ok(v) => v,
        Err(_) => panic!("select"),
    };
    let (nodes, roots) = outboards(&hmc, &selected);
    let entries = match bind_mixed_entries(hmc.source(), &selected, &nodes, &roots) {
        Ok(v) => v,
        Err(_) => panic!("bind entries"),
    };
    let mut clients = match bind_mixed_clients(&selected) {
        Ok(v) => v,
        Err(_) => panic!("bind clients"),
    };
    let ids = [MIXED_ID_EMF_OPAQUE];
    let intents = match mixed_intents(&hmc, &ids) {
        Ok(v) => v,
        Err(_) => panic!("intents"),
    };
    let sel = match mixed_selection(&intents) {
        Ok(v) => v,
        Err(_) => panic!("selection"),
    };
    let plan = match mixed_plan(&hmc) {
        Ok(v) => v,
        Err(_) => panic!("plan"),
    };
    let mut present = match mixed_present_ids(&hmc, &bound) {
        Ok(v) => v,
        Err(_) => panic!("present"),
    };
    present.retain(|id| *id != MIXED_ID_Q42);
    let mut mux = match compile_mixed_mux(&plan, &[sel], &present) {
        Ok(v) => v,
        Err(_) => panic!("compile"),
    };
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, hmc.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0u8; 16384];
    match transfer_mixed(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Err(MixedError::Unsupported) => {}
        Ok(_) => panic!("incomplete mandatory calibration must refuse"),
        Err(_) => panic!("incomplete maps to Unsupported"),
    }
}
