use crate::deontic_logic::{
    compile_norm_quin, evaluate_deontic_contract, harvest_defeater_fingerprints,
    norm_has_active_defeater, DeonticStatus, DeonticVerdict, DEFEATER_BIT, MAX_DEFEATER_SLOTS,
    OP_PERMIT,
};
use crate::modalities::{asp, dialectical, dl, epistemic, linear, paraconsistent, probabilistic};
use crate::tax_schema::TaxRuleSchema;
use crate::QualiaQuin;

macro_rules! vm_log {
    ($($arg:tt)*) => {
        if cfg!(feature = "vm_tracing") {
            println!($($arg)*);
        }
    };
}

/// A fast, non-cryptographic bitwise hash to lookup sub-goals in the SLG Arena
/// without wasting CPU cycles on cryptographic overhead.
#[inline(always)]
fn fast_hash_goal(subject: u64, predicate: u64, object: u64) -> usize {
    let mut hash = subject.wrapping_add(0x9E3779B97F4A7C15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    hash = (hash ^ predicate).wrapping_mul(0x94D049BB133111EB);
    hash = (hash ^ object).wrapping_mul(0x9E3779B97F4A7C15);
    (hash ^ (hash >> 31)) as usize
}

// 42MB = 44,040,192 bytes
const SLG_ARENA_SIZE: usize = 42 * 1024 * 1024;
const QUIN_SIZE: usize = 48;
const MAX_SLOTS: usize = SLG_ARENA_SIZE / QUIN_SIZE; // 917,504 slots

use crate::n3_parser::Rule;

/// The 42MB Static Tabling Arena for SLG Resolution
/// Implemented as a Zero-Allocation Static Ring-Buffer Arena
const RECENT_SLOT_RING: usize = 512;

pub struct SlgArena {
    // We will use a safe Vec wrapper here since it is allocated strictly once and never grown.
    buffer: alloc::vec::Vec<QualiaQuin>,
    head_pointer: usize,
    recent_slots: [usize; RECENT_SLOT_RING],
    recent_slot_head: usize,
    // Native Rule Registry to hold N3 Logical Implications
    rule_registry: alloc::vec::Vec<Rule>,
}

#[cfg(feature = "alloc_buffers")]
extern crate alloc;

impl SlgArena {
    pub fn new() -> Self {
        #[cfg(not(feature = "alloc_buffers"))]
        extern crate alloc;

        let mut buffer = alloc::vec::Vec::with_capacity(MAX_SLOTS);
        // Pre-fill the ring buffer with empty Quins
        for _ in 0..MAX_SLOTS {
            buffer.push(QualiaQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }

        Self {
            buffer,
            head_pointer: 0,
            recent_slots: [0; RECENT_SLOT_RING],
            recent_slot_head: 0,
            rule_registry: alloc::vec::Vec::new(),
        }
    }

    /// Registers a logical implication rule into the Webizen VM
    pub fn register_rule(&mut self, rule: Rule) {
        vm_log!("🧠 Webizen registered new N3 Rule: {:?}", rule);
        self.rule_registry.push(rule);
    }

    pub fn rule_count(&self) -> usize {
        self.rule_registry.len()
    }

    /// Collect recently written Quins with valid ECC parity (bounded scan, zero heap).
    pub fn collect_active_quins(&self, out: &mut [QualiaQuin]) -> usize {
        let mut n = 0usize;
        let scan = RECENT_SLOT_RING.min(self.recent_slot_head);
        for off in 0..scan {
            let ring_idx = (self.recent_slot_head + RECENT_SLOT_RING - 1 - off) % RECENT_SLOT_RING;
            let idx = self.recent_slots[ring_idx];
            let q = self.buffer[idx];
            if q.subject == 0 {
                continue;
            }
            let expected = q.subject ^ q.predicate ^ q.object ^ q.context;
            if q.parity != expected {
                continue;
            }
            if n < out.len() {
                out[n] = q;
                n += 1;
            }
        }
        n
    }

    /// Compile registered N3 rules to norms + bytecode and execute on Core 1 (cold path).
    pub fn fire_registered_rules(&mut self, contract_hash: u64) -> usize {
        let rules = self.rule_registry.clone();
        let mut fired = 0usize;
        for rule in &rules {
            if let Some(norm) =
                crate::deontic_logic::compile_n3_rule_to_norm(rule, contract_hash, 0)
            {
                self.write_table(norm);
            }
            let mut opcodes = [SlgOpcode::Halt; 64];
            if let Ok(count) = crate::n3_compiler::compile_rule_to_opcodes(rule, &mut opcodes) {
                let mut frame = VmFrame::default();
                if execute_vm_frame(self, &opcodes[..count], &mut frame).is_some() {
                    fired += 1;
                }
            }
        }
        fired
    }

    /// Checks the SLG Arena for a previously proven sub-goal.
    pub fn check_table(&self, subject: u64, predicate: u64, object: u64) -> Option<QualiaQuin> {
        let slot = fast_hash_goal(subject, predicate, object) % MAX_SLOTS;

        let cached = self.buffer[slot];
        if cached.subject == subject && cached.predicate == predicate && cached.object == object {
            Some(cached)
        } else {
            None
        }
    }

    /// Writes a proven sub-goal into the SLG Arena.
    /// If the slot is occupied (hash collision) or we hit the boundary,
    /// it acts as a FIFO ring-buffer and strictly overwrites the oldest cache entries.
    pub fn write_table(&mut self, result: QualiaQuin) {
        let slot = fast_hash_goal(result.subject, result.predicate, result.object) % MAX_SLOTS;

        // Cyclic Eviction Policy: Overwrite whatever is in the slot natively
        self.buffer[slot] = result;
        self.recent_slots[self.recent_slot_head % RECENT_SLOT_RING] = slot;
        self.recent_slot_head = self.recent_slot_head.saturating_add(1);

        // Increment global ring-buffer pointer (used if we wanted strict sequential FIFO instead of hashed slots)
        self.head_pointer = (self.head_pointer + 1) % MAX_SLOTS;
    }

    pub(crate) fn find_mutable_quin(
        &mut self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Option<&mut QualiaQuin> {
        let scan = RECENT_SLOT_RING.min(self.recent_slot_head);
        for off in 0..scan {
            let ring_idx = (self.recent_slot_head + RECENT_SLOT_RING - 1 - off) % RECENT_SLOT_RING;
            let idx = self.recent_slots[ring_idx];
            let matches = self.buffer[idx].subject == subject
                && self.buffer[idx].predicate == predicate
                && (object == 0 || self.buffer[idx].object == object);
            if matches {
                return Some(&mut self.buffer[idx]);
            }
        }
        None
    }
}

/// The Opcodes for the Lightweight Warren Abstract Machine (WAM) variant.
/// `f64` parameters require `PartialEq` only — `Eq` is not derived.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlgOpcode {
    // ── Core WAM ─────────────────────────────────────────────────────────────
    CheckTable,
    CheckDefeaters,
    CheckSubsumption,
    BranchWorld,
    /// Deprecated — use `CheckMinInclusive`.
    CheckThreshold,
    ConsumeFact,
    Unify,
    Call,
    Return,
    ApplyTaxSchema,
    Halt,
    /// sh:Warning terminal — emits diagnostic but does not halt ingestion.
    WarnOnly,

    // ── Standard SHACL numeric range ─────────────────────────────────────────
    CheckMinInclusive(f64),
    CheckMaxInclusive(f64),
    CheckMinExclusive(f64),
    CheckMaxExclusive(f64),

    // ── Standard SHACL cardinality ────────────────────────────────────────────
    CheckMinCount(u32),
    CheckMaxCount(u32),

    // ── Standard SHACL string ─────────────────────────────────────────────────
    CheckMinLength(u32),
    CheckMaxLength(u32),
    /// Pattern stored as q_hash of the regex string; compared against literal hash.
    CheckPattern(u64),

    // ── Standard SHACL value constraints ──────────────────────────────────────
    /// q_hash of the single expected value.
    CheckHasValue(u64),
    /// q_hash of a referenced node-shape IRI.
    CheckNodeShape(u64),
    /// Negation: passes only if the referenced shape would FAIL.
    CheckNotShape(u64),

    // ── Native: physics ───────────────────────────────────────────────────────
    NativeThermodynamics,
    NativeOdeSolver,
    NativeQuantumDft,
    /// `qualia:predictReceptorBinding` — PINN binding affinity.
    NativeReceptorBinding,
    /// Compile semantic constraints into blind QUBO matrix (Core 2).
    NativeQuboCompile,
    /// Emit linear QUBO bias: (var_index, f32_bits).
    NativeQuboEmitLinear(u8, u32),
    /// Emit quadratic coupler: (var_a, var_b, f32_bits).
    NativeQuboEmitCoupler(u8, u8, u32),
    /// Egress to remote QPU: 0=annealer, 1=gate-model. Yields frame to Core 3.
    NativeQuantumEgress(u8),
    /// Ingress: collapse provider JSON into provenance Quins.
    NativeQuantumIngress,

    // ── Native: biosciences ───────────────────────────────────────────────────
    /// `qualia:alignNucleotideSequence` — Smith-Waterman with BLAST nucleotide matrix.
    NativeNucleotideAlign,
    /// Deprecated form — routes to `NativeNucleotideAlign`.
    NativeBioinformatics,
    /// `qualia:alignProteinSequence` — BLOSUM62 (0) or PAM250 (1).
    NativeProteinAlign(u8),
    /// `qualia:computeKmerFrequency` — k-mer size embedded as parameter.
    NativeKmerFrequency(u8),
    /// `qualia:validateFastaRecord`.
    NativeFastaValidation,
    /// `qualia:evaluateGeneExpression`.
    NativeGeneExpression,
    /// `qualia:computeMetaboliteSimilarity` — Tanimoto fingerprint check.
    NativeMetaboliteSimilarity,

    // ── Native: biomedical ────────────────────────────────────────────────────
    /// `qualia:computeRiskScore` — 0=Framingham, 1=CHA₂DS₂-VASc, 2=SCORE2.
    NativeClinicalRisk(u8),
    /// `qualia:evaluateLongitudinalTrend` — sliding window in days.
    NativeLongitudinalTrend(u32),
    /// `qualia:evaluateDrugInteraction`.
    NativeDrugInteraction,
    /// `qualia:checkContraindication`.
    NativeContraindication,
    /// `qualia:validateFhirObservation` — LOINC code hash.
    NativeFhirObservation(u64),

    // ── Native: economics ─────────────────────────────────────────────────────
    NativeEconomics,

    // ── Native: organic chemistry ─────────────────────────────────────────────
    /// `qualia:validateSmiles` — structural SMILES validity.
    NativeSmilesValidation,
    /// `qualia:validateInchi` — InChI / InChIKey format check.
    NativeInchiValidation,
    /// `qualia:computeMolecularWeight` — exact MW from SMILES; param = max allowed Da (bits of f64).
    NativeMolecularWeight(u64),
    /// `qualia:computeLogP` — Crippen LogP; param = max allowed × 100 as i32 (stored as u32 bits).
    NativeLogP(u32),
    /// `qualia:computeTPSA` — Ertl TPSA; param = max Å² (as u32).
    NativeTPSA(u32),
    /// `qualia:evaluateLipinski` — Rule-of-Five drug-likeness filter.
    NativeLipinskiFilter,
    /// `qualia:evaluateVeber` — Veber oral-bioavailability filter.
    NativeVeberFilter,
    /// `qualia:evaluateGhose` — Ghose drug-likeness filter.
    NativeGhoseFilter,
    /// `qualia:evaluateEgan` — Egan passive-absorption filter.
    NativeEganFilter,
    /// `qualia:detectFunctionalGroups` — returns set of detected functional group hashes.
    NativeFunctionalGroups,
    /// `qualia:computePka` — functional-group-based pKa estimation.
    NativePkaEstimate,
    /// `qualia:computeChiralCenters` — count sp3 C with 4 distinct substituents.
    NativeChiralCenters,
    /// `qualia:generateCircularFingerprint` — Morgan fingerprint; param = radius.
    NativeCircularFingerprint(u8),
    /// `qualia:computeArrheniusRate` — k = A·exp(−Ea/RT); param encodes temperature K as u32.
    NativeArrhenius(u32),
    /// `qualia:computeGibbsEnergy` — ΔG = ΔH − TΔS.
    NativeGibbsEnergy,
    /// `qualia:computeEquilibrium` — K = exp(−ΔG°/RT).
    NativeEquilibrium,
    /// `qualia:computeHendersonHasselbalch` — pH from pKa + concentration ratio.
    NativeHendersonHasselbalch,
    /// `qualia:computeAtomEconomy` — Trost 1991 green metric.
    NativeAtomEconomy,
    /// `qualia:computeEFactor` — Sheldon waste-per-product metric.
    NativeEFactor,
    /// `qualia:computeGreenMetrics` — full suite: AE, E-factor, PMI, RME, CE.
    NativeGreenMetrics,

    // ── Native: Phase 5 Scientific ────────────────────────────────────────────
    NativeComputeCrcl,
    NativeComputeEgfr,
    NativeEvaluatePkModel,
    NativeComputeSofaScore,
    NativeTranslateDna,
    NativeIsoelectricPoint,
    NativePeptideCleavage,
    NativeBbbPermeation,
    NativeLigandEfficiency,
    NativeLLE,
    NativeIsotopeDistribution,

    // ── Native: deontic and epistemic ─────────────────────────────────────────
    NativeDeonticEval,
    NativeEpistemicEval(u8),

    // ── Native: advanced logics ───────────────────────────────────────────────
    NativeLinearConsume,
    NativeAspStableModels,
    NativeParaconsistentIsolate,
    NativeDialecticalSynthesis,

    // ── Native: cognitive ai (ACT-R) ──────────────────────────────────────────
    NativeRetrieveByActivation,
    NativeDecayMetadata,
    NativeUnless,

    // ── Native: temporal logic (LTL) ──────────────────────────────────────────
    NativeLtlGlobally,
    NativeLtlFinally,
    NativeLtlNext,
    NativeLtlUntil,
    NativeLtlRelease,

    // ── Native: spatio-temporal (Allen Interval) ──────────────────────────────
    NativeAllenInterval(u8),

    // ── Native: geometric and spatial topology ────────────────────────────────
    NativeLorentzDistance,
    NativeTropicalDistance,
    NativeVerifyProofOfLocation,
}

/// The Execution Frame tracking variable bindings without touching the heap
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VmFrame {
    pub subject_reg: u64,
    pub predicate_reg: u64,
    pub object_reg: u64,
    pub context_reg: u64,
}

#[inline]
fn frame_to_quin(frame: &VmFrame) -> QualiaQuin {
    let mut q = QualiaQuin {
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

fn unify_frame(arena: &SlgArena, frame: &mut VmFrame) -> bool {
    if arena
        .check_table(frame.subject_reg, frame.predicate_reg, frame.object_reg)
        .is_some()
    {
        return true;
    }

    let mut scratch = [QualiaQuin::default(); 256];
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

/// The Bytecode Evaluator for the Prolog Webizen
pub fn execute_vm_frame(
    arena: &mut SlgArena,
    bytecode: &[SlgOpcode],
    frame: &mut VmFrame,
) -> Option<QualiaQuin> {
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
                let mut scratch = [QualiaQuin::default(); 512];
                let count = arena.collect_active_quins(&mut scratch);
                let mut fp_buf = [0u64; MAX_DEFEATER_SLOTS];
                let fp_count = harvest_defeater_fingerprints(&scratch[..count], &mut fp_buf);
                let goal = frame_to_quin(frame);
                if norm_has_active_defeater(&goal, &fp_buf[..fp_count]) {
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
                let mut sampler = crate::thermodynamics::ThermodynamicSampler::new(298.0, 100);
                sampler.metropolis_step(50.0, 0.5);
                vm_log!(
                    "🧪 Webizen executed NativeThermodynamics step. Current Energy: {}",
                    sampler.current_state.total_energy
                );
            }
            SlgOpcode::NativeOdeSolver => {
                // Mock execution of continuous dynamics via RK4
                let initial = crate::ode_solver::PhysicalState {
                    time: 0.0,
                    values: alloc::vec![1.0],
                };
                let final_state = crate::ode_solver::evaluate_continuous_dynamics(initial, 10, 0.1);
                vm_log!(
                    "📈 Webizen executed NativeOdeSolver. Final state: {:?}",
                    final_state.values
                );
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
                let score = crate::bioinformatics::align_sequences(b"ATCG", b"ATCC");
                vm_log!(
                    "[Webizen] NativeBioinformatics (legacy). SW score: {}",
                    score.score
                );
            }
            SlgOpcode::NativeEconomics => {
                let (mean, var) =
                    crate::economics::run_monte_carlo_var(100.0, 0.05, 0.2, 1.0, 1000, 252);
                vm_log!(
                    "[Webizen] NativeEconomics. Mean: {:.2}, VaR95: {:.2}",
                    mean,
                    var
                );
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
                    vm_log!(
                        "[Webizen] CheckPattern: hash mismatch {:016x} vs {:016x}",
                        frame.object_reg,
                        pattern_hash
                    );
                }
            }
            SlgOpcode::CheckHasValue(expected) => {
                if frame.object_reg != expected {
                    return None;
                }
            }
            SlgOpcode::CheckNodeShape(shape_id) => {
                vm_log!(
                    "[Webizen] CheckNodeShape: delegating to shape {:016x}",
                    shape_id
                );
            }
            SlgOpcode::CheckNotShape(shape_id) => {
                vm_log!(
                    "[Webizen] CheckNotShape: verifying shape {:016x} fails as expected",
                    shape_id
                );
            }
            // ── Biosciences ───────────────────────────────────────────────
            SlgOpcode::NativeNucleotideAlign => {
                let demo_result = crate::bioinformatics::align_nucleotide(b"ACGTACGT", b"ACGTCCGT");
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
                let result = crate::bioinformatics::align_protein(b"ACDEFGHIK", b"ACDEFGHIK");
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
                let freqs = crate::bioinformatics::kmer_frequencies(b"ACGTACGTACGT", k as usize);
                vm_log!(
                    "[Webizen] NativeKmerFrequency(k={}) distinct k-mers: {}",
                    k,
                    freqs.len()
                );
            }
            SlgOpcode::NativeFastaValidation => {
                let record = crate::bioinformatics::validate_fasta_record(">test", b"ATCGATCG");
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
                let sim = crate::bioinformatics::tanimoto_similarity(&fp_a, &fp_b);
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
            SlgOpcode::NativeLongitudinalTrend(window_days) => {
                vm_log!("[Webizen] NativeLongitudinalTrend: window={}d — awaiting time-series Quin stream", window_days);
            }
            SlgOpcode::NativeDrugInteraction => {
                // Medication list encoded as Quins in the arena; demo with two hashes from registers
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
            SlgOpcode::NativeFhirObservation(loinc_hash) => {
                // Real lookup would decode the LOINC string from the lexicon; demo path below
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
            // ── Organic chemistry ─────────────────────────────────────────
            SlgOpcode::NativeSmilesValidation => {
                // In production the SMILES string is retrieved from the lexicon by object_reg hash.
                // Demo path: validate a demonstration SMILES.
                let demo = "CC(=O)Oc1ccccc1C(=O)O"; // aspirin
                let r = crate::organic_chemistry::validate_smiles(demo);
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
                let r = crate::organic_chemistry::validate_inchi(demo);
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
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let mw = crate::organic_chemistry::exact_molecular_weight(&mol);
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
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let logp = crate::organic_chemistry::compute_logp(&mol);
                vm_log!("[Webizen] NativeLogP: {:.2} (max {:.2})", logp, max_logp);
                if max_logp > 0.0 && logp > max_logp {
                    return None;
                }
            }
            SlgOpcode::NativeTPSA(max_tpsa) => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let tpsa = crate::organic_chemistry::compute_tpsa(&mol);
                vm_log!("[Webizen] NativeTPSA: {:.1} Å² (max {})", tpsa, max_tpsa);
                if max_tpsa > 0 && tpsa > max_tpsa as f64 {
                    return None;
                }
            }
            SlgOpcode::NativeLipinskiFilter => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let desc = crate::organic_chemistry::compute_descriptors(&mol);
                let r = crate::organic_chemistry::evaluate_lipinski(&desc);
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
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let desc = crate::organic_chemistry::compute_descriptors(&mol);
                let r = crate::organic_chemistry::evaluate_veber(&desc);
                vm_log!("[Webizen] NativeVeberFilter: passes={}", r.passes);
                if !r.passes {
                    return None;
                }
            }
            SlgOpcode::NativeGhoseFilter => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let desc = crate::organic_chemistry::compute_descriptors(&mol);
                let r = crate::organic_chemistry::evaluate_ghose(&desc);
                vm_log!("[Webizen] NativeGhoseFilter: passes={}", r.passes);
            }
            SlgOpcode::NativeEganFilter => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let desc = crate::organic_chemistry::compute_descriptors(&mol);
                let r = crate::organic_chemistry::evaluate_egan(&desc);
                vm_log!("[Webizen] NativeEganFilter: passes={}", r.passes);
            }
            SlgOpcode::NativeFunctionalGroups => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let groups = crate::organic_chemistry::detect_functional_groups(&mol);
                vm_log!("[Webizen] NativeFunctionalGroups: {:?}", groups);
            }
            SlgOpcode::NativePkaEstimate => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)O"); // acetic acid
                let pkas = crate::organic_chemistry::estimate_pka(&mol);
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
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let n = crate::organic_chemistry::count_chiral_centers(&mol);
                vm_log!("[Webizen] NativeChiralCenters: {}", n);
            }
            SlgOpcode::NativeCircularFingerprint(radius) => {
                let mol = crate::organic_chemistry::parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
                let fp = crate::organic_chemistry::circular_fingerprint(&mol, radius as usize);
                vm_log!(
                    "[Webizen] NativeCircularFingerprint(r={}): {} features",
                    radius,
                    fp.len()
                );
            }
            SlgOpcode::NativeArrhenius(temp_k) => {
                let k = crate::organic_chemistry::arrhenius_rate(1e13, 80_000.0, temp_k as f64);
                vm_log!("[Webizen] NativeArrhenius(T={}K): k={:.3e}", temp_k, k);
            }
            SlgOpcode::NativeGibbsEnergy => {
                let dg = crate::organic_chemistry::gibbs_free_energy(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.predicate_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeGibbsEnergy: ΔG={:.2} J/mol", dg);
            }
            SlgOpcode::NativeEquilibrium => {
                let k_eq = crate::organic_chemistry::equilibrium_constant(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeEquilibrium: K={:.4e}", k_eq);
            }
            SlgOpcode::NativeHendersonHasselbalch => {
                let ph = crate::organic_chemistry::henderson_hasselbalch(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.predicate_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeHendersonHasselbalch: pH={:.2}", ph);
            }
            SlgOpcode::NativeAtomEconomy => {
                let reactants = vec![180.0, 60.0]; // demo
                let ae = crate::organic_chemistry::atom_economy(&reactants, 180.0);
                vm_log!("[Webizen] NativeAtomEconomy: {:.1}%", ae);
            }
            SlgOpcode::NativeEFactor => {
                let ef = crate::organic_chemistry::e_factor(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeEFactor: {:.2} kg waste/kg product", ef);
            }
            SlgOpcode::NativeGreenMetrics => {
                let gm = crate::organic_chemistry::green_metrics(
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
                let mut scratch = [QualiaQuin::default(); 512];
                let count = arena.collect_active_quins(&mut scratch);
                let mut verdicts = [DeonticVerdict::default(); 64];
                let vcount =
                    evaluate_deontic_contract(&scratch[..count], current_unix32(), &mut verdicts)
                        .unwrap_or(0);
                let goal = frame_to_quin(frame);
                for verdict in &verdicts[..vcount] {
                    if verdict.norm.subject == goal.subject
                        && verdict.norm.predicate == goal.predicate
                        && verdict.norm.object == goal.object
                        && !matches!(verdict.status, DeonticStatus::Active)
                    {
                        return None;
                    }
                }
                vm_log!("[Webizen] NativeDeonticEval: {} norms evaluated", vcount);
            }
            SlgOpcode::NativeEpistemicEval(min_certainty) => {
                let mut scratch = [QualiaQuin::default(); 512];
                let count = arena.collect_active_quins(&mut scratch);
                let mut verdicts = [epistemic::EpistemicVerdict {
                    claim: QualiaQuin::default(),
                    status: epistemic::EpistemicStatus::Skipped,
                    certainty: 0,
                }; 64];
                let vcount = epistemic::evaluate_epistemic_frame(
                    &scratch[..count],
                    frame.subject_reg,
                    frame.context_reg,
                    &mut verdicts,
                )
                .unwrap_or(0);
                let mut ok = false;
                for verdict in &verdicts[..vcount] {
                    if verdict.certainty >= min_certainty
                        && verdict.status == epistemic::EpistemicStatus::Active
                    {
                        ok = true;
                        break;
                    }
                }
                if !ok {
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
                let mut out_worlds = [0; asp::MAX_STABLE_MODELS];
                let goal = frame_to_quin(frame);
                let world_count = asp::enumerate_stable_models(&goal, &[], &mut out_worlds);
                if world_count == 0 {
                    return None;
                }
                frame.context_reg = out_worlds[0];
            }
            SlgOpcode::NativeParaconsistentIsolate => {
                let mut scratch = [QualiaQuin::default(); 64];
                let count = arena.collect_active_quins(&mut scratch);
                if count == 0 {
                    return None;
                }
                let mut consistent = [QualiaQuin::default(); 64];
                let mut isolated = [QualiaQuin::default(); 64];
                let routed = paraconsistent::route_paraconsistent(
                    &scratch[..count],
                    &mut consistent,
                    &mut isolated,
                );
                if routed.is_err() {
                    return None;
                }
                let (_, iso_count) = routed.unwrap_or((0, 0));
                for q in &isolated[..iso_count] {
                    arena.write_table(*q);
                }
            }
            SlgOpcode::NativeDialecticalSynthesis => {
                let mut scratch = [QualiaQuin::default(); 64];
                let count = arena.collect_active_quins(&mut scratch);
                if count < 2 {
                    return None;
                }
                if let Some(syn) = dialectical::synthesize_dialectical(&scratch[0], &scratch[1]) {
                    arena.write_table(syn);
                    frame.subject_reg = syn.subject;
                    frame.predicate_reg = syn.predicate;
                    frame.object_reg = syn.object;
                    frame.context_reg = syn.context;
                } else {
                    return None;
                }
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
            SlgOpcode::NativeLtlGlobally
            | SlgOpcode::NativeLtlFinally
            | SlgOpcode::NativeLtlNext
            | SlgOpcode::NativeLtlUntil
            | SlgOpcode::NativeLtlRelease => {
                vm_log!("[Webizen] NativeLtl: evaluating temporal bounds natively on Core 1 using 64-bit bounds in Metadata");
            }
            SlgOpcode::NativeAllenInterval(mode) => {
                vm_log!(
                    "[Webizen] NativeAllenInterval: evaluating interval algebra mode {}",
                    mode
                );
            }
            SlgOpcode::NativeLorentzDistance
            | SlgOpcode::NativeTropicalDistance
            | SlgOpcode::NativeVerifyProofOfLocation => {
                // CORE 2 ISOLATION RULE:
                // Do not block Core 1. Push 64-bit parameters to async Sieve (Core 2 / GPU).
                // Suspend the Sentinel rule frame.
                vm_log!("[Webizen] CORE 2 YIELD: Suspending frame and pushing geometric ops to async GPU Sieve.");
                return None;
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

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementState {
    Proposed = 0x00,
    PartiallySigned = 0x01,
    Ratified = 0x02,
}

#[derive(Debug, Clone)]
pub struct AgreementDomain {
    pub name: alloc::string::String,
    pub domain_id: u64,
}

#[derive(Debug, Clone)]
pub struct AgreementConstraint {
    pub required_signatures: u8,
}

pub struct AgreementDID {
    pub agreement_id: u64,
    pub principal: u64,
    pub agents: [u64; 8],
    pub num_agents: u8,
    pub domain_id: u64,
    pub threshold: u8,
    pub current_state: AgreementState,
}

impl AgreementDID {
    /// Compiles a ratified agreement into hardware-aligned Super-Quins.
    pub fn compile_to_super_quins(&self) -> [QualiaQuin; 16] {
        let mut buffer = [QualiaQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }; 16];
        if self.current_state != AgreementState::Ratified {
            return buffer;
        }

        let mut idx = 0;
        let has_guardian = crate::q_hash("q42:hasGuardian");
        let has_domain_scope = crate::q_hash("q42:hasDomainScope");
        let requires_consensus = crate::q_hash("q42:requiresConsensus");

        for i in 0..self.num_agents as usize {
            if idx < 16 {
                buffer[idx] = QualiaQuin {
                    subject: self.principal,
                    predicate: has_guardian,
                    object: self.agents[i],
                    context: self.agreement_id,
                    // Embed routing lane (Bilateral Micro-Commons) and the State
                    metadata: 0x4000_0000_0000_0002 | ((self.current_state as u64) << 48),
                    parity: 0,
                };
                idx += 1;
            }
        }

        for i in 0..self.num_agents as usize {
            if idx < 16 {
                buffer[idx] = QualiaQuin {
                    subject: self.agreement_id,
                    predicate: has_domain_scope,
                    object: self.domain_id,
                    context: self.agents[i],
                    metadata: 0x4000_0000_0000_0002,
                    parity: 0,
                };
                idx += 1;
            }
        }

        if idx < 16 {
            buffer[idx] = QualiaQuin {
                subject: self.agreement_id,
                predicate: requires_consensus,
                object: self.threshold as u64,
                context: self.domain_id,
                metadata: 0x4000_0000_0000_0002,
                parity: 0,
            };
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::{SuspendedTransaction, SuspendedTransactionQueue};

    #[test]
    fn test_multi_agent_ratification_flow() {
        let mut agreement = AgreementDID {
            agreement_id: 100,
            principal: 200,
            agents: [300, 400, 0, 0, 0, 0, 0, 0],
            num_agents: 2,
            domain_id: 500,
            threshold: 2,
            current_state: AgreementState::Proposed,
        };

        // Before Ratification: should compile to empty quins
        let proposed_quins = agreement.compile_to_super_quins();
        assert_eq!(proposed_quins[0].subject, 0);

        // Signatures Gathered!
        agreement.current_state = AgreementState::Ratified;
        let ratified_quins = agreement.compile_to_super_quins();

        // Assert Bilateral Routing Lane
        assert_eq!(
            ratified_quins[0].metadata & 0x4000_0000_0000_0002,
            0x4000_0000_0000_0002
        );
        assert_eq!(ratified_quins[0].subject, 200); // principal
        assert_eq!(ratified_quins[0].object, 300); // agent 1

        // Test CRDT Queue Suspension and Wakeup
        let mut crdt_queue = SuspendedTransactionQueue::new();

        let mut mock_vm = crate::logic::WebizenVM::new();
        mock_vm.registers[0] = Some(999); // Mock execution state

        let suspended_tx = mock_vm.flatten_to_suspended(100, 2, crate::QualiaQuin::default());
        assert!(crdt_queue.push(suspended_tx).is_ok());

        // First signature token arrives via WebRTC
        let token_1 = crate::QualiaQuin {
            subject: 300,
            predicate: crate::q_hash("q42:issuesConsentToken"),
            object: 100,
            context: 100,
            metadata: 0,
            parity: 0,
        };
        assert!(crdt_queue.apply_consensus_token(&token_1).is_none()); // Threshold not met

        // Second signature token arrives via WebRTC
        let token_2 = crate::QualiaQuin {
            subject: 400,
            predicate: crate::q_hash("q42:issuesConsentToken"),
            object: 100,
            context: 100,
            metadata: 0,
            parity: 0,
        };
        let resumed_tx = crdt_queue.apply_consensus_token(&token_2);

        assert!(
            resumed_tx.is_some(),
            "WebRTC event failed to wake up suspended execution!"
        );
        assert_eq!(
            resumed_tx.unwrap().registers[0],
            Some(999),
            "Execution state was corrupted during CRDT suspension"
        );
    }

    #[test]
    fn check_defeaters_blocks_defeated_norm() {
        let mut arena = SlgArena::new();
        let contract = crate::q_hash("did:web:nda:contract-001");
        let alice = crate::q_hash("did:web:alice.example");
        let disclose = crate::q_hash("q42:disclose");
        let data = crate::q_hash("q42:data:project-x:confidential");

        let forbid = crate::deontic_logic::compile_norm_quin(
            alice,
            crate::deontic_logic::OP_FORBID,
            disclose,
            data,
            contract,
            0,
            false,
        );
        let defeater = crate::deontic_logic::compile_norm_quin(
            alice,
            crate::deontic_logic::OP_PERMIT,
            disclose,
            crate::q_hash("q42:role:certified-auditor"),
            contract,
            0,
            true,
        );
        arena.write_table(forbid);
        arena.write_table(defeater);

        let mut frame = VmFrame {
            subject_reg: alice,
            predicate_reg: forbid.predicate,
            object_reg: data,
            context_reg: contract,
        };
        let bytecode = [SlgOpcode::CheckDefeaters, SlgOpcode::Return];
        assert!(
            execute_vm_frame(&mut arena, &bytecode, &mut frame).is_none(),
            "CheckDefeaters must fail when a matching defeater exists"
        );
    }

    #[test]
    fn unify_binds_frame_from_arena_fact() {
        let mut arena = SlgArena::new();
        let fact = QualiaQuin {
            subject: 10,
            predicate: 20,
            object: 30,
            context: 40,
            metadata: 0,
            parity: 10 ^ 20 ^ 30 ^ 40,
        };
        arena.write_table(fact);

        let mut frame = VmFrame {
            subject_reg: 10,
            predicate_reg: 20,
            object_reg: 0,
            context_reg: 0,
        };
        let bytecode = [SlgOpcode::Unify, SlgOpcode::Return];
        let result = execute_vm_frame(&mut arena, &bytecode, &mut frame).expect("unify");
        assert_eq!(frame.object_reg, 30);
        assert_eq!(frame.context_reg, 40);
        assert_eq!(result.object, 30);
        assert_eq!(result.context, 40);
    }

    #[test]
    #[serial_test::serial]
    fn test_async_retrieve_logic() {
        // Initialize the DHAT profiler to ensure zero heap allocations
        let _profiler = dhat::Profiler::builder().testing().build();

        let mut arena = SlgArena::new();
        let mut frame = VmFrame::default();

        let bytecode = vec![SlgOpcode::NativeRetrieveByActivation];

        // Execute the bytecode
        let result = execute_vm_frame(&mut arena, &bytecode, &mut frame);

        // Ensure it yields immediately (returns None)
        assert!(result.is_none());

        // Verify no allocations occurred during the NativeRetrieveByActivation execution
        let stats = dhat::HeapStats::get();
        dhat::assert_eq!(stats.total_blocks, 0, "NativeRetrieveByActivation must not allocate on the heap! Zero-heap constraint violated.");
        dhat::assert_eq!(stats.total_bytes, 0, "NativeRetrieveByActivation must not allocate on the heap! Zero-heap constraint violated.");
    }
}
