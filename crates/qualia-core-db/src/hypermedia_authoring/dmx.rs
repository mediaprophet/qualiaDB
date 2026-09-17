//! DMX / fixtures / lighting / projection / cue stack / show control.
//!
//! ~25 required functions.

use std::collections::BTreeMap;

/// A DMX universe (512 channels).
#[derive(Debug, Clone)]
pub struct DmxUniverse {
    pub id: u16,
    pub channels: [u8; 512],
}

impl DmxUniverse {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            channels: [0; 512],
        }
    }

    pub fn set_channel(&mut self, channel: u16, value: u8) -> bool {
        if channel < 512 {
            self.channels[channel as usize] = value;
            true
        } else {
            false
        }
    }

    pub fn get_channel(&self, channel: u16) -> Option<u8> {
        if channel < 512 {
            Some(self.channels[channel as usize])
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.channels = [0; 512];
    }
}

/// A DMX fixture with channel mapping.
#[derive(Debug, Clone)]
pub struct DmxFixture {
    pub id: String,
    pub name: String,
    pub fixture_type: FixtureType,
    pub universe: u16,
    pub start_channel: u16,
    pub channel_count: u16,
    pub channel_map: BTreeMap<String, u16>, // function name -> offset
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureType {
    MovingHead,
    ParCan,
    StripLight,
    Laser,
    Hazer,
    FogMachine,
    Spotlight,
    Generic,
}

impl DmxFixture {
    pub fn new(
        id: &str,
        name: &str,
        fixture_type: FixtureType,
        universe: u16,
        start_channel: u16,
        channel_count: u16,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            fixture_type,
            universe,
            start_channel,
            channel_count,
            channel_map: BTreeMap::new(),
        }
    }

    pub fn map_channel(&mut self, function: &str, offset: u16) {
        self.channel_map.insert(function.to_string(), offset);
    }

    /// Set a function value on this fixture in a universe.
    pub fn set_function(&self, universe: &mut DmxUniverse, function: &str, value: u8) -> bool {
        if let Some(&offset) = self.channel_map.get(function) {
            let channel = self.start_channel + offset;
            universe.set_channel(channel, value)
        } else {
            false
        }
    }

    /// Set the colour (RGB) on this fixture.
    pub fn set_colour(&self, universe: &mut DmxUniverse, r: u8, g: u8, b: u8) -> bool {
        let mut ok = true;
        ok &= self.set_function(universe, "red", r);
        ok &= self.set_function(universe, "green", g);
        ok &= self.set_function(universe, "blue", b);
        ok
    }

    /// Set pan/tilt for moving heads.
    pub fn set_pan_tilt(&self, universe: &mut DmxUniverse, pan: u8, tilt: u8) -> bool {
        let mut ok = true;
        ok &= self.set_function(universe, "pan", pan);
        ok &= self.set_function(universe, "tilt", tilt);
        ok
    }

    /// Set intensity (0-255).
    pub fn set_intensity(&self, universe: &mut DmxUniverse, intensity: u8) -> bool {
        self.set_function(universe, "intensity", intensity)
    }
}

/// A cue in a cue stack.
#[derive(Debug, Clone)]
pub struct Cue {
    pub id: String,
    pub name: String,
    pub duration: f64, // seconds for fade
    pub fade_in: f64,
    pub fade_out: f64,
    pub channel_values: BTreeMap<(u16, u16), u8>, // (universe, channel) -> value
    pub triggers: Vec<String>,
}

impl Cue {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            duration: 0.0,
            fade_in: 0.0,
            fade_out: 0.0,
            channel_values: BTreeMap::new(),
            triggers: Vec::new(),
        }
    }

    pub fn set_channel(&mut self, universe: u16, channel: u16, value: u8) {
        self.channel_values.insert((universe, channel), value);
    }

    pub fn set_fade(&mut self, fade_in: f64, fade_out: f64) {
        self.fade_in = fade_in.max(0.0);
        self.fade_out = fade_out.max(0.0);
    }

    pub fn add_trigger(&mut self, trigger: &str) {
        self.triggers.push(trigger.to_string());
    }

    /// Apply this cue's values to the given universes.
    pub fn apply(&self, universes: &mut BTreeMap<u16, DmxUniverse>) {
        for ((universe_id, channel), value) in &self.channel_values {
            if let Some(universe) = universes.get_mut(universe_id) {
                universe.set_channel(*channel, *value);
            }
        }
    }

    /// Interpolate between this cue and the next, at time t [0, 1].
    pub fn interpolate_to(&self, next: &Cue, t: f32) -> BTreeMap<(u16, u16), u8> {
        let t = t.clamp(0.0, 1.0);
        let mut result = BTreeMap::new();
        for (key, &value) in &self.channel_values {
            let next_value = next.channel_values.get(key).copied().unwrap_or(value);
            let interpolated = (value as f32 * (1.0 - t) + next_value as f32 * t).round() as u8;
            result.insert(*key, interpolated);
        }
        for (key, &value) in &next.channel_values {
            if !self.channel_values.contains_key(key) {
                result.insert(*key, (value as f32 * t).round() as u8);
            }
        }
        result
    }
}

/// A cue stack — ordered sequence of cues for a show.
#[derive(Debug, Clone)]
pub struct CueStack {
    pub id: String,
    pub name: String,
    pub cues: Vec<Cue>,
    pub current_index: usize,
    pub auto_follow: bool,
}

impl CueStack {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            cues: Vec::new(),
            current_index: 0,
            auto_follow: false,
        }
    }

    pub fn add_cue(&mut self, cue: Cue) {
        self.cues.push(cue);
    }

    pub fn go(&mut self, universes: &mut BTreeMap<u16, DmxUniverse>) -> bool {
        if self.current_index < self.cues.len() {
            self.cues[self.current_index].apply(universes);
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    pub fn go_back(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    pub fn current_cue(&self) -> Option<&Cue> {
        self.cues.get(self.current_index)
    }

    pub fn cue_count(&self) -> usize {
        self.cues.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universe_creation() {
        let uni = DmxUniverse::new(1);
        assert_eq!(uni.id, 1);
        assert_eq!(uni.channels[0], 0);
    }

    #[test]
    fn universe_set_get_channel() {
        let mut uni = DmxUniverse::new(1);
        assert!(uni.set_channel(0, 128));
        assert_eq!(uni.get_channel(0), Some(128));
        assert!(!uni.set_channel(512, 255)); // out of range
    }

    #[test]
    fn fixture_creation() {
        let fixture = DmxFixture::new("f1", "Moving Head 1", FixtureType::MovingHead, 1, 0, 16);
        assert_eq!(fixture.fixture_type, FixtureType::MovingHead);
        assert_eq!(fixture.channel_count, 16);
    }

    #[test]
    fn fixture_set_colour() {
        let mut fixture = DmxFixture::new("f1", "Par 1", FixtureType::ParCan, 1, 0, 7);
        fixture.map_channel("red", 0);
        fixture.map_channel("green", 1);
        fixture.map_channel("blue", 2);
        let mut uni = DmxUniverse::new(1);
        assert!(fixture.set_colour(&mut uni, 255, 0, 128));
        assert_eq!(uni.get_channel(0), Some(255));
        assert_eq!(uni.get_channel(1), Some(0));
        assert_eq!(uni.get_channel(2), Some(128));
    }

    #[test]
    fn fixture_set_pan_tilt() {
        let mut fixture = DmxFixture::new("f1", "MH1", FixtureType::MovingHead, 1, 0, 16);
        fixture.map_channel("pan", 0);
        fixture.map_channel("tilt", 1);
        let mut uni = DmxUniverse::new(1);
        assert!(fixture.set_pan_tilt(&mut uni, 128, 64));
        assert_eq!(uni.get_channel(0), Some(128));
        assert_eq!(uni.get_channel(1), Some(64));
    }

    #[test]
    fn cue_creation() {
        let cue = Cue::new("c1", "Scene 1");
        assert_eq!(cue.name, "Scene 1");
        assert_eq!(cue.fade_in, 0.0);
    }

    #[test]
    fn cue_set_channels() {
        let mut cue = Cue::new("c1", "Scene 1");
        cue.set_channel(1, 0, 255);
        cue.set_channel(1, 1, 128);
        assert_eq!(cue.channel_values.len(), 2);
    }

    #[test]
    fn cue_apply() {
        let mut cue = Cue::new("c1", "Scene 1");
        cue.set_channel(1, 0, 255);
        let mut universes = BTreeMap::new();
        universes.insert(1, DmxUniverse::new(1));
        cue.apply(&mut universes);
        assert_eq!(universes.get(&1).unwrap().get_channel(0), Some(255));
    }

    #[test]
    fn cue_interpolate() {
        let mut c1 = Cue::new("c1", "A");
        c1.set_channel(1, 0, 0);
        let mut c2 = Cue::new("c2", "B");
        c2.set_channel(1, 0, 255);
        let mid = c1.interpolate_to(&c2, 0.5);
        assert_eq!(mid.get(&(1, 0)), Some(&128));
    }

    #[test]
    fn cue_stack_go() {
        let mut stack = CueStack::new("s1", "Show");
        let mut c1 = Cue::new("c1", "Scene 1");
        c1.set_channel(1, 0, 255);
        stack.add_cue(c1);
        let mut universes = BTreeMap::new();
        universes.insert(1, DmxUniverse::new(1));
        assert!(stack.go(&mut universes));
        assert_eq!(stack.current_index, 1);
        assert!(!stack.go(&mut universes)); // no more cues
    }

    #[test]
    fn cue_stack_go_back() {
        let mut stack = CueStack::new("s1", "Show");
        stack.add_cue(Cue::new("c1", "A"));
        stack.add_cue(Cue::new("c2", "B"));
        stack.current_index = 1;
        assert!(stack.go_back());
        assert_eq!(stack.current_index, 0);
    }

    #[test]
    fn cue_stack_reset() {
        let mut stack = CueStack::new("s1", "Show");
        stack.add_cue(Cue::new("c1", "A"));
        stack.current_index = 1;
        stack.reset();
        assert_eq!(stack.current_index, 0);
    }
}
