//! MIDI note utilities and transport state machine.

/// MIDI note names to frequencies (A4 = 440 Hz).
pub fn midi_note_to_freq(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0)
}

/// Convert frequency to nearest MIDI note number.
pub fn freq_to_midi_note(freq: f64) -> u8 {
    let note = 69.0 + 12.0 * (freq / 440.0).log2();
    note.round() as u8
}

/// Note name (e.g. "C4") to MIDI note number.
pub fn note_name_to_midi(name: &str) -> Option<u8> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let bytes = name.as_bytes();
    let pitch_class = match bytes[0].to_ascii_uppercase() {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let mut idx = 1;
    let mut semitone = pitch_class as i32;
    // Sharp/flat.
    if idx < bytes.len() && bytes[idx] == b'#' {
        semitone += 1;
        idx += 1;
    } else if idx < bytes.len() && (bytes[idx] == b'b' || bytes[idx] == b'B') {
        semitone -= 1;
        idx += 1;
    }
    // Octave number.
    let octave_str = &name[idx..];
    let octave: i32 = octave_str.parse().ok()?;
    // MIDI note = (octave + 1) * 12 + semitone.
    let midi = (octave + 1) * 12 + semitone;
    if (0..=127).contains(&midi) {
        Some(midi as u8)
    } else {
        None
    }
}

/// MIDI note number to note name.
pub fn midi_to_note_name(note: u8) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pitch_class = note % 12;
    let octave = (note as i32 / 12) - 1;
    format!("{}{}", names[pitch_class as usize], octave)
}

/// Quantize a beat position to the nearest grid division.
pub fn quantize(position_beats: f64, grid_division: f64) -> f64 {
    if grid_division <= 0.0 {
        return position_beats;
    }
    (position_beats / grid_division).round() * grid_division
}

/// Transpose a MIDI note by semitones (clamped to 0-127).
pub fn transpose(note: u8, semitones: i32) -> u8 {
    let result = note as i32 + semitones;
    result.clamp(0, 127) as u8
}

/// Transport state machine — play, stop, record, loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
    Recording,
}

#[derive(Debug, Clone)]
pub struct Transport {
    pub state: TransportState,
    pub tempo: f64, // BPM
    pub sample_rate: f64,
    pub position_samples: u64,
    pub loop_start: Option<u64>,
    pub loop_end: Option<u64>,
    pub is_looping: bool,
    is_metronome_on: bool,
    metronome_phase: f64,
}

impl Transport {
    pub fn new(tempo: f64, sample_rate: f64) -> Self {
        Self {
            state: TransportState::Stopped,
            tempo,
            sample_rate,
            position_samples: 0,
            loop_start: None,
            loop_end: None,
            is_looping: false,
            is_metronome_on: false,
            metronome_phase: 0.0,
        }
    }

    pub fn play(&mut self) {
        self.state = TransportState::Playing;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.position_samples = 0;
    }

    pub fn pause(&mut self) {
        if self.state == TransportState::Playing {
            self.state = TransportState::Paused;
        }
    }

    pub fn record(&mut self) {
        self.state = TransportState::Recording;
    }

    pub fn set_tempo(&mut self, bpm: f64) {
        self.tempo = bpm.max(1.0);
    }

    pub fn set_loop(&mut self, start: u64, end: u64) {
        self.loop_start = Some(start);
        self.loop_end = Some(end);
        self.is_looping = true;
    }

    pub fn clear_loop(&mut self) {
        self.loop_start = None;
        self.loop_end = None;
        self.is_looping = false;
    }

    pub fn set_metronome(&mut self, on: bool) {
        self.is_metronome_on = on;
    }

    /// Advance one sample. Returns true if at a beat boundary (for metronome).
    pub fn tick(&mut self) -> bool {
        if self.state == TransportState::Stopped {
            return false;
        }

        let samples_per_beat = 60.0 * self.sample_rate / self.tempo;
        let beat_position = self.position_samples as f64 / samples_per_beat;
        let beat_phase = beat_position - beat_position.floor();

        // Metronome click at beat boundaries.
        let click = self.is_metronome_on && beat_phase < 1.0 / samples_per_beat;

        self.position_samples += 1;

        // Loop handling.
        if self.is_looping {
            if let Some(end) = self.loop_end {
                if self.position_samples >= end {
                    if let Some(start) = self.loop_start {
                        self.position_samples = start;
                    }
                }
            }
        }

        click
    }

    /// Current position in beats.
    pub fn position_beats(&self) -> f64 {
        let samples_per_beat = 60.0 * self.sample_rate / self.tempo;
        self.position_samples as f64 / samples_per_beat
    }

    /// Current position in seconds.
    pub fn position_seconds(&self) -> f64 {
        self.position_samples as f64 / self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_note_freq_a4() {
        assert!((midi_note_to_freq(69) - 440.0).abs() < 0.01);
    }

    #[test]
    fn midi_note_freq_c0() {
        // C0 = MIDI 12, freq ≈ 16.35
        assert!((midi_note_to_freq(12) - 16.35).abs() < 0.1);
    }

    #[test]
    fn freq_to_midi_a4() {
        assert_eq!(freq_to_midi_note(440.0), 69);
    }

    #[test]
    fn note_name_to_midi_basic() {
        assert_eq!(note_name_to_midi("A4"), Some(69));
        assert_eq!(note_name_to_midi("C4"), Some(60));
        assert_eq!(note_name_to_midi("C#4"), Some(61));
        assert_eq!(note_name_to_midi("Bb3"), Some(58));
    }

    #[test]
    fn note_name_to_midi_invalid() {
        assert_eq!(note_name_to_midi("H4"), None);
        assert_eq!(note_name_to_midi(""), None);
    }

    #[test]
    fn midi_to_note_name_roundtrip() {
        assert_eq!(midi_to_note_name(69), "A4");
        assert_eq!(midi_to_note_name(60), "C4");
        assert_eq!(midi_to_note_name(61), "C#4");
    }

    #[test]
    fn quantize_to_quarter() {
        let result = quantize(1.3, 0.25);
        assert!((result - 1.25).abs() < 1e-10);
    }

    #[test]
    fn quantize_to_eighth() {
        let result = quantize(1.1, 0.5);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn transpose_up() {
        assert_eq!(transpose(60, 5), 65);
    }

    #[test]
    fn transpose_down() {
        assert_eq!(transpose(60, -12), 48);
    }

    #[test]
    fn transpose_clamped() {
        assert_eq!(transpose(0, -5), 0);
        assert_eq!(transpose(127, 5), 127);
    }

    #[test]
    fn transport_play_stop() {
        let mut t = Transport::new(120.0, 44100.0);
        assert_eq!(t.state, TransportState::Stopped);
        t.play();
        assert_eq!(t.state, TransportState::Playing);
        t.stop();
        assert_eq!(t.state, TransportState::Stopped);
        assert_eq!(t.position_samples, 0);
    }

    #[test]
    fn transport_pause() {
        let mut t = Transport::new(120.0, 44100.0);
        t.play();
        for _ in 0..100 {
            t.tick();
        }
        t.pause();
        assert_eq!(t.state, TransportState::Paused);
    }

    #[test]
    fn transport_position() {
        let mut t = Transport::new(120.0, 44100.0);
        t.play();
        for _ in 0..44100 {
            t.tick();
        }
        // At 120 BPM, samples_per_beat = 60 * 44100 / 120 = 22050.
        // 44100 samples = 2 beats.
        assert!((t.position_beats() - 2.0).abs() < 0.01);
    }

    #[test]
    fn transport_loop() {
        let mut t = Transport::new(120.0, 44100.0);
        t.set_loop(100, 200);
        t.play();
        for _ in 0..200 {
            t.tick();
        }
        // Should have looped back to start.
        assert!(t.position_samples < 200);
    }

    #[test]
    fn transport_metronome_click() {
        let mut t = Transport::new(120.0, 44100.0);
        t.set_metronome(true);
        t.play();
        // At 120 BPM, one beat = 22050 samples. First tick should click.
        let click = t.tick();
        assert!(click);
    }

    #[test]
    fn transport_tempo_change() {
        let mut t = Transport::new(120.0, 44100.0);
        t.set_tempo(60.0);
        assert_eq!(t.tempo, 60.0);
    }
}
