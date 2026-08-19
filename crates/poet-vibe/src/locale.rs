//! Keyword locale tables — multi-lingual keyword views (T37).
//!
//! VibeScript keywords are canonically English. This module provides
//! locale tables that map keywords from other languages to the
//! canonical English forms. The parser accepts any locale's keyword
//! and normalises it to the canonical English form.
//!
//! ## Design
//!
//! - [`Locale`] identifies a language locale (e.g. `en`, `zh`).
//! - [`LocaleTable`] maps locale-specific keywords to canonical forms.
//! - [`LocaleRegistry`] holds tables for all supported locales.
//! - The parser can use [`LocaleRegistry::resolve`] to check if a
//!   token is a keyword in any locale and get the canonical form.
//!
//! ## Ship `en` plus one second locale as proof of pipeline
//!
//! English (`en`) is the canonical locale. Chinese (`zh`) is shipped
//! as the second locale to prove the pipeline works. Adding more
//! locales is a matter of adding more tables.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.8 T37.

use std::collections::HashMap;

/// A language locale identifier (e.g. "en", "zh").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Locale(&'static str);

impl Locale {
    /// English locale (canonical).
    pub const EN: Locale = Locale("en");
    /// Chinese locale (second locale, proof of pipeline).
    pub const ZH: Locale = Locale("zh");

    /// Get the locale code as a string.
    pub fn code(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A keyword locale table — maps locale-specific keywords to canonical
/// English forms.
#[derive(Debug, Clone)]
pub struct LocaleTable {
    /// The locale this table is for.
    pub locale: Locale,
    /// Map from locale-specific keyword → canonical English keyword.
    pub keywords: HashMap<&'static str, &'static str>,
}

impl LocaleTable {
    /// Create a new empty table for the given locale.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            keywords: HashMap::new(),
        }
    }

    /// Look up the canonical form of a locale-specific keyword.
    pub fn resolve(&self, keyword: &str) -> Option<&'static str> {
        self.keywords.get(keyword).copied()
    }

    /// Number of keywords in this table.
    pub fn len(&self) -> usize {
        self.keywords.len()
    }

    /// Is this table empty?
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }
}

/// The English locale table (canonical — keywords map to themselves).
pub fn english_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::EN);
    // English keywords are canonical — they map to themselves.
    for kw in ENGLISH_KEYWORDS {
        t.keywords.insert(kw, kw);
    }
    t
}

/// The Chinese locale table (second locale, proof of pipeline).
pub fn chinese_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::ZH);
    // Map Chinese keywords to canonical English forms.
    t.keywords.insert("如果", "if");
    t.keywords.insert("否则", "else");
    t.keywords.insert("对于", "for");
    t.keywords.insert("在", "in");
    t.keywords.insert("当", "while");
    t.keywords.insert("匹配", "match");
    t.keywords.insert("返回", "return");
    t.keywords.insert("让", "let");
    t.keywords.insert("可变", "mut");
    t.keywords.insert("常量", "const");
    t.keywords.insert("真", "true");
    t.keywords.insert("假", "false");
    t.keywords.insert("空", "null");
    t.keywords.insert("函数", "fn");
    t.keywords.insert("模块", "module");
    t.keywords.insert("导入", "import");
    t.keywords.insert("作为", "as");
    t
}

/// The canonical English keyword list.
pub const ENGLISH_KEYWORDS: &[&str] = &[
    "module", "import", "as", "prefix", "requires", "capability",
    "fn", "async", "on", "let", "mut", "const", "enum",
    "field", "material", "law", "when",
    "if", "else", "for", "in", "while", "match", "return", "yield",
    "transaction", "await",
    "true", "false", "null",
    "effect", "pure", "hot", "cold",
];

/// A registry of locale tables.
#[derive(Debug, Clone)]
pub struct LocaleRegistry {
    tables: Vec<LocaleTable>,
    /// Reverse index: locale-specific keyword → (locale, canonical).
    keyword_index: HashMap<&'static str, (Locale, &'static str)>,
}

impl Default for LocaleRegistry {
    fn default() -> Self {
        Self::with_en_and_zh()
    }
}

impl LocaleRegistry {
    /// Create a registry with English and Chinese tables.
    pub fn with_en_and_zh() -> Self {
        Self::new(vec![english_table(), chinese_table()])
    }

    /// Create a registry with the given tables.
    pub fn new(tables: Vec<LocaleTable>) -> Self {
        let mut keyword_index = HashMap::new();
        for table in &tables {
            for (&kw, &canonical) in &table.keywords {
                keyword_index.insert(kw, (table.locale, canonical));
            }
        }
        Self { tables, keyword_index }
    }

    /// Resolve a keyword from any locale to its canonical English form.
    /// Returns (locale, canonical_keyword) if found.
    pub fn resolve(&self, keyword: &str) -> Option<(Locale, &'static str)> {
        self.keyword_index.get(keyword).copied()
    }

    /// Check if a string is a keyword in any locale.
    pub fn is_keyword(&self, text: &str) -> bool {
        self.keyword_index.contains_key(text)
    }

    /// Get the table for a specific locale.
    pub fn table_for(&self, locale: Locale) -> Option<&LocaleTable> {
        self.tables.iter().find(|t| t.locale == locale)
    }

    /// Number of registered locales.
    pub fn locale_count(&self) -> usize {
        self.tables.len()
    }

    /// Total number of keywords across all locales.
    pub fn total_keywords(&self) -> usize {
        self.keyword_index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_table_is_canonical() {
        let t = english_table();
        assert_eq!(t.locale, Locale::EN);
        // English keywords map to themselves.
        assert_eq!(t.resolve("if"), Some("if"));
        assert_eq!(t.resolve("let"), Some("let"));
        assert_eq!(t.resolve("return"), Some("return"));
        assert_eq!(t.resolve("fn"), Some("fn"));
    }

    #[test]
    fn english_table_has_all_keywords() {
        let t = english_table();
        for kw in ENGLISH_KEYWORDS {
            assert!(t.resolve(kw).is_some(), "missing English keyword: {kw}");
        }
    }

    #[test]
    fn chinese_table_maps_to_english() {
        let t = chinese_table();
        assert_eq!(t.locale, Locale::ZH);
        assert_eq!(t.resolve("如果"), Some("if"));
        assert_eq!(t.resolve("返回"), Some("return"));
        assert_eq!(t.resolve("让"), Some("let"));
        assert_eq!(t.resolve("函数"), Some("fn"));
        assert_eq!(t.resolve("真"), Some("true"));
    }

    #[test]
    fn chinese_table_not_empty() {
        let t = chinese_table();
        assert!(!t.is_empty());
        assert!(t.len() > 0);
    }

    #[test]
    fn registry_resolves_english() {
        let reg = LocaleRegistry::with_en_and_zh();
        let (locale, canonical) = reg.resolve("if").unwrap();
        assert_eq!(locale, Locale::EN);
        assert_eq!(canonical, "if");
    }

    #[test]
    fn registry_resolves_chinese() {
        let reg = LocaleRegistry::with_en_and_zh();
        let (locale, canonical) = reg.resolve("如果").unwrap();
        assert_eq!(locale, Locale::ZH);
        assert_eq!(canonical, "if");
    }

    #[test]
    fn registry_is_keyword() {
        let reg = LocaleRegistry::with_en_and_zh();
        assert!(reg.is_keyword("if"));
        assert!(reg.is_keyword("如果"));
        assert!(reg.is_keyword("let"));
        assert!(reg.is_keyword("让"));
        assert!(!reg.is_keyword("not_a_keyword"));
        assert!(!reg.is_keyword("xyz"));
    }

    #[test]
    fn registry_table_for_locale() {
        let reg = LocaleRegistry::with_en_and_zh();
        let en = reg.table_for(Locale::EN);
        assert!(en.is_some());
        assert_eq!(en.unwrap().locale, Locale::EN);
        let zh = reg.table_for(Locale::ZH);
        assert!(zh.is_some());
        assert_eq!(zh.unwrap().locale, Locale::ZH);
    }

    #[test]
    fn registry_locale_count() {
        let reg = LocaleRegistry::with_en_and_zh();
        assert_eq!(reg.locale_count(), 2);
    }

    #[test]
    fn registry_total_keywords() {
        let reg = LocaleRegistry::with_en_and_zh();
        // English keywords + Chinese keywords (minus overlaps, but
        // there shouldn't be any since they're different strings).
        let en_count = english_table().len();
        let zh_count = chinese_table().len();
        assert_eq!(reg.total_keywords(), en_count + zh_count);
    }

    #[test]
    fn locale_display() {
        assert_eq!(Locale::EN.to_string(), "en");
        assert_eq!(Locale::ZH.to_string(), "zh");
    }

    #[test]
    fn locale_code() {
        assert_eq!(Locale::EN.code(), "en");
        assert_eq!(Locale::ZH.code(), "zh");
    }

    #[test]
    fn locale_equality() {
        assert_eq!(Locale::EN, Locale::EN);
        assert_ne!(Locale::EN, Locale::ZH);
    }

    #[test]
    fn registry_default_is_en_and_zh() {
        let reg = LocaleRegistry::default();
        assert_eq!(reg.locale_count(), 2);
        assert!(reg.table_for(Locale::EN).is_some());
        assert!(reg.table_for(Locale::ZH).is_some());
    }
}
