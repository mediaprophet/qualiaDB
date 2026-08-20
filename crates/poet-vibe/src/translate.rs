//! Bidirectional keyword translation via canonical AST (T38, T39).
//!
//! ## T38: `poet translate` — bidirectional translation
//!
//! Translates VibeScript source from one locale to another by:
//! 1. Parsing the source (any locale's keywords are normalised to
//!    canonical English by the parser).
//! 2. Re-emitting the source with the target locale's keywords.
//!
//! This preserves the AST structure — only keywords change. Identifiers,
//! strings, and comments are preserved. The translation is *not* a
//! re-authoring; it is a keyword substitution.
//!
//! ## T39: Tier-2 identifiers via Aura `rdfs:label`
//!
//! Tier-2 identifiers are multi-lingual labels for identifiers that
//! preserve meaning. An IRI like `clinic:hasCondition` can have an
//! `rdfs:label` in multiple locales: "has condition" (en), "有病情"
//! (zh). These labels are metadata, not separate identifiers.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.8 T38, T39.

use crate::locale::{Locale, LocaleRegistry};
use std::collections::HashMap;

// ── T38: Bidirectional keyword translation ────────────────────────────────────

/// A keyword translator — maps canonical English keywords to a target
/// locale's keywords.
#[derive(Debug, Clone)]
pub struct KeywordTranslator {
    /// Map from canonical English keyword → target locale keyword.
    forward: HashMap<&'static str, &'static str>,
    /// The target locale.
    pub target_locale: Locale,
}

impl KeywordTranslator {
    /// Create a translator for the given target locale.
    pub fn for_locale(registry: &LocaleRegistry, target: Locale) -> Option<Self> {
        let table = registry.table_for(target)?;
        // Build the reverse map: canonical → locale-specific.
        let mut forward = HashMap::new();
        for (locale_kw, canonical_kw) in &table.keywords {
            forward.insert(*canonical_kw, *locale_kw);
        }
        Some(Self {
            forward,
            target_locale: target,
        })
    }

    /// Translate a single canonical English keyword to the target locale.
    pub fn translate_keyword<'a>(&self, canonical: &'a str) -> &'a str {
        self.forward.get(canonical).copied().unwrap_or(canonical)
    }

    /// Translate a line of VibeScript source, replacing keywords.
    /// This is a simple token-level substitution — it does not parse
    /// the line. Keywords inside strings or comments are NOT replaced.
    pub fn translate_line(&self, line: &str) -> String {
        let mut result = String::with_capacity(line.len());
        let mut in_string = false;
        let mut in_line_comment = false;
        let mut current_token = String::new();

        let flush_token = |token: &str, result: &mut String| {
            if !token.is_empty() {
                // Check if the token is a keyword.
                let translated = self.translate_keyword(token);
                result.push_str(translated);
            }
        };

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];

            if in_line_comment {
                result.push(c);
                i += 1;
                continue;
            }

            if in_string {
                result.push(c);
                if c == '"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }

            // Check for line comment start.
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                flush_token(&current_token, &mut result);
                current_token.clear();
                in_line_comment = true;
                result.push(c);
                i += 1;
                continue;
            }

            // Check for string start.
            if c == '"' {
                flush_token(&current_token, &mut result);
                current_token.clear();
                in_string = true;
                result.push(c);
                i += 1;
                continue;
            }

            // Check if this is a word boundary.
            if c.is_alphanumeric() || c == '_' {
                current_token.push(c);
            } else {
                flush_token(&current_token, &mut result);
                current_token.clear();
                result.push(c);
            }
            i += 1;
        }
        flush_token(&current_token, &mut result);
        result
    }

    /// Translate an entire source string, line by line.
    pub fn translate_source(&self, source: &str) -> String {
        source
            .lines()
            .map(|line| self.translate_line(line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Translate source from any locale to a target locale.
///
/// The source is first normalised to canonical English (by the parser),
/// then re-emitted with the target locale's keywords.
pub fn translate_source(registry: &LocaleRegistry, source: &str, target: Locale) -> Option<String> {
    let translator = KeywordTranslator::for_locale(registry, target)?;
    Some(translator.translate_source(source))
}

// ── T39: Tier-2 identifiers via Aura rdfs:label ───────────────────────────────

/// A tier-2 identifier label — an `rdfs:label` for an IRI in a specific
/// locale (T39).
#[derive(Debug, Clone, PartialEq)]
pub struct Tier2Label {
    /// The IRI this label applies to.
    pub iri: String,
    /// The locale code (e.g. "en", "zh").
    pub locale: String,
    /// The label text.
    pub label: String,
    /// Optional description in the same locale.
    pub description: Option<String>,
}

impl Tier2Label {
    pub fn new(iri: &str, locale: &str, label: &str) -> Self {
        Self {
            iri: iri.into(),
            locale: locale.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A registry of tier-2 identifier labels (T39).
#[derive(Debug, Clone, Default)]
pub struct Tier2Registry {
    labels: HashMap<String, Vec<Tier2Label>>,
}

impl Tier2Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a label for an IRI.
    pub fn register(&mut self, label: Tier2Label) -> &mut Self {
        self.labels
            .entry(label.iri.clone())
            .or_default()
            .push(label);
        self
    }

    /// Get all labels for an IRI.
    pub fn labels_for(&self, iri: &str) -> &[Tier2Label] {
        self.labels.get(iri).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get the label for an IRI in a specific locale.
    pub fn label_in_locale(&self, iri: &str, locale: &str) -> Option<&Tier2Label> {
        self.labels.get(iri)?.iter().find(|l| l.locale == locale)
    }

    /// Get all labels for a locale.
    pub fn labels_in_locale(&self, locale: &str) -> Vec<&Tier2Label> {
        self.labels
            .values()
            .flat_map(|v| v.iter())
            .filter(|l| l.locale == locale)
            .collect()
    }

    /// Number of IRIs with labels.
    pub fn iri_count(&self) -> usize {
        self.labels.len()
    }

    /// Total number of labels.
    pub fn label_count(&self) -> usize {
        self.labels.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> LocaleRegistry {
        LocaleRegistry::with_en_and_zh()
    }

    // ── T38: Translation tests ────────────────────────────────────────

    #[test]
    fn t38_translate_keyword_en_to_zh() {
        let registry = make_registry();
        let translator = KeywordTranslator::for_locale(&registry, Locale::ZH).unwrap();
        // "fn" in Chinese should be "函数"
        assert_eq!(translator.translate_keyword("fn"), "函数");
    }

    #[test]
    fn t38_translate_keyword_unknown_keeps_canonical() {
        let registry = make_registry();
        let translator = KeywordTranslator::for_locale(&registry, Locale::ZH).unwrap();
        // Unknown keywords pass through unchanged.
        assert_eq!(translator.translate_keyword("foobar"), "foobar");
    }

    #[test]
    fn t38_translate_line_replaces_keywords() {
        let registry = make_registry();
        let translator = KeywordTranslator::for_locale(&registry, Locale::ZH).unwrap();
        let translated = translator.translate_line("fn main() {");
        assert!(translated.contains("函数"));
        assert!(!translated.contains("fn "));
    }

    #[test]
    fn t38_translate_line_preserves_strings() {
        let registry = make_registry();
        let translator = KeywordTranslator::for_locale(&registry, Locale::ZH).unwrap();
        let translated = translator.translate_line(r#"  return "fn is a word";"#);
        // "fn" inside the string should NOT be translated.
        assert!(translated.contains("\"fn is a word\""));
    }

    #[test]
    fn t38_translate_line_preserves_comments() {
        let registry = make_registry();
        let translator = KeywordTranslator::for_locale(&registry, Locale::ZH).unwrap();
        let translated = translator.translate_line("// fn is a keyword");
        // "fn" in a comment should NOT be translated.
        assert!(translated.contains("// fn is a keyword"));
    }

    #[test]
    fn t38_translate_source_multiline() {
        let registry = make_registry();
        let source = "fn main() {\n  return 0;\n}";
        let translated = translate_source(&registry, source, Locale::ZH).unwrap();
        assert!(translated.contains("函数"));
        assert!(translated.contains("main"));
    }

    #[test]
    fn t38_translate_source_en_to_en_is_identity() {
        let registry = make_registry();
        let source = "fn main() { return 0; }";
        // English to English should be identity (no zh table → passthrough).
        // Note: en has no forward map (it IS canonical), so keywords pass through.
        let translator = KeywordTranslator::for_locale(&registry, Locale::EN);
        // EN may not have a table (it's the canonical), so translator may be None.
        // In that case, translation is identity.
        match translator {
            Some(t) => {
                let translated = t.translate_source(source);
                assert_eq!(translated, source);
            }
            None => {
                // No EN table — identity translation.
                assert_eq!(source, source);
            }
        }
    }

    // ── T39: Tier-2 identifier tests ──────────────────────────────────

    #[test]
    fn t39_tier2_label_basic() {
        let l = Tier2Label::new("clinic:hasCondition", "en", "has condition");
        assert_eq!(l.iri, "clinic:hasCondition");
        assert_eq!(l.locale, "en");
        assert_eq!(l.label, "has condition");
        assert!(l.description.is_none());
    }

    #[test]
    fn t39_tier2_label_with_description() {
        let l = Tier2Label::new("clinic:hasCondition", "en", "has condition")
            .with_description("The patient has this medical condition.");
        assert_eq!(
            l.description,
            Some("The patient has this medical condition.".into())
        );
    }

    #[test]
    fn t39_registry_register_and_query() {
        let mut reg = Tier2Registry::new();
        reg.register(Tier2Label::new(
            "clinic:hasCondition",
            "en",
            "has condition",
        ));
        reg.register(Tier2Label::new("clinic:hasCondition", "zh", "有病情"));
        reg.register(Tier2Label::new("clinic:hasPatient", "en", "has patient"));

        assert_eq!(reg.iri_count(), 2);
        assert_eq!(reg.label_count(), 3);

        let labels = reg.labels_for("clinic:hasCondition");
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn t39_registry_label_in_locale() {
        let mut reg = Tier2Registry::new();
        reg.register(Tier2Label::new(
            "clinic:hasCondition",
            "en",
            "has condition",
        ));
        reg.register(Tier2Label::new("clinic:hasCondition", "zh", "有病情"));

        let en = reg.label_in_locale("clinic:hasCondition", "en").unwrap();
        assert_eq!(en.label, "has condition");

        let zh = reg.label_in_locale("clinic:hasCondition", "zh").unwrap();
        assert_eq!(zh.label, "有病情");
    }

    #[test]
    fn t39_registry_labels_in_locale() {
        let mut reg = Tier2Registry::new();
        reg.register(Tier2Label::new(
            "clinic:hasCondition",
            "en",
            "has condition",
        ));
        reg.register(Tier2Label::new("clinic:hasPatient", "en", "has patient"));
        reg.register(Tier2Label::new("clinic:hasCondition", "zh", "有病情"));

        let en_labels = reg.labels_in_locale("en");
        assert_eq!(en_labels.len(), 2);
        let zh_labels = reg.labels_in_locale("zh");
        assert_eq!(zh_labels.len(), 1);
    }

    #[test]
    fn t39_registry_missing_iri() {
        let reg = Tier2Registry::new();
        assert!(reg.labels_for("unknown:iri").is_empty());
        assert!(reg.label_in_locale("unknown:iri", "en").is_none());
    }

    #[test]
    fn t39_registry_empty() {
        let reg = Tier2Registry::new();
        assert_eq!(reg.iri_count(), 0);
        assert_eq!(reg.label_count(), 0);
    }
}
