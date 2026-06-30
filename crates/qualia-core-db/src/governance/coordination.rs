//! Multi-agent **coordination opcodes** — `MULTI_AGENT_PROTOCOL.md`, Sentinel bytecode
//! block **0x70–0x7F** (the deontic ISA owns 0x50–0x53; this block is collision-free).
//!
//! These govern the mechanical ingestion + evaluation of coordination nquins and execute
//! atomically in the Sentinel VM. This module is the **decidable core** of the three
//! opcodes — the expiry gate, the anti-usury resource contract + circuit breakers, and the
//! fidelity/efficiency arithmetic — as deterministic, zero-heap, tested functions. The
//! non-decidable substrate is passed in as a seam, never stubbed here:
//!
//! * `0x70` root-delegation **signature verification** → a `verify` closure the daemon
//!   backs with the key-vault Root Key (secure enclave);
//! * `0x71` **`SuspendedTransactionQueue`** yield → the caller acts on
//!   [`CoordFault::InsufficientGlobalResources`];
//! * `0x72` **VC minting** to the Semantic Shared Context Graph → the caller writes the
//!   [`PerformanceRecord`] as an nquin; the privileged (daemon-only) gate is the caller's.
//!
//! Wiring an operand-stack execution path into `webizen_bytecode` (today a per-quin
//! matcher) + the substrate above is the next increment; the semantics are fixed here.

use crate::modalities::value_flow::{check_usury, UsuryError, USURY_OVERAGE_PERCENT_DEFAULT};

/// `0x70` — verify cryptographic delegation Human-Root → ephemeral agent.
pub const OP_AUTHORIZATION_GRANT: u8 = 0x70;
/// `0x71` — declare the hard computational boundaries (anti-usury layer) for a task.
pub const OP_RESOURCE_DECLARATION: u8 = 0x71;
/// `0x72` — privileged: mint the performance VC at task resolution (Sentinel daemon only).
pub const OP_PERFORMANCE_RATING: u8 = 0x72;

/// Coordination-layer faults. Each dumps the current VM frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordFault {
    /// `0x70` `ERR_GRANT_EXPIRED` — the delegation's validity timestamp is in the past.
    GrantExpired { now: u64, valid_until: u64 },
    /// `0x70` `ERR_UNAUTHORIZED_ACTOR` — root-delegation signature verification failed.
    UnauthorizedActor { agent: u64 },
    /// `0x71` `ERR_INSUFFICIENT_GLOBAL_RESOURCES` — declared ceiling exceeds the daemon's
    /// current global allowance; the intent must yield to the `SuspendedTransactionQueue`.
    InsufficientGlobalResources { declared: u64, global_limit: u64 },
    /// `0x72` — invoked by a non-privileged (synthetic) actor; only the Sentinel daemon may.
    PrivilegeViolation,
}

/// **`0x70` OP_AUTHORIZATION_GRANT.** Stack: `[Agent_DID_Hash, Human_Root_DID_Hash,
/// Metadata_Timestamp]`. Pops the timestamp first (expiry gate), then verifies the
/// delegation signature against the active Root Key (the `verify_root_delegation` seam).
///
/// Returns `Ok(true)` when valid — the VM then pushes `1` and registers the agent's session
/// in the Sentinel context. A failure is a fault (push `0`, dump the frame): `GrantExpired`
/// if `current_epoch > metadata_timestamp`, else `UnauthorizedActor` if the signature is
/// invalid.
pub fn eval_authorization_grant(
    agent_did_hash: u64,
    human_root_did_hash: u64,
    metadata_timestamp: u64,
    current_epoch: u64,
    verify_root_delegation: impl FnOnce(u64, u64) -> bool,
) -> Result<bool, CoordFault> {
    if current_epoch > metadata_timestamp {
        return Err(CoordFault::GrantExpired {
            now: current_epoch,
            valid_until: metadata_timestamp,
        });
    }
    if verify_root_delegation(agent_did_hash, human_root_did_hash) {
        Ok(true)
    } else {
        Err(CoordFault::UnauthorizedActor {
            agent: agent_did_hash,
        })
    }
}

/// The live resource contract + hardware circuit breakers a granted task runs under
/// (established by `0x71`). Deterministic, zero-heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceContract {
    pub task_id_hash: u64,
    /// Declared token ceiling — the anti-usury bound for [`Self::burn_tokens`].
    pub token_ceiling: u64,
    /// Decrementing clock-cycle breaker.
    pub cycles_remaining: u64,
    /// Summation tracker of tokens burned so far.
    pub tokens_burned: u64,
}

impl ResourceContract {
    /// Charge `n` clock cycles against the decrementing breaker. Returns `false` (breaker
    /// tripped) when the budget would underflow — the caller must halt the task.
    #[must_use]
    pub fn tick_cycles(&mut self, n: u64) -> bool {
        match self.cycles_remaining.checked_sub(n) {
            Some(rem) => {
                self.cycles_remaining = rem;
                true
            }
            None => {
                self.cycles_remaining = 0;
                false
            }
        }
    }

    /// Add `n` to the token-burn tracker. `Err(UsuryError)` once the cumulative burn breaches
    /// the **usury ceiling** (declared ceiling + the default overage) — the anti-usury layer.
    pub fn burn_tokens(&mut self, n: u64) -> Result<(), UsuryError> {
        self.tokens_burned = self.tokens_burned.saturating_add(n);
        check_usury(
            self.tokens_burned,
            self.token_ceiling,
            USURY_OVERAGE_PERCENT_DEFAULT,
        )
    }
}

/// **`0x71` OP_RESOURCE_DECLARATION.** Stack: `[Task_ID_Hash, Token_Ceiling,
/// Max_Clock_Cycles]`. Allocates the declared bounds to a fresh [`ResourceContract`] and
/// arms its circuit breakers. Errors with `InsufficientGlobalResources` (→ yield to the
/// `SuspendedTransactionQueue`) when the declared ceiling exceeds the daemon's global
/// allowance.
pub fn eval_resource_declaration(
    task_id_hash: u64,
    token_ceiling: u64,
    max_clock_cycles: u64,
    global_token_limit: u64,
) -> Result<ResourceContract, CoordFault> {
    if token_ceiling > global_token_limit {
        return Err(CoordFault::InsufficientGlobalResources {
            declared: token_ceiling,
            global_limit: global_token_limit,
        });
    }
    Ok(ResourceContract {
        task_id_hash,
        token_ceiling,
        cycles_remaining: max_clock_cycles,
        tokens_burned: 0,
    })
}

/// The performance verdict minted by `0x72` — the inputs to the performance-VC nquin written
/// to the Semantic Shared Context Graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerformanceRecord {
    pub agent_did_hash: u64,
    /// `1` = task validated; `0` = validation failed (hallucination / semantic conflict).
    pub fidelity: u8,
    /// `(declared − actual) / declared` in basis points (signed). **Negative ⇒ usury**
    /// (over-burn) ⇒ severe reputation slashing.
    pub efficiency_bp: i64,
    /// The cumulative burn breached the declared token ceiling (the bright-line extraction
    /// event the Darwinian scheduler hard-quarantines).
    pub usurious: bool,
}

/// **`0x72` OP_PERFORMANCE_RATING** (privileged). Stack: `[Agent_DID_Hash, Declared_Tokens,
/// Actual_Tokens_Burned, Validation_Boolean]`. Computes fidelity + efficiency and flags
/// usury. The VM then mints the VC nquin and pushes its hash; the daemon-only privilege gate
/// is the caller's (`require_privileged`).
pub fn eval_performance_rating(
    agent_did_hash: u64,
    declared_tokens: u64,
    actual_tokens_burned: u64,
    validation_ok: bool,
) -> PerformanceRecord {
    let fidelity = u8::from(validation_ok);
    let efficiency_bp = if declared_tokens == 0 {
        0
    } else {
        ((declared_tokens as i128 - actual_tokens_burned as i128) * 10_000
            / declared_tokens as i128) as i64
    };
    let usurious = check_usury(
        actual_tokens_burned,
        declared_tokens,
        USURY_OVERAGE_PERCENT_DEFAULT,
    )
    .is_err();
    PerformanceRecord {
        agent_did_hash,
        fidelity,
        efficiency_bp,
        usurious,
    }
}

/// Privilege gate for `0x72`: only the Sentinel daemon may mint performance VCs.
pub fn require_privileged(is_sentinel_daemon: bool) -> Result<(), CoordFault> {
    if is_sentinel_daemon {
        Ok(())
    } else {
        Err(CoordFault::PrivilegeViolation)
    }
}

// ─── Darwinian compute-priority weighting (daemon_swarm.rs policy) ─────────────────
//
// Exponential decay on *windowed* fidelity faults (an honest single miss is forgiven and can
// recover above the floor); a usury event is a bright-line extraction act → immediate hard
// quarantine (priority 0). Deterministic integer math.

/// Full compute priority of an unfaulted agent.
pub const PRIORITY_BASE: u64 = 10_000;
/// Redemption floor — an honest agent decaying on fidelity faults never fully starves and can
/// climb back. (Usury bypasses the floor: it quarantines to 0.)
pub const PRIORITY_FLOOR: u64 = 100;

/// Darwinian compute priority from an agent's recent record: `usury_event` ⇒ `0` (hard
/// quarantine); otherwise `PRIORITY_BASE × (1/2)^windowed_faults`, clamped at `PRIORITY_FLOOR`.
pub fn compute_priority(windowed_faults: u32, usury_event: bool) -> u64 {
    if usury_event {
        return 0;
    }
    let mut p = PRIORITY_BASE;
    for _ in 0..windowed_faults {
        p /= 2;
    }
    p.max(PRIORITY_FLOOR)
}

// ─── Coordination operand-stack VM (ISA 0x70–0x7F) ────────────────────────────────
//
// `webizen_bytecode` is a per-quin matcher; the coordination ISA is instead a small
// **fixed-depth operand-stack machine**. A program PUSHes the opcode operands then executes
// 0x70 / 0x71 / 0x72 with the stack effects in MULTI_AGENT_PROTOCOL.md §5. Zero-heap (the
// stack is a fixed array), keeping the Sentinel VM's bounded discipline.

/// Operand-stack depth — fixed/bounded like the rest of the VM.
pub const COORD_STACK_DEPTH: usize = 16;
/// `0x7F` — push the next 8 little-endian bytes as a `u64` operand.
pub const OP_PUSH_U64: u8 = 0x7F;

/// Host-provided seams the coordination VM cannot decide itself (the daemon backs these).
pub struct CoordContext<V: Fn(u64, u64) -> bool> {
    /// Current epoch — the `0x70` expiry gate.
    pub current_epoch: u64,
    /// The daemon's global token allowance — the `0x71` admission check.
    pub global_token_limit: u64,
    /// Whether the caller is the privileged Sentinel daemon — gates `0x72`.
    pub is_sentinel_daemon: bool,
    /// Root-delegation signature verification against the enclave Root Key — `0x70`.
    pub verify_root_delegation: V,
}

/// What a coordination program produced (besides the operand stack).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoordOutcome {
    /// `0x70` result, if executed.
    pub granted: Option<bool>,
    /// `0x71` contract, if executed — the host arms the breakers / yields to the queue.
    pub contract: Option<ResourceContract>,
    /// `0x72` record, if executed — the host mints the VC nquin into the context graph.
    pub performance: Option<PerformanceRecord>,
    /// Top of the operand stack at halt (e.g. the minted VC hash), if any.
    pub stack_top: Option<u64>,
}

/// Coordination VM faults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordVmError {
    StackUnderflow,
    StackOverflow,
    /// Truncated operand or an unknown opcode.
    InvalidProgram,
    /// An opcode raised a coordination fault — the frame is dumped.
    Fault(CoordFault),
}

/// Deterministic VC identity for a minted performance record — the `nquin_hash` `0x72` pushes
/// to confirm minting. The host writes the full [`PerformanceRecord`] to the context graph.
pub fn perf_vc_hash(rec: &PerformanceRecord) -> u64 {
    crate::q_hash(&format!(
        "q42:perfVC:{}:{}:{}:{}",
        rec.agent_did_hash, rec.fidelity, rec.efficiency_bp, rec.usurious
    ))
}

/// Execute a coordination-ISA program against a fresh fixed-depth operand stack.
pub fn execute_coordination<V: Fn(u64, u64) -> bool>(
    program: &[u8],
    ctx: &CoordContext<V>,
) -> Result<CoordOutcome, CoordVmError> {
    let mut stack = [0u64; COORD_STACK_DEPTH];
    let mut sp = 0usize;
    let mut ip = 0usize;
    let mut outcome = CoordOutcome::default();

    while ip < program.len() {
        match program[ip] {
            OP_PUSH_U64 => {
                if ip + 9 > program.len() {
                    return Err(CoordVmError::InvalidProgram);
                }
                if sp >= COORD_STACK_DEPTH {
                    return Err(CoordVmError::StackOverflow);
                }
                let bytes: [u8; 8] = program[ip + 1..ip + 9]
                    .try_into()
                    .map_err(|_| CoordVmError::InvalidProgram)?;
                stack[sp] = u64::from_le_bytes(bytes);
                sp += 1;
                ip += 9;
            }
            OP_AUTHORIZATION_GRANT => {
                if sp < 3 {
                    return Err(CoordVmError::StackUnderflow);
                }
                // [Agent, Root, Timestamp] — Timestamp on top.
                let timestamp = stack[sp - 1];
                let root = stack[sp - 2];
                let agent = stack[sp - 3];
                sp -= 3;
                let granted = eval_authorization_grant(
                    agent,
                    root,
                    timestamp,
                    ctx.current_epoch,
                    &ctx.verify_root_delegation,
                )
                .map_err(CoordVmError::Fault)?;
                stack[sp] = u64::from(granted);
                sp += 1;
                outcome.granted = Some(granted);
                ip += 1;
            }
            OP_RESOURCE_DECLARATION => {
                if sp < 3 {
                    return Err(CoordVmError::StackUnderflow);
                }
                // [Task, Ceiling, MaxCycles] — MaxCycles on top.
                let max_cycles = stack[sp - 1];
                let ceiling = stack[sp - 2];
                let task = stack[sp - 3];
                sp -= 3;
                let contract =
                    eval_resource_declaration(task, ceiling, max_cycles, ctx.global_token_limit)
                        .map_err(CoordVmError::Fault)?;
                outcome.contract = Some(contract);
                ip += 1;
            }
            OP_PERFORMANCE_RATING => {
                require_privileged(ctx.is_sentinel_daemon).map_err(CoordVmError::Fault)?;
                if sp < 4 {
                    return Err(CoordVmError::StackUnderflow);
                }
                // [Agent, Declared, Actual, Validation] — Validation on top.
                let validation = stack[sp - 1] != 0;
                let actual = stack[sp - 2];
                let declared = stack[sp - 3];
                let agent = stack[sp - 4];
                sp -= 4;
                let rec = eval_performance_rating(agent, declared, actual, validation);
                if sp >= COORD_STACK_DEPTH {
                    return Err(CoordVmError::StackOverflow);
                }
                stack[sp] = perf_vc_hash(&rec);
                sp += 1;
                outcome.performance = Some(rec);
                ip += 1;
            }
            _ => return Err(CoordVmError::InvalidProgram),
        }
    }

    outcome.stack_top = (sp > 0).then(|| stack[sp - 1]);
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Append `OP_PUSH_U64 <v LE>` to a program.
    fn push(p: &mut Vec<u8>, v: u64) {
        p.push(OP_PUSH_U64);
        p.extend_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn grant_gate_checks_expiry_then_signature() {
        // Valid + in-window → granted.
        assert_eq!(
            eval_authorization_grant(0xA, 0xB0, 100, 50, |_, _| true),
            Ok(true)
        );
        // Expired (now > valid_until) before signature is even checked.
        assert_eq!(
            eval_authorization_grant(0xA, 0xB0, 100, 101, |_, _| panic!("must not verify when expired")),
            Err(CoordFault::GrantExpired { now: 101, valid_until: 100 })
        );
        // In-window but bad signature → unauthorized.
        assert_eq!(
            eval_authorization_grant(0xA, 0xB0, 100, 50, |_, _| false),
            Err(CoordFault::UnauthorizedActor { agent: 0xA })
        );
    }

    #[test]
    fn resource_declaration_and_circuit_breakers() {
        // Over the global allowance → yield to the suspended queue.
        assert_eq!(
            eval_resource_declaration(7, 5000, 1000, 4000),
            Err(CoordFault::InsufficientGlobalResources { declared: 5000, global_limit: 4000 })
        );
        let mut c = eval_resource_declaration(7, 1000, 10, 4000).unwrap();
        // Cycle breaker decrements and trips on underflow.
        assert!(c.tick_cycles(6));
        assert_eq!(c.cycles_remaining, 4);
        assert!(!c.tick_cycles(5), "underflow trips the breaker");
        assert_eq!(c.cycles_remaining, 0);
        // Token burn is fine up to the 110% usury ceiling (1100), usurious past it.
        assert!(c.burn_tokens(1000).is_ok());
        assert!(c.burn_tokens(100).is_ok()); // exactly at ceiling
        assert!(c.burn_tokens(1).is_err(), "1101 > 1100 ceiling ⇒ usury");
    }

    #[test]
    fn performance_rating_computes_fidelity_efficiency_usury() {
        // Validated, under budget → fidelity 1, positive efficiency, not usurious.
        let good = eval_performance_rating(0xA, 1000, 800, true);
        assert_eq!(good.fidelity, 1);
        assert_eq!(good.efficiency_bp, 2000); // (1000-800)/1000 = +20% = 2000 bp
        assert!(!good.usurious);
        // Hallucination → fidelity 0.
        assert_eq!(eval_performance_rating(0xA, 1000, 900, false).fidelity, 0);
        // Over-burn → negative efficiency + usurious.
        let bad = eval_performance_rating(0xA, 1000, 1300, true);
        assert_eq!(bad.efficiency_bp, -3000); // (1000-1300)/1000 = -30%
        assert!(bad.usurious);
    }

    #[test]
    fn privilege_gate_blocks_synthetic_agents() {
        assert_eq!(require_privileged(true), Ok(()));
        assert_eq!(require_privileged(false), Err(CoordFault::PrivilegeViolation));
    }

    #[test]
    fn darwinian_priority_forgives_mistakes_quarantines_extraction() {
        // No faults → full priority.
        assert_eq!(compute_priority(0, false), PRIORITY_BASE);
        // Honest single miss → halved, not killed (recoverable).
        assert_eq!(compute_priority(1, false), 5000);
        assert_eq!(compute_priority(2, false), 2500);
        // Sustained failure decays toward — but never below — the redemption floor.
        assert_eq!(compute_priority(20, false), PRIORITY_FLOOR);
        // Usury is the bright line: immediate hard quarantine regardless of fault count.
        assert_eq!(compute_priority(0, true), 0);
        assert_eq!(compute_priority(1, true), 0);
    }

    #[test]
    fn coordination_vm_executes_grant_program() {
        // PUSH agent, PUSH root, PUSH timestamp(100), GRANT — epoch 50, valid signature.
        let mut prog = Vec::new();
        push(&mut prog, 0xA);
        push(&mut prog, 0xB0);
        push(&mut prog, 100);
        prog.push(OP_AUTHORIZATION_GRANT);
        let ctx = CoordContext {
            current_epoch: 50,
            global_token_limit: 10_000,
            is_sentinel_daemon: false,
            verify_root_delegation: |_a, _r| true,
        };
        let out = execute_coordination(&prog, &ctx).unwrap();
        assert_eq!(out.granted, Some(true));
        assert_eq!(out.stack_top, Some(1)); // pushed 1 (True)

        // Expired epoch → GrantExpired fault, signature never consulted.
        let ctx_exp = CoordContext {
            current_epoch: 101,
            global_token_limit: 10_000,
            is_sentinel_daemon: false,
            verify_root_delegation: |_a, _r| panic!("must not verify when expired"),
        };
        assert_eq!(
            execute_coordination(&prog, &ctx_exp),
            Err(CoordVmError::Fault(CoordFault::GrantExpired { now: 101, valid_until: 100 }))
        );

        // Bad signature → UnauthorizedActor.
        let ctx_bad = CoordContext {
            current_epoch: 50,
            global_token_limit: 10_000,
            is_sentinel_daemon: false,
            verify_root_delegation: |_a, _r| false,
        };
        assert_eq!(
            execute_coordination(&prog, &ctx_bad),
            Err(CoordVmError::Fault(CoordFault::UnauthorizedActor { agent: 0xA }))
        );
    }

    #[test]
    fn coordination_vm_executes_resource_and_performance_programs() {
        let ctx = CoordContext {
            current_epoch: 0,
            global_token_limit: 4000,
            is_sentinel_daemon: true,
            verify_root_delegation: |_a, _r| true,
        };
        // RESOURCE_DECLARATION: PUSH task, ceiling(1000), max_cycles(50), DECLARE.
        let mut prog = Vec::new();
        push(&mut prog, 7);
        push(&mut prog, 1000);
        push(&mut prog, 50);
        prog.push(OP_RESOURCE_DECLARATION);
        let c = execute_coordination(&prog, &ctx).unwrap().contract.unwrap();
        assert_eq!(c.token_ceiling, 1000);
        assert_eq!(c.cycles_remaining, 50);

        // Over the global allowance → InsufficientGlobalResources (→ suspended queue).
        let mut prog2 = Vec::new();
        push(&mut prog2, 7);
        push(&mut prog2, 5000);
        push(&mut prog2, 50);
        prog2.push(OP_RESOURCE_DECLARATION);
        assert_eq!(
            execute_coordination(&prog2, &ctx),
            Err(CoordVmError::Fault(CoordFault::InsufficientGlobalResources {
                declared: 5000,
                global_limit: 4000
            }))
        );

        // PERFORMANCE_RATING (privileged): PUSH agent, declared(1000), actual(800), valid(1), RATE.
        let mut prog3 = Vec::new();
        push(&mut prog3, 0xA);
        push(&mut prog3, 1000);
        push(&mut prog3, 800);
        push(&mut prog3, 1);
        prog3.push(OP_PERFORMANCE_RATING);
        let out3 = execute_coordination(&prog3, &ctx).unwrap();
        let rec = out3.performance.unwrap();
        assert_eq!(rec.fidelity, 1);
        assert_eq!(rec.efficiency_bp, 2000);
        assert_eq!(out3.stack_top, Some(perf_vc_hash(&rec))); // minted-VC hash pushed

        // A non-privileged caller cannot mint → PrivilegeViolation.
        let ctx_np = CoordContext {
            current_epoch: 0,
            global_token_limit: 4000,
            is_sentinel_daemon: false,
            verify_root_delegation: |_a, _r| true,
        };
        assert_eq!(
            execute_coordination(&prog3, &ctx_np),
            Err(CoordVmError::Fault(CoordFault::PrivilegeViolation))
        );
    }

    #[test]
    fn coordination_vm_guards_stack_bounds() {
        let ctx = CoordContext {
            current_epoch: 0,
            global_token_limit: 4000,
            is_sentinel_daemon: true,
            verify_root_delegation: |_a, _r| true,
        };
        // GRANT with too few operands → underflow (frame dumped, not silent).
        let mut prog = Vec::new();
        push(&mut prog, 1);
        prog.push(OP_AUTHORIZATION_GRANT);
        assert_eq!(execute_coordination(&prog, &ctx), Err(CoordVmError::StackUnderflow));
        // Truncated PUSH operand → invalid program.
        assert_eq!(
            execute_coordination(&[OP_PUSH_U64, 1, 2, 3], &ctx),
            Err(CoordVmError::InvalidProgram)
        );
    }
}
