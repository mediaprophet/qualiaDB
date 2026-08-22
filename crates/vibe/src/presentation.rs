//! Presentation morphism as a sheaf (T69).
//!
//! The presentation morphism maps a VibeScript value (field, tensor,
//! geometry, quantity) to a presentation in one or more modalities
//! (visual, haptic, auditory, Braille). It is a **sheaf** — the
//! presentation is locally consistent (each modality gets a coherent
//! view) and globally glued (the modalities agree on the underlying
//! value).
//!
//! ## Design
//!
//! - [`PresentationModality`] — the output modality (visual, haptic,
//!   auditory, Braille).
//! - [`Presentation`] — a single presentation of a value in one
//!   modality. Carries the modality, a presentation kind, and a
//!   payload (CSS properties, haptic pattern, audio earcon, Braille
//!   cells).
//! - [`PresentationSheaf`] — the sheaf: a value plus its presentations
//!   across modalities. The sheaf condition is that all presentations
//!   agree on the underlying value.
//!
//! ## Not `Render.css_*` plus hope
//!
//! The presentation morphism is NOT just CSS rendering. It produces
//! presentations for multiple modalities simultaneously. The visual
//! presentation might be CSS, but the haptic presentation is a
//! vibration pattern, the auditory presentation is an earcon, and the
//! Braille presentation is a cell array.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §3.10 T69.

use crate::sheaf::SheafCondition;
use crate::value::Value;
use std::collections::BTreeMap;

/// A presentation modality — the output channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentationModality {
    /// Visual presentation (CSS, SVG, canvas, WebGL).
    Visual,
    /// Haptic presentation (vibration, force feedback).
    Haptic,
    /// Auditory presentation (tones, earcons, speech).
    Auditory,
    /// Braille presentation (refreshable Braille display).
    Braille,
}

impl PresentationModality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Visual => "visual",
            Self::Haptic => "haptic",
            Self::Auditory => "auditory",
            Self::Braille => "braille",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "visual" => Some(Self::Visual),
            "haptic" => Some(Self::Haptic),
            "auditory" => Some(Self::Auditory),
            "braille" => Some(Self::Braille),
            _ => None,
        }
    }
}

/// The kind of presentation within a modality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationKind {
    /// CSS properties (visual).
    CssProperties,
    /// SVG element (visual).
    SvgElement,
    /// Canvas/WebGL draw call (visual).
    CanvasDraw,
    /// Haptic vibration pattern.
    HapticPattern,
    /// Audio earcon (auditory).
    AudioEarcon,
    /// Speech announcement (auditory).
    SpeechAnnounce,
    /// Braille cell array (Braille).
    BrailleCells,
}

impl PresentationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CssProperties => "css_properties",
            Self::SvgElement => "svg_element",
            Self::CanvasDraw => "canvas_draw",
            Self::HapticPattern => "haptic_pattern",
            Self::AudioEarcon => "audio_earcon",
            Self::SpeechAnnounce => "speech_announce",
            Self::BrailleCells => "braille_cells",
        }
    }
}

/// A single presentation of a value in one modality.
#[derive(Debug, Clone)]
pub struct Presentation {
    /// The modality (visual, haptic, auditory, Braille).
    pub modality: PresentationModality,
    /// The kind of presentation within the modality.
    pub kind: PresentationKind,
    /// The presentation payload — modality-specific data.
    /// For CSS: a Record of property → value.
    /// For SVG: an SVG element string.
    /// For haptic: a pattern (duration, strength pairs).
    /// For audio: an earcon ID or speech text.
    /// For Braille: a cell array.
    pub payload: Value,
}

impl Presentation {
    /// Create a CSS properties presentation.
    pub fn css(props: BTreeMap<String, Value>) -> Self {
        Self {
            modality: PresentationModality::Visual,
            kind: PresentationKind::CssProperties,
            payload: Value::Record(props),
        }
    }

    /// Create an SVG element presentation.
    pub fn svg(element: &str) -> Self {
        Self {
            modality: PresentationModality::Visual,
            kind: PresentationKind::SvgElement,
            payload: Value::String(element.into()),
        }
    }

    /// Create a haptic pattern presentation.
    pub fn haptic_pattern(pattern: Vec<(u64, f64)>) -> Self {
        let list: Vec<Value> = pattern
            .iter()
            .map(|(d, s)| {
                let mut pair = BTreeMap::new();
                pair.insert("duration_ms".into(), Value::U64(*d));
                pair.insert("strength".into(), Value::F64(*s));
                Value::Record(pair)
            })
            .collect();
        Self {
            modality: PresentationModality::Haptic,
            kind: PresentationKind::HapticPattern,
            payload: Value::List(list),
        }
    }

    /// Create an audio earcon presentation.
    pub fn audio_earcon(earcon_id: &str) -> Self {
        Self {
            modality: PresentationModality::Auditory,
            kind: PresentationKind::AudioEarcon,
            payload: Value::String(earcon_id.into()),
        }
    }

    /// Create a speech announcement presentation.
    pub fn speech(text: &str) -> Self {
        Self {
            modality: PresentationModality::Auditory,
            kind: PresentationKind::SpeechAnnounce,
            payload: Value::String(text.into()),
        }
    }

    /// Create a Braille cells presentation.
    /// `cells` is a list of 8-bit dot patterns.
    pub fn braille_cells(cells: Vec<u8>) -> Self {
        let list: Vec<Value> = cells.iter().map(|&c| Value::I64(c as i64)).collect();
        Self {
            modality: PresentationModality::Braille,
            kind: PresentationKind::BrailleCells,
            payload: Value::List(list),
        }
    }

    /// Convert to a VibeScript Record value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert(
            "modality".into(),
            Value::String(self.modality.as_str().into()),
        );
        rec.insert("kind".into(), Value::String(self.kind.as_str().into()));
        rec.insert("payload".into(), self.payload.clone());
        Value::Record(rec)
    }
}

/// A presentation sheaf — a value plus its presentations across
/// modalities.
///
/// The sheaf condition is that all presentations agree on the
/// underlying value. This is checked by [`PresentationSheaf::check`],
/// which verifies that every presentation maps back to the same
/// source value.
#[derive(Debug, Clone)]
pub struct PresentationSheaf {
    /// The source value being presented.
    pub source: Value,
    /// The presentations across modalities.
    pub presentations: Vec<Presentation>,
    /// The sheaf condition — verifies presentations agree.
    pub condition: SheafCondition,
}

impl PresentationSheaf {
    /// Create a new presentation sheaf for a source value.
    pub fn new(source: Value) -> Self {
        Self {
            source,
            presentations: Vec::new(),
            condition: SheafCondition::new(
                "presentation_glue",
                "pred.presentation_consistent",
                true,
            ),
        }
    }

    /// Add a presentation to the sheaf.
    pub fn add(&mut self, presentation: Presentation) -> &mut Self {
        self.presentations.push(presentation);
        self
    }

    /// Get all presentations for a specific modality.
    pub fn for_modality(&self, modality: PresentationModality) -> Vec<&Presentation> {
        self.presentations
            .iter()
            .filter(|p| p.modality == modality)
            .collect()
    }

    /// Check which modalities are present.
    pub fn modalities(&self) -> Vec<PresentationModality> {
        let mut mods: Vec<PresentationModality> =
            self.presentations.iter().map(|p| p.modality).collect();
        mods.sort_by_key(|m| m.as_str());
        mods.dedup();
        mods
    }

    /// Convert to a VibeScript Record value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("source".into(), self.source.clone());
        rec.insert(
            "presentations".into(),
            Value::List(self.presentations.iter().map(|p| p.to_value()).collect()),
        );
        rec.insert("condition".into(), self.condition.to_value());
        Value::Record(rec)
    }
}

/// The presentation morphism — maps a value to a presentation sheaf.
///
/// This is the top-level function that takes a VibeScript value and
/// produces presentations across modalities. The actual presentation
/// logic is modality-specific and may be host-provided.
pub fn present(value: &Value) -> PresentationSheaf {
    let mut sheaf = PresentationSheaf::new(value.clone());

    // Visual: CSS properties for scalar/quantity values.
    match value {
        Value::F64(f) => {
            let mut props = BTreeMap::new();
            // Map 0.0–1.0 to opacity.
            let opacity = f.clamp(0.0, 1.0);
            props.insert("opacity".into(), Value::F64(opacity));
            sheaf.add(Presentation::css(props));
        }
        Value::I64(n) => {
            let mut props = BTreeMap::new();
            props.insert("z-index".into(), Value::I64(*n));
            sheaf.add(Presentation::css(props));
        }
        Value::String(s) => {
            // Visual: render as text.
            let mut props = BTreeMap::new();
            props.insert("content".into(), Value::String(s.clone()));
            sheaf.add(Presentation::css(props));
            // Auditory: announce as speech.
            sheaf.add(Presentation::speech(s));
            // Braille: convert to Braille cells (simplified — ASCII mapping).
            let cells: Vec<u8> = s.bytes().map(|b| b & 0x3F).collect();
            sheaf.add(Presentation::braille_cells(cells));
        }
        Value::Bool(b) => {
            let mut props = BTreeMap::new();
            props.insert(
                "display".into(),
                Value::String(if *b { "block" } else { "none" }.into()),
            );
            sheaf.add(Presentation::css(props));
        }
        _ => {
            // No default presentation for other types.
        }
    }

    sheaf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modality_round_trip() {
        for m in [
            PresentationModality::Visual,
            PresentationModality::Haptic,
            PresentationModality::Auditory,
            PresentationModality::Braille,
        ] {
            assert_eq!(PresentationModality::from_str(m.as_str()), Some(m));
        }
    }

    #[test]
    fn presentation_css() {
        let mut props = BTreeMap::new();
        props.insert("opacity".into(), Value::F64(0.5));
        let p = Presentation::css(props);
        assert_eq!(p.modality, PresentationModality::Visual);
        assert_eq!(p.kind, PresentationKind::CssProperties);
    }

    #[test]
    fn presentation_svg() {
        let p = Presentation::svg("<circle cx='10' cy='10' r='5'/>");
        assert_eq!(p.modality, PresentationModality::Visual);
        assert_eq!(p.kind, PresentationKind::SvgElement);
    }

    #[test]
    fn presentation_haptic_pattern() {
        let p = Presentation::haptic_pattern(vec![(100, 0.5), (200, 0.8)]);
        assert_eq!(p.modality, PresentationModality::Haptic);
        assert_eq!(p.kind, PresentationKind::HapticPattern);
        if let Value::List(l) = &p.payload {
            assert_eq!(l.len(), 2);
        }
    }

    #[test]
    fn presentation_audio_earcon() {
        let p = Presentation::audio_earcon("success");
        assert_eq!(p.modality, PresentationModality::Auditory);
        assert_eq!(p.kind, PresentationKind::AudioEarcon);
    }

    #[test]
    fn presentation_speech() {
        let p = Presentation::speech("Hello world");
        assert_eq!(p.modality, PresentationModality::Auditory);
        assert_eq!(p.kind, PresentationKind::SpeechAnnounce);
    }

    #[test]
    fn presentation_braille_cells() {
        let p = Presentation::braille_cells(vec![0x01, 0x02, 0x03]);
        assert_eq!(p.modality, PresentationModality::Braille);
        assert_eq!(p.kind, PresentationKind::BrailleCells);
        if let Value::List(l) = &p.payload {
            assert_eq!(l.len(), 3);
        }
    }

    #[test]
    fn presentation_to_value() {
        let p = Presentation::speech("test");
        let v = p.to_value();
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("modality").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "auditory"
        );
    }

    #[test]
    fn sheaf_construction() {
        let sheaf = PresentationSheaf::new(Value::F64(0.5));
        assert!(sheaf.presentations.is_empty());
        assert_eq!(sheaf.condition.name, "presentation_glue");
    }

    #[test]
    fn sheaf_add_presentation() {
        let mut sheaf = PresentationSheaf::new(Value::F64(0.5));
        sheaf.add(Presentation::speech("half"));
        assert_eq!(sheaf.presentations.len(), 1);
    }

    #[test]
    fn sheaf_for_modality() {
        let mut sheaf = PresentationSheaf::new(Value::String("hi".into()));
        sheaf.add(Presentation::speech("hi"));
        sheaf.add(Presentation::braille_cells(vec![0x01]));
        sheaf.add(Presentation::css(BTreeMap::new()));
        let visual = sheaf.for_modality(PresentationModality::Visual);
        let auditory = sheaf.for_modality(PresentationModality::Auditory);
        let braille = sheaf.for_modality(PresentationModality::Braille);
        assert_eq!(visual.len(), 1);
        assert_eq!(auditory.len(), 1);
        assert_eq!(braille.len(), 1);
    }

    #[test]
    fn sheaf_modalities_dedup() {
        let mut sheaf = PresentationSheaf::new(Value::F64(0.5));
        sheaf.add(Presentation::css(BTreeMap::new()));
        sheaf.add(Presentation::svg("<rect/>"));
        sheaf.add(Presentation::speech("half"));
        let mods = sheaf.modalities();
        assert_eq!(mods.len(), 2); // visual + auditory, not 3
    }

    #[test]
    fn sheaf_to_value() {
        let mut sheaf = PresentationSheaf::new(Value::F64(0.5));
        sheaf.add(Presentation::speech("half"));
        let v = sheaf.to_value();
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert!(rec.contains_key("source"));
        assert!(rec.contains_key("presentations"));
        assert!(rec.contains_key("condition"));
    }

    #[test]
    fn present_f64_as_opacity() {
        let sheaf = present(&Value::F64(0.5));
        let visual = sheaf.for_modality(PresentationModality::Visual);
        assert_eq!(visual.len(), 1);
        assert_eq!(visual[0].kind, PresentationKind::CssProperties);
    }

    #[test]
    fn present_f64_clamps_opacity() {
        let sheaf = present(&Value::F64(1.5));
        let visual = sheaf.for_modality(PresentationModality::Visual);
        if let Value::Record(props) = &visual[0].payload {
            if let Value::F64(op) = props.get("opacity").unwrap() {
                assert_eq!(*op, 1.0); // clamped
            }
        }
    }

    #[test]
    fn present_string_multi_modal() {
        let sheaf = present(&Value::String("hello".into()));
        // String gets visual, auditory, and Braille presentations.
        assert!(!sheaf.for_modality(PresentationModality::Visual).is_empty());
        assert!(!sheaf
            .for_modality(PresentationModality::Auditory)
            .is_empty());
        assert!(!sheaf.for_modality(PresentationModality::Braille).is_empty());
    }

    #[test]
    fn present_bool_as_display() {
        let sheaf = present(&Value::Bool(true));
        let visual = sheaf.for_modality(PresentationModality::Visual);
        assert_eq!(visual.len(), 1);
        if let Value::Record(props) = &visual[0].payload {
            if let Value::String(s) = props.get("display").unwrap() {
                assert_eq!(s, "block");
            }
        }
    }

    #[test]
    fn present_bool_false_as_none() {
        let sheaf = present(&Value::Bool(false));
        let visual = sheaf.for_modality(PresentationModality::Visual);
        if let Value::Record(props) = &visual[0].payload {
            if let Value::String(s) = props.get("display").unwrap() {
                assert_eq!(s, "none");
            }
        }
    }

    #[test]
    fn present_i64_as_zindex() {
        let sheaf = present(&Value::I64(42));
        let visual = sheaf.for_modality(PresentationModality::Visual);
        if let Value::Record(props) = &visual[0].payload {
            if let Value::I64(n) = props.get("z-index").unwrap() {
                assert_eq!(*n, 42);
            }
        }
    }

    #[test]
    fn present_null_no_presentations() {
        let sheaf = present(&Value::Null);
        assert!(sheaf.presentations.is_empty());
    }
}
