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
    /// Chinese locale.
    pub const ZH: Locale = Locale("zh");
    /// Spanish locale.
    pub const ES: Locale = Locale("es");
    /// Japanese locale.
    pub const JA: Locale = Locale("ja");
    /// Arabic locale.
    pub const AR: Locale = Locale("ar");
    /// Hindi locale.
    pub const HI: Locale = Locale("hi");
    /// French locale.
    pub const FR: Locale = Locale("fr");
    /// German locale.
    pub const DE: Locale = Locale("de");

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
    for kw in ENGLISH_KEYWORDS {
        t.keywords.insert(kw, kw);
    }
    t
}

/// The Chinese locale table.
pub fn chinese_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::ZH);
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
    t.keywords.insert("单元", "cell");
    t.keywords.insert("模块", "module");
    t.keywords.insert("导入", "import");
    t.keywords.insert("作为", "as");
    t.keywords.insert("要求", "requires");
    t.keywords.insert("能力", "capability");
    t.keywords.insert("异步", "async");
    t.keywords.insert("作用", "effect");
    t.keywords.insert("纯", "pure");
    t.keywords.insert("事务", "transaction");
    t.keywords.insert("等待", "await");
    t.keywords.insert("产出", "yield");
    t.keywords.insert("前缀", "prefix");
    t.keywords.insert("于", "on");
    t.keywords.insert("枚举", "enum");
    t.keywords.insert("场", "field");
    t.keywords.insert("材料", "material");
    t.keywords.insert("定律", "law");
    t.keywords.insert("当条件", "when");
    t.keywords.insert("热", "hot");
    t.keywords.insert("冷", "cold");
    t.keywords.insert("使用", "using");
    t.keywords.insert("语言", "locale");
    t.keywords.insert("呈现", "present");
    t.keywords.insert("义务", "obligate");
    t.keywords.insert("允许", "permit");
    t.keywords.insert("禁止", "forbid");
    t.keywords.insert("知道", "knows");
    t.keywords.insert("相信", "believes");
    t.keywords.insert("总是", "always");
    t.keywords.insert("终将", "eventually");
    t.keywords.insert("直到", "until");
    t.keywords.insert("绑定", "bind");
    t
}

/// The Spanish locale table.
pub fn spanish_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::ES);
    t.keywords.insert("si", "if");
    t.keywords.insert("sino", "else");
    t.keywords.insert("para", "for");
    t.keywords.insert("en", "in");
    t.keywords.insert("mientras", "while");
    t.keywords.insert("coincidir", "match");
    t.keywords.insert("retornar", "return");
    t.keywords.insert("sea", "let");
    t.keywords.insert("mutable", "mut");
    t.keywords.insert("constante", "const");
    t.keywords.insert("verdadero", "true");
    t.keywords.insert("falso", "false");
    t.keywords.insert("nulo", "null");
    t.keywords.insert("fun", "fn");
    t.keywords.insert("célula", "cell");
    t.keywords.insert("módulo", "module");
    t.keywords.insert("importar", "import");
    t.keywords.insert("como", "as");
    t.keywords.insert("requiere", "requires");
    t.keywords.insert("capacidad", "capability");
    t.keywords.insert("asinc", "async");
    t.keywords.insert("efecto", "effect");
    t.keywords.insert("puro", "pure");
    t.keywords.insert("transacción", "transaction");
    t.keywords.insert("esperar", "await");
    t.keywords.insert("ceder", "yield");
    t.keywords.insert("prefijo", "prefix");
    t.keywords.insert("al", "on");
    t.keywords.insert("enumeración", "enum");
    t.keywords.insert("campo", "field");
    t.keywords.insert("material", "material");
    t.keywords.insert("ley", "law");
    t.keywords.insert("cuando", "when");
    t.keywords.insert("caliente", "hot");
    t.keywords.insert("frío", "cold");
    t.keywords.insert("usando", "using");
    t.keywords.insert("idioma", "locale");
    t.keywords.insert("presentar", "present");
    t.keywords.insert("obligar", "obligate");
    t.keywords.insert("permitir", "permit");
    t.keywords.insert("prohibir", "forbid");
    t.keywords.insert("conoce", "knows");
    t.keywords.insert("cree", "believes");
    t.keywords.insert("siempre", "always");
    t.keywords.insert("eventualmente", "eventually");
    t.keywords.insert("hastaque", "until");
    t.keywords.insert("vincular", "bind");
    t
}

/// The Japanese locale table.
pub fn japanese_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::JA);
    t.keywords.insert("もし", "if");
    t.keywords.insert("他", "else");
    t.keywords.insert("反復", "for");
    t.keywords.insert("要素", "in");
    t.keywords.insert("間", "while");
    t.keywords.insert("一致", "match");
    t.keywords.insert("戻す", "return");
    t.keywords.insert("設", "let");
    t.keywords.insert("可変", "mut");
    t.keywords.insert("定数", "const");
    t.keywords.insert("真", "true");
    t.keywords.insert("偽", "false");
    t.keywords.insert("無", "null");
    t.keywords.insert("関数", "fn");
    t.keywords.insert("セル", "cell");
    t.keywords.insert("モジュール", "module");
    t.keywords.insert("インポート", "import");
    t.keywords.insert("別名", "as");
    t.keywords.insert("要求", "requires");
    t.keywords.insert("機能", "capability");
    t.keywords.insert("非同期", "async");
    t.keywords.insert("作用", "effect");
    t.keywords.insert("純粋", "pure");
    t.keywords.insert("トランザクション", "transaction");
    t.keywords.insert("待機", "await");
    t.keywords.insert("譲渡", "yield");
    t.keywords.insert("接頭辞", "prefix");
    t.keywords.insert("時", "on");
    t.keywords.insert("列挙", "enum");
    t.keywords.insert("場", "field");
    t.keywords.insert("物質", "material");
    t.keywords.insert("法則", "law");
    t.keywords.insert("場合", "when");
    t.keywords.insert("高速", "hot");
    t.keywords.insert("低速", "cold");
    t.keywords.insert("使用中", "using");
    t.keywords.insert("言語", "locale");
    t.keywords.insert("提示", "present");
    t.keywords.insert("義務", "obligate");
    t.keywords.insert("許可", "permit");
    t.keywords.insert("禁止", "forbid");
    t.keywords.insert("知る", "knows");
    t.keywords.insert("信じる", "believes");
    t.keywords.insert("常に", "always");
    t.keywords.insert("やがて", "eventually");
    t.keywords.insert("迄", "until");
    t.keywords.insert("結合", "bind");
    t
}

/// The Arabic locale table.
pub fn arabic_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::AR);
    t.keywords.insert("إذا", "if");
    t.keywords.insert("وإلا", "else");
    t.keywords.insert("لكل", "for");
    t.keywords.insert("في", "in");
    t.keywords.insert("طالما", "while");
    t.keywords.insert("طابق", "match");
    t.keywords.insert("أرجع", "return");
    t.keywords.insert("دع", "let");
    t.keywords.insert("متغير", "mut");
    t.keywords.insert("ثابت", "const");
    t.keywords.insert("صحيح", "true");
    t.keywords.insert("خطأ", "false");
    t.keywords.insert("فارغ", "null");
    t.keywords.insert("دالة", "fn");
    t.keywords.insert("خلية", "cell");
    t.keywords.insert("وحدة", "module");
    t.keywords.insert("استيراد", "import");
    t.keywords.insert("باسم", "as");
    t.keywords.insert("يتطلب", "requires");
    t.keywords.insert("صلاحية", "capability");
    t.keywords.insert("لاتزامني", "async");
    t.keywords.insert("تأثير", "effect");
    t.keywords.insert("نقي", "pure");
    t.keywords.insert("معاملة", "transaction");
    t.keywords.insert("انتظر", "await");
    t.keywords.insert("أنتج", "yield");
    t.keywords.insert("بادئة", "prefix");
    t.keywords.insert("عند", "on");
    t.keywords.insert("تعداد", "enum");
    t.keywords.insert("حقل", "field");
    t.keywords.insert("مادة", "material");
    t.keywords.insert("قانون", "law");
    t.keywords.insert("حين", "when");
    t.keywords.insert("ساخن", "hot");
    t.keywords.insert("بارد", "cold");
    t.keywords.insert("باستخدام", "using");
    t.keywords.insert("لغة", "locale");
    t.keywords.insert("اعرض", "present");
    t.keywords.insert("يلزم", "obligate");
    t.keywords.insert("يسمح", "permit");
    t.keywords.insert("يحظر", "forbid");
    t.keywords.insert("يعلم", "knows");
    t.keywords.insert("يعتقد", "believes");
    t.keywords.insert("دائما", "always");
    t.keywords.insert("أخيرا", "eventually");
    t.keywords.insert("حتى", "until");
    t.keywords.insert("ربط", "bind");
    t
}

/// The Hindi locale table.
pub fn hindi_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::HI);
    t.keywords.insert("यदि", "if");
    t.keywords.insert("अन्यथा", "else");
    t.keywords.insert("के_लिए", "for");
    t.keywords.insert("में", "in");
    t.keywords.insert("जब_तक", "while");
    t.keywords.insert("मिलान", "match");
    t.keywords.insert("वापस", "return");
    t.keywords.insert("मान", "let");
    t.keywords.insert("परिवर्तनीय", "mut");
    t.keywords.insert("स्थिरांक", "const");
    t.keywords.insert("सत्य", "true");
    t.keywords.insert("असत्य", "false");
    t.keywords.insert("रिक्त", "null");
    t.keywords.insert("कार्य", "fn");
    t.keywords.insert("सेल", "cell");
    t.keywords.insert("मॉड्यूल", "module");
    t.keywords.insert("आयात", "import");
    t.keywords.insert("के_रूप_में", "as");
    t.keywords.insert("आवश्यकता", "requires");
    t.keywords.insert("क्षमता", "capability");
    t.keywords.insert("अतुल्यकालिक", "async");
    t.keywords.insert("प्रभाव", "effect");
    t.keywords.insert("शुद्ध", "pure");
    t.keywords.insert("लेनदेन", "transaction");
    t.keywords.insert("प्रतीक्षा", "await");
    t.keywords.insert("उत्पादन", "yield");
    t.keywords.insert("उपसर्ग", "prefix");
    t.keywords.insert("पर", "on");
    t.keywords.insert("गणना", "enum");
    t.keywords.insert("क्षेत्र", "field");
    t.keywords.insert("सामग्री", "material");
    t.keywords.insert("नियम", "law");
    t.keywords.insert("जब", "when");
    t.keywords.insert("तप्त", "hot");
    t.keywords.insert("शीत", "cold");
    t.keywords.insert("उपयोग", "using");
    t.keywords.insert("भाषा", "locale");
    t.keywords.insert("प्रस्तुत", "present");
    t.keywords.insert("बाध्य", "obligate");
    t.keywords.insert("अनुमति", "permit");
    t.keywords.insert("निषेध", "forbid");
    t.keywords.insert("जानता", "knows");
    t.keywords.insert("मानता", "believes");
    t.keywords.insert("हमेशा", "always");
    t.keywords.insert("अंततः", "eventually");
    t.keywords.insert("जबतक", "until");
    t.keywords.insert("बांध", "bind");
    t
}

/// The French locale table.
pub fn french_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::FR);
    t.keywords.insert("si", "if");
    t.keywords.insert("sinon", "else");
    t.keywords.insert("pour", "for");
    t.keywords.insert("dans", "in");
    t.keywords.insert("tantque", "while");
    t.keywords.insert("correspondre", "match");
    t.keywords.insert("retourner", "return");
    t.keywords.insert("soit", "let");
    t.keywords.insert("mutable", "mut");
    t.keywords.insert("constante", "const");
    t.keywords.insert("vrai", "true");
    t.keywords.insert("faux", "false");
    t.keywords.insert("nul", "null");
    t.keywords.insert("fonction", "fn");
    t.keywords.insert("cellule", "cell");
    t.keywords.insert("module", "module");
    t.keywords.insert("importer", "import");
    t.keywords.insert("comme", "as");
    t.keywords.insert("exige", "requires");
    t.keywords.insert("capacité", "capability");
    t.keywords.insert("asynchrone", "async");
    t.keywords.insert("effet", "effect");
    t.keywords.insert("pur", "pure");
    t.keywords.insert("transaction", "transaction");
    t.keywords.insert("attendre", "await");
    t.keywords.insert("produire", "yield");
    t.keywords.insert("préfixe", "prefix");
    t.keywords.insert("sur", "on");
    t.keywords.insert("énum", "enum");
    t.keywords.insert("champ", "field");
    t.keywords.insert("matériau", "material");
    t.keywords.insert("loi", "law");
    t.keywords.insert("lorsque", "when");
    t.keywords.insert("chaud", "hot");
    t.keywords.insert("froid", "cold");
    t.keywords.insert("utilisant", "using");
    t.keywords.insert("langue", "locale");
    t.keywords.insert("présenter", "present");
    t.keywords.insert("obliger", "obligate");
    t.keywords.insert("permettre", "permit");
    t.keywords.insert("interdire", "forbid");
    t.keywords.insert("connait", "knows");
    t.keywords.insert("croit", "believes");
    t.keywords.insert("toujours", "always");
    t.keywords.insert("finalement", "eventually");
    t.keywords.insert("jusqua", "until");
    t.keywords.insert("lier", "bind");
    t
}

/// The German locale table.
pub fn german_table() -> LocaleTable {
    let mut t = LocaleTable::new(Locale::DE);
    t.keywords.insert("wenn", "if");
    t.keywords.insert("sonst", "else");
    t.keywords.insert("für", "for");
    t.keywords.insert("in", "in");
    t.keywords.insert("solange", "while");
    t.keywords.insert("abgleichen", "match");
    t.keywords.insert("zurück", "return");
    t.keywords.insert("sei", "let");
    t.keywords.insert("veränderlich", "mut");
    t.keywords.insert("konstante", "const");
    t.keywords.insert("wahr", "true");
    t.keywords.insert("falsch", "false");
    t.keywords.insert("null", "null");
    t.keywords.insert("funktion", "fn");
    t.keywords.insert("zelle", "cell");
    t.keywords.insert("modul", "module");
    t.keywords.insert("importieren", "import");
    t.keywords.insert("als", "as");
    t.keywords.insert("erfordert", "requires");
    t.keywords.insert("fähigkeit", "capability");
    t.keywords.insert("asynchron", "async");
    t.keywords.insert("wirkung", "effect");
    t.keywords.insert("rein", "pure");
    t.keywords.insert("transaktion", "transaction");
    t.keywords.insert("abwarten", "await");
    t.keywords.insert("ergeben", "yield");
    t.keywords.insert("präfix", "prefix");
    t.keywords.insert("bei", "on");
    t.keywords.insert("aufzählung", "enum");
    t.keywords.insert("feld", "field");
    t.keywords.insert("material", "material");
    t.keywords.insert("gesetz", "law");
    t.keywords.insert("wann", "when");
    t.keywords.insert("heiß", "hot");
    t.keywords.insert("kalt", "cold");
    t.keywords.insert("nutzend", "using");
    t.keywords.insert("sprache", "locale");
    t.keywords.insert("präsentieren", "present");
    t.keywords.insert("verpflichten", "obligate");
    t.keywords.insert("erlauben", "permit");
    t.keywords.insert("verbieten", "forbid");
    t.keywords.insert("weiss", "knows");
    t.keywords.insert("glaubt", "believes");
    t.keywords.insert("immer", "always");
    t.keywords.insert("schliesslich", "eventually");
    t.keywords.insert("solangebis", "until");
    t.keywords.insert("binden", "bind");
    t
}

/// The canonical English keyword list.
pub const ENGLISH_KEYWORDS: &[&str] = &[
    "module",
    "import",
    "as",
    "prefix",
    "requires",
    "capability",
    "fn",
    "cell",
    "async",
    "on",
    "let",
    "mut",
    "const",
    "enum",
    "field",
    "material",
    "law",
    "when",
    "if",
    "else",
    "for",
    "in",
    "while",
    "match",
    "return",
    "yield",
    "transaction",
    "await",
    "true",
    "false",
    "null",
    "effect",
    "pure",
    "hot",
    "cold",
    "using",
    "locale",
    "present",
    "obligate",
    "permit",
    "forbid",
    "knows",
    "believes",
    "always",
    "eventually",
    "until",
    "bind",
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
        Self::with_all_locales()
    }
}

impl LocaleRegistry {
    /// Create a registry with all supported locales: en, zh, es, ja, ar, hi, fr, de.
    pub fn with_all_locales() -> Self {
        Self::new(vec![
            english_table(),
            chinese_table(),
            spanish_table(),
            japanese_table(),
            arabic_table(),
            hindi_table(),
            french_table(),
            german_table(),
        ])
    }

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
        Self {
            tables,
            keyword_index,
        }
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
    fn registry_resolves_all_locales() {
        let reg = LocaleRegistry::with_all_locales();
        assert_eq!(reg.resolve("sea").unwrap(), (Locale::ES, "let"));
        assert_eq!(reg.resolve("soit").unwrap(), (Locale::FR, "let"));
        assert_eq!(reg.resolve("もし").unwrap(), (Locale::JA, "if"));
        assert_eq!(reg.resolve("إذا").unwrap(), (Locale::AR, "if"));
        assert_eq!(reg.resolve("यदि").unwrap(), (Locale::HI, "if"));
        assert_eq!(reg.resolve("wenn").unwrap(), (Locale::DE, "if"));
        assert_eq!(reg.resolve("如果").unwrap(), (Locale::ZH, "if"));
        assert_eq!(reg.resolve("if").unwrap(), (Locale::EN, "if"));
    }

    #[test]
    fn registry_locale_count_all() {
        let reg = LocaleRegistry::with_all_locales();
        assert_eq!(reg.locale_count(), 8);
    }

    #[test]
    fn registry_default_is_all_locales() {
        let reg = LocaleRegistry::default();
        assert_eq!(reg.locale_count(), 8);
        assert!(reg.table_for(Locale::EN).is_some());
        assert!(reg.table_for(Locale::ZH).is_some());
        assert!(reg.table_for(Locale::ES).is_some());
        assert!(reg.table_for(Locale::JA).is_some());
        assert!(reg.table_for(Locale::AR).is_some());
        assert!(reg.table_for(Locale::HI).is_some());
        assert!(reg.table_for(Locale::FR).is_some());
        assert!(reg.table_for(Locale::DE).is_some());
    }

    #[test]
    fn chinese_modal_verbs_map_to_english() {
        let t = chinese_table();
        assert_eq!(t.resolve("义务"), Some("obligate"));
        assert_eq!(t.resolve("允许"), Some("permit"));
        assert_eq!(t.resolve("禁止"), Some("forbid"));
        assert_eq!(t.resolve("知道"), Some("knows"));
        assert_eq!(t.resolve("相信"), Some("believes"));
        assert_eq!(t.resolve("总是"), Some("always"));
        assert_eq!(t.resolve("终将"), Some("eventually"));
        assert_eq!(t.resolve("直到"), Some("until"));
    }
}
