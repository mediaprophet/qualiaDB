//! Empirical zero-heap verification of the VM modality gate handlers.
//!
//! The "zero-heap per the mandate" claim must be MEASURED, not asserted (it was
//! previously overclaimed). A counting global allocator records every heap
//! allocation; we measure only the `execute_vm_frame` window (arena/fact setup is
//! excluded) and assert ZERO allocations for each modality handler — including
//! argumentation, whose grounded-extension was rewritten to a bounded stack form.
//!
//! Single test in this file on purpose: a global allocation counter would be
//! polluted by tests running in parallel within the same binary.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use qualia_core_db::webizen::{execute_vm_frame, SlgArena, SlgOpcode, VmFrame};
use qualia_core_db::{q_hash, NQuin};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
}

#[global_allocator]
static GA: Counting = Counting;

fn q(subject: u64, predicate: u64, object: u64) -> NQuin {
    let mut n = NQuin { subject, predicate, object, context: 0, metadata: 1, parity: 0 };
    n.parity = n.subject ^ n.predicate ^ n.object ^ n.context;
    n
}

/// Allocations performed strictly during `[op, Return]` execution.
fn allocs_during(arena: &mut SlgArena, op: SlgOpcode, mut frame: VmFrame) -> usize {
    let before = ALLOCS.load(Ordering::SeqCst);
    let _ = execute_vm_frame(arena, &[op, SlgOpcode::Return], &mut frame);
    ALLOCS.load(Ordering::SeqCst).wrapping_sub(before)
}

fn prop_frame(p: u64) -> VmFrame {
    VmFrame { subject_reg: 0, predicate_reg: p, object_reg: 0, context_reg: 0 }
}

#[test]
fn modality_handlers_are_zero_heap() {
    // Warm up any one-time lazy init outside the measured windows.
    {
        let mut w = SlgArena::new();
        let _ = allocs_during(&mut w, SlgOpcode::NativeAllenInterval(0),
            VmFrame { subject_reg: 1, predicate_reg: 2, object_reg: 5, context_reg: 6 });
    }

    // LTL (trace scan + in-place reverse)
    {
        let mut a = SlgArena::new();
        let safe = q_hash("zh:safe");
        for i in 0..3 { a.write_table(q(i + 1, safe, 0)); }
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeLtlGlobally, prop_frame(safe)), 0,
            "LTL Globally handler must not allocate");
    }

    // Allen interval (pure register arithmetic)
    {
        let mut a = SlgArena::new();
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeAllenInterval(0),
            VmFrame { subject_reg: 1, predicate_reg: 2, object_reg: 5, context_reg: 6 }), 0,
            "Allen interval handler must not allocate");
    }

    // Description-logic subsumption (TBox scan)
    {
        let mut a = SlgArena::new();
        let (dog, animal, sub) = (q_hash("zh:Dog"), q_hash("zh:Animal"), q_hash("rdfs:subClassOf"));
        a.write_table(q(dog, sub, animal));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeDlSubsumption,
            VmFrame { subject_reg: dog, predicate_reg: 0, object_reg: animal, context_reg: 0 }), 0,
            "DL subsumption handler must not allocate");
    }

    // Probabilistic threshold (scan + f32 extract)
    {
        let mut a = SlgArena::new();
        let (s, p, o) = (q_hash("zh:rain"), q_hash("zh:p"), q_hash("zh:true"));
        let mut bel = NQuin { subject: s, predicate: p, object: o, context: 0,
            metadata: (0.9f32).to_bits() as u64, parity: 0 };
        bel.parity = bel.subject ^ bel.predicate ^ bel.object ^ bel.context;
        a.write_table(bel);
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeProbabilisticThreshold((0.5f32).to_bits()),
            VmFrame { subject_reg: s, predicate_reg: p, object_reg: o, context_reg: 0 }), 0,
            "probabilistic threshold handler must not allocate");
    }

    // ASP (rule collect + enumerate, fixed buffers)
    {
        let mut a = SlgArena::new();
        a.write_table(q(q_hash("zh:r1"), q_hash("zh:then"), q_hash("zh:x")));
        a.write_table(q(q_hash("zh:r2"), q_hash("zh:then"), q_hash("zh:y")));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeAspStableModels,
            VmFrame { subject_reg: 0, predicate_reg: 0, object_reg: 0, context_reg: q_hash("zh:base") }), 0,
            "ASP stable-model handler must not allocate");
    }

    // Argumentation (the rewritten bounded zero-heap grounded extension)
    {
        let mut a = SlgArena::new();
        let (asserts, attacks) = (q_hash("arg:asserts"), q_hash("arg:attacks"));
        let (aa, bb, cc) = (q_hash("zh:A"), q_hash("zh:B"), q_hash("zh:C"));
        a.write_table(q(aa, asserts, 0));
        a.write_table(q(bb, asserts, 0));
        a.write_table(q(cc, asserts, 0));
        a.write_table(q(aa, attacks, bb));
        a.write_table(q(bb, attacks, cc));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeArgumentationGrounded,
            VmFrame { subject_reg: aa, predicate_reg: 0, object_reg: 0, context_reg: 0 }), 0,
            "argumentation grounded handler must not allocate (bounded stack form)");
    }

    // Metric temporal (timestamp scan)
    {
        let mut a = SlgArena::new();
        let (br, rm) = (q_hash("zh:breach"), q_hash("zh:remedy"));
        let mut e1 = NQuin { subject: 1, predicate: br, object: 0, context: 0, metadata: 10, parity: 0 };
        e1.parity = e1.subject ^ e1.predicate ^ e1.object ^ e1.context;
        let mut e2 = NQuin { subject: 2, predicate: rm, object: 0, context: 0, metadata: 25, parity: 0 };
        e2.parity = e2.subject ^ e2.predicate ^ e2.object ^ e2.context;
        a.write_table(e1);
        a.write_table(e2);
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeMtlWithin(30),
            VmFrame { subject_reg: 0, predicate_reg: br, object_reg: rm, context_reg: 0 }), 0,
            "metric-temporal handler must not allocate");
    }

    // Contrary-to-duty (fact scan)
    {
        let mut a = SlgArena::new();
        let (party, primary, rep) = (q_hash("zh:p"), q_hash("zh:prim"), q_hash("zh:rep"));
        a.write_table(q(party, q_hash("q42:breached"), primary));
        a.write_table(q(party, q_hash("q42:fulfilled"), rep));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeContraryToDuty,
            VmFrame { subject_reg: party, predicate_reg: primary, object_reg: rep, context_reg: 0 }), 0,
            "contrary-to-duty handler must not allocate");
    }

    // Causal necessity (bounded BFS over fixed buffers)
    {
        let mut a = SlgArena::new();
        let causes = q_hash("causal:causes");
        let (root, c, e) = (q_hash("zh:root"), q_hash("zh:C"), q_hash("zh:E"));
        a.write_table(q(root, causes, c));
        a.write_table(q(c, causes, e));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeCausalNecessary,
            VmFrame { subject_reg: c, predicate_reg: 0, object_reg: e, context_reg: root }), 0,
            "causal-necessity handler must not allocate");
    }

    // Abductive (backward chain scan)
    {
        let mut a = SlgArena::new();
        let ex = q_hash("abduces:explains");
        a.write_table(q(q_hash("zh:hyp"), ex, q_hash("zh:obs")));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeAbduce,
            VmFrame { subject_reg: 0, predicate_reg: 0, object_reg: q_hash("zh:obs"), context_reg: 0 }), 0,
            "abductive handler must not allocate");
    }

    // Closed-world / NAF (absence scan)
    {
        let mut a = SlgArena::new();
        a.write_table(q(q_hash("zh:x"), q_hash("zh:y"), q_hash("zh:z")));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeClosedWorld,
            VmFrame { subject_reg: q_hash("zh:absent"), predicate_reg: q_hash("zh:p"), object_reg: q_hash("zh:o"), context_reg: 0 }), 0,
            "closed-world handler must not allocate");
    }

    // Fuzzy conjunction (degree scan)
    {
        let mut a = SlgArena::new();
        let p = q_hash("zh:fuzzy");
        let mut f1 = NQuin { subject: 1, predicate: p, object: 0, context: 0, metadata: (0.9f32).to_bits() as u64, parity: 0 };
        f1.parity = f1.subject ^ f1.predicate ^ f1.object ^ f1.context;
        let mut f2 = NQuin { subject: 2, predicate: p, object: 0, context: 0, metadata: (0.6f32).to_bits() as u64, parity: 0 };
        f2.parity = f2.subject ^ f2.predicate ^ f2.object ^ f2.context;
        a.write_table(f1);
        a.write_table(f2);
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeFuzzyConjunction((0.5f32).to_bits()),
            VmFrame { subject_reg: 0, predicate_reg: p, object_reg: 0, context_reg: 0 }), 0,
            "fuzzy-conjunction handler must not allocate");
    }

    // CTL EF (bounded BFS over transition graph)
    {
        let mut a = SlgArena::new();
        let (next, holds) = (q_hash("ctl:next"), q_hash("ctl:holds"));
        let (s1, s2, goal) = (q_hash("zh:s1"), q_hash("zh:s2"), q_hash("zh:goal"));
        a.write_table(q(s1, next, s2));
        a.write_table(q(s2, holds, goal));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeCtlExistsFinally,
            VmFrame { subject_reg: s1, predicate_reg: 0, object_reg: goal, context_reg: 0 }), 0,
            "CTL EF handler must not allocate");
    }

    // Modal □ (accessibility scan)
    {
        let mut a = SlgArena::new();
        let (acc, holds) = (q_hash("modal:accesses"), q_hash("modal:holds"));
        let (w0, w1, p) = (q_hash("zh:w0"), q_hash("zh:w1"), q_hash("zh:p"));
        a.write_table(q(w0, acc, w1));
        a.write_table(q(w1, holds, p));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeModalNecessary,
            VmFrame { subject_reg: w0, predicate_reg: 0, object_reg: p, context_reg: 0 }), 0,
            "modal necessity handler must not allocate");
    }

    // ── Remaining original modalities (closing the coverage gap) ──────────────

    // Deontic (verdict scan over a stack [DeonticVerdict; N])
    {
        let mut a = SlgArena::new();
        a.write_table(q(q_hash("zh:alice"), q_hash("zh:forbid"), q_hash("zh:act")));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeDeonticEval, VmFrame::default()), 0,
            "deontic eval handler must not allocate");
    }

    // Epistemic (verdict scan over a stack [EpistemicVerdict; N])
    {
        let mut a = SlgArena::new();
        let agent = q_hash("zh:agent");
        let pred = 0x20u64 | (200u64 << 8); // OP_KNOWS, certainty 200
        let mut k = NQuin { subject: agent, predicate: pred, object: q_hash("zh:prop"), context: q_hash("zh:world"), metadata: 0, parity: 0 };
        k.parity = k.subject ^ k.predicate ^ k.object ^ k.context;
        a.write_table(k);
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeEpistemicEval(128),
            VmFrame { subject_reg: agent, predicate_reg: 0, object_reg: 0, context_reg: q_hash("zh:world") }), 0,
            "epistemic eval handler must not allocate");
    }

    // Linear consumption (find_mutable_quin scan)
    {
        let mut a = SlgArena::new();
        let (s, p, o) = (q_hash("zh:lock"), q_hash("zh:tok"), q_hash("zh:r1"));
        a.write_table(q(s, p, o));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeLinearConsume,
            VmFrame { subject_reg: s, predicate_reg: p, object_reg: o, context_reg: 0 }), 0,
            "linear consume handler must not allocate");
    }

    // Paraconsistent isolation (route over fixed [NQuin; N] buffers)
    {
        let mut a = SlgArena::new();
        a.write_table(q(q_hash("zh:c1"), q_hash("zh:p"), q_hash("zh:v1")));
        a.write_table(q(q_hash("zh:c2"), q_hash("zh:p"), q_hash("zh:v2")));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeParaconsistentIsolate, VmFrame::default()), 0,
            "paraconsistent isolate handler must not allocate");
    }

    // Dialectical synthesis (contradiction → synthesized quin)
    {
        let mut a = SlgArena::new();
        a.write_table(q(q_hash("zh:claim"), q_hash("zh:is"), q_hash("zh:blue")));
        a.write_table(q(q_hash("zh:claim"), q_hash("zh:is"), q_hash("zh:grey")));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeDialecticalSynthesis, VmFrame::default()), 0,
            "dialectical synthesis handler must not allocate");
    }

    // Defeasible (NativeUnless injects a defeater)
    {
        let mut a = SlgArena::new();
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeUnless,
            VmFrame { subject_reg: q_hash("zh:a"), predicate_reg: (q_hash("zh:d") << 8) | 0x12, object_reg: q_hash("zh:o"), context_reg: q_hash("zh:c") }), 0,
            "defeasible (unless) handler must not allocate");
    }

    // CTL AG (second operator — separate handler arm from EF)
    {
        let mut a = SlgArena::new();
        let (next, holds) = (q_hash("ctl:next"), q_hash("ctl:holds"));
        let (s1, s2, inv) = (q_hash("zh:a1"), q_hash("zh:a2"), q_hash("zh:inv"));
        a.write_table(q(s1, next, s2));
        a.write_table(q(s1, holds, inv));
        a.write_table(q(s2, holds, inv));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeCtlAlwaysGlobally,
            VmFrame { subject_reg: s1, predicate_reg: 0, object_reg: inv, context_reg: 0 }), 0,
            "CTL AG handler must not allocate");
    }

    // Modal ◇ (second operator — separate handler arm from □)
    {
        let mut a = SlgArena::new();
        let (acc, holds) = (q_hash("modal:accesses"), q_hash("modal:holds"));
        let (w0, w1, p) = (q_hash("zh:m0"), q_hash("zh:m1"), q_hash("zh:mp"));
        a.write_table(q(w0, acc, w1));
        a.write_table(q(w1, holds, p));
        assert_eq!(allocs_during(&mut a, SlgOpcode::NativeModalPossible,
            VmFrame { subject_reg: w0, predicate_reg: 0, object_reg: p, context_reg: 0 }), 0,
            "modal possibility handler must not allocate");
    }
}
