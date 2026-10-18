//! Scene ownership, frame ticking and backend upload of admitted SKL1 actors.
use super::{BodyRendererBackend, QualiaPortal};
use crate::render::skeletal::activity::{ActivityCue, ActivityTimeline, MAX_ACTIVITY_CUES};
use crate::render::skeletal::SkeletalPlayer;
use wasm_bindgen::prelude::*;

pub(crate) struct SceneActor {
    pub(crate) start: u32,
    pub(crate) player: SkeletalPlayer,
}

#[derive(Default)]
pub(crate) struct SceneAnimation {
    pub(crate) actors: Vec<SceneActor>,
    pub(crate) resident_bytes: usize,
    pub(crate) compact_gpu: bool,
    reduced_motion: bool,
    frames: u64,
    uploaded_vertices: u64,
    failed_uploads: u64,
    timeline: ActivityTimeline,
}
impl SceneAnimation {
    /// Replaces the active scene animation with `next`, optionally compacting GPU state.
    pub(crate) fn install(&mut self, mut next: Self, compact_gpu: bool) {
        next.reduced_motion = self.reduced_motion;
        next.compact_gpu = compact_gpu;
        next.timeline = self.timeline.clone();
        next.sync_activity_cues();
        *self = next;
    }
    fn sync_activity_cues(&mut self) {
        for actor in &mut self.actors {
            let cue = self
                .timeline
                .cues()
                .find(|cue| actor.player.attachment(cue.joint).is_some())
                .copied();
            if let Some(cue) = cue {
                let clip = if cue.finished { cue.idle } else { cue.clip };
                if actor.player.select_clip(clip, 0.0).is_err() {
                    self.failed_uploads += 1;
                    continue;
                }
                let time = if cue.finished {
                    self.timeline.time()
                } else {
                    cue.elapsed
                };
                let _ = actor.player.seek(time);
            } else {
                let _ = actor.player.seek(self.timeline.time());
            }
            if actor.player.advance(0.0, self.reduced_motion).is_err() {
                self.failed_uploads += 1;
            }
        }
    }
}

impl QualiaPortal {
    /// Sets the root actor for scene animation.
    pub fn set_scene_actor_root(
        &mut self,
        joint: u64,
        root: crate::render::skeletal::Motor,
    ) -> Result<u32, JsValue> {
        let mut count = 0;
        for actor in &mut self.scene_animation.actors {
            if actor.player.has_joint(joint) {
                actor
                    .player
                    .set_root(root)
                    .map_err(|_| JsValue::from_str("actor root transform invalid"))?;
                count += 1;
            }
        }
        Ok(count)
    }
    /// Applies a per-joint animation delta.
    pub fn set_scene_joint_delta(
        &mut self,
        group: u64,
        joint: u64,
        delta: crate::render::skeletal::Motor,
    ) -> Result<u32, JsValue> {
        let mut count = 0;
        for actor in &mut self.scene_animation.actors {
            if actor.player.has_joint(group) && actor.player.has_joint(joint) {
                actor
                    .player
                    .set_joint_delta(joint, delta)
                    .map_err(|_| JsValue::from_str("actor joint delta invalid"))?;
                count += 1;
            }
        }
        Ok(count)
    }
    /// Advances scene animation by `dt_ms`, updating GPU state.
    pub(crate) fn tick_scene_animation(&mut self, dt_ms: f32) -> Result<(), JsValue> {
        if !dt_ms.is_finite() || dt_ms < 0.0 {
            return Err(JsValue::from_str("invalid animation frame duration"));
        }
        let dt = (dt_ms as f64 * 0.001).min(1.0);
        let empty = ActivityCue {
            token: 1,
            joint: 1,
            clip: 1,
            idle: 1,
            duration: 1.0,
            elapsed: 0.0,
            fade: 0.0,
            finished: false,
        };
        let mut completed = [empty; MAX_ACTIVITY_CUES];
        let count = self
            .scene_animation
            .timeline
            .advance_into(dt, &mut completed)
            .map_err(|_| JsValue::from_str("animation activity timeline failed"))?;
        for cue in &completed[..count] {
            for actor in &mut self.scene_animation.actors {
                if actor.player.attachment(cue.joint).is_some() {
                    actor
                        .player
                        .select_clip(cue.idle, cue.fade)
                        .map_err(|_| JsValue::from_str("activity return clip missing"))?;
                }
            }
        }
        if self.scene_animation.actors.is_empty() {
            return Ok(());
        }
        let compact = self.scene_animation.compact_gpu;
        for actor in &mut self.scene_animation.actors {
            actor
                .player
                .advance(dt, self.scene_animation.reduced_motion)
                .map_err(|_| JsValue::from_str("native skeletal frame failed"))?;
            if let Some(body) = self.cpu_body.as_mut() {
                body.update_authored_span(actor.start, actor.player.positions());
            }
            if self.body_renderer == BodyRendererBackend::WebGpu && !compact {
                if let Some(gpu) = self.gpu.as_mut() {
                    if gpu.write_mesh_animated_frames(
                        actor.start,
                        actor.player.positions(),
                        actor.player.normals(),
                        actor.player.tangents(),
                    ) {
                        self.scene_animation.uploaded_vertices +=
                            actor.player.positions().len() as u64;
                    } else {
                        self.scene_animation.failed_uploads += 1;
                    }
                }
            }
        }
        // Compact backends use the replay mesh's lineage, never full-scene offsets.
        if let Some(body) = self.cpu_body.as_ref() {
            if self.body_renderer == BodyRendererBackend::WebGpu && compact {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.write_mesh_vertices(0, &body.positions);
                }
            } else if self.body_renderer == BodyRendererBackend::WebGl2 {
                if let Some(gl) = self.anatomy_webgl2.as_mut() {
                    gl.write_vertices(0, &body.positions);
                }
            }
        }
        self.scene_animation.frames += 1;
        Ok(())
    }
}

#[wasm_bindgen]
impl QualiaPortal {
    /// A timed presentation cue from an accepted game event. This changes no
    /// campaign facts; completed and pending cues survive scene replacement.
    pub fn present_scene_activity(
        &mut self,
        token: u64,
        joint: u64,
        clip: u64,
        idle: u64,
        duration: f64,
        fade: f64,
    ) -> Result<u32, JsValue> {
        if self.scene_animation.actors.iter().any(|actor| {
            actor.player.attachment(joint).is_some()
                && (!actor.player.has_clip(clip) || !actor.player.has_clip(idle))
        }) {
            return Err(JsValue::from_str("activity clip missing"));
        }
        let cue = ActivityCue {
            token,
            joint,
            clip,
            idle,
            duration,
            elapsed: 0.0,
            fade,
            finished: false,
        };
        let scheduled = self
            .scene_animation
            .timeline
            .schedule(cue)
            .map_err(|_| JsValue::from_str("invalid activity cue or timeline capacity"))?;
        if !scheduled {
            return Ok(0);
        }
        let mut selected = 0;
        for actor in &mut self.scene_animation.actors {
            if actor.player.attachment(joint).is_some() {
                actor
                    .player
                    .restart_clip(clip, fade)
                    .map_err(|_| JsValue::from_str("activity start failed"))?;
                selected += 1;
            }
        }
        Ok(selected)
    }
    /// Serializes current animation state to JSON.
    pub fn animation_snapshot_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.scene_animation.timeline)
            .map_err(|_| JsValue::from_str("animation snapshot encoding failed"))
    }
    /// Clears scene activities idle since `idle`.
    pub fn clear_scene_activities(&mut self, idle: u64) -> Result<(), JsValue> {
        self.scene_animation.timeline = ActivityTimeline::default();
        for actor in &mut self.scene_animation.actors {
            if !actor.player.has_clip(idle) {
                continue;
            }
            actor
                .player
                .restart_clip(idle, 0.0)
                .map_err(|_| JsValue::from_str("idle reset failed"))?;
            actor
                .player
                .advance(0.0, self.scene_animation.reduced_motion)
                .map_err(|_| JsValue::from_str("idle frame failed"))?;
        }
        Ok(())
    }
    /// Restores animation state from `json`.
    pub fn restore_animation_snapshot_json(&mut self, json: &str) -> Result<(), JsValue> {
        if json.len() > 16384 {
            return Err(JsValue::from_str("animation snapshot exceeds bound"));
        }
        let timeline: ActivityTimeline = serde_json::from_str(json)
            .map_err(|_| JsValue::from_str("invalid animation snapshot"))?;
        timeline
            .validate()
            .map_err(|_| JsValue::from_str("invalid animation timeline"))?;
        if timeline.cues().any(|cue| {
            self.scene_animation.actors.iter().any(|actor| {
                actor.player.attachment(cue.joint).is_some()
                    && (!actor.player.has_clip(cue.clip) || !actor.player.has_clip(cue.idle))
            })
        }) {
            return Err(JsValue::from_str("snapshot clip missing"));
        }
        self.scene_animation.timeline = timeline;
        self.scene_animation.sync_activity_cues();
        Ok(())
    }
    /// Joint IDs group the pieces of one actor without game-specific naming.
    pub fn select_scene_animation_clip(
        &mut self,
        joint_id: u64,
        clip_id: u64,
        fade_seconds: f64,
    ) -> Result<u32, JsValue> {
        if !fade_seconds.is_finite() || !(0.0..=10.0).contains(&fade_seconds) {
            return Err(JsValue::from_str("invalid animation transition"));
        }
        if self.scene_animation.actors.iter().any(|actor| {
            actor.player.attachment(joint_id).is_some() && !actor.player.has_clip(clip_id)
        }) {
            return Err(JsValue::from_str("actor clip missing"));
        }
        let mut selected = 0;
        for actor in &mut self.scene_animation.actors {
            if actor.player.attachment(joint_id).is_some() {
                actor
                    .player
                    .select_clip(clip_id, fade_seconds)
                    .map_err(|_| JsValue::from_str("actor clip missing"))?;
                selected += 1;
            }
        }
        Ok(selected)
    }
    /// Sets reduced-motion accessibility flag.
    pub fn set_animation_reduced_motion(&mut self, reduced: bool) {
        self.scene_animation.reduced_motion = reduced;
    }
    /// Seeks scene animation to `time`.
    pub fn seek_scene_animation(&mut self, time: f64) -> Result<(), JsValue> {
        if !time.is_finite() || time.abs() > 1e12 {
            return Err(JsValue::from_str("invalid animation seek"));
        }
        for actor in &mut self.scene_animation.actors {
            actor
                .player
                .seek(time)
                .map_err(|_| JsValue::from_str("animation seek failed"))?;
        }
        self.tick_scene_animation(0.0)
    }
    /// Returns an attestation receipt of current animation state.
    pub fn scene_animation_receipt(&self) -> Result<JsValue, JsValue> {
        #[derive(serde::Serialize)]
        struct Receipt {
            actors: usize,
            resident_bytes: usize,
            frames: u64,
            uploaded_vertices: u64,
            failed_uploads: u64,
            reduced_motion: bool,
        }
        let s = &self.scene_animation;
        serde_wasm_bindgen::to_value(&Receipt {
            actors: s.actors.len(),
            resident_bytes: s.resident_bytes,
            frames: s.frames,
            uploaded_vertices: s.uploaded_vertices,
            failed_uploads: s.failed_uploads,
            reduced_motion: s.reduced_motion,
        })
        .map_err(|_| JsValue::from_str("animation receipt encoding failed"))
    }
}
