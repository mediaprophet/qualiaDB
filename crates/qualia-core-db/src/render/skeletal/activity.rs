//! Bounded visual activity timeline, independent of campaign time and rendering.
use serde::{Deserialize, Serialize};

pub const MAX_ACTIVITY_CUES: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActivityCue {
    pub token: u64,
    pub joint: u64,
    pub clip: u64,
    pub idle: u64,
    pub duration: f64,
    pub elapsed: f64,
    pub fade: f64,
    pub finished: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityError {
    InvalidInput,
    Capacity,
    OutputTooSmall,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ActivityTimeline {
    cues: [Option<ActivityCue>; MAX_ACTIVITY_CUES],
    time: f64,
}
impl Default for ActivityTimeline {
    fn default() -> Self {
        Self {
            cues: [None; MAX_ACTIVITY_CUES],
            time: 0.0,
        }
    }
}
impl ActivityCue {
    fn valid(self) -> bool {
        self.token != 0
            && self.joint != 0
            && self.clip != 0
            && self.idle != 0
            && self.duration.is_finite()
            && (0.01..=120.0).contains(&self.duration)
            && self.elapsed.is_finite()
            && (0.0..=self.duration).contains(&self.elapsed)
            && self.fade.is_finite()
            && (0.0..=10.0).contains(&self.fade)
            && self.finished == (self.elapsed == self.duration)
    }
}
impl ActivityTimeline {
    /// Current timeline position.
    pub fn time(&self) -> f64 {
        self.time
    }
    /// All scheduled cues.
    pub fn cues(&self) -> impl Iterator<Item = &ActivityCue> {
        self.cues.iter().flatten()
    }
    /// Cue for joint `joint`.
    pub fn cue(&self, joint: u64) -> Option<ActivityCue> {
        self.cues().find(|cue| cue.joint == joint).copied()
    }
    /// Validates the timeline; `ActivityError` on violation.
    pub fn validate(&self) -> Result<(), ActivityError> {
        if !self.time.is_finite() || !(0.0..=1e12).contains(&self.time) {
            return Err(ActivityError::InvalidInput);
        }
        for (i, cue) in self.cues.iter().enumerate() {
            if let Some(cue) = cue {
                if !cue.valid()
                    || self.cues[..i]
                        .iter()
                        .flatten()
                        .any(|old| old.joint == cue.joint)
                {
                    return Err(ActivityError::InvalidInput);
                }
            }
        }
        Ok(())
    }
    /// An accepted event token is idempotent for its actor, including after
    /// finishing. A newer actor activity replaces the preceding visual cue.
    pub fn schedule(&mut self, cue: ActivityCue) -> Result<bool, ActivityError> {
        self.validate()?;
        if !cue.valid() || cue.elapsed != 0.0 || cue.finished {
            return Err(ActivityError::InvalidInput);
        }
        if let Some(index) = self
            .cues
            .iter()
            .position(|slot| slot.is_some_and(|old| old.joint == cue.joint))
        {
            if self.cues[index].is_some_and(|old| old.token == cue.token) {
                return Ok(false);
            }
            self.cues[index] = Some(cue);
            return Ok(true);
        }
        let index = self
            .cues
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                self.cues
                    .iter()
                    .position(|slot| slot.is_some_and(|old| old.finished))
            })
            .ok_or(ActivityError::Capacity)?;
        self.cues[index] = Some(cue);
        Ok(true)
    }
    /// Outputs only newly completed cues. A fixed caller buffer covers the
    /// worst case so invalid capacity/duration never partially advances time.
    pub fn advance_into(
        &mut self,
        dt: f64,
        completed: &mut [ActivityCue],
    ) -> Result<usize, ActivityError> {
        self.validate()?;
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) || self.time + dt > 1e12 {
            return Err(ActivityError::InvalidInput);
        }
        if completed.len() < MAX_ACTIVITY_CUES {
            return Err(ActivityError::OutputTooSmall);
        }
        self.time += dt;
        let mut count = 0;
        for cue in self.cues.iter_mut().flatten() {
            if cue.finished {
                continue;
            }
            cue.elapsed = (cue.elapsed + dt).min(cue.duration);
            if cue.elapsed == cue.duration {
                cue.finished = true;
                completed[count] = *cue;
                count += 1;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cue() -> ActivityCue {
        ActivityCue {
            token: 1,
            joint: 2,
            clip: 3,
            idle: 4,
            duration: 0.5,
            elapsed: 0.0,
            fade: 0.2,
            finished: false,
        }
    }
    #[test]
    fn completed_tokens_do_not_restart_and_other_actor_can_interrupt() {
        let mut timeline = ActivityTimeline::default();
        let mut done = [cue(); MAX_ACTIVITY_CUES];
        assert_eq!(timeline.schedule(cue()), Ok(true));
        assert_eq!(timeline.advance_into(0.5, &mut done), Ok(1));
        assert!(done[0].finished);
        assert_eq!(timeline.schedule(cue()), Ok(false));
        assert_eq!(timeline.advance_into(0.5, &mut done), Ok(0));
        let mut newer = cue();
        newer.token = 5;
        assert_eq!(timeline.schedule(newer), Ok(true));
        assert_eq!(timeline.cue(2).unwrap().elapsed, 0.0);
    }
    #[test]
    fn short_output_and_invalid_restore_are_atomic() {
        let mut timeline = ActivityTimeline::default();
        timeline.schedule(cue()).unwrap();
        assert_eq!(
            timeline.advance_into(0.1, &mut []),
            Err(ActivityError::OutputTooSmall)
        );
        assert_eq!(timeline.time(), 0.0);
        timeline.cues[1] = Some(cue());
        assert_eq!(timeline.validate(), Err(ActivityError::InvalidInput));
    }
    #[test]
    fn snapshot_retains_pending_and_finished_receipts() {
        let mut timeline = ActivityTimeline::default();
        let mut done = [cue(); MAX_ACTIVITY_CUES];
        timeline.schedule(cue()).unwrap();
        timeline.advance_into(0.2, &mut done).unwrap();
        let encoded = serde_json::to_string(&timeline).unwrap();
        let mut restored: ActivityTimeline = serde_json::from_str(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.cue(2).unwrap().elapsed, 0.2);
        assert_eq!(restored.schedule(cue()), Ok(false));
        assert_eq!(restored.advance_into(0.3, &mut done), Ok(1));
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn timeline_frame_is_zero_heap() {
        use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
        let mut timeline = ActivityTimeline::default();
        timeline.schedule(cue()).unwrap();
        let mut done = [cue(); MAX_ACTIVITY_CUES];
        assert_zero_alloc("activity_timeline_frame", || {
            timeline.advance_into(0.1, &mut done).unwrap();
        });
    }
}
