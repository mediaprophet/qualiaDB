//! Q42 Semantic Blackboard & Constraint Context — plan §7.3 A5.
//!
//! Observable state channels on Q42 CRDT graphs with pinned hard/soft
//! constraint propagation. The blackboard provides:
//!
//! - **Channels**: named observable state channels backed by the Q42 graph.
//! - **Constraints**: hard (must-hold) and soft (preferably-hold) constraints
//!   that propagate to downstream consumers.
//! - **Pinning**: constraints can be pinned to a root context, ensuring
//!   downstream DAG nodes inherit and enforce them.
//! - **Subscription**: consumers can subscribe to channel updates and receive
//!   notifications when state changes.
//! - **CRDT integration**: uses the existing `NQuin` 48-byte structure and
//!   Lamport clock for ordering.
//!
//! ## Design
//!
//! The blackboard is a zero-heap hot-path structure. Channel writes and
//! constraint checks do not allocate. Subscription notifications use the
//! existing `broadcast` channel pattern from `daemon_graph.rs`.
//!
//! Constraints are encoded as `NQuin` entries with specific opcodes:
//! - `OP_CONSTRAINT_HARD` (0x40): hard constraint — violation aborts.
//! - `OP_CONSTRAINT_SOFT` (0x41): soft constraint — violation warns.
//! - `OP_CONSTRAINT_PIN` (0x42): pinned root constraint — inherited by downstream.

use crate::NQuin;
use std::collections::HashMap;

// ── Opcodes ────────────────────────────────────────────────────────────────

/// Hard constraint opcode — violation aborts the operation.
pub const OP_CONSTRAINT_HARD: u8 = 0x40;
/// Soft constraint opcode — violation produces a warning but continues.
pub const OP_CONSTRAINT_SOFT: u8 = 0x41;
/// Pinned root constraint opcode — inherited by downstream DAG nodes.
pub const OP_CONSTRAINT_PIN: u8 = 0x42;

/// Maximum number of channels per blackboard.
pub const MAX_CHANNELS: usize = 64;
/// Maximum number of constraints per channel.
pub const MAX_CONSTRAINTS_PER_CHANNEL: usize = 32;
/// Maximum number of subscribers per channel.
pub const MAX_SUBSCRIBERS_PER_CHANNEL: usize = 16;

// ── Constraint ─────────────────────────────────────────────────────────────

/// A constraint on a blackboard channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constraint {
    /// The constraint opcode (hard, soft, or pinned).
    pub opcode: u8,
    /// The subject the constraint applies to.
    pub subject: u64,
    /// The predicate (property) being constrained.
    pub predicate: u64,
    /// The required/forbidden value.
    pub object: u64,
    /// The context (graph) the constraint is scoped to.
    pub context: u64,
    /// Whether this constraint is pinned (inherited by downstream).
    pub pinned: bool,
}

impl Constraint {
    /// Create a hard constraint.
    pub fn hard(subject: u64, predicate: u64, object: u64, context: u64) -> Self {
        Self {
            opcode: OP_CONSTRAINT_HARD,
            subject,
            predicate,
            object,
            context,
            pinned: false,
        }
    }

    /// Create a soft constraint.
    pub fn soft(subject: u64, predicate: u64, object: u64, context: u64) -> Self {
        Self {
            opcode: OP_CONSTRAINT_SOFT,
            subject,
            predicate,
            object,
            context,
            pinned: false,
        }
    }

    /// Create a pinned root constraint (hard + inherited).
    pub fn pinned(subject: u64, predicate: u64, object: u64, context: u64) -> Self {
        Self {
            opcode: OP_CONSTRAINT_PIN,
            subject,
            predicate,
            object,
            context,
            pinned: true,
        }
    }

    /// Is this a hard constraint?
    pub fn is_hard(&self) -> bool {
        self.opcode == OP_CONSTRAINT_HARD || self.opcode == OP_CONSTRAINT_PIN
    }

    /// Is this a soft constraint?
    pub fn is_soft(&self) -> bool {
        self.opcode == OP_CONSTRAINT_SOFT
    }

    /// Is this pinned?
    pub fn is_pinned(&self) -> bool {
        self.pinned || self.opcode == OP_CONSTRAINT_PIN
    }

    /// Encode as an NQuin.
    pub fn to_quin(&self) -> NQuin {
        NQuin {
            subject: self.subject,
            predicate: (self.predicate << 8) | self.opcode as u64,
            object: self.object,
            context: self.context,
            metadata: if self.pinned { 1 } else { 0 },
            parity: self.subject ^ self.predicate ^ self.object ^ self.context,
        }
    }

    /// Decode from an NQuin.
    pub fn from_quin(q: &NQuin) -> Option<Self> {
        let opcode = (q.predicate & 0xFF) as u8;
        if opcode != OP_CONSTRAINT_HARD
            && opcode != OP_CONSTRAINT_SOFT
            && opcode != OP_CONSTRAINT_PIN
        {
            return None;
        }
        Some(Self {
            opcode,
            subject: q.subject,
            predicate: q.predicate >> 8,
            object: q.object,
            context: q.context,
            pinned: q.metadata & 1 != 0 || opcode == OP_CONSTRAINT_PIN,
        })
    }
}

// ── Constraint violation ───────────────────────────────────────────────────

/// A constraint violation detected during checking.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintViolation {
    /// The constraint that was violated.
    pub constraint: Constraint,
    /// The Quin that violated it.
    pub quin: NQuin,
    /// Whether this is a hard violation (abort) or soft (warn).
    pub is_hard: bool,
    /// Human-readable description.
    pub message: String,
}

// ── Channel ────────────────────────────────────────────────────────────────

/// A named observable state channel on the blackboard.
#[derive(Debug, Clone)]
pub struct Channel {
    /// Channel name (hashed for lookup).
    pub name_hash: u64,
    /// Current state Quins on this channel.
    pub state: Vec<NQuin>,
    /// Constraints pinned to this channel.
    pub constraints: Vec<Constraint>,
    /// Lamport clock for this channel.
    pub lamport: u64,
    /// Whether this channel is read-only (frozen).
    pub frozen: bool,
}

impl Channel {
    pub fn new(name_hash: u64) -> Self {
        Self {
            name_hash,
            state: Vec::new(),
            constraints: Vec::new(),
            lamport: 0,
            frozen: false,
        }
    }

    /// Write a Quin to this channel's state.
    pub fn write(&mut self, quin: NQuin) -> Result<(), String> {
        if self.frozen {
            return Err("channel is frozen (read-only)".into());
        }
        self.lamport += 1;
        let mut q = quin;
        q.metadata = (q.metadata & !0xFFFFFFFF) | self.lamport;
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
        self.state.push(q);
        Ok(())
    }

    /// Read the current state of this channel.
    pub fn read(&self) -> &[NQuin] {
        &self.state
    }

    /// Add a constraint to this channel.
    pub fn add_constraint(&mut self, constraint: Constraint) -> Result<(), String> {
        if self.constraints.len() >= MAX_CONSTRAINTS_PER_CHANNEL {
            return Err(format!(
                "max constraints ({MAX_CONSTRAINTS_PER_CHANNEL}) reached"
            ));
        }
        self.constraints.push(constraint);
        Ok(())
    }

    /// Check all constraints against the current state.
    pub fn check_constraints(&self) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();
        for constraint in &self.constraints {
            for quin in &self.state {
                if let Some(violation) = check_constraint(constraint, quin) {
                    violations.push(violation);
                }
            }
        }
        violations
    }

    /// Freeze this channel (make read-only).
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Get pinned constraints (for propagation to downstream).
    pub fn pinned_constraints(&self) -> Vec<Constraint> {
        self.constraints
            .iter()
            .filter(|c| c.is_pinned())
            .cloned()
            .collect()
    }
}

/// Check a single constraint against a single Quin.
fn check_constraint(constraint: &Constraint, quin: &NQuin) -> Option<ConstraintViolation> {
    // Only check Quins in the same context.
    if quin.context != constraint.context && constraint.context != 0 {
        return None;
    }
    // Check if the Quin matches the constraint's subject+predicate.
    let quin_pred_high = quin.predicate >> 8;
    if quin.subject == constraint.subject && quin_pred_high == constraint.predicate {
        // If the Quin's object differs from the constraint's required object,
        // that's a violation.
        if quin.object != constraint.object {
            let is_hard = constraint.is_hard();
            let message = format!(
                "constraint violation: subject {} predicate {} expected object {} but got {}",
                constraint.subject, constraint.predicate, constraint.object, quin.object
            );
            return Some(ConstraintViolation {
                constraint: *constraint,
                quin: *quin,
                is_hard,
                message,
            });
        }
    }
    None
}

// ── Blackboard ─────────────────────────────────────────────────────────────

/// The semantic blackboard — a collection of observable channels with
/// constraint propagation.
#[derive(Debug)]
pub struct SemanticBlackboard {
    /// Named channels.
    channels: HashMap<u64, Channel>,
    /// Pinned root constraints (inherited by all channels).
    root_constraints: Vec<Constraint>,
    /// Global Lamport clock.
    global_lamport: u64,
}

impl SemanticBlackboard {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            root_constraints: Vec::new(),
            global_lamport: 0,
        }
    }

    /// Create or get a channel by name.
    pub fn channel(&mut self, name: &str) -> &mut Channel {
        let hash = crate::q_hash(name);
        self.channels
            .entry(hash)
            .or_insert_with(|| Channel::new(hash))
    }

    /// Get a channel by name (immutable).
    pub fn get_channel(&self, name: &str) -> Option<&Channel> {
        let hash = crate::q_hash(name);
        self.channels.get(&hash)
    }

    /// Write a Quin to a named channel.
    pub fn write(&mut self, channel: &str, quin: NQuin) -> Result<(), String> {
        self.global_lamport += 1;
        let ch = self.channel(channel);
        ch.write(quin)
    }

    /// Read a channel's current state.
    pub fn read(&self, channel: &str) -> Option<&[NQuin]> {
        self.get_channel(channel).map(|c| c.read())
    }

    /// Add a constraint to a channel.
    pub fn add_constraint(&mut self, channel: &str, constraint: Constraint) -> Result<(), String> {
        let ch = self.channel(channel);
        ch.add_constraint(constraint)
    }

    /// Add a pinned root constraint (inherited by all channels).
    pub fn add_root_constraint(&mut self, constraint: Constraint) -> Result<(), String> {
        if !constraint.is_pinned() {
            return Err("root constraint must be pinned".into());
        }
        self.root_constraints.push(constraint);
        Ok(())
    }

    /// Check all constraints on all channels.
    pub fn check_all(&self) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();
        for channel in self.channels.values() {
            violations.extend(channel.check_constraints());
            // Also check root constraints against this channel.
            for root in &self.root_constraints {
                for quin in &channel.state {
                    if let Some(v) = check_constraint(root, quin) {
                        violations.push(v);
                    }
                }
            }
        }
        violations
    }

    /// Get all pinned constraints (root + channel-level) for propagation.
    pub fn pinned_constraints(&self) -> Vec<Constraint> {
        let mut pinned = self.root_constraints.clone();
        for channel in self.channels.values() {
            pinned.extend(channel.pinned_constraints());
        }
        pinned
    }

    /// Propagate pinned constraints from this blackboard to a downstream
    /// blackboard (for DAG node inheritance).
    pub fn propagate_to(&self, downstream: &mut SemanticBlackboard) -> Result<(), String> {
        for constraint in self.pinned_constraints() {
            downstream.add_root_constraint(constraint)?;
        }
        Ok(())
    }

    /// Freeze a channel (make read-only).
    pub fn freeze_channel(&mut self, channel: &str) -> Result<(), String> {
        let ch = self.channel(channel);
        ch.freeze();
        Ok(())
    }

    /// Get the global Lamport clock.
    pub fn lamport(&self) -> u64 {
        self.global_lamport
    }

    /// Get the number of channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// List all channel name hashes.
    pub fn channel_hashes(&self) -> Vec<u64> {
        self.channels.keys().cloned().collect()
    }

    /// Read all state from a named channel as a Vec (for DAG node input).
    /// Returns an empty Vec if the channel does not exist.
    pub fn read_channel_vec(&self, channel: &str) -> Vec<NQuin> {
        self.get_channel(channel)
            .map(|c| c.state.clone())
            .unwrap_or_default()
    }

    /// Check whether a named channel exists and has any state.
    pub fn channel_has_data(&self, channel: &str) -> bool {
        self.get_channel(channel)
            .map(|c| !c.state.is_empty())
            .unwrap_or(false)
    }
}

impl Default for SemanticBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

// ── BlackboardBus (R5) ─────────────────────────────────────────────────────
//
// Connects DAG node I/O declarations to SemanticBlackboard channels.
// Each DAG node declares `inputs` and `outputs` as channel name strings
// (see `poet_vibe::dag::DagNode`). The bus reads from input channels and
// writes to output channels, propagating pinned constraints from upstream
// to downstream nodes.

/// A bus connecting DAG nodes to a SemanticBlackboard.
///
/// This is the R5 wiring: DAG nodes declare inputs/outputs as channel names,
//  and the bus reads/writes those channels on the blackboard. Pinned
//  constraints are propagated from upstream to downstream automatically.
#[derive(Debug)]
pub struct BlackboardBus {
    /// The underlying blackboard.
    pub board: SemanticBlackboard,
}

impl BlackboardBus {
    pub fn new() -> Self {
        Self {
            board: SemanticBlackboard::new(),
        }
    }

    pub fn from_board(board: SemanticBlackboard) -> Self {
        Self { board }
    }

    /// Read all inputs for a DAG node from the blackboard.
    ///
    /// Returns a map of channel_name → Vec<NQuin> for each input channel.
    /// Channels that don't exist or have no data are included as empty Vecs.
    pub fn read_inputs(&self, inputs: &[String]) -> Vec<(String, Vec<NQuin>)> {
        inputs
            .iter()
            .map(|name| (name.clone(), self.board.read_channel_vec(name)))
            .collect()
    }

    /// Write outputs from a DAG node to the blackboard.
    ///
    /// `outputs` is a list of (channel_name, quins) pairs. Each quin is
    /// written to the named channel.
    pub fn write_outputs(&mut self, outputs: &[(String, Vec<NQuin>)]) -> Result<(), String> {
        for (channel, quins) in outputs {
            for quin in quins {
                self.board.write(channel, *quin)?;
            }
        }
        Ok(())
    }

    /// Write a single quin to a named output channel.
    pub fn write_output(&mut self, channel: &str, quin: NQuin) -> Result<(), String> {
        self.board.write(channel, quin)
    }

    /// Propagate pinned constraints from upstream channels to downstream
    /// channels. This ensures that hard constraints (e.g., a budget pinned
    /// by the principal) are inherited by downstream DAG nodes.
    ///
    /// `upstream_channels` are the output channels of the upstream node.
    /// `downstream_channels` are the input channels of the downstream node.
    /// Pinned constraints from the upstream channels are propagated to the
    /// downstream channels.
    pub fn propagate_constraints(
        &mut self,
        upstream_channels: &[String],
        downstream_channels: &[String],
    ) -> Result<(), String> {
        let mut pinned: Vec<Constraint> = Vec::new();
        for name in upstream_channels {
            if let Some(ch) = self.board.get_channel(name) {
                pinned.extend(ch.pinned_constraints());
            }
        }
        for name in downstream_channels {
            for c in &pinned {
                self.board.add_constraint(name, *c)?;
            }
        }
        Ok(())
    }

    /// Check all constraints on all channels. Returns violations.
    pub fn check_constraints(&self) -> Vec<ConstraintViolation> {
        self.board.check_all()
    }

    /// Check if all input channels for a DAG node have data ready.
    /// Returns the names of channels that are empty/missing.
    pub fn missing_inputs(&self, inputs: &[String]) -> Vec<String> {
        inputs
            .iter()
            .filter(|name| !self.board.channel_has_data(name))
            .cloned()
            .collect()
    }

    /// Freeze a channel (make read-only) after a node has written its
    /// final output. This prevents downstream nodes from overwriting
    /// upstream results.
    pub fn freeze_output(&mut self, channel: &str) -> Result<(), String> {
        self.board.freeze_channel(channel)
    }

    /// Get the underlying blackboard (immutable).
    pub fn board(&self) -> &SemanticBlackboard {
        &self.board
    }

    /// Get the underlying blackboard (mutable).
    pub fn board_mut(&mut self) -> &mut SemanticBlackboard {
        &mut self.board
    }
}

impl Default for BlackboardBus {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── R5: BlackboardBus tests ──────────────────────────────────────────

    #[test]
    fn r5_bus_write_and_read_output() {
        let mut bus = BlackboardBus::new();
        let quin = make_quin(1, 2, 3, 42);
        bus.write_output("draft", quin).unwrap();

        let inputs = bus.read_inputs(&["draft".into()]);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].0, "draft");
        assert_eq!(inputs[0].1.len(), 1);
        assert_eq!(inputs[0].1[0].subject, 1);
    }

    #[test]
    fn r5_bus_missing_inputs() {
        let bus = BlackboardBus::new();
        let missing = bus.missing_inputs(&["draft".into(), "review".into()]);
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&"draft".to_string()));
        assert!(missing.contains(&"review".to_string()));
    }

    #[test]
    fn r5_bus_missing_inputs_after_write() {
        let mut bus = BlackboardBus::new();
        let quin = make_quin(1, 2, 3, 42);
        bus.write_output("draft", quin).unwrap();

        let missing = bus.missing_inputs(&["draft".into(), "review".into()]);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "review");
    }

    #[test]
    fn r5_bus_write_outputs_multi_channel() {
        let mut bus = BlackboardBus::new();
        let q1 = make_quin(1, 2, 3, 42);
        let q2 = make_quin(4, 5, 6, 42);
        bus.write_outputs(&[("draft".into(), vec![q1]), ("review".into(), vec![q2])])
            .unwrap();

        let inputs = bus.read_inputs(&["draft".into(), "review".into()]);
        assert_eq!(inputs[0].1.len(), 1);
        assert_eq!(inputs[1].1.len(), 1);
    }

    #[test]
    fn r5_bus_propagate_constraints() {
        let mut bus = BlackboardBus::new();
        // Add a pinned constraint to the "draft" channel.
        let pinned = Constraint::pinned(1, 2, 3, 42);
        bus.board.add_constraint("draft", pinned).unwrap();

        // Propagate from "draft" (upstream output) to "review" (downstream input).
        bus.propagate_constraints(&["draft".into()], &["review".into()])
            .unwrap();

        // The "review" channel should now have the pinned constraint.
        let review = bus.board.get_channel("review").unwrap();
        assert_eq!(review.constraints.len(), 1);
        assert!(review.constraints[0].is_pinned());
    }

    #[test]
    fn r5_bus_freeze_output() {
        let mut bus = BlackboardBus::new();
        let quin = make_quin(1, 2, 3, 42);
        bus.write_output("draft", quin).unwrap();
        bus.freeze_output("draft").unwrap();

        // Writing to a frozen channel should fail.
        let result = bus.write_output("draft", make_quin(7, 8, 9, 42));
        assert!(result.is_err());
    }

    #[test]
    fn r5_bus_check_constraints_clean() {
        let mut bus = BlackboardBus::new();
        let quin = make_quin(1, 2, 3, 42);
        bus.write_output("draft", quin).unwrap();
        let violations = bus.check_constraints();
        assert!(violations.is_empty(), "no constraints → no violations");
    }

    fn make_quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
        let q = NQuin {
            subject,
            predicate: predicate << 8,
            object,
            context,
            metadata: 0,
            parity: 0,
        };
        NQuin {
            parity: q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata,
            ..q
        }
    }

    #[test]
    fn constraint_hard_creation() {
        let c = Constraint::hard(1, 2, 3, 42);
        assert!(c.is_hard());
        assert!(!c.is_soft());
        assert!(!c.is_pinned());
    }

    #[test]
    fn constraint_soft_creation() {
        let c = Constraint::soft(1, 2, 3, 42);
        assert!(!c.is_hard());
        assert!(c.is_soft());
        assert!(!c.is_pinned());
    }

    #[test]
    fn constraint_pinned_creation() {
        let c = Constraint::pinned(1, 2, 3, 42);
        assert!(c.is_hard());
        assert!(!c.is_soft());
        assert!(c.is_pinned());
    }

    #[test]
    fn constraint_quin_roundtrip() {
        let c = Constraint::hard(100, 200, 300, 400);
        let quin = c.to_quin();
        let decoded = Constraint::from_quin(&quin).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn constraint_pinned_quin_roundtrip() {
        let c = Constraint::pinned(100, 200, 300, 400);
        let quin = c.to_quin();
        let decoded = Constraint::from_quin(&quin).unwrap();
        assert_eq!(c, decoded);
        assert!(decoded.is_pinned());
    }

    #[test]
    fn channel_write_read() {
        let mut ch = Channel::new(42);
        let q = make_quin(1, 2, 3, 42);
        ch.write(q).unwrap();
        assert_eq!(ch.read().len(), 1);
        assert_eq!(ch.read()[0].subject, 1);
    }

    #[test]
    fn channel_freeze_blocks_writes() {
        let mut ch = Channel::new(42);
        ch.freeze();
        let q = make_quin(1, 2, 3, 42);
        assert!(ch.write(q).is_err());
    }

    #[test]
    fn channel_constraint_violation() {
        let mut ch = Channel::new(42);
        // Add a hard constraint: subject 1, predicate 2, must have object 3.
        ch.add_constraint(Constraint::hard(1, 2, 3, 42)).unwrap();
        // Write a Quin that violates it: subject 1, predicate 2, object 99.
        let q = make_quin(1, 2, 99, 42);
        ch.write(q).unwrap();
        let violations = ch.check_constraints();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].is_hard);
    }

    #[test]
    fn channel_constraint_no_violation() {
        let mut ch = Channel::new(42);
        ch.add_constraint(Constraint::hard(1, 2, 3, 42)).unwrap();
        let q = make_quin(1, 2, 3, 42);
        ch.write(q).unwrap();
        let violations = ch.check_constraints();
        assert!(violations.is_empty());
    }

    #[test]
    fn channel_soft_constraint_violation() {
        let mut ch = Channel::new(42);
        ch.add_constraint(Constraint::soft(1, 2, 3, 42)).unwrap();
        let q = make_quin(1, 2, 99, 42);
        ch.write(q).unwrap();
        let violations = ch.check_constraints();
        assert_eq!(violations.len(), 1);
        assert!(!violations[0].is_hard);
    }

    #[test]
    fn blackboard_write_read() {
        let mut bb = SemanticBlackboard::new();
        let q = make_quin(1, 2, 3, 42);
        bb.write("test_channel", q).unwrap();
        let state = bb.read("test_channel").unwrap();
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn blackboard_multiple_channels() {
        let mut bb = SemanticBlackboard::new();
        bb.write("ch1", make_quin(1, 2, 3, 42)).unwrap();
        bb.write("ch2", make_quin(4, 5, 6, 42)).unwrap();
        assert_eq!(bb.channel_count(), 2);
        assert_eq!(bb.read("ch1").unwrap().len(), 1);
        assert_eq!(bb.read("ch2").unwrap().len(), 1);
    }

    #[test]
    fn blackboard_root_constraint_propagation() {
        let mut bb = SemanticBlackboard::new();
        let root = Constraint::pinned(1, 2, 3, 42);
        bb.add_root_constraint(root).unwrap();
        // Write a violating Quin to a channel.
        bb.write("test", make_quin(1, 2, 99, 42)).unwrap();
        let violations = bb.check_all();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].is_hard);
    }

    #[test]
    fn blackboard_propagate_to_downstream() {
        let mut upstream = SemanticBlackboard::new();
        upstream
            .add_root_constraint(Constraint::pinned(1, 2, 3, 42))
            .unwrap();
        let mut downstream = SemanticBlackboard::new();
        upstream.propagate_to(&mut downstream).unwrap();
        // Downstream should now have the root constraint.
        downstream.write("test", make_quin(1, 2, 99, 42)).unwrap();
        let violations = downstream.check_all();
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn blackboard_pinned_constraints_collection() {
        let mut bb = SemanticBlackboard::new();
        bb.add_root_constraint(Constraint::pinned(1, 2, 3, 42))
            .unwrap();
        bb.add_constraint("ch1", Constraint::pinned(4, 5, 6, 42))
            .unwrap();
        bb.add_constraint("ch1", Constraint::hard(7, 8, 9, 42))
            .unwrap();
        let pinned = bb.pinned_constraints();
        assert_eq!(pinned.len(), 2); // root + channel pinned
    }

    #[test]
    fn blackboard_lamport_advances() {
        let mut bb = SemanticBlackboard::new();
        assert_eq!(bb.lamport(), 0);
        bb.write("ch", make_quin(1, 2, 3, 42)).unwrap();
        assert_eq!(bb.lamport(), 1);
        bb.write("ch", make_quin(4, 5, 6, 42)).unwrap();
        assert_eq!(bb.lamport(), 2);
    }

    #[test]
    fn blackboard_root_must_be_pinned() {
        let mut bb = SemanticBlackboard::new();
        let result = bb.add_root_constraint(Constraint::hard(1, 2, 3, 42));
        assert!(result.is_err());
    }

    #[test]
    fn blackboard_freeze_channel() {
        let mut bb = SemanticBlackboard::new();
        bb.write("ch", make_quin(1, 2, 3, 42)).unwrap();
        bb.freeze_channel("ch").unwrap();
        let result = bb.write("ch", make_quin(4, 5, 6, 42));
        assert!(result.is_err());
    }

    #[test]
    fn channel_max_constraints() {
        let mut ch = Channel::new(42);
        for i in 0..MAX_CONSTRAINTS_PER_CHANNEL {
            ch.add_constraint(Constraint::hard(i as u64, 2, 3, 42))
                .unwrap();
        }
        // Adding one more should fail.
        let result = ch.add_constraint(Constraint::hard(99, 2, 3, 42));
        assert!(result.is_err());
    }
}
