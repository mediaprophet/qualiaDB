//! DAW session operation history (cold path — not in audio callback).

use crate::production::{ProcessPlan, TrackState, MAX_TRACKS};

pub const MAX_HISTORY: usize = 64;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    SetGain = 1,
    SetPan = 2,
    SetMute = 3,
    SetSolo = 4,
    SetLowpass = 5,
    AddTrack = 6,
}

#[derive(Debug, Clone, Copy)]
pub struct SessionOp {
    pub kind: OpKind,
    pub track: u8,
    pub value_f32: f32,
    pub value_bool: bool,
    /// Prior value for undo.
    pub prev_f32: f32,
    pub prev_bool: bool,
}

/// Ring of ops + cursor for undo/redo.
#[derive(Debug, Clone)]
pub struct SessionHistory {
    ops: [Option<SessionOp>; MAX_HISTORY],
    /// Next write index.
    len: usize,
    /// Undo cursor (ops applied are [0..cursor)).
    cursor: usize,
}

impl Default for SessionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionHistory {
    pub fn new() -> Self {
        Self {
            ops: [None; MAX_HISTORY],
            len: 0,
            cursor: 0,
        }
    }

    pub fn push(&mut self, op: SessionOp) {
        if self.cursor < self.len {
            // drop redo tail
            self.len = self.cursor;
        }
        if self.len >= MAX_HISTORY {
            // drop oldest
            for i in 0..MAX_HISTORY - 1 {
                self.ops[i] = self.ops[i + 1];
            }
            self.len = MAX_HISTORY - 1;
            self.cursor = self.len;
        }
        self.ops[self.len] = Some(op);
        self.len += 1;
        self.cursor = self.len;
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor < self.len
    }

    pub fn undo(&mut self, plan: &mut ProcessPlan) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        if let Some(op) = self.ops[self.cursor] {
            apply_inverse(plan, op);
        }
        true
    }

    pub fn redo(&mut self, plan: &mut ProcessPlan) -> bool {
        if self.cursor >= self.len {
            return false;
        }
        if let Some(op) = self.ops[self.cursor] {
            apply_forward(plan, op);
        }
        self.cursor += 1;
        true
    }

    pub fn apply_and_record(&mut self, plan: &mut ProcessPlan, op: SessionOp) {
        apply_forward(plan, op);
        self.push(op);
    }
}

fn apply_forward(plan: &mut ProcessPlan, op: SessionOp) {
    let t = op.track as usize;
    if t >= MAX_TRACKS {
        return;
    }
    match op.kind {
        OpKind::SetGain => {
            if t < plan.n_tracks {
                plan.tracks[t].gain = op.value_f32;
            }
        }
        OpKind::SetPan => {
            if t < plan.n_tracks {
                plan.tracks[t].pan = op.value_f32;
            }
        }
        OpKind::SetMute => {
            if t < plan.n_tracks {
                plan.tracks[t].mute = op.value_bool;
            }
        }
        OpKind::SetSolo => {
            if t < plan.n_tracks {
                plan.tracks[t].solo = op.value_bool;
            }
        }
        OpKind::SetLowpass => {
            if t < plan.n_tracks {
                plan.tracks[t].lowpass = op.value_f32;
            }
        }
        OpKind::AddTrack => {
            let _ = plan.add_track(TrackState {
                gain: op.value_f32,
                ..TrackState::default()
            });
        }
    }
}

fn apply_inverse(plan: &mut ProcessPlan, op: SessionOp) {
    let t = op.track as usize;
    match op.kind {
        OpKind::SetGain => {
            if t < plan.n_tracks {
                plan.tracks[t].gain = op.prev_f32;
            }
        }
        OpKind::SetPan => {
            if t < plan.n_tracks {
                plan.tracks[t].pan = op.prev_f32;
            }
        }
        OpKind::SetMute => {
            if t < plan.n_tracks {
                plan.tracks[t].mute = op.prev_bool;
            }
        }
        OpKind::SetSolo => {
            if t < plan.n_tracks {
                plan.tracks[t].solo = op.prev_bool;
            }
        }
        OpKind::SetLowpass => {
            if t < plan.n_tracks {
                plan.tracks[t].lowpass = op.prev_f32;
            }
        }
        OpKind::AddTrack => {
            if plan.n_tracks > 0 {
                plan.n_tracks -= 1;
            }
        }
    }
}

/// Sample-accurate automation point (cold compile into per-block gains).
#[derive(Debug, Clone, Copy)]
pub struct AutomationPoint {
    pub frame: u64,
    pub value: f32,
}

pub const MAX_AUTO_POINTS: usize = 32;

#[derive(Debug, Clone)]
pub struct AutomationLane {
    pub track: u8,
    pub points: [AutomationPoint; MAX_AUTO_POINTS],
    pub n_points: usize,
}

impl AutomationLane {
    pub fn new(track: u8) -> Self {
        Self {
            track,
            points: [AutomationPoint {
                frame: 0,
                value: 1.0,
            }; MAX_AUTO_POINTS],
            n_points: 0,
        }
    }

    pub fn add(&mut self, frame: u64, value: f32) -> bool {
        if self.n_points >= MAX_AUTO_POINTS {
            return false;
        }
        self.points[self.n_points] = AutomationPoint { frame, value };
        self.n_points += 1;
        // keep sorted by frame
        let n = self.n_points;
        self.points[..n].sort_by_key(|p| p.frame);
        true
    }

    /// Linear interpolate gain at `frame`.
    pub fn value_at(&self, frame: u64) -> f32 {
        if self.n_points == 0 {
            return 1.0;
        }
        if frame <= self.points[0].frame {
            return self.points[0].value;
        }
        for i in 1..self.n_points {
            if frame <= self.points[i].frame {
                let a = self.points[i - 1];
                let b = self.points[i];
                let span = (b.frame - a.frame).max(1) as f32;
                let t = (frame - a.frame) as f32 / span;
                return a.value + (b.value - a.value) * t;
            }
        }
        self.points[self.n_points - 1].value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_gain() {
        let mut plan = ProcessPlan::new(48000, 64);
        plan.add_track(TrackState::default());
        let mut h = SessionHistory::new();
        let op = SessionOp {
            kind: OpKind::SetGain,
            track: 0,
            value_f32: 0.5,
            value_bool: false,
            prev_f32: 1.0,
            prev_bool: false,
        };
        h.apply_and_record(&mut plan, op);
        assert!((plan.tracks[0].gain - 0.5).abs() < 1e-6);
        assert!(h.undo(&mut plan));
        assert!((plan.tracks[0].gain - 1.0).abs() < 1e-6);
        assert!(h.redo(&mut plan));
        assert!((plan.tracks[0].gain - 0.5).abs() < 1e-6);
    }

    #[test]
    fn automation_interp() {
        let mut lane = AutomationLane::new(0);
        lane.add(0, 0.0);
        lane.add(100, 1.0);
        assert!((lane.value_at(50) - 0.5).abs() < 1e-5);
    }
}
