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
    let mut n = NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 1,
        parity: 0,
    };
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
    VmFrame {
        subject_reg: 0,
        predicate_reg: p,
        object_reg: 0,
        context_reg: 0,
    }
}

// ── Temporal LTL ────────────────────────────────────────────────────────────────

#[test]
fn ltl_globally_active() {
    let safe = q_hash("ltl:safe");
    let mut a = SlgArena::new();
    write_trace(&mut a, &[safe, safe, safe]);
    assert!(
        gate(&mut a, SlgOpcode::NativeLtlGlobally, prop_frame(safe)),
        "G(safe) must hold over an all-safe trace"
    );

    let mut b = SlgArena::new();
    write_trace(&mut b, &[safe, q_hash("ltl:unsafe"), safe]);
    assert!(
        !gate(&mut b, SlgOpcode::NativeLtlGlobally, prop_frame(safe)),
        "G(safe) must fail when a state is unsafe"
    );
}

#[test]
fn ltl_finally_active() {
    let goal = q_hash("ltl:committed");
    let mut a = SlgArena::new();
    write_trace(&mut a, &[q_hash("ltl:a"), q_hash("ltl:b"), goal]);
    assert!(
        gate(&mut a, SlgOpcode::NativeLtlFinally, prop_frame(goal)),
        "F(committed) must hold when the goal eventually occurs"
    );

    let mut b = SlgArena::new();
    write_trace(&mut b, &[q_hash("ltl:a"), q_hash("ltl:b")]);
    assert!(
        !gate(&mut b, SlgOpcode::NativeLtlFinally, prop_frame(goal)),
        "F(committed) must fail when the goal never occurs"
    );
}

#[test]
fn ltl_next_active() {
    let p = q_hash("ltl:p");
    let mut a = SlgArena::new();
    write_trace(&mut a, &[q_hash("ltl:x"), p]);
    assert!(
        gate(&mut a, SlgOpcode::NativeLtlNext, prop_frame(p)),
        "X(p) holds when the second state is p"
    );

    let mut b = SlgArena::new();
    write_trace(&mut b, &[p, q_hash("ltl:x")]);
    assert!(
        !gate(&mut b, SlgOpcode::NativeLtlNext, prop_frame(p)),
        "X(p) fails when the second state is not p"
    );
}

#[test]
fn ltl_until_active() {
    let ante = q_hash("ltl:request");
    let cons = q_hash("ltl:grant");
    // handler: ante = predicate_reg, consequent = object_reg
    let frame = VmFrame {
        subject_reg: 0,
        predicate_reg: ante,
        object_reg: cons,
        context_reg: 0,
    };

    let mut a = SlgArena::new();
    write_trace(&mut a, &[ante, ante, cons]);
    assert!(
        gate(&mut a, SlgOpcode::NativeLtlUntil, frame),
        "request U grant holds when request persists until grant"
    );

    let mut b = SlgArena::new();
    write_trace(&mut b, &[ante, ante, ante]);
    assert!(
        !gate(&mut b, SlgOpcode::NativeLtlUntil, frame),
        "request U grant fails when grant never arrives"
    );
}

#[test]
fn ltl_release_active() {
    let trigger = q_hash("ltl:release-trigger");
    let invariant = q_hash("ltl:invariant");
    let frame = VmFrame {
        subject_reg: 0,
        predicate_reg: trigger,
        object_reg: invariant,
        context_reg: 0,
    };

    let mut a = SlgArena::new();
    write_trace(&mut a, &[invariant, invariant, invariant]);
    assert!(
        gate(&mut a, SlgOpcode::NativeLtlRelease, frame),
        "trigger R invariant holds when the invariant always holds"
    );

    let mut b = SlgArena::new();
    write_trace(&mut b, &[invariant, q_hash("ltl:other"), trigger]);
    assert!(
        !gate(&mut b, SlgOpcode::NativeLtlRelease, frame),
        "trigger R invariant fails when the invariant breaks before the trigger"
    );
}

// ── Allen interval algebra ──────────────────────────────────────────────────────
// Frame registers carry the two intervals: subject=t1_start, predicate=t1_end,
// object=t2_start, context=t2_end. mode: 0=Before,1=Meets,2=Overlaps,3=Starts,
// 4=During,5=Finishes,6=Equals.

fn interval_frame(t1s: u64, t1e: u64, t2s: u64, t2e: u64) -> VmFrame {
    VmFrame {
        subject_reg: t1s,
        predicate_reg: t1e,
        object_reg: t2s,
        context_reg: t2e,
    }
}

#[test]
fn allen_before_active() {
    let mut a = SlgArena::new();
    assert!(
        gate(
            &mut a,
            SlgOpcode::NativeAllenInterval(0),
            interval_frame(1, 2, 5, 6)
        ),
        "[1,2] Before [5,6] (2 < 5)"
    );
    assert!(
        !gate(
            &mut a,
            SlgOpcode::NativeAllenInterval(0),
            interval_frame(1, 8, 5, 6)
        ),
        "[1,8] NOT Before [5,6] (8 !< 5)"
    );
}

#[test]
fn allen_during_and_equals_active() {
    let mut a = SlgArena::new();
    assert!(
        gate(
            &mut a,
            SlgOpcode::NativeAllenInterval(4),
            interval_frame(3, 4, 1, 10)
        ),
        "[3,4] During [1,10]"
    );
    assert!(
        gate(
            &mut a,
            SlgOpcode::NativeAllenInterval(6),
            interval_frame(1, 5, 1, 5)
        ),
        "[1,5] Equals [1,5]"
    );
    assert!(
        !gate(
            &mut a,
            SlgOpcode::NativeAllenInterval(6),
            interval_frame(1, 5, 1, 6)
        ),
        "[1,5] NOT Equals [1,6]"
    );
}

// ── Linear logic (resource consumption) ──────────────────────────────────────────

#[test]
fn linear_consume_active() {
    let (s, p, o) = (q_hash("res:lock"), q_hash("linear:token"), q_hash("res:1"));
    let frame = VmFrame {
        subject_reg: s,
        predicate_reg: p,
        object_reg: o,
        context_reg: 0,
    };

    let mut a = SlgArena::new();
    a.write_table(q(s, p, o));
    assert!(
        gate(&mut a, SlgOpcode::NativeLinearConsume, frame),
        "a present linear resource is consumed (gate passes)"
    );

    let mut b = SlgArena::new();
    assert!(
        !gate(&mut b, SlgOpcode::NativeLinearConsume, frame),
        "an absent linear resource fails the gate"
    );
}

// ── Dialectical & paraconsistent guards (count-based) ─────────────────────────────

#[test]
fn dialectical_synthesis_active() {
    let frame = VmFrame::default();

    // A genuine contradiction: same subject + predicate, different object.
    let mut a = SlgArena::new();
    a.write_table(q(q_hash("claim:sky"), q_hash("is"), q_hash("blue")));
    a.write_table(q(q_hash("claim:sky"), q_hash("is"), q_hash("grey")));
    assert!(
        gate(&mut a, SlgOpcode::NativeDialecticalSynthesis, frame),
        "dialectical synthesis resolves a thesis/antithesis contradiction"
    );

    let mut b = SlgArena::new();
    b.write_table(q(1, 2, 3));
    assert!(
        !gate(&mut b, SlgOpcode::NativeDialecticalSynthesis, frame),
        "dialectical synthesis fails with < 2 facts"
    );
}

#[test]
fn paraconsistent_isolate_empty_fails() {
    let mut a = SlgArena::new();
    assert!(
        !gate(
            &mut a,
            SlgOpcode::NativeParaconsistentIsolate,
            VmFrame::default()
        ),
        "paraconsistent isolation over an empty arena fails the gate"
    );
}

// ── Probabilistic (belief threshold) ──────────────────────────────────────────────

/// A quin carrying an f32 belief weight in `metadata` (parity excludes metadata,
/// matching collect_active_quins' fold).
fn belief(subject: u64, predicate: u64, object: u64, weight: f32) -> NQuin {
    let mut n = NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: weight.to_bits() as u64,
        parity: 0,
    };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

#[test]
fn probabilistic_threshold_active() {
    let (s, p, o) = (q_hash("belief:rain"), q_hash("p:holds"), q_hash("val:true"));
    let mut a = SlgArena::new();
    a.write_table(belief(s, p, o, 0.8));
    let frame = VmFrame {
        subject_reg: s,
        predicate_reg: p,
        object_reg: o,
        context_reg: 0,
    };

    assert!(
        gate(
            &mut a,
            SlgOpcode::NativeProbabilisticThreshold((0.5f32).to_bits()),
            frame
        ),
        "belief 0.8 ≥ threshold 0.5 → passes"
    );
    assert!(
        !gate(
            &mut a,
            SlgOpcode::NativeProbabilisticThreshold((0.9f32).to_bits()),
            frame
        ),
        "belief 0.8 < threshold 0.9 → fails"
    );
}

// ── Description Logic (subsumption / transitive subClassOf) ────────────────────────

#[test]
fn dl_subsumption_active() {
    let (dog, mammal, animal) = (
        q_hash("cls:Dog"),
        q_hash("cls:Mammal"),
        q_hash("cls:Animal"),
    );
    let sub = q_hash("rdfs:subClassOf");
    let mut a = SlgArena::new();
    a.write_table(q(dog, sub, mammal));
    a.write_table(q(mammal, sub, animal));

    let f1 = VmFrame {
        subject_reg: dog,
        predicate_reg: 0,
        object_reg: animal,
        context_reg: 0,
    };
    assert!(
        gate(&mut a, SlgOpcode::NativeDlSubsumption, f1),
        "Dog ⊑ Animal via transitive subClassOf closure"
    );

    let f2 = VmFrame {
        subject_reg: animal,
        predicate_reg: 0,
        object_reg: dog,
        context_reg: 0,
    };
    assert!(
        !gate(&mut a, SlgOpcode::NativeDlSubsumption, f2),
        "Animal is NOT ⊑ Dog"
    );
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

    let fa = VmFrame {
        subject_reg: a_arg,
        predicate_reg: 0,
        object_reg: 0,
        context_reg: 0,
    };
    assert!(
        gate(&mut a, SlgOpcode::NativeArgumentationGrounded, fa),
        "A is justified (unattacked)"
    );

    let fc = VmFrame {
        subject_reg: c_arg,
        predicate_reg: 0,
        object_reg: 0,
        context_reg: 0,
    };
    assert!(
        gate(&mut a, SlgOpcode::NativeArgumentationGrounded, fc),
        "C is justified (its attacker B is defeated by A)"
    );

    let fb = VmFrame {
        subject_reg: b_arg,
        predicate_reg: 0,
        object_reg: 0,
        context_reg: 0,
    };
    assert!(
        !gate(&mut a, SlgOpcode::NativeArgumentationGrounded, fb),
        "B is NOT justified (defeated by A)"
    );
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
    assert!(
        gate(&mut a, SlgOpcode::NativeUnless, frame),
        "NativeUnless executes (injects a defeater) and the frame proceeds"
    );
}

// ── Epistemic (knowledge / belief) ────────────────────────────────────────────────

/// An epistemic claim quin: predicate packs the modal opcode (low byte) and an
/// 8-bit certainty (bits 8–15); subject = agent, context = world.
fn claim(agent: u64, opcode: u8, certainty: u8, object: u64, world: u64) -> NQuin {
    let predicate = (opcode as u64) | ((certainty as u64) << 8);
    let mut n = NQuin {
        subject: agent,
        predicate,
        object,
        context: world,
        metadata: 1,
        parity: 0,
    };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

#[test]
fn epistemic_eval_active() {
    let agent = q_hash("agent:alice");
    let world = q_hash("world:w0");
    let prop = q_hash("prop:sky-blue");
    let frame = VmFrame {
        subject_reg: agent,
        predicate_reg: 0,
        object_reg: 0,
        context_reg: world,
    };

    // Knows@200 is Active with certainty 200.
    let mut a = SlgArena::new();
    a.write_table(claim(agent, OP_KNOWS, 200, prop, world));
    assert!(
        gate(&mut a, SlgOpcode::NativeEpistemicEval(128), frame),
        "Knows@200 meets min-certainty 128 → active"
    );
    assert!(
        !gate(&mut a, SlgOpcode::NativeEpistemicEval(255), frame),
        "Knows@200 does not meet min-certainty 255 → fails"
    );

    // Believes@50 is Uncertain (certainty < 128, not Knows) → never active.
    let mut b = SlgArena::new();
    b.write_table(claim(agent, OP_BELIEVES, 50, prop, world));
    assert!(
        !gate(&mut b, SlgOpcode::NativeEpistemicEval(0), frame),
        "a low-certainty belief is Uncertain, not active"
    );
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

    let mut frame = VmFrame {
        subject_reg: 0,
        predicate_reg: 0,
        object_reg: 0,
        context_reg: base_ctx,
    };
    let result = execute_vm_frame(
        &mut a,
        &[SlgOpcode::NativeAspStableModels, SlgOpcode::Return],
        &mut frame,
    );
    let bound = result.expect("ASP must yield at least one stable model");
    assert_ne!(
        bound.context, base_ctx,
        "ASP must enumerate non-trivial worlds from the arena rules, not an empty rule set"
    );
}

// ── Metric/timed temporal (MTL "within") ──────────────────────────────────────────

/// A timed event quin: timestamp in `metadata`.
fn timed(subject: u64, predicate: u64, t: u64) -> NQuin {
    let mut n = NQuin {
        subject,
        predicate,
        object: 0,
        context: 0,
        metadata: t,
        parity: 0,
    };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

#[test]
fn mtl_within_active() {
    let breach = q_hash("mtl:breach");
    let remedy = q_hash("mtl:remedy");
    let mut a = SlgArena::new();
    a.write_table(timed(1, breach, 10));
    a.write_table(timed(2, remedy, 25));
    let frame = VmFrame {
        subject_reg: 0,
        predicate_reg: breach,
        object_reg: remedy,
        context_reg: 0,
    };

    assert!(
        gate(&mut a, SlgOpcode::NativeMtlWithin(30), frame),
        "remedy (t=25) is within 30 of breach (t=10)"
    );
    assert!(
        !gate(&mut a, SlgOpcode::NativeMtlWithin(10), frame),
        "remedy (t=25) is NOT within 10 of breach (t=10) — deadline missed"
    );
}

// ── Contrary-to-duty (dyadic deontic / reparation) ────────────────────────────────

#[test]
fn contrary_to_duty_active() {
    let party = q_hash("ctd:acme");
    let primary = q_hash("ctd:protectData");
    let reparation = q_hash("ctd:remedy");
    let frame = VmFrame {
        subject_reg: party,
        predicate_reg: primary,
        object_reg: reparation,
        context_reg: 0,
    };

    // Breach without reparation → the secondary obligation is unmet.
    let mut a = SlgArena::new();
    a.write_table(q(party, q_hash("q42:breached"), primary));
    assert!(
        !gate(&mut a, SlgOpcode::NativeContraryToDuty, frame),
        "a breach without reparation fails the contrary-to-duty obligation"
    );

    // Breach WITH reparation fulfilled → satisfied.
    let mut b = SlgArena::new();
    b.write_table(q(party, q_hash("q42:breached"), primary));
    b.write_table(q(party, q_hash("q42:fulfilled"), reparation));
    assert!(
        gate(&mut b, SlgOpcode::NativeContraryToDuty, frame),
        "breach + fulfilled reparation satisfies the contrary-to-duty obligation"
    );

    // No breach → CTD not triggered (vacuously satisfied).
    let mut c = SlgArena::new();
    assert!(
        gate(&mut c, SlgOpcode::NativeContraryToDuty, frame),
        "no breach → the contrary-to-duty obligation is not triggered"
    );
}

// ── Causal necessity (but-for) ────────────────────────────────────────────────────

#[test]
fn causal_necessity_active() {
    let causes = q_hash("causal:causes");
    let (root, c, d, effect) = (
        q_hash("c:root"),
        q_hash("c:C"),
        q_hash("c:D"),
        q_hash("c:E"),
    );
    let frame = VmFrame {
        subject_reg: c,
        predicate_reg: 0,
        object_reg: effect,
        context_reg: root,
    };

    // Chain root → C → effect: C is a necessary (but-for) cause.
    let mut a = SlgArena::new();
    a.write_table(q(root, causes, c));
    a.write_table(q(c, causes, effect));
    assert!(
        gate(&mut a, SlgOpcode::NativeCausalNecessary, frame),
        "C is necessary: removing it disconnects the effect"
    );

    // Diamond root → C → effect AND root → D → effect: C is NOT necessary.
    let mut b = SlgArena::new();
    b.write_table(q(root, causes, c));
    b.write_table(q(c, causes, effect));
    b.write_table(q(root, causes, d));
    b.write_table(q(d, causes, effect));
    assert!(
        !gate(&mut b, SlgOpcode::NativeCausalNecessary, frame),
        "C is not necessary when an alternative causal path (via D) exists"
    );
}

// ── Abductive (inference to best explanation) ─────────────────────────────────────

#[test]
fn abductive_active() {
    let explains = q_hash("abduces:explains");
    let (disease, fever, temp) = (q_hash("ab:disease"), q_hash("ab:fever"), q_hash("ab:temp"));

    // disease → fever → observed temperature: an explanatory hypothesis exists.
    let mut a = SlgArena::new();
    a.write_table(q(disease, explains, fever));
    a.write_table(q(fever, explains, temp));
    let frame = VmFrame {
        subject_reg: 0,
        predicate_reg: 0,
        object_reg: temp,
        context_reg: 0,
    };
    assert!(
        gate(&mut a, SlgOpcode::NativeAbduce, frame),
        "an observation with a backward explanatory chain is abductively explained"
    );

    // An observation with no explanatory hypothesis.
    let mut b = SlgArena::new();
    b.write_table(q(disease, explains, fever));
    let f2 = VmFrame {
        subject_reg: 0,
        predicate_reg: 0,
        object_reg: q_hash("ab:unrelated"),
        context_reg: 0,
    };
    assert!(
        !gate(&mut b, SlgOpcode::NativeAbduce, f2),
        "an unexplained observation fails the abductive gate"
    );
}

// ── Closed-world / negation-as-failure ────────────────────────────────────────────

#[test]
fn closed_world_active() {
    let (s, p, o) = (q_hash("cw:s"), q_hash("cw:p"), q_hash("cw:o"));
    let frame = VmFrame {
        subject_reg: s,
        predicate_reg: p,
        object_reg: o,
        context_reg: 0,
    };

    // The proposition is absent → its negation holds by default → gate passes.
    let mut a = SlgArena::new();
    a.write_table(q(q_hash("cw:other"), q_hash("cw:x"), q_hash("cw:y")));
    assert!(
        gate(&mut a, SlgOpcode::NativeClosedWorld, frame),
        "an unprovable proposition → the closed-world default (its negation) holds"
    );

    // The proposition is provable → the closed-world default is defeated.
    let mut b = SlgArena::new();
    b.write_table(q(s, p, o));
    assert!(
        !gate(&mut b, SlgOpcode::NativeClosedWorld, frame),
        "a provable proposition defeats the closed-world default"
    );
}

// ── Fuzzy / many-valued conjunction (Gödel t-norm) ────────────────────────────────

fn fz(subject: u64, predicate: u64, degree: f32) -> NQuin {
    let mut n = NQuin {
        subject,
        predicate,
        object: 0,
        context: 0,
        metadata: degree.to_bits() as u64,
        parity: 0,
    };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

#[test]
fn fuzzy_conjunction_active() {
    let pred = q_hash("fz:satisfies");
    let mut a = SlgArena::new();
    a.write_table(fz(1, pred, 0.9));
    a.write_table(fz(2, pred, 0.6));
    a.write_table(fz(3, pred, 0.8));
    let frame = VmFrame {
        subject_reg: 0,
        predicate_reg: pred,
        object_reg: 0,
        context_reg: 0,
    };

    assert!(
        gate(
            &mut a,
            SlgOpcode::NativeFuzzyConjunction((0.5f32).to_bits()),
            frame
        ),
        "fuzzy conjunction min(0.9,0.6,0.8)=0.6 ≥ threshold 0.5"
    );
    assert!(
        !gate(
            &mut a,
            SlgOpcode::NativeFuzzyConjunction((0.7f32).to_bits()),
            frame
        ),
        "fuzzy conjunction 0.6 < threshold 0.7"
    );
}

// ── CTL (branching-time temporal) ─────────────────────────────────────────────────

#[test]
fn ctl_active() {
    let next = q_hash("ctl:next");
    let holds = q_hash("ctl:holds");
    let goal = q_hash("ctl:goal");
    let safe = q_hash("ctl:safe");
    let (s1, s2, s3) = (q_hash("st:1"), q_hash("st:2"), q_hash("st:3"));
    let mut a = SlgArena::new();
    a.write_table(q(s1, next, s2));
    a.write_table(q(s2, next, s3));
    a.write_table(q(s3, holds, goal));
    a.write_table(q(s1, holds, safe));
    a.write_table(q(s2, holds, safe));
    a.write_table(q(s3, holds, safe));

    let ef = VmFrame {
        subject_reg: s1,
        predicate_reg: 0,
        object_reg: goal,
        context_reg: 0,
    };
    assert!(
        gate(&mut a, SlgOpcode::NativeCtlExistsFinally, ef),
        "EF goal: a path reaches state 3 where the goal holds"
    );

    let ag = VmFrame {
        subject_reg: s1,
        predicate_reg: 0,
        object_reg: safe,
        context_reg: 0,
    };
    assert!(
        gate(&mut a, SlgOpcode::NativeCtlAlwaysGlobally, ag),
        "AG safe: every reachable state satisfies the invariant"
    );

    let ag_bad = VmFrame {
        subject_reg: s1,
        predicate_reg: 0,
        object_reg: goal,
        context_reg: 0,
    };
    assert!(
        !gate(&mut a, SlgOpcode::NativeCtlAlwaysGlobally, ag_bad),
        "AG goal fails: the start state does not satisfy the invariant"
    );
}

// ── General modal (Kripke □ / ◇) ──────────────────────────────────────────────────

#[test]
fn modal_active() {
    let accesses = q_hash("modal:accesses");
    let holds = q_hash("modal:holds");
    let p = q_hash("modal:p");
    let (w0, w1, w2) = (q_hash("w:0"), q_hash("w:1"), q_hash("w:2"));
    let mut a = SlgArena::new();
    a.write_table(q(w0, accesses, w1));
    a.write_table(q(w0, accesses, w2));
    a.write_table(q(w1, holds, p));
    let frame = VmFrame {
        subject_reg: w0,
        predicate_reg: 0,
        object_reg: p,
        context_reg: 0,
    };

    assert!(
        gate(&mut a, SlgOpcode::NativeModalPossible, frame),
        "◇p: an accessible world (w1) satisfies p"
    );
    assert!(
        !gate(&mut a, SlgOpcode::NativeModalNecessary, frame),
        "□p fails: an accessible world (w2) does not satisfy p"
    );

    a.write_table(q(w2, holds, p));
    assert!(
        gate(&mut a, SlgOpcode::NativeModalNecessary, frame),
        "□p: all accessible worlds satisfy p"
    );
}

// ── RCC-8 spatial topology (full polygon, zero-heap) ──────────────────────────────

/// Write a region as boundary-point quins (region_id, spatial:boundary, packed_xy;
/// metadata = vertex sequence).
fn write_region(a: &mut SlgArena, id: u64, pts: &[(f64, f64)]) {
    let boundary = q_hash("spatial:boundary");
    for (seq, &(x, y)) in pts.iter().enumerate() {
        let mut n = NQuin {
            subject: id,
            predicate: boundary,
            object: qualia_core_db::modalities::spatio_temporal::pack_point(x, y),
            context: 0,
            metadata: seq as u64,
            parity: 0,
        };
        n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
        a.write_table(n);
    }
}

#[test]
fn rcc8_active() {
    let region_a = q_hash("geo:A");
    let region_b = q_hash("geo:B");
    let mut arena = SlgArena::new();
    // A = [0,10]^2 ; B = [3,7]^2 strictly inside A.
    write_region(
        &mut arena,
        region_a,
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    );
    write_region(
        &mut arena,
        region_b,
        &[(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)],
    );

    // B is a NON-tangential proper part of A (relation 5).
    let frame = VmFrame {
        subject_reg: region_b,
        predicate_reg: 0,
        object_reg: region_a,
        context_reg: 0,
    };
    assert!(
        gate(&mut arena, SlgOpcode::NativeRcc8(5), frame),
        "B is a non-tangential proper part of A (NTPP=5)"
    );
    assert!(
        !gate(&mut arena, SlgOpcode::NativeRcc8(0), frame),
        "B is NOT disconnected from A (DC=0)"
    );

    // A contains B → inverse relation (NTPPi=6).
    let inv = VmFrame {
        subject_reg: region_a,
        predicate_reg: 0,
        object_reg: region_b,
        context_reg: 0,
    };
    assert!(
        gate(&mut arena, SlgOpcode::NativeRcc8(6), inv),
        "A non-tangentially contains B (NTPPi=6)"
    );
}
