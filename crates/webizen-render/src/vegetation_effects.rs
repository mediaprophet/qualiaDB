//! Deterministic, fixed-capacity ambient vegetation/effects support.
//!
//! Re-exports the canonical ambient vegetation and particle pool from
//! `etalon_render::geo_viewport::vegetation_effects`.

pub use etalon_render::geo_viewport::vegetation_effects::*;

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
        assert_eq!(
            EffectPool::new(0, DEFAULT_WORLD_SEED).emit(EffectEvent::default()),
            SpawnOutcome::Disabled
        );
    }
}
