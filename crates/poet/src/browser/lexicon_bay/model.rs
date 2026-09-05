//! Framing chips, held-gate copy, and `lexicon_manifest` result mapping.

/// Live ALL_BOUND id. Do not invent a Host method.
pub const INVOKE_ID: &str = "GraphDatabase.lexicon_manifest";

/// Soft why-text for missing / unknown / E300. Never "broken".
pub const HELD_WHY: &str = "held / not yet — open lexicon pack";

pub const LIVING_SAYABLE: &str = "person / living / country";
pub const ARTIFACT_SAYABLE: &str = "tool / volume / file";
pub const MACHINE_SAYABLE: &str = "Capability.method";

pub const LIVING_CHIP: &str = "living";
pub const ARTIFACT_CHIP: &str = "artifact";
pub const MACHINE_CHIP: &str = "machine";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    LivingShacl,
    ArtifactOwl,
    Mixed,
}

impl Framing {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LivingShacl => "living-SHACL",
            Self::ArtifactOwl => "artifact-OWL",
            Self::Mixed => "mixed",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "living-SHACL" | "living_shacl" | "living" => Some(Self::LivingShacl),
            "artifact-OWL" | "artifact_owl" | "artifact" => Some(Self::ArtifactOwl),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramingChip {
    Living,
    Artifact,
    Machine,
}

impl FramingChip {
    pub fn token(self) -> &'static str {
        match self {
            Self::Living => LIVING_CHIP,
            Self::Artifact => ARTIFACT_CHIP,
            Self::Machine => MACHINE_CHIP,
        }
    }

    pub fn sayable(self) -> &'static str {
        match self {
            Self::Living => LIVING_SAYABLE,
            Self::Artifact => ARTIFACT_SAYABLE,
            Self::Machine => MACHINE_SAYABLE,
        }
    }

    pub fn tone(self) -> &'static str {
        match self {
            Self::Living => "warm",
            Self::Artifact => "crisp",
            Self::Machine => "muted",
        }
    }
}

/// Framing drives the chip set. Mixed splits living + artifact; machine is
/// always the Capability.method filter. Never collapse living into Thing.
pub fn chips_for_framing(framing: Framing) -> &'static [FramingChip] {
    match framing {
        Framing::LivingShacl => &[FramingChip::Living, FramingChip::Machine],
        Framing::ArtifactOwl => &[FramingChip::Artifact, FramingChip::Machine],
        Framing::Mixed => &[
            FramingChip::Living,
            FramingChip::Artifact,
            FramingChip::Machine,
        ],
    }
}

/// Catalog filter row before a pack arrives — all three chips, muted.
pub fn catalog_filter_chips() -> &'static [FramingChip] {
    &[
        FramingChip::Living,
        FramingChip::Artifact,
        FramingChip::Machine,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeBeat {
    Arrive,
    Hold,
    Leave,
    Commit,
}

impl RecipeBeat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arrive => "arrive",
            Self::Hold => "hold",
            Self::Leave => "leave",
            Self::Commit => "commit",
        }
    }

    /// Named beats only (entrance · dwell · exit). Reduced-motion still maps.
    pub fn named_beat(self) -> &'static str {
        match self {
            Self::Arrive => "entrance",
            Self::Hold | Self::Commit => "dwell",
            Self::Leave => "exit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeEvent {
    PackOpen,
    BreakingIdsShown,
    Dismiss,
    PackWriteOk,
    PackWriteHeld,
}

pub fn recipe_beat(event: RecipeEvent) -> RecipeBeat {
    match event {
        RecipeEvent::PackOpen => RecipeBeat::Arrive,
        RecipeEvent::BreakingIdsShown | RecipeEvent::PackWriteHeld => RecipeBeat::Hold,
        RecipeEvent::Dismiss => RecipeBeat::Leave,
        RecipeEvent::PackWriteOk => RecipeBeat::Commit,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackCard {
    pub pack_id: String,
    pub pack_semver: String,
    pub framing: Framing,
    pub uplift_from: String,
    pub concept_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestOutcome {
    Held { why: String },
    Open(PackCard),
}

pub fn held_outcome(why: impl Into<String>) -> ManifestOutcome {
    ManifestOutcome::Held {
        why: sanitize_held_why(&why.into()),
    }
}

/// Never surface "broken" or Thing-wash living senses.
pub fn sanitize_held_why(raw: &str) -> String {
    let folded = raw.to_ascii_lowercase();
    if folded.contains("broken")
        || raw.trim().is_empty()
        || folded.contains("owl:thing")
        || folded.contains("a thing")
    {
        return HELD_WHY.to_string();
    }
    if folded.contains("held") || folded.contains("open lexicon") {
        raw.trim().to_string()
    } else {
        HELD_WHY.to_string()
    }
}

pub fn interpret_invoke(ok: bool, value: &str, diagnostic: Option<&str>) -> ManifestOutcome {
    if ok {
        if let Some(card) = parse_pack_card(value) {
            return ManifestOutcome::Open(card);
        }
        // ok:true but strict card parse missed — still arrive if framing+semver present
        // (format_value key order / extra fields must not strand the bay on held).
        if let Some(card) = parse_pack_card_lenient(value) {
            return ManifestOutcome::Open(card);
        }
    }
    let blob = diagnostic.unwrap_or(value);
    let why = if blob.to_ascii_lowercase().contains("e300")
        || blob.contains("held / not yet")
        || blob.contains("open lexicon pack")
    {
        extract_held_fix(blob)
            .map(|fix| sanitize_held_why(&fix))
            .unwrap_or_else(|| HELD_WHY.to_string())
    } else {
        HELD_WHY.to_string()
    };
    ManifestOutcome::Held { why }
}

fn extract_held_fix(blob: &str) -> Option<String> {
    for needle in ["held / not yet", "suggested_fix"] {
        if let Some(idx) = blob.find(needle) {
            let rest = blob[idx..].lines().next().unwrap_or("").trim();
            let rest = rest
                .trim_start_matches("suggested_fix")
                .trim_start_matches(':')
                .trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

pub fn parse_pack_card(src: &str) -> Option<PackCard> {
    let framing = extract_quoted_field(src, "framing").and_then(|s| Framing::parse(&s))?;
    let pack_semver = extract_quoted_field(src, "packSemVer")
        .or_else(|| extract_quoted_field(src, "pack_semver"))?;
    let pack_id = extract_quoted_field(src, "pack_id")
        .or_else(|| extract_quoted_field(src, "packId"))
        .or_else(|| extract_quoted_field(src, "id"))
        .unwrap_or_default();
    let uplift_from = extract_quoted_field(src, "upliftFrom")
        .or_else(|| extract_quoted_field(src, "uplift_from"))
        .unwrap_or_default();
    let concept_ids = extract_string_list(src, "conceptIds")
        .or_else(|| extract_string_list(src, "concept_ids"))
        .unwrap_or_default();
    Some(PackCard {
        pack_id,
        pack_semver,
        framing,
        uplift_from,
        concept_ids,
    })
}

/// Best-effort card when invoke ok but one field is oddly formatted.
fn parse_pack_card_lenient(src: &str) -> Option<PackCard> {
    let framing = extract_quoted_field(src, "framing")
        .and_then(|s| Framing::parse(&s))
        .or_else(|| {
            if src.contains("mixed") {
                Some(Framing::Mixed)
            } else if src.contains("living") {
                Some(Framing::LivingShacl)
            } else if src.contains("artifact") {
                Some(Framing::ArtifactOwl)
            } else {
                None
            }
        })?;
    let pack_semver = extract_quoted_field(src, "packSemVer")
        .or_else(|| extract_quoted_field(src, "pack_semver"))
        .or_else(|| {
            // bare 0.1.0 near packSemVer key
            src.find("packSemVer").and_then(|i| {
                let rest = &src[i..i.saturating_add(40)];
                rest.split('"').nth(1).map(|s| s.to_string())
            })
        })?;
    let pack_id = extract_quoted_field(src, "pack_id")
        .or_else(|| extract_quoted_field(src, "packId"))
        .unwrap_or_default();
    Some(PackCard {
        pack_id,
        pack_semver,
        framing,
        uplift_from: extract_quoted_field(src, "upliftFrom").unwrap_or_default(),
        concept_ids: extract_string_list(src, "conceptIds").unwrap_or_default(),
    })
}

fn extract_quoted_field(src: &str, key: &str) -> Option<String> {
    let patterns = [
        format!("{key}: \""),
        format!("{key}:\""),
        format!("\"{key}\": \""),
        format!("\"{key}\":\""),
    ];
    for pat in patterns {
        if let Some(start) = src.find(&pat) {
            let rest = &src[start + pat.len()..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

fn extract_string_list(src: &str, key: &str) -> Option<Vec<String>> {
    let markers = [format!("{key}: ["), format!("\"{key}\": [")];
    for marker in markers {
        if let Some(start) = src.find(&marker) {
            let rest = &src[start + marker.len()..];
            let end = rest.find(']')?;
            let inner = &rest[..end];
            let ids = inner
                .split(',')
                .filter_map(|part| {
                    let p = part.trim().trim_matches('"');
                    (!p.is_empty()).then(|| p.to_string())
                })
                .collect();
            return Some(ids);
        }
    }
    None
}

pub fn copy_avoids_broken(text: &str) -> bool {
    !text.to_ascii_lowercase().contains("broken")
}

pub fn copy_avoids_thing_wash(text: &str) -> bool {
    let folded = text.to_ascii_lowercase();
    !folded.contains("owl:thing") && !folded.contains("a thing")
}

pub fn framing_copy(framing: Framing) -> &'static str {
    match framing {
        Framing::LivingShacl => LIVING_SAYABLE,
        Framing::ArtifactOwl => ARTIFACT_SAYABLE,
        Framing::Mixed => "living and artifact senses stay split",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_is_live_lexicon_manifest() {
        assert_eq!(INVOKE_ID, "GraphDatabase.lexicon_manifest");
        assert!(!INVOKE_ID.contains("qualia."));
    }

    #[test]
    fn framing_chips_split_mixed_and_never_thing_wash_living() {
        assert_eq!(
            chips_for_framing(Framing::LivingShacl),
            &[FramingChip::Living, FramingChip::Machine]
        );
        assert_eq!(
            chips_for_framing(Framing::ArtifactOwl),
            &[FramingChip::Artifact, FramingChip::Machine]
        );
        let mixed = chips_for_framing(Framing::Mixed);
        assert!(mixed.contains(&FramingChip::Living));
        assert!(mixed.contains(&FramingChip::Artifact));
        assert!(mixed.contains(&FramingChip::Machine));
        assert_eq!(FramingChip::Living.sayable(), LIVING_SAYABLE);
        assert_eq!(FramingChip::Artifact.sayable(), ARTIFACT_SAYABLE);
        assert!(!LIVING_SAYABLE.to_ascii_lowercase().contains("thing"));
        assert_eq!(Framing::parse("mixed"), Some(Framing::Mixed));
    }

    #[test]
    fn held_gate_copy_never_says_broken() {
        assert_eq!(sanitize_held_why("volume broken"), HELD_WHY);
        assert_eq!(sanitize_held_why(""), HELD_WHY);
        assert_eq!(
            sanitize_held_why("held / not yet — open lexicon pack"),
            HELD_WHY
        );
        assert!(copy_avoids_broken(HELD_WHY));
        assert!(copy_avoids_thing_wash(HELD_WHY));
        assert!(copy_avoids_thing_wash(LIVING_SAYABLE));
        assert!(copy_avoids_thing_wash(ARTIFACT_SAYABLE));
        match interpret_invoke(false, "", Some("E300@0..0: lexicon pack not found")) {
            ManifestOutcome::Held { why } => {
                assert_eq!(why, HELD_WHY);
                assert!(copy_avoids_broken(&why));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn success_parses_live_format_value_shape() {
        let src = r#"{conceptIds: ["concept:arrive", "concept:hold", "concept:leave"], framing: "mixed", gate: "open", manifest_path: "/tmp/en-core.lexicon.json", packSemVer: "0.1.0", pack_id: "en-core@0.1.0", upliftFrom: "", volume_ok: false, volume_path: ""}"#;
        match interpret_invoke(true, src, None) {
            ManifestOutcome::Open(card) => {
                assert_eq!(card.pack_semver, "0.1.0");
                assert_eq!(card.framing, Framing::Mixed);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn success_parses_pack_card_and_arrive_beat() {
        let src = r#"{pack_id: "en-core@0.1.0", packSemVer: "0.1.0", framing: "mixed", upliftFrom: "", conceptIds: ["concept:arrive", "concept:hold"]}"#;
        let card = parse_pack_card(src).expect("card");
        assert_eq!(card.pack_semver, "0.1.0");
        assert_eq!(card.framing, Framing::Mixed);
        assert_eq!(card.concept_ids.len(), 2);
        match interpret_invoke(true, src, None) {
            ManifestOutcome::Open(open) => assert_eq!(open.pack_id, "en-core@0.1.0"),
            other => panic!("{other:?}"),
        }
        assert_eq!(recipe_beat(RecipeEvent::PackOpen), RecipeBeat::Arrive);
        assert_eq!(recipe_beat(RecipeEvent::BreakingIdsShown), RecipeBeat::Hold);
        assert_eq!(recipe_beat(RecipeEvent::Dismiss), RecipeBeat::Leave);
        assert_eq!(recipe_beat(RecipeEvent::PackWriteOk), RecipeBeat::Commit);
        assert_eq!(recipe_beat(RecipeEvent::PackWriteHeld), RecipeBeat::Hold);
        assert_eq!(RecipeBeat::Arrive.named_beat(), "entrance");
        assert_eq!(RecipeBeat::Leave.named_beat(), "exit");
    }

    #[test]
    fn chip_tones_are_warm_crisp_muted() {
        assert_eq!(FramingChip::Living.tone(), "warm");
        assert_eq!(FramingChip::Artifact.tone(), "crisp");
        assert_eq!(FramingChip::Machine.tone(), "muted");
        assert_eq!(FramingChip::Machine.sayable(), MACHINE_SAYABLE);
    }
}
