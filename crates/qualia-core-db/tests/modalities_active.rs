//! Breadth proof: multiple logic modalities actively EVALUATE through the Webizen
//! VM (`execute_vm_frame`), not just deontic. Each `Native*` opcode is a gate —
//! it early-returns `None` when its property fails, otherwise execution continues.
//! We append `SlgOpcode::Return` (which returns `Some(frame_to_quin)`), so:
//!
//!     execute_vm_frame([gate_op, Return]).is_some()  ==  the gate PASSED
//!     execute_vm_frame([gate_op, Return]).is_none()  ==  the gate FIRED (failed)
//!
//! Covers temporal LTL (Globally/Finally/Next/Until/Release — newly wired off the
//! `temporal_ltl::evaluate_ltl_trace` evaluator), Allen interval algebra (newly
//! wired off `spatio_temporal::evaluate_temporal`), linear consumption, and
//! dialectical / paraconsistent guards. (Deontic has its own `deontic_smoke` test.)

use qualia_core_db::webizen::{execute_vm_frame, SlgArena, SlgOpcode, VmFrame};
use qualia_core_db::{q_hash, NQuin};

/// A valid (ECC-parity-correct) data quin.
fn q(subject: u64, predicate: u64, object: u64) -> NQuin {
    let mut n = NQuin { subject, predicate, object, context: 0, metadata: 1, parity: 0 };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

/// Run a single gate opcode followed by `Return`. `true` => gate passed.
fn gate(arena: &mut SlgArena, op: SlgOpcode, frame: VmFrame) -> bool {
    let mut f = frame;
    execute_vm_frame(arena, &[op, SlgOpcode::Return], &mut f).is_some()
}

/// Write an ordered temporal trace: one distinct quin per state, all carrying the
/// given proposition as their (unpacked) predicate.
fn write_trace(arena: &mut SlgArena, preds: &[u64]) {
    for (i, &p) in preds.iter().enumerate() {
        arena.write_table(q((i as u64) + 1, p, 0));
    }
}

fn prop_frame(p: u64) -> VmFrame {
    VmFrame { subject_reg: 0, predicate_reg: p, object_reg: 0, context_reg: 0 }
}

// ── Temporal LTL ────────────────────────────────────────────────────────────────

#[test]
fn ltl_globally_active() {
    let safe = q_hash("ltl:safe");
    let mut a = SlgArena::new();
    write_trace(&mut a, &[safe, safe, safe]);
    assert!(gate(&mut a, SlgOpcode::NativeLtlGlobally, prop_frame(safe)),
        "G(safe) must hold over an all-safe trace");

    let mut b = SlgArena::new();
    write_trace(&mut b, &[safe, q_hash("ltl:unsafe"), safe]);
    assert!(!gate(&mut b, SlgOpcode::NativeLtlGlobally, prop_frame(safe)),
        "G(safe) must fail when a state is unsafe");
}

#[test]
fn ltl_finally_active() {
    let goal = q_hash("ltl:committed");
    let mut a = SlgArena::new();
    write_trace(&mut a, &[q_hash("ltl:a"), q_hash("ltl:b"), goal]);
    assert!(gate(&mut a, SlgOpcode::NativeLtlFinally, prop_frame(goal)),
        "F(committed) must hold when the goal eventually occurs");

    let mut b = SlgArena::new();
    write_trace(&mut b, &[q_hash("ltl:a"), q_hash("ltl:b")]);
    assert!(!gate(&mut b, SlgOpcode::NativeLtlFinally, prop_frame(goal)),
        "F(committed) must fail when the goal never occurs");
}

#[test]
fn ltl_next_active() {
    let p = q_hash("ltl:p");
    let mut a = SlgArena::new();
    write_trace(&mut a, &[q_hash("ltl:x"), p]);
    assert!(gate(&mut a, SlgOpcode::NativeLtlNext, prop_frame(p)),
        "X(p) holds when the second state is p");

    let mut b = SlgArena::new();
    write_trace(&mut b, &[p, q_hash("ltl:x")]);
    assert!(!gate(&mut b, SlgOpcode::NativeLtlNext, prop_frame(p)),
        "X(p) fails when the second state is not p");
}

#[test]
fn ltl_until_active() {
    let ante = q_hash("ltl:request");
    let cons = q_hash("ltl:grant");
    // handler: ante = predicate_reg, consequent = object_reg
    let frame = VmFrame { subject_reg: 0, predicate_reg: ante, object_reg: cons, context_reg: 0 };

    let mut a = SlgArena::new();
    write_trace(&mut a, &[ante, ante, cons]);
    assert!(gate(&mut a, SlgOpcode::NativeLtlUntil, frame),
        "request U grant holds when request persists until grant");

    let mut b = SlgArena::new();
    write_trace(&mut b, &[ante, ante, ante]);
    assert!(!gate(&mut b, SlgOpcode::NativeLtlUntil, frame),
        "request U grant fails when grant never arrives");
}

#[test]
fn ltl_release_active() {
    let trigger = q_hash("ltl:release-trigger");
    let invariant = q_hash("ltl:invariant");
    let frame = VmFrame { subject_reg: 0, predicate_reg: trigger, object_reg: invariant, context_reg: 0 };

    let mut a = SlgArena::new();
    write_trace(&mut a, &[invariant, invariant, invariant]);
    assert!(gate(&mut a, SlgOpcode::NativeLtlRelease, frame),
        "trigger R invariant holds when the invariant always holds");

    let mut b = SlgArena::new();
    write_trace(&mut b, &[invariant, q_hash("ltl:other"), trigger]);
    assert!(!gate(&mut b, SlgOpcode::NativeLtlRelease, frame),
        "trigger R invariant fails when the invariant breaks before the trigger");
}

// ── Allen interval algebra ──────────────────────────────────────────────────────
// Frame registers carry the two intervals: subject=t1_start, predicate=t1_end,
// object=t2_start, context=t2_end. mode: 0=Before,1=Meets,2=Overlaps,3=Starts,
// 4=During,5=Finishes,6=Equals.

fn interval_frame(t1s: u64, t1e: u64, t2s: u64, t2e: u64) -> VmFrame {
    VmFrame { subject_reg: t1s, predicate_reg: t1e, object_reg: t2s, context_reg: t2e }
}

#[test]
fn allen_before_active() {
    let mut a = SlgArena::new();
    assert!(gate(&mut a, SlgOpcode::NativeAllenInterval(0), interval_frame(1, 2, 5, 6)),
        "[1,2] Before [5,6] (2 < 5)");
    assert!(!gate(&mut a, SlgOpcode::NativeAllenInterval(0), interval_frame(1, 8, 5, 6)),
        "[1,8] NOT Before [5,6] (8 !< 5)");
}

#[test]
fn allen_during_and_equals_active() {
    let mut a = SlgArena::new();
    assert!(gate(&mut a, SlgOpcode::NativeAllenInterval(4), interval_frame(3, 4, 1, 10)),
        "[3,4] During [1,10]");
    assert!(gate(&mut a, SlgOpcode::NativeAllenInterval(6), interval_frame(1, 5, 1, 5)),
        "[1,5] Equals [1,5]");
    assert!(!gate(&mut a, SlgOpcode::NativeAllenInterval(6), interval_frame(1, 5, 1, 6)),
        "[1,5] NOT Equals [1,6]");
}

// ── Linear logic (resource consumption) ──────────────────────────────────────────

#[test]
fn linear_consume_active() {
    let (s, p, o) = (q_hash("res:lock"), q_hash("linear:token"), q_hash("res:1"));
    let frame = VmFrame { subject_reg: s, predicate_reg: p, object_reg: o, context_reg: 0 };

    let mut a = SlgArena::new();
    a.write_table(q(s, p, o));
    assert!(gate(&mut a, SlgOpcode::NativeLinearConsume, frame),
        "a present linear resource is consumed (gate passes)");

    let mut b = SlgArena::new();
    assert!(!gate(&mut b, SlgOpcode::NativeLinearConsume, frame),
        "an absent linear resource fails the gate");
}

// ── Dialectical & paraconsistent guards (count-based) ─────────────────────────────

#[test]
fn dialectical_synthesis_active() {
    let frame = VmFrame::default();

    // A genuine contradiction: same subject + predicate, different object.
    let mut a = SlgArena::new();
    a.write_table(q(q_hash("claim:sky"), q_hash("is"), q_hash("blue")));
    a.write_table(q(q_hash("claim:sky"), q_hash("is"), q_hash("grey")));
    assert!(gate(&mut a, SlgOpcode::NativeDialecticalSynthesis, frame),
        "dialectical synthesis resolves a thesis/antithesis contradiction");

    let mut b = SlgArena::new();
    b.write_table(q(1, 2, 3));
    assert!(!gate(&mut b, SlgOpcode::NativeDialecticalSynthesis, frame),
        "dialectical synthesis fails with < 2 facts");
}

#[test]
fn paraconsistent_isolate_empty_fails() {
    let mut a = SlgArena::new();
    assert!(!gate(&mut a, SlgOpcode::NativeParaconsistentIsolate, VmFrame::default()),
        "paraconsistent isolation over an empty arena fails the gate");
}
