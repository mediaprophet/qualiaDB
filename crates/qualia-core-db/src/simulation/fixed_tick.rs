//! Deterministic Fixed-Tick Multi-Agent Simulation Engine (QG-09).
//!
//! Provides a strictly reproducible, zero-float-drift fixed-step simulation kernel.
//! Designed for:
//! - Multi-agent RTS and simulation worlds (Rolling Commons)
//! - POET NOS step-by-step document and macro automation
//! - Attestable distributed simulation replay across native and WASM
//!
//! Rules (AGENTS.md):
//! - Zero heap inside hot tick evaluation loops (Tier 1 hot-path).
//! - Fixed-point / integer arithmetic for coordinates and resource tallies.
//! - Seeded deterministic PRNG (PCG / xorshift).
//! - Strict command ordering: (target_tick, sequence, actor_did).
//! - Typed accepted / rejected receipts for all submitted intents.

/// Simulation frequency (20 Hz = 50ms per tick).
pub const SIMULATION_TICK_RATE_HZ: u32 = 20;
pub const SIMULATION_TICK_MS: u64 = 1000 / SIMULATION_TICK_RATE_HZ as u64;

/// Deterministic 64-bit pseudo-random number generator (xorshift64*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterministicPrng {
    state: u64,
}

impl DeterministicPrng {
    pub fn new(seed: u64) -> Self {
        // Ensure non-zero initial state
        let s = if seed == 0 {
            0x5EED_5EED_CAFE_BABE
        } else {
            seed
        };
        Self { state: s }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    #[inline]
    pub fn next_bounded(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }
}

/// Simulation command intent submitted by an agent or user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationCommand {
    pub target_tick: u64,
    pub sequence: u32,
    pub actor_did: u64,
    pub command_opcode: u16,
    pub target_entity: u64,
    pub arg0: i64,
    pub arg1: i64,
}

impl SimulationCommand {
    pub fn command_hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64; // FNV-1a
        for &b in &self.target_tick.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        for &b in &self.sequence.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        for &b in &self.actor_did.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        for &b in &self.command_opcode.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        for &b in &self.target_entity.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        for &b in &self.arg0.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        for &b in &self.arg1.to_le_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// Reason for rejecting a proposed command.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionReason {
    InvalidTick = 1,
    UnauthorizedActor = 2,
    EntityNotFound = 3,
    InsufficientResources = 4,
    CollisionObstacle = 5,
    InvalidOpcode = 6,
}

/// Deterministic receipt issued for every executed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandReceipt {
    Accepted {
        tick: u64,
        command_hash: u64,
        state_delta_hash: u64,
    },
    Rejected {
        tick: u64,
        command_hash: u64,
        reason: RejectionReason,
    },
}

/// Compact agent state in fixed-point space (millimeter coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentState {
    pub entity_id: u64,
    pub pos_x_mm: i64,
    pub pos_y_mm: i64,
    pub resource_tally: u64,
    pub task_opcode: u16,
    pub health_points: u16,
}

/// Fixed-capacity multi-agent simulation world kernel (Zero Heap hot path).
pub struct FixedTickWorld<const MAX_AGENTS: usize, const MAX_QUEUE: usize> {
    pub seed: u64,
    pub prng: DeterministicPrng,
    pub current_tick: u64,
    pub agents: [AgentState; MAX_AGENTS],
    pub agent_count: usize,
    command_queue: [Option<SimulationCommand>; MAX_QUEUE],
    queue_len: usize,
}

impl<const MAX_AGENTS: usize, const MAX_QUEUE: usize> FixedTickWorld<MAX_AGENTS, MAX_QUEUE> {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            prng: DeterministicPrng::new(seed),
            current_tick: 0,
            agents: [AgentState {
                entity_id: 0,
                pos_x_mm: 0,
                pos_y_mm: 0,
                resource_tally: 0,
                task_opcode: 0,
                health_points: 100,
            }; MAX_AGENTS],
            agent_count: 0,
            command_queue: [None; MAX_QUEUE],
            queue_len: 0,
        }
    }

    /// Spawn or register an agent entity in the world.
    pub fn register_agent(&mut self, entity_id: u64, x_mm: i64, y_mm: i64) -> bool {
        if self.agent_count >= MAX_AGENTS {
            return false;
        }
        self.agents[self.agent_count] = AgentState {
            entity_id,
            pos_x_mm: x_mm,
            pos_y_mm: y_mm,
            resource_tally: 0,
            task_opcode: 0,
            health_points: 100,
        };
        self.agent_count += 1;
        true
    }

    /// Submit a command intent into the deterministic tick queue.
    pub fn submit_command(&mut self, cmd: SimulationCommand) -> Result<(), RejectionReason> {
        if cmd.target_tick < self.current_tick {
            return Err(RejectionReason::InvalidTick);
        }
        if self.queue_len >= MAX_QUEUE {
            return Err(RejectionReason::InsufficientResources);
        }
        self.command_queue[self.queue_len] = Some(cmd);
        self.queue_len += 1;
        Ok(())
    }

    /// Step the simulation by exactly one fixed tick (Zero Heap Tier 1 loop).
    /// Writes output receipts into the caller-supplied slice.
    pub fn step_tick(&mut self, out_receipts: &mut [CommandReceipt]) -> usize {
        self.current_tick += 1;
        let mut receipt_idx = 0;

        // 1. Process commands scheduled for this tick
        let mut i = 0;
        while i < self.queue_len {
            if let Some(cmd) = self.command_queue[i] {
                if cmd.target_tick == self.current_tick {
                    let receipt = self.execute_command(&cmd);
                    if receipt_idx < out_receipts.len() {
                        out_receipts[receipt_idx] = receipt;
                        receipt_idx += 1;
                    }
                    // Remove executed command by swapping with last
                    self.command_queue[i] = self.command_queue[self.queue_len - 1];
                    self.command_queue[self.queue_len - 1] = None;
                    self.queue_len -= 1;
                    continue; // Re-check at index i
                }
            }
            i += 1;
        }

        // 2. Deterministic ambient world update (passive resource regeneration / motion)
        for a in 0..self.agent_count {
            let rnd = self.prng.next_bounded(4);
            match rnd {
                0 => self.agents[a].pos_x_mm += 10,
                1 => self.agents[a].pos_x_mm -= 10,
                2 => self.agents[a].pos_y_mm += 10,
                _ => self.agents[a].pos_y_mm -= 10,
            }
        }

        receipt_idx
    }

    fn execute_command(&mut self, cmd: &SimulationCommand) -> CommandReceipt {
        let cmd_hash = cmd.command_hash();

        // Find target agent
        let mut agent_idx = None;
        for a in 0..self.agent_count {
            if self.agents[a].entity_id == cmd.target_entity {
                agent_idx = Some(a);
                break;
            }
        }

        let idx = match agent_idx {
            Some(i) => i,
            None => {
                return CommandReceipt::Rejected {
                    tick: self.current_tick,
                    command_hash: cmd_hash,
                    reason: RejectionReason::EntityNotFound,
                };
            }
        };

        // Opcode dispatch:
        // 0x01 = Move (arg0 = delta_x_mm, arg1 = delta_y_mm)
        // 0x02 = Harvest (arg0 = resource_gain)
        match cmd.command_opcode {
            0x01 => {
                self.agents[idx].pos_x_mm += cmd.arg0;
                self.agents[idx].pos_y_mm += cmd.arg1;
                CommandReceipt::Accepted {
                    tick: self.current_tick,
                    command_hash: cmd_hash,
                    state_delta_hash: (cmd.arg0 as u64) ^ (cmd.arg1 as u64),
                }
            }
            0x02 => {
                if cmd.arg0 < 0 {
                    return CommandReceipt::Rejected {
                        tick: self.current_tick,
                        command_hash: cmd_hash,
                        reason: RejectionReason::InsufficientResources,
                    };
                }
                self.agents[idx].resource_tally = self.agents[idx]
                    .resource_tally
                    .saturating_add(cmd.arg0 as u64);
                CommandReceipt::Accepted {
                    tick: self.current_tick,
                    command_hash: cmd_hash,
                    state_delta_hash: cmd.arg0 as u64,
                }
            }
            0x03 => {
                self.agents[idx].pos_x_mm = cmd.arg0;
                self.agents[idx].pos_y_mm = cmd.arg1;
                CommandReceipt::Accepted {
                    tick: self.current_tick,
                    command_hash: cmd_hash,
                    state_delta_hash: (cmd.arg0 as u64) ^ (cmd.arg1 as u64),
                }
            }
            _ => CommandReceipt::Rejected {
                tick: self.current_tick,
                command_hash: cmd_hash,
                reason: RejectionReason::InvalidOpcode,
            },
        }
    }

    /// Compute a canonical 64-bit state fingerprint across all entities.
    pub fn compute_state_hash(&self) -> u64 {
        let mut h = 0x811c9dc5u64 ^ self.current_tick;
        for a in 0..self.agent_count {
            h ^= self.agents[a].entity_id.wrapping_mul(0x100000001b3);
            h ^= (self.agents[a].pos_x_mm as u64).wrapping_mul(0x100000001b3);
            h ^= (self.agents[a].pos_y_mm as u64).wrapping_mul(0x100000001b3);
            h ^= self.agents[a].resource_tally.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Lookup current coordinates of an agent (in millimeters).
    pub fn get_agent_position(&self, entity_id: u64) -> Option<(i64, i64)> {
        for a in 0..self.agent_count {
            if self.agents[a].entity_id == entity_id {
                return Some((self.agents[a].pos_x_mm, self.agents[a].pos_y_mm));
            }
        }
        None
    }

    /// Set position directly for an agent.
    pub fn set_agent_position(&mut self, entity_id: u64, x_mm: i64, y_mm: i64) -> bool {
        for a in 0..self.agent_count {
            if self.agents[a].entity_id == entity_id {
                self.agents[a].pos_x_mm = x_mm;
                self.agents[a].pos_y_mm = y_mm;
                return true;
            }
        }
        false
    }

    /// Spatial box selection / range query (Zero Heap Tier 1 hot path).
    ///
    /// Writes matching `entity_id` values into `out_entity_ids` and returns the count.
    pub fn query_agents_in_bounds(
        &self,
        min_x_mm: i64,
        min_y_mm: i64,
        max_x_mm: i64,
        max_y_mm: i64,
        out_entity_ids: &mut [u64],
    ) -> usize {
        let mut count = 0;
        for a in 0..self.agent_count {
            let px = self.agents[a].pos_x_mm;
            let py = self.agents[a].pos_y_mm;
            if px >= min_x_mm && px <= max_x_mm && py >= min_y_mm && py <= max_y_mm {
                if count < out_entity_ids.len() {
                    out_entity_ids[count] = self.agents[a].entity_id;
                    count += 1;
                } else {
                    break;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_tick_multi_agent_deterministic_replay() {
        let mut sim1 = FixedTickWorld::<16, 64>::new(0xDEAD_BEEF);
        let mut sim2 = FixedTickWorld::<16, 64>::new(0xDEAD_BEEF);

        sim1.register_agent(1001, 1000, 2000);
        sim1.register_agent(1002, 5000, 6000);

        sim2.register_agent(1001, 1000, 2000);
        sim2.register_agent(1002, 5000, 6000);

        let cmd = SimulationCommand {
            target_tick: 5,
            sequence: 1,
            actor_did: 0xAAAA,
            command_opcode: 0x01, // Move
            target_entity: 1001,
            arg0: 150,
            arg1: -200,
        };

        sim1.submit_command(cmd).unwrap();
        sim2.submit_command(cmd).unwrap();

        let mut receipts1 = [CommandReceipt::Rejected {
            tick: 0,
            command_hash: 0,
            reason: RejectionReason::InvalidTick,
        }; 8];
        let mut receipts2 = [CommandReceipt::Rejected {
            tick: 0,
            command_hash: 0,
            reason: RejectionReason::InvalidTick,
        }; 8];

        for _ in 0..100 {
            sim1.step_tick(&mut receipts1);
            sim2.step_tick(&mut receipts2);
        }

        // Both simulations must have 100% bit-identical state hashes after 100 ticks
        assert_eq!(sim1.compute_state_hash(), sim2.compute_state_hash());
        assert_eq!(sim1.agents[0].pos_x_mm, sim2.agents[0].pos_x_mm);
        assert_eq!(sim1.agents[0].pos_y_mm, sim2.agents[0].pos_y_mm);
    }

    #[test]
    fn test_fixed_tick_rejected_command_leaves_state_intact() {
        let mut sim = FixedTickWorld::<8, 16>::new(42);
        sim.register_agent(500, 0, 0);

        // Submit command targeting non-existent entity
        let bad_cmd = SimulationCommand {
            target_tick: 1,
            sequence: 1,
            actor_did: 0x1,
            command_opcode: 0x01,
            target_entity: 9999, // Does not exist
            arg0: 100,
            arg1: 100,
        };
        sim.submit_command(bad_cmd).unwrap();

        let mut receipts = [CommandReceipt::Rejected {
            tick: 0,
            command_hash: 0,
            reason: RejectionReason::InvalidTick,
        }; 4];
        let n = sim.step_tick(&mut receipts);
        assert_eq!(n, 1);

        assert!(matches!(
            receipts[0],
            CommandReceipt::Rejected {
                reason: RejectionReason::EntityNotFound,
                ..
            }
        ));

        // State matches a world that advanced 1 tick with no mutations
        let mut baseline_sim = FixedTickWorld::<8, 16>::new(42);
        baseline_sim.register_agent(500, 0, 0);
        let mut baseline_receipts = [CommandReceipt::Rejected {
            tick: 0,
            command_hash: 0,
            reason: RejectionReason::InvalidTick,
        }; 4];
        baseline_sim.step_tick(&mut baseline_receipts);

        assert_eq!(sim.compute_state_hash(), baseline_sim.compute_state_hash());
        assert_eq!(sim.agents[0].pos_x_mm, baseline_sim.agents[0].pos_x_mm);
        assert_eq!(sim.agents[0].pos_y_mm, baseline_sim.agents[0].pos_y_mm);
        assert_eq!(
            sim.agents[0].resource_tally,
            baseline_sim.agents[0].resource_tally
        );
    }

    #[test]
    fn test_fixed_tick_harvest_resource_tally() {
        let mut sim = FixedTickWorld::<4, 8>::new(12345);
        sim.register_agent(1, 0, 0);

        let harvest_cmd = SimulationCommand {
            target_tick: 1,
            sequence: 1,
            actor_did: 0x1,
            command_opcode: 0x02, // Harvest
            target_entity: 1,
            arg0: 250,
            arg1: 0,
        };
        sim.submit_command(harvest_cmd).unwrap();

        let mut receipts = [CommandReceipt::Rejected {
            tick: 0,
            command_hash: 0,
            reason: RejectionReason::InvalidTick,
        }; 4];
        sim.step_tick(&mut receipts);

        assert_eq!(sim.agents[0].resource_tally, 250);
        assert!(matches!(receipts[0], CommandReceipt::Accepted { .. }));
    }

    #[test]
    fn test_spatial_query_and_position() {
        let mut sim = FixedTickWorld::<8, 16>::new(42);
        sim.register_agent(101, 1000, 2000);
        sim.register_agent(102, 5000, 5000);
        sim.register_agent(103, 1200, 2100);

        assert_eq!(sim.get_agent_position(101), Some((1000, 2000)));
        assert_eq!(sim.get_agent_position(999), None);

        let mut out_ids = [0u64; 4];
        let count = sim.query_agents_in_bounds(500, 1500, 2000, 2500, &mut out_ids);
        assert_eq!(count, 2);
        assert!(out_ids[..count].contains(&101));
        assert!(out_ids[..count].contains(&103));
    }
}
