/// WebizenVM — zero-allocation constraint engine for health policy enforcement.
///
/// Port of the Core 1 Webizen from qualia-core-db, extended with numeric comparison
/// opcodes needed by the N3 clinical rules and SHACL health constraints.
/// No wgpu or external deps — compiles in the default WASM binary.
///
/// Routing lanes match qualiaDB PermissiveRoutingLane:
///   0 = PassthroughStandard       — local queries, telemetry
///   1 = EnforcePermissiveCommons  — cooperative obligation gates
///   2 = EnforceBilateralMicroCommons — N3Logic / guardian rules (requires identity)
///   3 = SpatiotemporalAmbiguous   — GeoSPARQL / NPU path (future)

/// One instruction in the Webizen ISA.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebizenOpcode {
    /// Exact-match opcodes (match qualiaDB Core 1)
    MatchSubject(u64),
    MatchPredicate(u64),
    MatchObject(u64),
    EvalMetadataMask(u32),
    BindRegister { vector_id: u8, register_index: usize },
    MatchRegister { vector_id: u8, register_index: usize },
    HaltIfFalse,
    /// Numeric comparison opcodes (extension for N3Logic rules)
    /// Compare the float value in register[register_index] against threshold.
    LessThan    { register_index: usize, threshold: f64 },
    GreaterThan { register_index: usize, threshold: f64 },
    /// Load a literal f64 into a float register slot (slots 8-15).
    LoadFloat   { register_index: usize, value: f64 },
    /// Halt-and-pass if the condition is false (inverse of HaltIfFalse).
    HaltIfTrue,
}

/// Routing lane constants matching qualiaDB PermissiveRoutingLane.
pub const LANE_PASSTHROUGH:    u8 = 0;
pub const LANE_PERMISSIVE:     u8 = 1;
pub const LANE_BILATERAL:      u8 = 2;
pub const LANE_SPATIOTEMPORAL: u8 = 3;

/// A named constraint rule: bytecode + routing lane.
pub struct WebizenRule {
    pub name:         &'static str,
    pub routing_lane: u8,
    pub bytecode:     &'static [WebizenOpcode],
}

/// Zero-allocation VM: 16 u64 registers (0-7) + 16 f64 float slots (8-15 via float_regs).
pub struct WebizenVM {
    pub registers:   [Option<u64>; 16],
    pub float_regs:  [f64; 16],
    bytecode_buf:    [Option<WebizenOpcode>; 64],
}

impl WebizenVM {
    pub fn new() -> Self {
        Self {
            registers:  [None; 16],
            float_regs: [0.0; 16],
            bytecode_buf: [None; 64],
        }
    }

    pub fn load_bytecode(&mut self, ops: &[WebizenOpcode]) {
        for slot in self.bytecode_buf.iter_mut() { *slot = None; }
        for (i, &op) in ops.iter().enumerate().take(64) {
            self.bytecode_buf[i] = Some(op);
        }
    }

    /// Evaluate the loaded bytecode against a quin tuple (s,p,o,c,m).
    /// Returns `true` if all constraints pass.
    pub fn execute(&mut self, s: u64, p: u64, o: u64, c: u64, m: u64) -> bool {
        let mut flag = true;
        for op in self.bytecode_buf.iter().flatten().copied() {
            match op {
                WebizenOpcode::MatchSubject(v)   => flag = s == v,
                WebizenOpcode::MatchPredicate(v) => flag = p == v,
                WebizenOpcode::MatchObject(v)    => flag = o == v,
                WebizenOpcode::EvalMetadataMask(mask) => {
                    flag = ((m & 0xFFFF) as u32 & mask) == mask;
                }
                WebizenOpcode::BindRegister { vector_id, register_index } => {
                    let v = match vector_id { 0 => s, 1 => p, 2 => o, 3 => c, _ => 0 };
                    self.registers[register_index] = Some(v);
                    flag = true;
                }
                WebizenOpcode::MatchRegister { vector_id, register_index } => {
                    let v = match vector_id { 0 => s, 1 => p, 2 => o, 3 => c, _ => 0 };
                    flag = self.registers[register_index] == Some(v);
                }
                WebizenOpcode::LoadFloat { register_index, value } => {
                    self.float_regs[register_index] = value;
                    flag = true;
                }
                WebizenOpcode::LessThan { register_index, threshold } => {
                    flag = self.float_regs[register_index] < threshold;
                }
                WebizenOpcode::GreaterThan { register_index, threshold } => {
                    flag = self.float_regs[register_index] > threshold;
                }
                WebizenOpcode::HaltIfFalse => { if !flag { return false; } }
                WebizenOpcode::HaltIfTrue  => { if  flag { return true;  } }
            }
        }
        flag
    }

    /// Evaluate numeric thresholds only (no quin context needed).
    /// Loads `value` into float register 0 and runs comparison opcodes.
    pub fn check_threshold(&mut self, value: f64, ops: &[WebizenOpcode]) -> bool {
        self.float_regs[0] = value;
        self.load_bytecode(ops);
        self.execute(0, 0, 0, 0, 0)
    }
}

/// Pre-compiled SHACL access-profile constraints from shapes.rs, expressed as
/// WebizenOpcode bytecode for per-quin evaluation (W6 requirement).
///
/// These operate on Lexicon IDs — the JS Lexicon assigns u64 to each URI/literal.
/// For policy gates, we check the routing lane in the metadata field.
pub mod policy_rules {
    use super::*;

    // Routing lane 1 (PermissiveCommons) — cooperative work obligation satisfied gate.
    // Bytecode: EvalMetadataMask(MASK_WORK_OBLIGATION_SATISFIED = 0b0000_1000 = 8)
    pub const COOPERATIVE_OBLIGATION_GATE: &[WebizenOpcode] = &[
        WebizenOpcode::EvalMetadataMask(0x0008),
        WebizenOpcode::HaltIfFalse,
    ];

    // Routing lane 2 (BilateralMicroCommons) — guardian identity verified.
    // Bytecode: EvalMetadataMask(MASK_AUTHENTICATED_NATURAL_PERSON = 0b0000_0001 = 1)
    pub const GUARDIAN_IDENTITY_GATE: &[WebizenOpcode] = &[
        WebizenOpcode::EvalMetadataMask(0x0001),
        WebizenOpcode::HaltIfFalse,
    ];

    // Routing lane 2 — bilateral commercial gate blocked.
    // Bytecode: EvalMetadataMask(MASK_COMMERCIAL_BILLABLE_GATE = 0b0000_0100 = 4) → halt if set.
    pub const COMMERCIAL_BLOCK_GATE: &[WebizenOpcode] = &[
        WebizenOpcode::EvalMetadataMask(0x0004),
        WebizenOpcode::HaltIfTrue, // if commercial flag is set, reject
    ];
}

/// Evaluate a named policy constraint against a quin.
/// Returns `(passed: bool, lane: u8)`.
pub fn evaluate_policy_constraint(name: &str, s: u64, p: u64, o: u64, c: u64, m: u64) -> (bool, u8) {
    let mut vm = WebizenVM::new();
    let (ops, lane) = match name {
        "cooperative_obligation" => (policy_rules::COOPERATIVE_OBLIGATION_GATE, LANE_PERMISSIVE),
        "guardian_identity"      => (policy_rules::GUARDIAN_IDENTITY_GATE,      LANE_BILATERAL),
        "commercial_block"       => (policy_rules::COMMERCIAL_BLOCK_GATE,        LANE_BILATERAL),
        _ => return (true, LANE_PASSTHROUGH), // unknown constraint → passthrough
    };
    vm.load_bytecode(ops);
    (vm.execute(s, p, o, c, m), lane)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_comparison() {
        let mut vm = WebizenVM::new();
        // sleepHours < 6 — should pass for 5.5h
        let ops = &[
            WebizenOpcode::LessThan { register_index: 0, threshold: 6.0 },
            WebizenOpcode::HaltIfFalse,
        ];
        assert!(vm.check_threshold(5.5, ops), "5.5h should be < 6h");
        assert!(!vm.check_threshold(7.0, ops), "7h should not be < 6h");
    }

    #[test]
    fn test_policy_gate_passthrough() {
        // metadata = 0 → no obligation satisfied → gate should reject
        let (passed, lane) = evaluate_policy_constraint("cooperative_obligation", 0, 0, 0, 0, 0);
        assert!(!passed);
        assert_eq!(lane, LANE_PERMISSIVE);
    }

    #[test]
    fn test_policy_gate_satisfied() {
        // metadata with MASK_WORK_OBLIGATION_SATISFIED (bit 3) set
        let m = 0x0008u64;
        let (passed, lane) = evaluate_policy_constraint("cooperative_obligation", 0, 0, 0, 0, m);
        assert!(passed);
        assert_eq!(lane, LANE_PERMISSIVE);
    }
}
