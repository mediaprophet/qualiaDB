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

use qualia_core_db::modalities::epistemic::{OP_BELIEVES, OP_KNOWS};
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

// ── Probabilistic (belief threshold) ──────────────────────────────────────────────

/// A quin carrying an f32 belief weight in `metadata` (parity excludes metadata,
/// matching collect_active_quins' fold).
fn belief(subject: u64, predicate: u64, object: u64, weight: f32) -> NQuin {
    let mut n = NQuin { subject, predicate, object, context: 0, metadata: weight.to_bits() as u64, parity: 0 };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

#[test]
fn probabilistic_threshold_active() {
    let (s, p, o) = (q_hash("belief:rain"), q_hash("p:holds"), q_hash("val:true"));
    let mut a = SlgArena::new();
    a.write_table(belief(s, p, o, 0.8));
    let frame = VmFrame { subject_reg: s, predicate_reg: p, object_reg: o, context_reg: 0 };

    assert!(gate(&mut a, SlgOpcode::NativeProbabilisticThreshold((0.5f32).to_bits()), frame),
        "belief 0.8 ≥ threshold 0.5 → passes");
    assert!(!gate(&mut a, SlgOpcode::NativeProbabilisticThreshold((0.9f32).to_bits()), frame),
        "belief 0.8 < threshold 0.9 → fails");
}

// ── Description Logic (subsumption / transitive subClassOf) ────────────────────────

#[test]
fn dl_subsumption_active() {
    let (dog, mammal, animal) = (q_hash("cls:Dog"), q_hash("cls:Mammal"), q_hash("cls:Animal"));
    let sub = q_hash("rdfs:subClassOf");
    let mut a = SlgArena::new();
    a.write_table(q(dog, sub, mammal));
    a.write_table(q(mammal, sub, animal));

    let f1 = VmFrame { subject_reg: dog, predicate_reg: 0, object_reg: animal, context_reg: 0 };
    assert!(gate(&mut a, SlgOpcode::NativeDlSubsumption, f1),
        "Dog ⊑ Animal via transitive subClassOf closure");

    let f2 = VmFrame { subject_reg: animal, predicate_reg: 0, object_reg: dog, context_reg: 0 };
    assert!(!gate(&mut a, SlgOpcode::NativeDlSubsumption, f2),
        "Animal is NOT ⊑ Dog");
}

// ── Argumentation (Dung grounded semantics) ───────────────────────────────────────

#[test]
fn argumentation_grounded_active() {
    let asserts = q_hash("arg:asserts");
    let attacks = q_hash("arg:attacks");
    let (a_arg, b_arg, c_arg) = (q_hash("arg:A"), q_hash("arg:B"), q_hash("arg:C"));

    let mut a = SlgArena::new();
    a.write_table(q(a_arg, asserts, 0));
    a.write_table(q(b_arg, asserts, 0));
    a.write_table(q(c_arg, asserts, 0));
    a.write_table(q(a_arg, attacks, b_arg)); // A attacks B
    a.write_table(q(b_arg, attacks, c_arg)); // B attacks C
    // Grounded extension = {A, C}: A is unattacked; B is defeated by A; C's only
    // attacker (B) is defeated, so C is reinstated.

    let fa = VmFrame { subject_reg: a_arg, predicate_reg: 0, object_reg: 0, context_reg: 0 };
    assert!(gate(&mut a, SlgOpcode::NativeArgumentationGrounded, fa),
        "A is justified (unattacked)");

    let fc = VmFrame { subject_reg: c_arg, predicate_reg: 0, object_reg: 0, context_reg: 0 };
    assert!(gate(&mut a, SlgOpcode::NativeArgumentationGrounded, fc),
        "C is justified (its attacker B is defeated by A)");

    let fb = VmFrame { subject_reg: b_arg, predicate_reg: 0, object_reg: 0, context_reg: 0 };
    assert!(!gate(&mut a, SlgOpcode::NativeArgumentationGrounded, fb),
        "B is NOT justified (defeated by A)");
}

// ── Defeasible (q42:unless defeater injection) ────────────────────────────────────

#[test]
fn unless_runs_and_injects_defeater() {
    // NativeUnless is not a gate; it writes a q42:unless defeater for the goal and
    // lets the frame continue. Confirm it executes through the VM without failing.
    let mut a = SlgArena::new();
    let frame = VmFrame {
        subject_reg: q_hash("alice"),
        predicate_reg: (q_hash("discloses") << 8) | 0x12, // packed forbid path
        object_reg: q_hash("data"),
        context_reg: q_hash("nda"),
    };
    assert!(gate(&mut a, SlgOpcode::NativeUnless, frame),
        "NativeUnless executes (injects a defeater) and the frame proceeds");
}

// ── Epistemic (knowledge / belief) ────────────────────────────────────────────────

/// An epistemic claim quin: predicate packs the modal opcode (low byte) and an
/// 8-bit certainty (bits 8–15); subject = agent, context = world.
fn claim(agent: u64, opcode: u8, certainty: u8, object: u64, world: u64) -> NQuin {
    let predicate = (opcode as u64) | ((certainty as u64) << 8);
    let mut n = NQuin { subject: agent, predicate, object, context: world, metadata: 1, parity: 0 };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

#[test]
fn epistemic_eval_active() {
    let agent = q_hash("agent:alice");
    let world = q_hash("world:w0");
    let prop = q_hash("prop:sky-blue");
    let frame = VmFrame { subject_reg: agent, predicate_reg: 0, object_reg: 0, context_reg: world };

    // Knows@200 is Active with certainty 200.
    let mut a = SlgArena::new();
    a.write_table(claim(agent, OP_KNOWS, 200, prop, world));
    assert!(gate(&mut a, SlgOpcode::NativeEpistemicEval(128), frame),
        "Knows@200 meets min-certainty 128 → active");
    assert!(!gate(&mut a, SlgOpcode::NativeEpistemicEval(255), frame),
        "Knows@200 does not meet min-certainty 255 → fails");

    // Believes@50 is Uncertain (certainty < 128, not Knows) → never active.
    let mut b = SlgArena::new();
    b.write_table(claim(agent, OP_BELIEVES, 50, prop, world));
    assert!(!gate(&mut b, SlgOpcode::NativeEpistemicEval(0), frame),
        "a low-certainty belief is Uncertain, not active");
}

// ── Answer-Set Programming (stable-model enumeration) ─────────────────────────────

#[test]
fn asp_enumerates_stable_models_from_arena() {
    // Two rules in the arena → enumerate_stable_models bifurcates into 2^2 worlds,
    // so the bound world differs from the base context. (Before the fix the handler
    // passed an empty rule set and always yielded the single base world.)
    let base_ctx = q_hash("world:base");
    let mut a = SlgArena::new();
    a.write_table(q(q_hash("rule:a"), q_hash("then"), q_hash("x")));
    a.write_table(q(q_hash("rule:b"), q_hash("then"), q_hash("y")));

    let mut frame = VmFrame { subject_reg: 0, predicate_reg: 0, object_reg: 0, context_reg: base_ctx };
    let result = execute_vm_frame(&mut a, &[SlgOpcode::NativeAspStableModels, SlgOpcode::Return], &mut frame);
    let bound = result.expect("ASP must yield at least one stable model");
    assert_ne!(bound.context, base_ctx,
        "ASP must enumerate non-trivial worlds from the arena rules, not an empty rule set");
}
