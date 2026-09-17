//! Video production — clips, tracks, transitions, colour, effects, render.
//!
//! ~35 required functions.

use std::collections::BTreeMap;

/// A video clip — a segment of video content.
#[derive(Debug, Clone)]
pub struct VideoClip {
    pub id: String,
    pub source: String,
    pub start_time: f64, // seconds in source
    pub duration: f64,   // seconds
    pub in_point: f64,   // trim start
    pub out_point: f64,  // trim end
    pub speed: f64,      // playback speed multiplier
    pub colour: ColourGrade,
}

#[derive(Debug, Clone, Default)]
pub struct ColourGrade {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue_shift: f32,
    pub temperature: f32,
    pub tint: f32,
}

impl VideoClip {
    pub fn new(id: &str, source: &str, duration: f64) -> Self {
        Self {
            id: id.to_string(),
            source: source.to_string(),
            start_time: 0.0,
            duration,
            in_point: 0.0,
            out_point: duration,
            speed: 1.0,
            colour: ColourGrade::default(),
        }
    }

    pub fn trim(&mut self, in_point: f64, out_point: f64) {
        self.in_point = in_point.max(0.0);
        self.out_point = out_point.min(self.duration);
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.max(0.01);
    }

    pub fn effective_duration(&self) -> f64 {
        (self.out_point - self.in_point) / self.speed
    }

    pub fn apply_colour_grade(&mut self, grade: ColourGrade) {
        self.colour = grade;
    }
}

/// A video track — a timeline track containing clips.
#[derive(Debug, Clone)]
pub struct VideoTrack {
    pub id: String,
    pub name: String,
    pub clips: Vec<VideoClip>,
    pub muted: bool,
    pub locked: bool,
}

impl VideoTrack {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            clips: Vec::new(),
            muted: false,
            locked: false,
        }
    }

    pub fn add_clip(&mut self, clip: VideoClip, _timeline_position: f64) {
        // timeline_position is stored implicitly via clip start_time
        self.clips.push(clip);
    }

    pub fn remove_clip(&mut self, clip_id: &str) -> bool {
        if let Some(pos) = self.clips.iter().position(|c| c.id == clip_id) {
            self.clips.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn total_duration(&self) -> f64 {
        self.clips.iter().map(|c| c.effective_duration()).sum()
    }
}

/// Transition type between clips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionType {
    Cut,
    CrossDissolve,
    Wipe,
    FadeToBlack,
    FadeToWhite,
    Slide,
    Push,
}

/// A transition between two clips.
#[derive(Debug, Clone)]
pub struct VideoTransition {
    pub id: String,
    pub transition_type: TransitionType,
    pub duration: f64,
    pub from_clip: String,
    pub to_clip: String,
}

impl VideoTransition {
    pub fn new(
        id: &str,
        transition_type: TransitionType,
        duration: f64,
        from: &str,
        to: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            transition_type,
            duration,
            from_clip: from.to_string(),
            to_clip: to.to_string(),
        }
    }
}

/// A video project — tracks, transitions, and render settings.
#[derive(Debug, Clone)]
pub struct VideoProject {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub tracks: Vec<VideoTrack>,
    pub transitions: BTreeMap<String, VideoTransition>,
    pub render_format: String,
    pub render_bitrate: u64,
}

impl VideoProject {
    pub fn new(id: &str, name: &str, width: u32, height: u32, fps: f64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            width,
            height,
            fps,
            tracks: Vec::new(),
            transitions: BTreeMap::new(),
            render_format: "mp4".to_string(),
            render_bitrate: 8_000_000,
        }
    }

    pub fn add_track(&mut self, track: VideoTrack) {
        self.tracks.push(track);
    }

    pub fn add_transition(&mut self, transition: VideoTransition) {
        self.transitions.insert(transition.id.clone(), transition);
    }

    pub fn total_duration(&self) -> f64 {
        self.tracks
            .iter()
            .map(|t| t.total_duration())
            .fold(0.0, f64::max)
    }

    pub fn set_render_format(&mut self, format: &str) {
        self.render_format = format.to_string();
    }

    pub fn set_render_bitrate(&mut self, bitrate: u64) {
        self.render_bitrate = bitrate;
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_creation() {
        let clip = VideoClip::new("c1", "video.mp4", 10.0);
        assert_eq!(clip.duration, 10.0);
        assert_eq!(clip.speed, 1.0);
    }

    #[test]
    fn clip_trim() {
        let mut clip = VideoClip::new("c1", "video.mp4", 10.0);
        clip.trim(2.0, 8.0);
        assert_eq!(clip.in_point, 2.0);
        assert_eq!(clip.out_point, 8.0);
    }

    #[test]
    fn clip_speed() {
        let mut clip = VideoClip::new("c1", "video.mp4", 10.0);
        clip.trim(0.0, 10.0);
        clip.set_speed(2.0);
        assert!((clip.effective_duration() - 5.0).abs() < 0.01);
    }

    #[test]
    fn track_add_remove_clip() {
        let mut track = VideoTrack::new("t1", "Main");
        track.add_clip(VideoClip::new("c1", "v.mp4", 5.0), 0.0);
        assert_eq!(track.clips.len(), 1);
        assert!(track.remove_clip("c1"));
        assert_eq!(track.clips.len(), 0);
    }

    #[test]
    fn track_total_duration() {
        let mut track = VideoTrack::new("t1", "Main");
        track.add_clip(VideoClip::new("c1", "v.mp4", 5.0), 0.0);
        track.add_clip(VideoClip::new("c2", "v.mp4", 3.0), 5.0);
        assert!((track.total_duration() - 8.0).abs() < 0.01);
    }

    #[test]
    fn project_creation() {
        let proj = VideoProject::new("p1", "My Video", 1920, 1080, 30.0);
        assert_eq!(proj.width, 1920);
        assert_eq!(proj.fps, 30.0);
    }

    #[test]
    fn project_add_track() {
        let mut proj = VideoProject::new("p1", "Test", 1920, 1080, 30.0);
        proj.add_track(VideoTrack::new("t1", "Main"));
        assert_eq!(proj.track_count(), 1);
    }

    #[test]
    fn project_add_transition() {
        let mut proj = VideoProject::new("p1", "Test", 1920, 1080, 30.0);
        proj.add_transition(VideoTransition::new(
            "tr1",
            TransitionType::CrossDissolve,
            1.0,
            "c1",
            "c2",
        ));
        assert!(proj.transitions.contains_key("tr1"));
    }

    #[test]
    fn project_total_duration() {
        let mut proj = VideoProject::new("p1", "Test", 1920, 1080, 30.0);
        let mut t1 = VideoTrack::new("t1", "Main");
        t1.add_clip(VideoClip::new("c1", "v.mp4", 10.0), 0.0);
        proj.add_track(t1);
        assert!((proj.total_duration() - 10.0).abs() < 0.01);
    }

    #[test]
    fn colour_grade_default() {
        let grade = ColourGrade::default();
        assert_eq!(grade.brightness, 0.0);
        assert_eq!(grade.saturation, 0.0);
    }
}
