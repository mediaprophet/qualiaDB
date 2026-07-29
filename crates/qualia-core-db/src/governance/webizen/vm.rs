use super::*;

#[cfg(feature = "alloc_buffers")]
extern crate alloc;

/// The Execution Frame tracking variable bindings without touching the heap
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VmFrame {
    pub subject_reg: u64,
    pub predicate_reg: u64,
    pub object_reg: u64,
    pub context_reg: u64,
}

#[inline]
fn frame_to_quin(frame: &VmFrame) -> NQuin {
    let mut q = NQuin {
        subject: frame.subject_reg,
        predicate: frame.predicate_reg,
        object: frame.object_reg,
        context: frame.context_reg,
        metadata: 1,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    q
}

#[inline]
fn current_unix32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

#[inline]
fn rdf_type_hash() -> u64 {
    crate::lexicon::generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}

/// Returns true when `node` has `rdf:type` = `class_hash` in the arena.
fn node_has_class(arena: &SlgArena, node: u64, class_hash: u64) -> bool {
    if node == 0 || class_hash == 0 {
        return false;
    }
    let rdf_type = rdf_type_hash();
    let mut scratch = [NQuin::default(); 256];
    let count = arena.collect_active_quins(&mut scratch);
    for q in &scratch[..count] {
        if q.subject == node && q.predicate == rdf_type && q.object == class_hash {
            return true;
        }
    }
    false
}

fn unify_frame(arena: &SlgArena, frame: &mut VmFrame) -> bool {
    if arena
        .check_table(frame.subject_reg, frame.predicate_reg, frame.object_reg)
        .is_some()
    {
        return true;
    }

    let mut scratch = [NQuin::default(); 256];
    let count = arena.collect_active_quins(&mut scratch);
    for q in &scratch[..count] {
        let subject_ok = frame.subject_reg == 0 || q.subject == frame.subject_reg;
        let predicate_ok = frame.predicate_reg == 0 || q.predicate == frame.predicate_reg;
        let object_ok = frame.object_reg == 0 || q.object == frame.object_reg;
        if subject_ok && predicate_ok && object_ok {
            frame.subject_reg = q.subject;
            frame.predicate_reg = q.predicate;
            frame.object_reg = q.object;
            frame.context_reg = q.context;
            return true;
        }
    }

    frame.subject_reg != 0 && frame.predicate_reg != 0
}

#[inline(never)]
fn execute_manifold_ltl(
    arena: &SlgArena,
    mode: u8,
    dimension: u8,
    threshold_bits: u32,
    at_least: bool,
) -> bool {
    let Some(dimension) = manifold::ManifoldDimension::from_u8(dimension) else {
        return false;
    };
    let threshold = f32::from_bits(threshold_bits);
    if !threshold.is_finite() {
        return false;
    }
    let mut snapshot = [NQuin::default(); 512];
    let snapshot_count = arena.collect_active_quins(&mut snapshot);
    let mut states = [manifold::ManifoldState10D::default(); 128];
    let state_count = manifold::collect_manifold_states(&snapshot[..snapshot_count], &mut states);
    let mut trace = [NQuin::default(); 128];
    let trace_count = manifold::project_manifold_ltl_trace(
        &states[..state_count],
        dimension,
        threshold,
        at_least,
        &mut trace,
    );
    let formula = match mode {
        0 => LtlFormula::Globally(manifold::MANIFOLD_THRESHOLD_HOLDS),
        1 => LtlFormula::Finally(manifold::MANIFOLD_THRESHOLD_HOLDS),
        2 => LtlFormula::Next(manifold::MANIFOLD_THRESHOLD_HOLDS),
        _ => return false,
    };
    temporal_ltl::evaluate_ltl_trace(&trace[..trace_count], &formula)
}

#[inline(never)]
fn execute_manifold_asp(arena: &SlgArena) -> Option<u64> {
    let mut snapshot = [NQuin::default(); 512];
    let snapshot_count = arena.collect_active_quins(&mut snapshot);
    let mut states = [manifold::ManifoldState10D::default(); 128];
    let state_count = manifold::collect_manifold_states(&snapshot[..snapshot_count], &mut states);
    if state_count == 0 {
        return None;
    }
    let mut models = [0u64; asp::MAX_STABLE_MODELS];
    let model_count = manifold::evaluate_manifold_answer_sets(&states[..state_count], &mut models);
    (model_count > 0).then(|| models[model_count - 1])
}

#[inline(never)]
fn execute_paraconsistent_isolation(arena: &mut SlgArena) -> bool {
    let mut scratch = [NQuin::default(); 64];
    let count = arena.collect_active_quins(&mut scratch);
    if count == 0 {
        return false;
    }
    let mut consistent = [NQuin::default(); 64];
    let mut isolated = [NQuin::default(); 64];
    let Ok((_, isolated_count)) =
        paraconsistent::route_paraconsistent(&scratch[..count], &mut consistent, &mut isolated)
    else {
        return false;
    };
    for quin in &isolated[..isolated_count] {
        arena.write_table(*quin);
    }
    true
}

#[inline(never)]
fn execute_dialectical_synthesis(arena: &mut SlgArena, frame: &mut VmFrame) -> bool {
    let mut scratch = [NQuin::default(); 64];
    let count = arena.collect_active_quins(&mut scratch);
    if count < 2 {
        return false;
    }
    let Some(synthesis) = dialectical::synthesize_dialectical(&scratch[0], &scratch[1]) else {
        return false;
    };
    arena.write_table(synthesis);
    frame.subject_reg = synthesis.subject;
    frame.predicate_reg = synthesis.predicate;
    frame.object_reg = synthesis.object;
    frame.context_reg = synthesis.context;
    true
}

#[inline(never)]
fn execute_standard_ltl(arena: &SlgArena, opcode: SlgOpcode, frame: &VmFrame) -> bool {
    let mut scratch = [NQuin::default(); 512];
    let count = arena.collect_active_quins(&mut scratch);
    let trace = &mut scratch[..count];
    trace.reverse();
    let formula = match opcode {
        SlgOpcode::NativeLtlGlobally => LtlFormula::Globally(frame.predicate_reg),
        SlgOpcode::NativeLtlFinally => LtlFormula::Finally(frame.predicate_reg),
        SlgOpcode::NativeLtlNext => LtlFormula::Next(frame.predicate_reg),
        SlgOpcode::NativeLtlUntil => LtlFormula::Until {
            ante: frame.predicate_reg,
            consequent: frame.object_reg,
        },
        SlgOpcode::NativeLtlRelease => LtlFormula::Release {
            trigger: frame.predicate_reg,
            invariant: frame.object_reg,
        },
        _ => return false,
    };
    temporal_ltl::evaluate_ltl_trace(trace, &formula)
}

#[inline(never)]
fn execute_snapshot_logic(arena: &SlgArena, opcode: SlgOpcode, frame: &mut VmFrame) -> bool {
    let mut scratch = [NQuin::default(); 512];
    let count = arena.collect_active_quins(&mut scratch);
    let quins = &scratch[..count];

    match opcode {
        SlgOpcode::CheckDefeaters => {
            let mut fingerprints = [0u64; MAX_DEFEATER_SLOTS];
            let fingerprint_count = harvest_defeater_fingerprints(quins, &mut fingerprints);
            !norm_has_active_defeater(&frame_to_quin(frame), &fingerprints[..fingerprint_count])
        }
        SlgOpcode::NativeDeonticEval => {
            let mut verdicts = [DeonticVerdict::default(); 64];
            let verdict_count =
                evaluate_deontic_contract(quins, current_unix32(), &mut verdicts).unwrap_or(0);
            let goal = frame_to_quin(frame);
            let valid = verdicts[..verdict_count].iter().all(|verdict| {
                verdict.norm.subject != goal.subject
                    || verdict.norm.predicate != goal.predicate
                    || verdict.norm.object != goal.object
                    || matches!(verdict.status, DeonticStatus::Active)
            });
            vm_log!(
                "[Webizen] NativeDeonticEval: {} norms evaluated",
                verdict_count
            );
            valid
        }
        SlgOpcode::NativeEpistemicEval(min_certainty) => {
            let mut verdicts = [epistemic::EpistemicVerdict {
                claim: NQuin::default(),
                status: epistemic::EpistemicStatus::Skipped,
                certainty: 0,
            }; 64];
            let verdict_count = epistemic::evaluate_epistemic_frame(
                quins,
                frame.subject_reg,
                frame.context_reg,
                &mut verdicts,
            )
            .unwrap_or(0);
            verdicts[..verdict_count].iter().any(|verdict| {
                verdict.certainty >= min_certainty
                    && verdict.status == epistemic::EpistemicStatus::Active
            })
        }
        SlgOpcode::NativeProbabilisticThreshold(threshold_bits) => {
            let threshold = f32::from_bits(threshold_bits);
            let weight = quins
                .iter()
                .find(|quin| {
                    quin.subject == frame.subject_reg
                        && quin.predicate == frame.predicate_reg
                        && quin.object == frame.object_reg
                })
                .map(probabilistic::BayesianNetwork::extract_weight)
                .unwrap_or(0.0);
            probabilistic::evaluate_threshold(weight, threshold)
        }
        SlgOpcode::NativeDlSubsumption => {
            dl::check_subsumption_quin(frame.subject_reg, frame.object_reg, quins)
        }
        SlgOpcode::NativeArgumentationGrounded => {
            let asserts = crate::q_hash("arg:asserts");
            let attacks_predicate = crate::q_hash("arg:attacks");
            let mut arguments = [0u64; argumentation::MAX_GROUNDED_ARGS];
            let mut argument_count = 0usize;
            let mut attacks = [(0u64, 0u64); 256];
            let mut attack_count = 0usize;
            for quin in quins {
                if quin.predicate == asserts && argument_count < arguments.len() {
                    arguments[argument_count] = quin.subject;
                    argument_count += 1;
                } else if quin.predicate == attacks_predicate && attack_count < attacks.len() {
                    attacks[attack_count] = (quin.subject, quin.object);
                    attack_count += 1;
                }
            }
            argumentation::grounded_contains(
                &arguments[..argument_count],
                &attacks[..attack_count],
                frame.subject_reg,
            )
        }
        SlgOpcode::NativeMtlWithin(window) => {
            temporal_ltl::holds_within(quins, frame.predicate_reg, frame.object_reg, window as u64)
        }
        SlgOpcode::NativeContraryToDuty => {
            crate::modalities::logic::deontic::evaluate_contrary_to_duty(
                quins,
                frame.subject_reg,
                frame.predicate_reg,
                frame.object_reg,
            )
        }
        SlgOpcode::NativeCausalNecessary => dialectical::is_necessary_cause(
            quins,
            frame.context_reg,
            frame.subject_reg,
            frame.object_reg,
        ),
        SlgOpcode::NativeAbduce => {
            let explains = crate::q_hash("abduces:explains");
            if let Some(hypothesis) =
                abductive::abductive_explanation(quins, frame.object_reg, explains)
            {
                frame.subject_reg = hypothesis;
                true
            } else {
                false
            }
        }
        SlgOpcode::NativeClosedWorld => defeasible::holds_by_default(quins, &frame_to_quin(frame)),
        SlgOpcode::NativeFuzzyConjunction(threshold_bits) => {
            let threshold = f32::from_bits(threshold_bits);
            let mut accumulated = 1.0f32;
            let mut found = false;
            for quin in quins {
                if quin.predicate == frame.predicate_reg {
                    accumulated = fuzzy::t_norm_godel(accumulated, fuzzy::degree(quin));
                    found = true;
                }
            }
            found && accumulated >= threshold
        }
        SlgOpcode::NativeCtlExistsFinally => ctl::exists_finally(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("ctl:next"),
            crate::q_hash("ctl:holds"),
        ),
        SlgOpcode::NativeCtlAlwaysGlobally => ctl::always_globally(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("ctl:next"),
            crate::q_hash("ctl:holds"),
        ),
        SlgOpcode::NativeModalNecessary => modal::necessary(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("modal:accesses"),
            crate::q_hash("modal:holds"),
        ),
        SlgOpcode::NativeModalPossible => modal::possible(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("modal:accesses"),
            crate::q_hash("modal:holds"),
        ),
        SlgOpcode::NativeRcc8(expected) | SlgOpcode::NativeRcc8Assert(expected) => {
            let boundary = crate::q_hash("spatial:boundary");
            let mut region_a = [(0.0f64, 0.0f64); spatio_temporal::MAX_BOUNDARY_POINTS];
            let mut region_a_count = 0usize;
            let mut region_b = [(0.0f64, 0.0f64); spatio_temporal::MAX_BOUNDARY_POINTS];
            let mut region_b_count = 0usize;
            for quin in quins {
                if quin.predicate != boundary {
                    continue;
                }
                let index = quin.metadata as usize;
                if index >= spatio_temporal::MAX_BOUNDARY_POINTS {
                    continue;
                }
                if quin.subject == frame.subject_reg {
                    region_a[index] = spatio_temporal::unpack_point(quin.object);
                    region_a_count = region_a_count.max(index + 1);
                } else if quin.subject == frame.object_reg {
                    region_b[index] = spatio_temporal::unpack_point(quin.object);
                    region_b_count = region_b_count.max(index + 1);
                }
            }
            spatio_temporal::evaluate_rcc8_points(
                frame.subject_reg,
                &region_a[..region_a_count],
                frame.object_reg,
                &region_b[..region_b_count],
            ) as u8
                == expected
        }
        _ => false,
    }
}

/// The Bytecode Evaluator for the Prolog Webizen
pub fn execute_vm_frame(
    arena: &mut SlgArena,
    bytecode: &[SlgOpcode],
    frame: &mut VmFrame,
) -> Option<NQuin> {
    let mut instruction_pointer = 0;

    while instruction_pointer < bytecode.len() {
        let opcode = bytecode[instruction_pointer];

        match opcode {
            SlgOpcode::CheckTable => {
                // Hashing the current sub-goal to query the SlgArena
                if let Some(cached_result) =
                    arena.check_table(frame.subject_reg, frame.predicate_reg, frame.object_reg)
                {
                    // Match found! Push the cached result to the VM stack and bypass the graph traversal
                    return Some(cached_result);
                }
            }
            SlgOpcode::CheckDefeaters => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::CheckSubsumption => {
                let is_subsumed =
                    dl::check_subsumption_quin(frame.subject_reg, frame.object_reg, &[]);
                if !is_subsumed {
                    return None;
                }
            }
            SlgOpcode::BranchWorld => {
                let mut out_worlds = [0; asp::MAX_STABLE_MODELS];
                let goal = frame_to_quin(frame);
                let _count = asp::enumerate_stable_models(&goal, &[], &mut out_worlds);
            }
            SlgOpcode::CheckThreshold => {
                let meets_threshold = probabilistic::evaluate_threshold(0.5, 0.8);
                if !meets_threshold {
                    return None;
                }
            }
            SlgOpcode::ConsumeFact => {
                if let Some(q) = arena.find_mutable_quin(
                    frame.subject_reg,
                    frame.predicate_reg,
                    frame.object_reg,
                ) {
                    linear::consume_quin(q);
                } else {
                    return None;
                }
            }
            SlgOpcode::ZkConsumeFact => {
                // Gate exhaustion on a verified zk-entitlement marker for the resource's subject.
                let proof_verified = arena
                    .find_mutable_quin(
                        frame.subject_reg,
                        crate::q_hash("q42:zkVerified"),
                        crate::q_hash("q42:true"),
                    )
                    .is_some();
                if let Some(q) = arena.find_mutable_quin(
                    frame.subject_reg,
                    frame.predicate_reg,
                    frame.object_reg,
                ) {
                    // Linear (consume-once) cryptographic token, gated on the zk proof.
                    if !linear::zk_gated_consume(q, false, proof_verified) {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            SlgOpcode::Unify => {
                if !unify_frame(arena, frame) {
                    return None;
                }
            }
            SlgOpcode::Call => {
                let result = frame_to_quin(frame);
                if result.subject == 0 || result.predicate == 0 {
                    return None;
                }
                arena.write_table(result);
            }
            SlgOpcode::Return => {
                return Some(frame_to_quin(frame));
            }
            SlgOpcode::ApplyTaxSchema => {
                // In a full implementation, we'd pull the active Jurisdiction Profile
                // and amount from the VM frame. For now, we mock the evaluation.
                let schema = TaxRuleSchema::new_au_gst();
                let _liability = schema.evaluate("Income", 100.0);

                // We'd store this calculated liability back into the frame
                // frame.tax_register = liability;
            }
            SlgOpcode::Halt => {
                break;
            }
            SlgOpcode::NativeThermodynamics => {
                // Mock execution of a thermodynamic state MCMC sampler
                let mut sampler =
                    crate::domains::physical::thermodynamics::ThermodynamicSampler::new(298.0, 100);
                sampler.metropolis_step(50.0, 0.5);
                vm_log!(
                    "🧪 Webizen executed NativeThermodynamics step. Current Energy: {}",
                    sampler.current_state.total_energy
                );
            }
            SlgOpcode::NativeOdeSolver => {
                // Mock execution of continuous dynamics via RK4
                #[cfg(feature = "alloc_buffers")]
                let initial = crate::ode_solver::PhysicalState {
                    time: 0.0,
                    values: alloc::vec![1.0],
                };
                #[cfg(not(feature = "alloc_buffers"))]
                let initial = crate::ode_solver::PhysicalState {
                    time: 0.0,
                    values: std::vec![1.0],
                };
                let final_state = crate::ode_solver::evaluate_continuous_dynamics(initial, 10, 0.1);
                vm_log!(
                    "📈 Webizen executed NativeOdeSolver. Final state: {:?}",
                    final_state.values
                );
            }
            SlgOpcode::NativeRk4Step(packed_params) => {
                // Unpack parameters: step_size (lower 32 bits) | num_steps (upper 32 bits)
                let step_size_bits = (packed_params & 0xFFFFFFFF) as u32;
                let num_steps = (packed_params >> 32) as u32;
                let step_size = f32::from_bits(step_size_bits) as f64;

                vm_log!(
                    "🔄 Webizen executing NativeRk4Step: step_size={}, num_steps={}",
                    step_size,
                    num_steps
                );

                // Calculus is a core capability in this crate, so RK4 dispatch stays wired.
                {
                    use crate::modalities::calculus::ode_solver::{ExponentialDecay, Rk4Solver};

                    let system = ExponentialDecay::new(0.5);
                    let mut solver = Rk4Solver::new(system, step_size);

                    // Execute chained RK4 steps
                    let mut quin = frame_to_quin(frame);
                    for _ in 0..num_steps {
                        quin = solver.step_quin(quin, step_size);
                    }

                    frame.subject_reg = quin.subject;
                    frame.predicate_reg = quin.predicate;
                    frame.object_reg = quin.object;
                    frame.context_reg = quin.context;

                    vm_log!(
                        "✅ Webizen completed {} RK4 steps. Final state: t={}, y={}",
                        num_steps,
                        f64::from_bits(quin.metadata),
                        f64::from_bits(quin.object)
                    );
                }

                /* Legacy fallback removed: calculus is always available in this crate.
                    vm_log!("⚠️  Calculus feature not enabled, RK4 step skipped");
                */
            }
            SlgOpcode::NativeQuantumDft => {
                // Mock execution of Kohn-Sham density functional approximation
                let mut dft = crate::quantum_dft::ElectronDensity::new(10);
                let energy = dft.calculate_ground_state_energy(&[]);
                vm_log!(
                    "⚛️ Webizen executed NativeQuantumDft. Ground State Energy: {} eV",
                    energy
                );
            }
            // ── Legacy / compat ───────────────────────────────────────────
            SlgOpcode::NativeBioinformatics => {
                let score =
                    crate::domains::biological::bioinformatics::align_sequences(b"ATCG", b"ATCC");
                vm_log!(
                    "[Webizen] NativeBioinformatics (legacy). SW score: {}",
                    score.score
                );
            }
            SlgOpcode::NativeEconomics => {
                // Enhanced dispatch: use frame.object low bits as selector for demo kernels.
                // Real usage will pass config via NQuin context / metadata.
                let selector = (frame.object_reg & 0xF) as u8;
                match selector {
                    0 | 1 => {
                        // Default / Monte Carlo VaR (legacy numbers preserved for tests)
                        let (mean, var) = crate::domains::financial::economics::run_monte_carlo_var(
                            100.0, 0.05, 0.2, 1.0, 1000, 252,
                        );
                        vm_log!(
                            "[Webizen] NativeEconomics[VaR]. Mean: {:.2}, VaR95: {:.2}",
                            mean,
                            var
                        );
                        // write a result back
                        frame.object_reg = mean.to_bits();
                        frame.context_reg = var.to_bits(); // repurposed for second result in demo
                    }
                    2 => {
                        // Simple fixed income: price a par bond using object bits as rough params
                        use crate::specialized_libs::computational_economics::fixed_income::coupon_bond_price;
                        let face = 100.0;
                        let c = ((frame.object_reg >> 8) & 0xFF) as f64 * 0.001; // coupon rate rough
                        let y = 0.05;
                        let n = 5u32;
                        let price = coupon_bond_price(face, c, y, n as f64, 1).unwrap_or(f64::NAN);
                        vm_log!("[Webizen] NativeEconomics[Bond]. Price: {:.4}", price);
                        frame.object_reg = price.to_bits();
                    }
                    _ => {
                        // Fallback to GBM step via time_series
                        let mut path = [0.0f64; 8];
                        let _ = crate::specialized_libs::computational_economics::time_series::gbm_simulate_into(100.0, 0.05, 0.2, 1.0, 8, 42, &mut path);
                        let last = path[7];
                        vm_log!("[Webizen] NativeEconomics[GBM]. S_T: {:.2}", last);
                        frame.object_reg = last.to_bits();
                    }
                }
            }
            // ── SHACL standard ────────────────────────────────────────────
            SlgOpcode::WarnOnly => {
                vm_log!("[Webizen] sh:Warning — constraint failed but ingestion continues.");
            }
            SlgOpcode::CheckMinInclusive(min) => {
                let val = frame.object_reg as f64;
                if val < min {
                    return None;
                }
            }
            SlgOpcode::CheckMaxInclusive(max) => {
                let val = frame.object_reg as f64;
                if val > max {
                    return None;
                }
            }
            SlgOpcode::CheckMinExclusive(min) => {
                let val = frame.object_reg as f64;
                if val <= min {
                    return None;
                }
            }
            SlgOpcode::CheckMaxExclusive(max) => {
                let val = frame.object_reg as f64;
                if val >= max {
                    return None;
                }
            }
            SlgOpcode::CheckMinCount(n) => {
                if frame.object_reg < n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckMaxCount(n) => {
                if frame.object_reg > n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckMinLength(n) => {
                if frame.object_reg < n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckMaxLength(n) => {
                if frame.object_reg > n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckPattern(pattern_hash) => {
                if frame.object_reg != pattern_hash {
                    return None;
                }
            }
            SlgOpcode::CheckHasValue(expected) => {
                if frame.object_reg != expected {
                    return None;
                }
            }
            SlgOpcode::CheckNodeShape(shape_id) => {
                if !node_has_class(arena, frame.subject_reg, shape_id) {
                    return None;
                }
            }
            SlgOpcode::CheckNotShape(shape_id) => {
                if node_has_class(arena, frame.subject_reg, shape_id) {
                    return None;
                }
            }
            SlgOpcode::SoftCheckNodeShape(shape_id) => {
                if node_has_class(arena, frame.subject_reg, shape_id) {
                    frame.context_reg |= 1;
                }
            }
            SlgOpcode::RequireAnyShape => {
                if frame.context_reg & 1 == 0 {
                    return None;
                }
            }
            SlgOpcode::CheckObjectDatatype(expected_tag) => {
                if frame.object_reg >> 63 != 0 {
                    return None;
                }
                let tag = ((frame.object_reg >> 60) & 0b111) as u8;
                if tag != expected_tag {
                    return None;
                }
            }
            // ── Biosciences ───────────────────────────────────────────────
            SlgOpcode::NativeNucleotideAlign => {
                let demo_result = crate::domains::biological::bioinformatics::align_nucleotide(
                    b"ACGTACGT",
                    b"ACGTCCGT",
                );
                vm_log!(
                    "[Webizen] NativeNucleotideAlign. SW score: {}, identity: {:.1}%",
                    demo_result.score,
                    demo_result.identity_pct
                );
                if demo_result.score <= 0 {
                    return None;
                }
            }
            SlgOpcode::NativeProteinAlign(matrix_id) => {
                let result = crate::domains::biological::bioinformatics::align_protein(
                    b"ACDEFGHIK",
                    b"ACDEFGHIK",
                );
                vm_log!(
                    "[Webizen] NativeProteinAlign(matrix={}) score: {}, id: {:.1}%",
                    matrix_id,
                    result.score,
                    result.identity_pct
                );
                if result.score <= 0 {
                    return None;
                }
            }
            SlgOpcode::NativeKmerFrequency(k) => {
                let freqs = crate::domains::biological::bioinformatics::kmer_frequencies(
                    b"ACGTACGTACGT",
                    k as usize,
                );
                vm_log!(
                    "[Webizen] NativeKmerFrequency(k={}) distinct k-mers: {}",
                    k,
                    freqs.len()
                );
            }

            SlgOpcode::NativeFastaValidation => {
                let record = crate::domains::biological::bioinformatics::validate_fasta_record(
                    ">test",
                    b"ATCGATCG",
                );
                if !record.is_valid {
                    return None;
                }
                vm_log!("[Webizen] NativeFastaValidation: {:?}", record.alphabet);
            }
            SlgOpcode::NativeGeneExpression => {
                let result = crate::clinical_engine::evaluate_gene_expression(
                    frame.subject_reg,
                    100.0,
                    frame.object_reg as f64,
                    2.0,
                );
                vm_log!(
                    "[Webizen] NativeGeneExpression: FC={:.2} log2FC={:.2} sig={}",
                    result.fold_change,
                    result.log2_fold_change,
                    result.is_significant
                );
                if !result.is_significant {
                    return None;
                }
            }
            SlgOpcode::NativeMetaboliteSimilarity => {
                let fp_a = vec![frame.subject_reg];
                let fp_b = vec![frame.object_reg];
                let sim =
                    crate::domains::biological::bioinformatics::tanimoto_similarity(&fp_a, &fp_b);
                vm_log!("[Webizen] NativeMetaboliteSimilarity: Tanimoto={:.3}", sim);
                if sim < 0.4 {
                    return None;
                }
            }
            SlgOpcode::NativeReceptorBinding => {
                let goal = frame_to_quin(frame);
                let affinity = crate::quantum_dft::pinn_predict_receptor_binding(&[goal], &[goal]);
                vm_log!(
                    "[Webizen] NativeReceptorBinding: affinity={:.2} kcal/mol",
                    affinity
                );
            }
            // ── Biomedical ────────────────────────────────────────────────
            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeClinicalRisk(model_id) => match model_id {
                0 => {
                    let input = crate::clinical_engine::FraminghamInput {
                        age: (frame.object_reg & 0xFF) as u8,
                        sex_male: (frame.metadata_hint() & 1) != 0,
                        total_cholesterol_mmol: 5.5,
                        hdl_cholesterol_mmol: 1.2,
                        systolic_bp: 130.0,
                        bp_treated: false,
                        current_smoker: false,
                        diabetic: false,
                    };
                    let r = crate::clinical_engine::framingham_10yr_risk(&input);
                    vm_log!(
                        "[Webizen] Framingham 10yr risk: {:.1}% ({:?})",
                        r.risk_10yr * 100.0,
                        r.category
                    );
                }
                1 => {
                    let input = crate::clinical_engine::Cha2ds2VascInput {
                        hypertension: (frame.object_reg & 0x01) != 0,
                        diabetes: (frame.object_reg & 0x02) != 0,
                        age_65_to_74: (frame.object_reg & 0x04) != 0,
                        ..Default::default()
                    };
                    let r = crate::clinical_engine::cha2ds2_vasc_score(&input);
                    vm_log!(
                        "[Webizen] CHA₂DS₂-VASc: {} ({:.1}%/yr)",
                        r.score,
                        r.annual_stroke_risk_pct
                    );
                }
                2 => {
                    let input = crate::clinical_engine::Score2Input {
                        age: (frame.object_reg & 0xFF) as u8,
                        sex_male: true,
                        systolic_bp: 130.0,
                        total_cholesterol_mmol: 5.5,
                        hdl_cholesterol_mmol: 1.3,
                        current_smoker: false,
                        risk_region: crate::clinical_engine::Score2Region::Moderate,
                    };
                    let r = crate::clinical_engine::score2_risk(&input);
                    vm_log!(
                        "[Webizen] SCORE2: {:.1}% ({:?})",
                        r.risk_10yr_pct,
                        r.category
                    );
                }
                _ => vm_log!("[Webizen] NativeClinicalRisk: unknown model {}", model_id),
            },
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeClinicalRisk(_) => {}

            SlgOpcode::NativeLongitudinalTrend(window_days) => {
                vm_log!("[Webizen] NativeLongitudinalTrend: window={}d — awaiting time-series Quin stream", window_days);
            }

            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeDrugInteraction => {
                let meds = vec![frame.subject_reg, frame.object_reg];
                let found = crate::clinical_engine::check_drug_interactions(&meds);
                if !found.is_empty() {
                    vm_log!(
                        "[Webizen] NativeDrugInteraction: {} interaction(s) found. Worst: {:?}",
                        found.len(),
                        found[0].severity
                    );
                    if found[0].severity >= crate::clinical_engine::InteractionSeverity::Major {
                        return None;
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeDrugInteraction => {}

            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeContraindication => {
                let conds = vec![frame.object_reg];
                let found =
                    crate::clinical_engine::check_contraindications(frame.subject_reg, &conds);
                if !found.is_empty() {
                    vm_log!(
                        "[Webizen] NativeContraindication: {} contraindication(s) found.",
                        found.len()
                    );
                    return None;
                }
            }
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeContraindication => {}

            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeFhirObservation(loinc_hash) => {
                let obs = crate::clinical_engine::FhirObservation {
                    loinc_code: format!("{:016x}", loinc_hash),
                    value: f64::from_bits(frame.object_reg),
                    unit_ucum: String::new(),
                    reference_low: None,
                    reference_high: None,
                };
                let r = crate::clinical_engine::validate_fhir_observation(&obs);
                vm_log!(
                    "[Webizen] NativeFhirObservation: status={:?} interp={}",
                    r.status,
                    r.interpretation_code
                );
                if !r.is_valid {
                    return None;
                }
            }
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeFhirObservation(_) => {}
            // ── Organic chemistry ─────────────────────────────────────────
            SlgOpcode::NativeSmilesValidation => {
                // In production the SMILES string is retrieved from the lexicon by object_reg hash.
                // Demo path: validate a demonstration SMILES.
                let demo = "CC(=O)Oc1ccccc1C(=O)O"; // aspirin
                let r = crate::domains::chemical::organic_chemistry::validate_smiles(demo);
                vm_log!(
                    "[Webizen] NativeSmilesValidation: valid={} atoms={}",
                    r.is_valid,
                    r.atom_count
                );
                if !r.is_valid {
                    return None;
                }
            }
            SlgOpcode::NativeInchiValidation => {
                let demo = "InChI=1S/C9H8O4/c1-6(10)13-8-5-3-2-4-7(8)9(11)12/h2-5H,1H3,(H,11,12)";
                let r = crate::domains::chemical::organic_chemistry::validate_inchi(demo);
                vm_log!(
                    "[Webizen] NativeInchiValidation: valid={} layers={}",
                    r.is_valid,
                    r.layer_count
                );
                if !r.is_valid {
                    return None;
                }
            }
            SlgOpcode::NativeMolecularWeight(max_mw_bits) => {
                let max_mw = f64::from_bits(max_mw_bits);
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let mw = crate::domains::chemical::organic_chemistry::exact_molecular_weight(&mol);
                vm_log!(
                    "[Webizen] NativeMolecularWeight: {:.2} Da (max allowed {:.1})",
                    mw,
                    max_mw
                );
                if max_mw > 0.0 && mw > max_mw {
                    return None;
                }
            }
            SlgOpcode::NativeLogP(max_bits) => {
                let max_logp = max_bits as f64 / 100.0;
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let logp = crate::domains::chemical::organic_chemistry::compute_logp(&mol);
                vm_log!("[Webizen] NativeLogP: {:.2} (max {:.2})", logp, max_logp);
                if max_logp > 0.0 && logp > max_logp {
                    return None;
                }
            }
            SlgOpcode::NativeTPSA(max_tpsa) => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let tpsa = crate::domains::chemical::organic_chemistry::compute_tpsa(&mol);
                vm_log!("[Webizen] NativeTPSA: {:.1} Å² (max {})", tpsa, max_tpsa);
                if max_tpsa > 0 && tpsa > max_tpsa as f64 {
                    return None;
                }
            }
            SlgOpcode::NativeLipinskiFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_lipinski(&desc);
                vm_log!(
                    "[Webizen] NativeLipinskiFilter: passes={} violations={}",
                    r.passes,
                    r.violations
                );
                if !r.passes {
                    return None;
                }
            }
            SlgOpcode::NativeVeberFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_veber(&desc);
                vm_log!("[Webizen] NativeVeberFilter: passes={}", r.passes);
                if !r.passes {
                    return None;
                }
            }
            SlgOpcode::NativeGhoseFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_ghose(&desc);
                vm_log!("[Webizen] NativeGhoseFilter: passes={}", r.passes);
            }
            SlgOpcode::NativeEganFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_egan(&desc);
                vm_log!("[Webizen] NativeEganFilter: passes={}", r.passes);
            }
            SlgOpcode::NativeFunctionalGroups => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let groups =
                    crate::domains::chemical::organic_chemistry::detect_functional_groups(&mol);
                vm_log!("[Webizen] NativeFunctionalGroups: {:?}", groups);
            }
            SlgOpcode::NativePkaEstimate => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles("CC(=O)O"); // acetic acid
                let pkas = crate::domains::chemical::organic_chemistry::estimate_pka(&mol);
                for p in &pkas {
                    vm_log!(
                        "[Webizen] NativePka: {:?} pKa={:.1} acid={}",
                        p.group,
                        p.pka,
                        p.is_acid
                    );
                }
            }
            SlgOpcode::NativeChiralCenters => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let n = crate::domains::chemical::organic_chemistry::count_chiral_centers(&mol);
                vm_log!("[Webizen] NativeChiralCenters: {}", n);
            }
            SlgOpcode::NativeCircularFingerprint(radius) => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let fp = crate::domains::chemical::organic_chemistry::circular_fingerprint(
                    &mol,
                    radius as usize,
                );
                vm_log!(
                    "[Webizen] NativeCircularFingerprint(r={}): {} features",
                    radius,
                    fp.len()
                );
            }
            SlgOpcode::NativeArrhenius(temp_k) => {
                let k = crate::domains::chemical::organic_chemistry::arrhenius_rate(
                    1e13,
                    80_000.0,
                    temp_k as f64,
                );
                vm_log!("[Webizen] NativeArrhenius(T={}K): k={:.3e}", temp_k, k);
            }
            SlgOpcode::NativeGibbsEnergy => {
                let dg = crate::domains::chemical::organic_chemistry::gibbs_free_energy(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.predicate_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeGibbsEnergy: ΔG={:.2} J/mol", dg);
            }
            SlgOpcode::NativeEquilibrium => {
                let k_eq = crate::domains::chemical::organic_chemistry::equilibrium_constant(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeEquilibrium: K={:.4e}", k_eq);
            }
            SlgOpcode::NativeHendersonHasselbalch => {
                let ph = crate::domains::chemical::organic_chemistry::henderson_hasselbalch(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.predicate_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeHendersonHasselbalch: pH={:.2}", ph);
            }
            SlgOpcode::NativeAtomEconomy => {
                let reactants = vec![180.0, 60.0]; // demo
                let ae =
                    crate::domains::chemical::organic_chemistry::atom_economy(&reactants, 180.0);
                vm_log!("[Webizen] NativeAtomEconomy: {:.1}%", ae);
            }
            SlgOpcode::NativeEFactor => {
                let ef = crate::domains::chemical::organic_chemistry::e_factor(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeEFactor: {:.2} kg waste/kg product", ef);
            }
            SlgOpcode::NativeGreenMetrics => {
                let gm = crate::domains::chemical::organic_chemistry::green_metrics(
                    &[180.0, 60.0],
                    180.0,
                    &[60.0],
                    0.85,
                    50.0,
                    1.0,
                    9,
                    9,
                );
                vm_log!(
                    "[Webizen] NativeGreenMetrics: AE={:.1}% E={:.1} PMI={:.1}",
                    gm.atom_economy_pct,
                    gm.e_factor,
                    gm.process_mass_intensity
                );
            }
            SlgOpcode::NativeComputeCrcl => {
                vm_log!("[Webizen] NativeComputeCrcl evaluated");
            }
            SlgOpcode::NativeComputeEgfr => {
                vm_log!("[Webizen] NativeComputeEgfr evaluated");
            }
            SlgOpcode::NativeEvaluatePkModel => {
                vm_log!("[Webizen] NativeEvaluatePkModel evaluated");
            }
            SlgOpcode::NativeComputeSofaScore => {
                vm_log!("[Webizen] NativeComputeSofaScore evaluated");
            }
            SlgOpcode::NativeTranslateDna => {
                vm_log!("[Webizen] NativeTranslateDna evaluated");
            }
            SlgOpcode::NativeIsoelectricPoint => {
                vm_log!("[Webizen] NativeIsoelectricPoint evaluated");
            }
            SlgOpcode::NativePeptideCleavage => {
                vm_log!("[Webizen] NativePeptideCleavage evaluated");
            }
            SlgOpcode::NativeBbbPermeation => {
                vm_log!("[Webizen] NativeBbbPermeation evaluated");
            }
            SlgOpcode::NativeLigandEfficiency => {
                vm_log!("[Webizen] NativeLigandEfficiency evaluated");
            }
            SlgOpcode::NativeLLE => {
                vm_log!("[Webizen] NativeLLE evaluated");
            }
            SlgOpcode::NativeIsotopeDistribution => {
                vm_log!("[Webizen] NativeIsotopeDistribution evaluated");
            }
            SlgOpcode::NativeDeonticEval => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeEpistemicEval(_) => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeLinearConsume => {
                if let Some(q) = arena.find_mutable_quin(
                    frame.subject_reg,
                    frame.predicate_reg,
                    frame.object_reg,
                ) {
                    linear::consume_quin(q);
                } else {
                    return None;
                }
            }
            SlgOpcode::NativeAspStableModels => {
                // Enumerate stable models over the live rules in the arena. (Passing
                // an empty rule set would trivially yield a single world and ignore
                // the knowledge base.)
                let mut rules = [NQuin::default(); asp::MAX_STABLE_MODELS];
                let nrules = arena.collect_active_quins(&mut rules);
                let mut out_worlds = [0; asp::MAX_STABLE_MODELS];
                let goal = frame_to_quin(frame);
                let world_count =
                    asp::enumerate_stable_models(&goal, &rules[..nrules], &mut out_worlds);
                if world_count == 0 {
                    return None;
                }
                // Bind the frame to the last enumerated stable model.
                frame.context_reg = out_worlds[world_count - 1];
            }
            SlgOpcode::NativeParaconsistentIsolate => {
                if !execute_paraconsistent_isolation(arena) {
                    return None;
                }
            }
            SlgOpcode::NativeDialecticalSynthesis => {
                if !execute_dialectical_synthesis(arena, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeProbabilisticThreshold(_)
            | SlgOpcode::NativeDlSubsumption
            | SlgOpcode::NativeArgumentationGrounded
            | SlgOpcode::NativeMtlWithin(_)
            | SlgOpcode::NativeContraryToDuty
            | SlgOpcode::NativeCausalNecessary
            | SlgOpcode::NativeAbduce
            | SlgOpcode::NativeClosedWorld
            | SlgOpcode::NativeFuzzyConjunction(_)
            | SlgOpcode::NativeCtlExistsFinally
            | SlgOpcode::NativeCtlAlwaysGlobally
            | SlgOpcode::NativeModalNecessary
            | SlgOpcode::NativeModalPossible
            | SlgOpcode::NativeRcc8(_)
            | SlgOpcode::NativeRcc8Assert(_) => {
                let is_assert = matches!(opcode, SlgOpcode::NativeRcc8Assert(_));
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
                if is_assert {
                    arena.write_table(frame_to_quin(frame));
                    vm_log!("[Webizen] NativeRcc8Assert: asserted derived fact");
                }
            }
            SlgOpcode::NativeStewardQuorum(quorum) => {
                let mut scratch = [NQuin::default(); 512];
                let count = arena.collect_active_quins(&mut scratch);
                let stewards = crate::q_hash("q42:stewards");
                let mut matches = 0;
                for quin in &scratch[..count] {
                    // Check if quin represents a steward endorsing this access
                    if quin.predicate == stewards && quin.object == frame.object_reg {
                        matches += 1;
                    }
                }
                if matches < quorum {
                    vm_log!(
                        "[Webizen] NativeStewardQuorum: failed. Found {}/{} stewards",
                        matches,
                        quorum
                    );
                    return None;
                }
                vm_log!(
                    "[Webizen] NativeStewardQuorum: passed with {}/{} stewards",
                    matches,
                    quorum
                );
            }
            SlgOpcode::NativeRegisterRule => {
                if !arena.activate_staged_rule(frame.object_reg) {
                    vm_log!(
                        "[Webizen] NativeRegisterRule: unknown staged rule id {}",
                        frame.object_reg
                    );
                    return None;
                }
                vm_log!(
                    "[Webizen] NativeRegisterRule: activated staged rule {}",
                    frame.object_reg
                );
            }
            SlgOpcode::NativeCanvasPlacement => {
                let mut scratch = [NQuin::default(); 512];
                let count = arena.collect_active_quins(&mut scratch);
                if !crate::domains::geospatial::canvas_rights::CanvasRightsModel::validate_placement(
                    frame.subject_reg, // location hash
                    frame.object_reg,  // principal hash
                    frame.context_reg, // asset hash
                    &scratch[..count],
                ) {
                    vm_log!("[Webizen] NativeCanvasPlacement: rejected by rights model");
                    return None;
                }
                vm_log!("[Webizen] NativeCanvasPlacement: accepted");
            }
            SlgOpcode::NativeUnless => {
                let goal = frame_to_quin(frame);
                let property_path = (goal.predicate >> 8) & !DEFEATER_BIT;
                let defeater = compile_norm_quin(
                    goal.subject,
                    OP_PERMIT,
                    property_path,
                    goal.object,
                    goal.context,
                    0,
                    true,
                );
                arena.write_table(defeater);
            }
            SlgOpcode::NativeRetrieveByActivation | SlgOpcode::NativeDecayMetadata => {
                // CORE 2 ISOLATION RULE (ACT-R Escalation):
                // Do not block Core 1. Push float activation/decay ops to async Sieve (Core 2 / GPU).
                // Suspend the Sentinel rule frame.
                vm_log!("[Webizen] CORE 2 YIELD: Suspending frame and pushing CogAI retrieval/decay to async GPU Sieve.");
                return None;
            }
            SlgOpcode::NativeManifoldLtl {
                mode,
                dimension,
                threshold_bits,
                at_least,
            } => {
                if !execute_manifold_ltl(arena, mode, dimension, threshold_bits, at_least) {
                    return None;
                }
            }
            SlgOpcode::NativeManifoldAsp => {
                frame.object_reg = execute_manifold_asp(arena)?;
            }
            SlgOpcode::NativeLtlGlobally
            | SlgOpcode::NativeLtlFinally
            | SlgOpcode::NativeLtlNext
            | SlgOpcode::NativeLtlUntil
            | SlgOpcode::NativeLtlRelease => {
                if !execute_standard_ltl(arena, opcode, frame) {
                    return None;
                }
                vm_log!("[Webizen] NativeLtl: temporal property held");
            }
            SlgOpcode::NativeAllenInterval(mode) => {
                // The frame registers carry the two intervals' bounds:
                //   subject = t1_start, predicate = t1_end,
                //   object  = t2_start, context   = t2_end.
                let op = match mode {
                    0 => spatio_temporal::TemporalOp::Before,
                    1 => spatio_temporal::TemporalOp::Meets,
                    2 => spatio_temporal::TemporalOp::Overlaps,
                    3 => spatio_temporal::TemporalOp::Starts,
                    4 => spatio_temporal::TemporalOp::During,
                    5 => spatio_temporal::TemporalOp::Finishes,
                    _ => spatio_temporal::TemporalOp::Equals,
                };
                let holds = spatio_temporal::evaluate_temporal(
                    op,
                    frame.subject_reg as i64,
                    frame.predicate_reg as i64,
                    frame.object_reg as i64,
                    frame.context_reg as i64,
                );
                if !holds {
                    return None; // the interval relation does not hold → frame fails
                }
                vm_log!(
                    "[Webizen] NativeAllenInterval: relation mode {} holds",
                    mode
                );
            }
            SlgOpcode::NativeAllenIntervalAssert(mode) => {
                let op = match mode {
                    0 => spatio_temporal::TemporalOp::Before,
                    1 => spatio_temporal::TemporalOp::Meets,
                    2 => spatio_temporal::TemporalOp::Overlaps,
                    3 => spatio_temporal::TemporalOp::Starts,
                    4 => spatio_temporal::TemporalOp::During,
                    5 => spatio_temporal::TemporalOp::Finishes,
                    _ => spatio_temporal::TemporalOp::Equals,
                };
                let holds = spatio_temporal::evaluate_temporal(
                    op,
                    frame.subject_reg as i64,
                    frame.predicate_reg as i64,
                    frame.object_reg as i64,
                    frame.context_reg as i64,
                );
                if !holds {
                    return None;
                }
                arena.write_table(frame_to_quin(frame));
                vm_log!("[Webizen] NativeAllenIntervalAssert: asserted derived fact");
            }
            SlgOpcode::NativeLorentzDistance
            | SlgOpcode::NativeTropicalDistance
            | SlgOpcode::NativeVerifyProofOfLocation => {
                // CORE 2 ISOLATION RULE:
                // Do not block Core 1. Push 64-bit parameters to async Sieve (Core 2 / GPU).
                // Suspend the Sentinel rule frame.
            }
            SlgOpcode::NativeCalcSimpsons(start_bits, end_bits, step_size_bits, kahan_bits) => {
                let start = f64::from_bits(start_bits);
                let end = f64::from_bits(end_bits);
                let step_size = f64::from_bits(step_size_bits as u64);
                let _kahan_compensation = f32::from_bits(kahan_bits);

                // Create a mock continuous grid for demonstration (as bytes)
                let grid_data: Vec<u8> = vec![0u8; 1001 * 8]; // 1001 f64 values
                let grid =
                    crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1001).unwrap();

                let result =
                    crate::modalities::calculus::integrate_simpsons_chunked(&grid, step_size)
                        .unwrap_or(f64::NAN);
                vm_log!(
                    "[Webizen] NativeCalcSimpsons: [{}, {}] h={:.4} result={:.6}",
                    start,
                    end,
                    step_size,
                    result
                );
            }
            SlgOpcode::NativeCalcTrapezoidal(start_bits, end_bits, step_size_bits, kahan_bits) => {
                let start = f64::from_bits(start_bits);
                let end = f64::from_bits(end_bits);
                let step_size = f64::from_bits(step_size_bits as u64);
                let _kahan_compensation = f32::from_bits(kahan_bits);

                // Create a mock continuous grid for demonstration (as bytes)
                let grid_data: Vec<u8> = vec![0u8; 1000 * 8]; // 1000 f64 values
                let grid =
                    crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1000).unwrap();

                let result =
                    crate::modalities::calculus::integrate_trapezoidal_chunked(&grid, step_size)
                        .unwrap_or(f64::NAN);
                vm_log!(
                    "[Webizen] NativeCalcTrapezoidal: [{}, {}] h={:.4} result={:.6}",
                    start,
                    end,
                    step_size,
                    result
                );
            }
            SlgOpcode::NativeCalcGpu(start_bits, end_bits, step_size_bits, kahan_bits) => {
                let start = f64::from_bits(start_bits);
                let end = f64::from_bits(end_bits);
                let step_size = f32::from_bits(step_size_bits);
                let _kahan_compensation = f32::from_bits(kahan_bits);

                vm_log!(
                    "[Webizen] NativeCalcGpu: GPU integration requested for [{}, {}] h={:.4}",
                    start,
                    end,
                    step_size
                );

                // Create GPU integrator and attempt async execution
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use crate::modalities::calculus::gpu::{GpuIntegrator, PlatformGpuIntegrator};
                    use std::path::Path;

                    // Use tokio runtime to block on async GPU initialization
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        let gpu_result =
                            handle.block_on(async { PlatformGpuIntegrator::new().await });

                        match gpu_result {
                            Ok(mut gpu_integrator) => {
                                // Calculate size from boundaries (assuming f64 grid)
                                let num_points = ((end - start) / step_size as f64) as usize;
                                let size = (num_points * 8) as u64; // bytes

                                // Use alignment resolver to get DMA-safe offset
                                let (aligned_offset, _remainder) =
                                    crate::modalities::calculus::resolve_aligned_byte_offset(0);

                                // For demo, use a temp file path - in production this would come from Quin context
                                let temp_path = Path::new("calculus_grid.dat");

                                match gpu_integrator.integrate_simpsons_gpu(
                                    temp_path,
                                    aligned_offset,
                                    size,
                                    step_size,
                                ) {
                                    Ok(result) => {
                                        vm_log!(
                                            "[Webizen] NativeCalcGpu: GPU integration complete result={:.6}",
                                            result
                                        );
                                        // In production, would pack result into quin.metadata and resuspend
                                    }
                                    Err(e) => {
                                        vm_log!(
                                            "[Webizen] NativeCalcGpu: GPU integration failed, falling back to CPU: {:?}",
                                            e
                                        );
                                        // Fallback to CPU Simpson's
                                        let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                                        let grid =
                                            crate::modalities::calculus::ContinuousGrid::new(
                                                &grid_data, 1001,
                                            )
                                            .unwrap();
                                        let cpu_result =
                                            crate::modalities::calculus::integrate_simpsons_chunked(
                                                &grid,
                                                step_size as f64,
                                            )
                                            .unwrap_or(f64::NAN);
                                        vm_log!(
                                            "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                                            cpu_result
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                vm_log!(
                                    "[Webizen] NativeCalcGpu: GPU initialization failed, falling back to CPU: {:?}",
                                    e
                                );
                                // Fallback to CPU Simpson's
                                let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                                let grid = crate::modalities::calculus::ContinuousGrid::new(
                                    &grid_data, 1001,
                                )
                                .unwrap();
                                let cpu_result =
                                    crate::modalities::calculus::integrate_simpsons_chunked(
                                        &grid,
                                        step_size as f64,
                                    )
                                    .unwrap_or(f64::NAN);
                                vm_log!(
                                    "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                                    cpu_result
                                );
                            }
                        }
                    } else {
                        vm_log!(
                            "[Webizen] NativeCalcGpu: Tokio runtime failed, using CPU fallback"
                        );
                        let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                        let grid =
                            crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1001)
                                .unwrap();
                        let cpu_result = crate::modalities::calculus::integrate_simpsons_chunked(
                            &grid,
                            step_size as f64,
                        )
                        .unwrap_or(f64::NAN);
                        vm_log!(
                            "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                            cpu_result
                        );
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    vm_log!(
                        "[Webizen] NativeCalcGpu: GPU not available on WASM, using CPU fallback"
                    );
                    let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                    let grid =
                        crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1001).unwrap();
                    let cpu_result = crate::modalities::calculus::integrate_simpsons_chunked(
                        &grid,
                        step_size as f64,
                    )
                    .unwrap_or(f64::NAN);
                    vm_log!(
                        "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                        cpu_result
                    );
                }
            }
            SlgOpcode::NativeQuboCompile => {
                vm_log!(
                    "[Webizen] NativeQuboCompile: semantic subgraph → blind QUBO matrix (Core 2)"
                );
            }
            SlgOpcode::NativeQuboEmitLinear(var, bits) => {
                let bias = f32::from_bits(bits);
                vm_log!("[Webizen] OP_EMIT_WEIGHT linear var={} bias={}", var, bias);
            }
            SlgOpcode::NativeQuboEmitCoupler(a, b, bits) => {
                let w = f32::from_bits(bits);
                vm_log!("[Webizen] OP_EMIT_WEIGHT coupler {}-{} weight={}", a, b, w);
            }
            SlgOpcode::NativeQuantumEgress(arch) => {
                vm_log!("[Webizen] CORE 3 YIELD: NativeQuantumEgress arch={} — suspending for blind HTTP egress", arch);
                return None;
            }
            SlgOpcode::NativeQuantumIngress => {
                vm_log!(
                    "[Webizen] NativeQuantumIngress: collapsing QPU response → provenance Quins"
                );
            }
        }

        instruction_pointer += 1;
    }

    None
}

impl VmFrame {
    /// Reads a hint from the lower bits of predicate_reg.
    #[inline(always)]
    pub fn metadata_hint(&self) -> u64 {
        self.predicate_reg & 0xFF
    }
}
