//! Audio DAW invoke seams — oscillator, envelope, filter, LFO, effects, MIDI, transport, meters.
//!
//! Exposes the `audio::dsp` module through VibeScript invoke IDs.

use super::super::args;
use crate::audio::dsp;
use poet_vibe::{Diagnostic, Span, Value};

/// `Audio.oscillator` — render a waveform buffer.
///
/// Takes `waveform` (string: "sine"/"square"/"sawtooth"/"triangle"),
/// `frequency` (f64, Hz), `sample_rate` (f64), `n` (u64, sample count),
/// and optional `gain` (f64). Returns list of f64 samples.
pub fn oscillator(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let waveform_str = args::rec_str(args, "waveform")
        .ok_or_else(|| args::bad(span, "Audio.oscillator needs waveform"))?;
    let waveform = match waveform_str {
        "sine" => dsp::Waveform::Sine,
        "square" => dsp::Waveform::Square,
        "sawtooth" => dsp::Waveform::Sawtooth,
        "triangle" => dsp::Waveform::Triangle,
        _ => {
            return Err(args::bad(
                span,
                format!("Audio.oscillator: unknown waveform '{waveform_str}'"),
            ))
        }
    };
    let frequency = args::rec_f64(args, "frequency")
        .ok_or_else(|| args::bad(span, "Audio.oscillator needs frequency"))?;
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);
    let n = args::rec_u64(args, "n").unwrap_or(1024) as usize;
    let gain = args::rec_f64(args, "gain").unwrap_or(1.0);

    let mut osc = dsp::Oscillator::new(waveform, frequency, sample_rate);
    osc.set_gain(gain);
    let mut samples = vec![0.0f64; n];
    osc.render(&mut samples);
    Ok(args::f64_list_value(samples))
}

/// `Audio.envelope` — render an ADSR envelope buffer.
///
/// Takes `attack`, `decay`, `sustain`, `release` (f64, seconds/level),
/// `sample_rate` (f64), `n` (u64, sample count), and `note_on_samples`
/// (u64, when to trigger note-off). Returns list of f64 values.
pub fn envelope(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let attack = args::rec_f64(args, "attack").unwrap_or(0.01);
    let decay = args::rec_f64(args, "decay").unwrap_or(0.1);
    let sustain = args::rec_f64(args, "sustain").unwrap_or(0.7);
    let release = args::rec_f64(args, "release").unwrap_or(0.2);
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);
    let n = args::rec_u64(args, "n").unwrap_or(44100) as usize;
    let note_off = args::rec_u64(args, "note_off_samples").unwrap_or(n as u64 / 2) as usize;

    let mut env = dsp::AdsrEnvelope::new(attack, decay, sustain, release, sample_rate);
    env.note_on();
    let mut samples = vec![0.0f64; n];
    for (i, s) in samples.iter_mut().enumerate() {
        if i == note_off {
            env.note_off();
        }
        *s = env.tick();
    }
    Ok(args::f64_list_value(samples))
}

/// `Audio.filter` — apply a biquad filter to a signal.
///
/// Takes `input` (list of f64), `filter_type` (string: "lowpass"/"highpass"/"bandpass"/"notch"),
/// `cutoff` (f64, Hz), `q` (f64), and `sample_rate` (f64). Returns filtered list.
pub fn filter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = args::rec_f64_list(args, "input")
        .ok_or_else(|| args::bad(span, "Audio.filter needs input"))?;
    let filter_type_str = args::rec_str(args, "filter_type")
        .ok_or_else(|| args::bad(span, "Audio.filter needs filter_type"))?;
    let filter_type = match filter_type_str {
        "lowpass" | "lp" => dsp::FilterType::LowPass,
        "highpass" | "hp" => dsp::FilterType::HighPass,
        "bandpass" | "bp" => dsp::FilterType::BandPass,
        "notch" => dsp::FilterType::Notch,
        _ => {
            return Err(args::bad(
                span,
                format!("Audio.filter: unknown filter_type '{filter_type_str}'"),
            ))
        }
    };
    let cutoff = args::rec_f64(args, "cutoff")
        .ok_or_else(|| args::bad(span, "Audio.filter needs cutoff"))?;
    let q = args::rec_f64(args, "q").unwrap_or(0.707);
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);

    let mut filt = dsp::BiquadFilter::new(filter_type, cutoff, q, sample_rate);
    let mut output = vec![0.0f64; input.len()];
    filt.process(&input, &mut output);
    Ok(args::f64_list_value(output))
}

/// `Audio.lfo` — render an LFO modulation buffer.
///
/// Takes `waveform` (string), `frequency` (f64, Hz), `sample_rate` (f64),
/// `n` (u64), and `depth` (f64). Returns list of f64 modulation values.
pub fn lfo(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let waveform_str = args::rec_str(args, "waveform").unwrap_or("sine");
    let waveform = match waveform_str {
        "sine" => dsp::Waveform::Sine,
        "square" => dsp::Waveform::Square,
        "sawtooth" => dsp::Waveform::Sawtooth,
        "triangle" => dsp::Waveform::Triangle,
        _ => {
            return Err(args::bad(
                span,
                format!("Audio.lfo: unknown waveform '{waveform_str}'"),
            ))
        }
    };
    let frequency = args::rec_f64(args, "frequency")
        .ok_or_else(|| args::bad(span, "Audio.lfo needs frequency"))?;
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);
    let n = args::rec_u64(args, "n").unwrap_or(1024) as usize;
    let depth = args::rec_f64(args, "depth").unwrap_or(1.0);

    let mut lfo = dsp::Lfo::new(waveform, frequency, sample_rate, depth);
    let mut samples = vec![0.0f64; n];
    lfo.render(&mut samples);
    Ok(args::f64_list_value(samples))
}

/// `Audio.delay` — apply a delay effect to a signal.
///
/// Takes `input` (list of f64), `delay_samples` (u64), `feedback` (f64),
/// and `mix` (f64). Returns processed list.
pub fn delay(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = args::rec_f64_list(args, "input")
        .ok_or_else(|| args::bad(span, "Audio.delay needs input"))?;
    let delay_samples = args::rec_u64(args, "delay_samples").unwrap_or(4410) as usize;
    let feedback = args::rec_f64(args, "feedback").unwrap_or(0.3);
    let mix = args::rec_f64(args, "mix").unwrap_or(0.5);

    let mut delay = dsp::Delay::new(delay_samples.max(1), feedback, mix);
    let mut output = vec![0.0f64; input.len()];
    delay.process(&input, &mut output);
    Ok(args::f64_list_value(output))
}

/// `Audio.reverb` — apply a reverb effect to a signal.
///
/// Takes `input` (list of f64), `room_size` (f64), `damping` (f64),
/// `mix` (f64), and `sample_rate` (f64). Returns processed list.
pub fn reverb(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = args::rec_f64_list(args, "input")
        .ok_or_else(|| args::bad(span, "Audio.reverb needs input"))?;
    let room_size = args::rec_f64(args, "room_size").unwrap_or(0.5);
    let damping = args::rec_f64(args, "damping").unwrap_or(0.3);
    let mix = args::rec_f64(args, "mix").unwrap_or(0.3);
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);

    let mut reverb = dsp::Reverb::new(sample_rate, room_size, damping, mix);
    let mut output = vec![0.0f64; input.len()];
    reverb.process(&input, &mut output);
    Ok(args::f64_list_value(output))
}

/// `Audio.compressor` — apply dynamic range compression.
///
/// Takes `input` (list of f64), `threshold` (f64, dB), `ratio` (f64),
/// `attack` (f64, seconds), `release` (f64, seconds), and `sample_rate` (f64).
pub fn compressor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = args::rec_f64_list(args, "input")
        .ok_or_else(|| args::bad(span, "Audio.compressor needs input"))?;
    let threshold = args::rec_f64(args, "threshold").unwrap_or(-20.0);
    let ratio = args::rec_f64(args, "ratio").unwrap_or(4.0);
    let attack = args::rec_f64(args, "attack").unwrap_or(0.003);
    let release = args::rec_f64(args, "release").unwrap_or(0.1);
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);

    let mut comp = dsp::Compressor::new(threshold, ratio, attack, release, sample_rate);
    let mut output = vec![0.0f64; input.len()];
    comp.process(&input, &mut output);
    Ok(args::f64_list_value(output))
}

/// `Audio.eq` — apply a 3-band EQ.
///
/// Takes `input` (list of f64), `low_gain`, `mid_gain`, `high_gain` (f64, dB),
/// and `sample_rate` (f64).
pub fn eq(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input =
        args::rec_f64_list(args, "input").ok_or_else(|| args::bad(span, "Audio.eq needs input"))?;
    let low_gain = args::rec_f64(args, "low_gain").unwrap_or(0.0);
    let mid_gain = args::rec_f64(args, "mid_gain").unwrap_or(0.0);
    let high_gain = args::rec_f64(args, "high_gain").unwrap_or(0.0);
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);

    let mut eq = dsp::Equalizer::new(sample_rate);
    eq.set_band_gains(low_gain, mid_gain, high_gain);
    let mut output = vec![0.0f64; input.len()];
    eq.process(&input, &mut output);
    Ok(args::f64_list_value(output))
}

/// `Audio.midi_note` — convert between MIDI note numbers, names, and frequencies.
///
/// Takes `action` (string: "to_freq", "to_name", "from_name", "from_freq")
/// and `value` (number or string depending on action).
pub fn midi_note(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let action = args::rec_str(args, "action")
        .ok_or_else(|| args::bad(span, "Audio.midi_note needs action"))?;
    match action {
        "to_freq" => {
            let note = args::rec_u64(args, "note")
                .ok_or_else(|| args::bad(span, "Audio.midi_note needs note"))?
                as u8;
            let freq = dsp::midi_note_to_freq(note);
            Ok(args::record([
                ("frequency", Value::F64(freq)),
                ("note", Value::U64(note as u64)),
            ]))
        }
        "to_name" => {
            let note = args::rec_u64(args, "note")
                .ok_or_else(|| args::bad(span, "Audio.midi_note needs note"))?
                as u8;
            let name = dsp::midi_to_note_name(note);
            Ok(args::record([
                ("name", Value::String(name)),
                ("note", Value::U64(note as u64)),
            ]))
        }
        "from_name" => {
            let name = args::rec_str(args, "name")
                .ok_or_else(|| args::bad(span, "Audio.midi_note needs name"))?;
            match dsp::note_name_to_midi(name) {
                Some(note) => Ok(args::record([
                    ("note", Value::U64(note as u64)),
                    ("name", Value::String(name.to_string())),
                ])),
                None => Err(args::bad(
                    span,
                    format!("Audio.midi_note: invalid note name '{name}'"),
                )),
            }
        }
        "from_freq" => {
            let freq = args::rec_f64(args, "frequency")
                .ok_or_else(|| args::bad(span, "Audio.midi_note needs frequency"))?;
            let note = dsp::freq_to_midi_note(freq);
            Ok(args::record([
                ("note", Value::U64(note as u64)),
                ("frequency", Value::F64(freq)),
            ]))
        }
        _ => Err(args::bad(
            span,
            format!("Audio.midi_note: unknown action '{action}'"),
        )),
    }
}

/// `Audio.quantize` — quantize a beat position to a grid.
///
/// Takes `position` (f64, beats) and `grid` (f64, beats per grid division).
pub fn quantize(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let position = args::rec_f64(args, "position")
        .ok_or_else(|| args::bad(span, "Audio.quantize needs position"))?;
    let grid =
        args::rec_f64(args, "grid").ok_or_else(|| args::bad(span, "Audio.quantize needs grid"))?;
    let result = dsp::quantize(position, grid);
    Ok(args::record([
        ("position", Value::F64(result)),
        ("original", Value::F64(position)),
        ("grid", Value::F64(grid)),
    ]))
}

/// `Audio.transpose` — transpose a MIDI note by semitones.
///
/// Takes `note` (u64) and `semitones` (i64).
pub fn transpose(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let note = args::rec_u64(args, "note")
        .ok_or_else(|| args::bad(span, "Audio.transpose needs note"))? as u8;
    let semitones = args::rec_i64(args, "semitones")
        .ok_or_else(|| args::bad(span, "Audio.transpose needs semitones"))?
        as i32;
    let result = dsp::transpose(note, semitones);
    Ok(args::record([
        ("note", Value::U64(result as u64)),
        ("original", Value::U64(note as u64)),
        ("semitones", Value::I64(semitones as i64)),
    ]))
}

/// `Audio.transport` — transport state machine control.
///
/// Takes `action` (string: "play"/"stop"/"pause"/"record"/"status"),
/// `tempo` (f64, BPM), `sample_rate` (f64), and optional loop parameters.
pub fn transport(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let action = args::rec_str(args, "action")
        .ok_or_else(|| args::bad(span, "Audio.transport needs action"))?;
    let tempo = args::rec_f64(args, "tempo").unwrap_or(120.0);
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);

    let mut t = dsp::Transport::new(tempo, sample_rate);
    match action {
        "play" => t.play(),
        "stop" => t.stop(),
        "pause" => t.pause(),
        "record" => t.record(),
        "status" => {}
        _ => {
            return Err(args::bad(
                span,
                format!("Audio.transport: unknown action '{action}'"),
            ))
        }
    }

    // Optional loop setup.
    if let (Some(start), Some(end)) = (
        args::rec_u64(args, "loop_start"),
        args::rec_u64(args, "loop_end"),
    ) {
        t.set_loop(start, end);
    }

    Ok(args::record([
        ("state", Value::String(format!("{:?}", t.state))),
        ("tempo", Value::F64(t.tempo)),
        ("position_samples", Value::U64(t.position_samples)),
        ("position_beats", Value::F64(t.position_beats())),
        ("position_seconds", Value::F64(t.position_seconds())),
        ("is_looping", Value::Bool(t.is_looping)),
    ]))
}

/// `Audio.waveform_meter` — analyse a signal for peak/RMS and waveform display.
///
/// Takes `input` (list of f64) and optional `buckets` (u64 for display).
pub fn waveform_meter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = args::rec_f64_list(args, "input")
        .ok_or_else(|| args::bad(span, "Audio.waveform_meter needs input"))?;
    let buckets = args::rec_u64(args, "buckets").unwrap_or(100) as usize;

    let mut meter = dsp::WaveformMeter::new();
    meter.analyse(&input);

    let display = dsp::WaveformMeter::waveform_display(&input, buckets);
    let display_values: Vec<Value> = display
        .iter()
        .map(|(min, max)| Value::List(vec![Value::F64(*min), Value::F64(*max)]))
        .collect();

    Ok(args::record([
        ("peak", Value::F64(meter.peak)),
        ("rms", Value::F64(meter.rms)),
        ("display", Value::List(display_values)),
    ]))
}

/// `Audio.phase_meter` — measure stereo phase correlation.
///
/// Takes `left` and `right` (lists of f64).
pub fn phase_meter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let left = args::rec_f64_list(args, "left")
        .ok_or_else(|| args::bad(span, "Audio.phase_meter needs left"))?;
    let right = args::rec_f64_list(args, "right")
        .ok_or_else(|| args::bad(span, "Audio.phase_meter needs right"))?;

    let mut meter = dsp::PhaseMeter::new();
    meter.analyse(&left, &right);
    Ok(args::record([(
        "correlation",
        Value::F64(meter.correlation),
    )]))
}

/// `Audio.loudness_meter` — measure LUFS loudness.
///
/// Takes `input` (list of f64) and `sample_rate` (f64).
pub fn loudness_meter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = args::rec_f64_list(args, "input")
        .ok_or_else(|| args::bad(span, "Audio.loudness_meter needs input"))?;
    let sample_rate = args::rec_f64(args, "sample_rate").unwrap_or(44100.0);

    let mut meter = dsp::LoudnessMeter::new(sample_rate);
    meter.process(&input);
    Ok(args::record([
        ("momentary_lufs", Value::F64(meter.momentary_lufs)),
        ("short_term_lufs", Value::F64(meter.short_term_lufs)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn oscillator_sine() {
        let mut m = BTreeMap::new();
        m.insert("waveform".into(), Value::String("sine".into()));
        m.insert("frequency".into(), Value::F64(440.0));
        m.insert("n".into(), Value::U64(100));
        let result = oscillator(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::List(samples) => {
                assert_eq!(samples.len(), 100);
                // Check some samples are non-zero.
                assert!(samples.iter().any(|s| {
                    if let Value::F64(v) = s {
                        v.abs() > 0.01
                    } else {
                        false
                    }
                }));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn oscillator_unknown_waveform() {
        let mut m = BTreeMap::new();
        m.insert("waveform".into(), Value::String("noise".into()));
        m.insert("frequency".into(), Value::F64(440.0));
        let result = oscillator(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn envelope_basic() {
        let mut m = BTreeMap::new();
        m.insert("attack".into(), Value::F64(0.01));
        m.insert("decay".into(), Value::F64(0.1));
        m.insert("sustain".into(), Value::F64(0.7));
        m.insert("release".into(), Value::F64(0.2));
        m.insert("n".into(), Value::U64(1000));
        let result = envelope(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::List(samples) => assert_eq!(samples.len(), 1000),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn filter_lowpass() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.5); 100]));
        m.insert("filter_type".into(), Value::String("lowpass".into()));
        m.insert("cutoff".into(), Value::F64(1000.0));
        let result = filter(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn delay_basic() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.5); 100]));
        m.insert("delay_samples".into(), Value::U64(50));
        m.insert("feedback".into(), Value::F64(0.3));
        let result = delay(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn reverb_basic() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.5); 500]));
        m.insert("room_size".into(), Value::F64(0.5));
        let result = reverb(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn compressor_basic() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.9); 100]));
        m.insert("threshold".into(), Value::F64(-10.0));
        m.insert("ratio".into(), Value::F64(4.0));
        let result = compressor(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn eq_basic() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.5); 100]));
        m.insert("low_gain".into(), Value::F64(3.0));
        m.insert("mid_gain".into(), Value::F64(0.0));
        m.insert("high_gain".into(), Value::F64(-3.0));
        let result = eq(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn midi_note_to_freq() {
        let mut m = BTreeMap::new();
        m.insert("action".into(), Value::String("to_freq".into()));
        m.insert("note".into(), Value::U64(69));
        let result = midi_note(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("frequency") {
                Some(Value::F64(f)) => assert!((f - 440.0).abs() < 0.01),
                _ => panic!("expected f64"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn midi_note_from_name() {
        let mut m = BTreeMap::new();
        m.insert("action".into(), Value::String("from_name".into()));
        m.insert("name".into(), Value::String("A4".into()));
        let result = midi_note(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => assert_eq!(rec.get("note"), Some(&Value::U64(69))),
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn quantize_basic() {
        let mut m = BTreeMap::new();
        m.insert("position".into(), Value::F64(1.3));
        m.insert("grid".into(), Value::F64(0.25));
        let result = quantize(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn transpose_basic() {
        let mut m = BTreeMap::new();
        m.insert("note".into(), Value::U64(60));
        m.insert("semitones".into(), Value::I64(7));
        let result = transpose(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => assert_eq!(rec.get("note"), Some(&Value::U64(67))),
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn transport_play() {
        let mut m = BTreeMap::new();
        m.insert("action".into(), Value::String("play".into()));
        m.insert("tempo".into(), Value::F64(120.0));
        let result = transport(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("state"), Some(&Value::String("Playing".into())));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn waveform_meter_basic() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.5); 100]));
        m.insert("buckets".into(), Value::U64(10));
        let result = waveform_meter(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("peak"));
                assert!(rec.contains_key("rms"));
                assert!(rec.contains_key("display"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn phase_meter_correlated() {
        let mut m = BTreeMap::new();
        let signal: Vec<Value> = (0..100)
            .map(|i| Value::F64((i as f64 * 0.1).sin()))
            .collect();
        m.insert("left".into(), Value::List(signal.clone()));
        m.insert("right".into(), Value::List(signal));
        let result = phase_meter(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn loudness_meter_basic() {
        let mut m = BTreeMap::new();
        m.insert("input".into(), Value::List(vec![Value::F64(0.5); 1000]));
        let result = loudness_meter(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => assert!(rec.contains_key("momentary_lufs")),
            _ => panic!("expected record"),
        }
    }
}
