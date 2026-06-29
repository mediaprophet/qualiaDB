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

#[cfg(test)]
mod tests {
    use super::*;

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
}
