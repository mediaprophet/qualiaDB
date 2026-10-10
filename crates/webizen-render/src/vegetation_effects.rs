//! Deterministic, fixed-capacity ambient vegetation/effects support.
//!
//! The pool is deliberately independent of a graphics device.  It owns the
//! cold-path seed generation and event admission policy; the renderer can hand
//! its GPU-facing particle slice to a backend without allocating in the frame
//! loop.  GPU wind is evaluated from the same seed and world frame.

use crate::telemetry::SystemTelemetry;

pub const MAX_AMBIENT_PARTICLES: usize = 50_000;
pub const MAX_EFFECT_EVENTS: usize = 256;
pub const DEFAULT_WORLD_SEED: u32 = 0x6d2b_79f5;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct GpuParticle {
    pub position: [f32; 3],
    pub seed: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldFrame {
    pub index: u32,
    pub time_seconds: f32,
    pub delta_seconds: f32,
    pub seed: u32,
}

impl WorldFrame {
    #[inline]
    pub fn from_time(time_seconds: f32, seed: u32) -> Self {
        let time_seconds = if time_seconds.is_finite() {
            time_seconds.max(0.0)
        } else {
            0.0
        };
        let index = (time_seconds * 60.0).floor().min(u32::MAX as f32) as u32;
        Self {
            index,
            time_seconds,
            delta_seconds: 1.0 / 60.0,
            seed,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindField {
    pub direction: [f32; 2],
    pub speed: f32,
    pub strength: f32,
}

impl WindField {
    #[inline]
    pub fn from_telemetry(telemetry: &SystemTelemetry) -> Self {
        let strength = telemetry.ambient_wind_strength();
        Self {
            direction: [0.94, 0.34],
            speed: 0.8 + telemetry.baking_crystallization * 0.4,
            strength,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectEvent {
    pub origin: [f32; 3],
    pub energy: f32,
    pub seed: u32,
    pub priority: u8,
}

impl Default for EffectEvent {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            energy: 0.0,
            seed: 0,
            priority: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpawnOutcome {
    Queued,
    QueueOverflowDropped,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectsMode {
    Disabled,
    CpuDegraded,
    Gpu,
}

#[inline]
pub fn select_mode(enabled: bool, gpu_available: bool) -> EffectsMode {
    if !enabled {
        EffectsMode::Disabled
    } else if gpu_available {
        EffectsMode::Gpu
    } else {
        EffectsMode::CpuDegraded
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct EffectSlot {
    velocity: [f32; 3],
    age: f32,
    lifetime: f32,
    priority: u8,
    active: bool,
}

pub struct EffectPool {
    particles: Vec<GpuParticle>,
    slots: Vec<EffectSlot>,
    events: [EffectEvent; MAX_EFFECT_EVENTS],
    event_len: usize,
    next_slot: usize,
    dropped_events: u32,
    seed: u32,
}

impl EffectPool {
    /// Construct the pool once on renderer creation. Capacity is clamped to a
    /// hard ceiling; callers retain their requested config for diagnostics.
    pub fn new(requested_capacity: usize, seed: u32) -> Self {
        let capacity = requested_capacity.min(MAX_AMBIENT_PARTICLES);
        let mut particles = Vec::with_capacity(capacity);
        let mut slots = Vec::with_capacity(capacity);
        for index in 0..capacity {
            let particle_seed = stable_seed(seed, index as u32);
            particles.push(GpuParticle {
                position: initial_position(particle_seed),
                seed: particle_seed,
            });
            slots.push(EffectSlot {
                active: true,
                lifetime: f32::INFINITY,
                ..EffectSlot::default()
            });
        }
        Self {
            particles,
            slots,
            events: [EffectEvent::default(); MAX_EFFECT_EVENTS],
            event_len: 0,
            next_slot: 0,
            dropped_events: 0,
            seed,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.particles.len()
    }

    #[inline]
    pub fn particles(&self) -> &[GpuParticle] {
        &self.particles
    }

    #[inline]
    pub fn dropped_events(&self) -> u32 {
        self.dropped_events
    }

    /// Queue an event without allocating. If the event queue is full, the
    /// event is dropped deterministically and the caller can degrade quietly.
    pub fn emit(&mut self, event: EffectEvent) -> SpawnOutcome {
        if self.particles.is_empty() {
            return SpawnOutcome::Disabled;
        }
        if self.event_len == self.events.len() {
            self.dropped_events = self.dropped_events.saturating_add(1);
            return SpawnOutcome::QueueOverflowDropped;
        }
        self.events[self.event_len] = event;
        self.event_len += 1;
        SpawnOutcome::Queued
    }

    /// Apply queued events and advance CPU-owned effects. This is intended for
    /// event/cold paths; normal ambient wind remains GPU-side and needs no
    /// per-frame upload.
    pub fn update(&mut self, frame: WorldFrame, wind: WindField) {
        let queued = self.event_len;
        for index in 0..queued {
            let event = self.events[index];
            let slot = self.select_slot(event.priority);
            let seed = stable_seed(self.seed ^ frame.seed ^ event.seed, slot as u32);
            self.particles[slot] = GpuParticle {
                position: event.origin,
                seed,
            };
            let phase = unit_float(seed).mul_add(2.0, -1.0);
            self.slots[slot] = EffectSlot {
                velocity: [
                    wind.direction[0] * wind.speed * event.energy,
                    phase * 0.15 * event.energy,
                    wind.direction[1] * wind.speed * event.energy,
                ],
                age: 0.0,
                lifetime: (0.25 + unit_float(seed.rotate_left(7)) * 1.5).max(0.05),
                priority: event.priority,
                active: true,
            };
        }
        self.event_len = 0;

        let dt = frame.delta_seconds.clamp(0.0, 0.25);
        for slot in 0..self.particles.len() {
            let state = &mut self.slots[slot];
            if !state.active || !state.lifetime.is_finite() {
                continue;
            }
            state.age += dt;
            if state.age >= state.lifetime {
                state.active = false;
                continue;
            }
            let particle = &mut self.particles[slot];
            particle.position[0] += state.velocity[0] * dt;
            particle.position[1] += state.velocity[1] * dt;
            particle.position[2] += state.velocity[2] * dt;
        }
    }

    fn select_slot(&mut self, priority: u8) -> usize {
        let capacity = self.slots.len();
        for offset in 0..capacity {
            let index = (self.next_slot + offset) % capacity;
            if !self.slots[index].active {
                self.next_slot = (index + 1) % capacity;
                return index;
            }
        }

        let mut selected = self.next_slot % capacity;
        for offset in 1..capacity {
            let index = (self.next_slot + offset) % capacity;
            let candidate = self.slots[index];
            let current = self.slots[selected];
            if candidate.priority < current.priority
                || (candidate.priority == current.priority && candidate.age > current.age)
            {
                selected = index;
            }
        }
        if self.slots[selected].priority > priority {
            selected = self.next_slot % capacity;
        }
        self.next_slot = (selected + 1) % capacity;
        selected
    }
}

#[inline]
pub fn stable_seed(seed: u32, index: u32) -> u32 {
    let mut value = seed.wrapping_add(index.wrapping_mul(0x9e37_79b9));
    value = (value ^ (value >> 16)).wrapping_mul(0x85eb_ca6b);
    value = (value ^ (value >> 13)).wrapping_mul(0xc2b2_ae35);
    value ^ (value >> 16)
}

#[inline]
fn unit_float(value: u32) -> f32 {
    (value as f32) / (u32::MAX as f32)
}

#[inline]
fn initial_position(seed: u32) -> [f32; 3] {
    [
        unit_float(stable_seed(seed, 0)).mul_add(2.0, -1.0),
        unit_float(stable_seed(seed, 1)).mul_add(2.0, -1.0),
        unit_float(stable_seed(seed, 2)).mul_add(2.0, -1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_seeds_and_initial_positions_repeat() {
        let first = EffectPool::new(8, DEFAULT_WORLD_SEED);
        let second = EffectPool::new(8, DEFAULT_WORLD_SEED);
        assert_eq!(first.particles(), second.particles());
        assert_ne!(first.particles()[0].seed, first.particles()[1].seed);
    }

    #[test]
    fn capacity_is_bounded_and_event_overflow_is_explicit() {
        let mut pool = EffectPool::new(2, DEFAULT_WORLD_SEED);
        for index in 0..MAX_EFFECT_EVENTS {
            assert_eq!(
                pool.emit(EffectEvent {
                    seed: index as u32,
                    ..EffectEvent::default()
                }),
                SpawnOutcome::Queued
            );
        }
        assert_eq!(
            pool.emit(EffectEvent::default()),
            SpawnOutcome::QueueOverflowDropped
        );
        assert_eq!(pool.dropped_events(), 1);
        assert_eq!(pool.capacity(), 2);
    }

    #[test]
    fn update_is_deterministic_for_same_event_and_frame() {
        let event = EffectEvent {
            origin: [1.0, 2.0, 3.0],
            energy: 0.8,
            seed: 44,
            priority: 2,
        };
        let frame = WorldFrame::from_time(1.0, DEFAULT_WORLD_SEED);
        let wind = WindField {
            direction: [0.8, 0.6],
            speed: 1.0,
            strength: 1.0,
        };
        let mut first = EffectPool::new(4, DEFAULT_WORLD_SEED);
        let mut second = EffectPool::new(4, DEFAULT_WORLD_SEED);
        assert_eq!(first.emit(event), SpawnOutcome::Queued);
        assert_eq!(second.emit(event), SpawnOutcome::Queued);
        first.update(frame, wind);
        second.update(frame, wind);
        assert_eq!(first.particles(), second.particles());
    }

    #[test]
    fn fallback_selection_is_conservative() {
        assert_eq!(select_mode(false, true), EffectsMode::Disabled);
        assert_eq!(select_mode(true, false), EffectsMode::CpuDegraded);
        assert_eq!(select_mode(true, true), EffectsMode::Gpu);
        assert_eq!(EffectPool::new(0, DEFAULT_WORLD_SEED).emit(EffectEvent::default()), SpawnOutcome::Disabled);
    }
}
