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
    /// Linear-logic **zero-knowledge–gated** resource exhaustion: like `ConsumeFact`, but the
    /// cryptographic-token resource `(subject_reg, predicate_reg, object_reg)` is spent **only**
    /// if a verified zk-entitlement marker `(subject_reg, q42:zkVerified, q42:true)` is live in
    /// the arena. Absent the proof, or if the token is already spent, the frame fails (`None`).
    /// Delegates to `modalities::linear::zk_gated_consume`.
    ZkConsumeFact,
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
    /// OR-branch: records a passing `rdf:type` match without failing the frame.
    SoftCheckNodeShape(u64),
    /// Fails unless at least one preceding `SoftCheckNodeShape` matched.
    RequireAnyShape,
    /// Validates inline literal tag on `object_reg` (0=IRI, 1=int, 2=decimal, 3=bool).
    CheckObjectDatatype(u8),

    // ── Native: physics ───────────────────────────────────────────────────────
    NativeThermodynamics,
    NativeOdeSolver,
    /// RK4 ODE time-stepper with chaining support
    /// Parameters packed: step_size (lower 32 bits) | num_steps (upper 32 bits)
    NativeRk4Step(u64),
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
    /// Probabilistic gate: the goal quin's f32 belief weight (in `metadata`) must
    /// be ≥ the threshold (param = `f32::to_bits`).
    NativeProbabilisticThreshold(u32),
    /// Description-logic gate: `frame.subject_reg ⊑ frame.object_reg` over the arena
    /// TBox (transitive `rdfs:subClassOf` closure).
    NativeDlSubsumption,
    /// Argumentation gate (Dung grounded semantics): the goal argument
    /// (`frame.subject_reg`) must be justified — in the grounded extension built
    /// from `arg:asserts` / `arg:attacks` quins in the arena.
    NativeArgumentationGrounded,
    /// Metric-temporal gate (MTL "within"): `target` (`frame.object_reg`) must
    /// occur within the param window of the earliest `trigger` (`frame.predicate_reg`);
    /// event timestamps are in each quin's `metadata`.
    NativeMtlWithin(u32),
    /// Contrary-to-duty gate (dyadic deontic): if the party (`frame.subject_reg`)
    /// breached the primary obligation (`frame.predicate_reg`), the reparation
    /// (`frame.object_reg`) must be fulfilled.
    NativeContraryToDuty,
    /// Causal-necessity gate (but-for): `frame.subject_reg` must be a necessary
    /// cause of `frame.object_reg` from origin `frame.context_reg`.
    NativeCausalNecessary,
    /// Abductive gate: the observation (`frame.object_reg`) must have an explanatory
    /// hypothesis (backward `abduces:explains` chain) in the arena.
    NativeAbduce,
    /// Closed-world / negation-as-failure gate: passes iff the frame goal is ABSENT
    /// from the arena (its negation holds by default).
    NativeClosedWorld,
    /// Fuzzy-conjunction gate: the Gödel t-norm (min) of the truth degrees of all
    /// quins with predicate `frame.predicate_reg` must be ≥ the param threshold.
    NativeFuzzyConjunction(u32),
    /// CTL EF gate: from `frame.subject_reg`, SOME path reaches a state satisfying
    /// `frame.object_reg` (branching-time reachability over `ctl:next`/`ctl:holds`).
    NativeCtlExistsFinally,
    /// CTL AG gate: EVERY state reachable from `frame.subject_reg` satisfies the
    /// invariant `frame.object_reg`.
    NativeCtlAlwaysGlobally,
    /// Modal □ gate: `frame.object_reg` holds in ALL worlds accessible from
    /// `frame.subject_reg` (`modal:accesses`/`modal:holds`).
    NativeModalNecessary,
    /// Modal ◇ gate: `frame.object_reg` holds in SOME world accessible from
    /// `frame.subject_reg`.
    NativeModalPossible,
    /// RCC-8 spatial gate: the topological relation between region `frame.subject_reg`
    /// and region `frame.object_reg` (each = boundary-point quins) must equal the
    /// param (0=DC,1=EC,2=PO,3=TPP,4=TPPi,5=NTPP,6=NTPPi,7=EQ).
    NativeRcc8(u8),
    /// RCC-8 fact-emitting assertion: if the relation holds, asserts a derived fact.
    NativeRcc8Assert(u8),
    /// Fact-emitting temporal interval relation assertion.
    NativeAllenIntervalAssert(u8),
    /// qapp-facing dynamic N3 rule registration
    NativeRegisterRule,
    /// M-of-N steward quorum gate
    NativeStewardQuorum(u8),
    /// Governed canvas placement gate
    NativeCanvasPlacement,

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
    /// Evaluate a threshold proposition projected from chronological 10D
    /// manifold states through the existing LTL evaluator.
    ///
    /// `mode`: 0=Globally, 1=Finally, 2=Next.
    /// `dimension`: [`manifold::ManifoldDimension`] discriminant.
    /// `threshold_bits`: IEEE-754 f32 threshold.
    /// `at_least`: true for >=, false for <=.
    NativeManifoldLtl {
        mode: u8,
        dimension: u8,
        threshold_bits: u32,
        at_least: bool,
    },
    /// Derive topology facts from 10D states and execute the bounded real ASP
    /// stable-model evaluator. The selected model bitset is bound to object_reg.
    NativeManifoldAsp,

    // ── Native: spatio-temporal (Allen Interval) ──────────────────────────────
    NativeAllenInterval(u8),

    // ── Native: geometric and spatial topology ────────────────────────────────
    NativeLorentzDistance,
    NativeTropicalDistance,
    NativeVerifyProofOfLocation,

    // ── Native: calculus modality ──────────────────────────────────────────────
    /// `calc:SimpsonsIntegration` — CPU-based Simpson's rule (start_bits, end_bits, step_size_bits, kahan_bits)
    NativeCalcSimpsons(u64, u64, u32, u32),
    /// `calc:TrapezoidalIntegration` — CPU-based trapezoidal rule (start_bits, end_bits, step_size_bits, kahan_bits)
    NativeCalcTrapezoidal(u64, u64, u32, u32),
    /// `calc:GpuIntegration` — GPU-accelerated integration via WebGPU (start_bits, end_bits, step_size_bits, kahan_bits)
    NativeCalcGpu(u64, u64, u32, u32),
}
