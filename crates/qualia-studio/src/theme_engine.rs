use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ThemeDefinition {
    pub id: String,
    #[serde(default)]
    pub stylesheet_href: Option<String>,
    #[serde(default)]
    pub class_name: Option<String>,
    #[serde(default)]
    pub tokens: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ThemeBinding {
    #[serde(default)]
    pub theme_id: Option<String>,
    #[serde(default)]
    pub stylesheet_href: Option<String>,
    #[serde(default)]
    pub class_name: Option<String>,
    #[serde(default)]
    pub tokens: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedTheme {
    pub theme_key: Option<String>,
    pub class_name: Option<String>,
    pub stylesheets: Vec<String>,
    pub tokens: HashMap<String, String>,
}

pub fn builtin_theme_catalog() -> Vec<ThemeDefinition> {
    vec![
        ThemeDefinition {
            id: "fiduciary-dark".to_string(),
            class_name: Some("theme-fiduciary-dark".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#09090b".to_string()),
                ("surface".to_string(), "rgba(24, 24, 27, 0.7)".to_string()),
                ("border".to_string(), "rgba(63, 63, 70, 0.5)".to_string()),
                ("text".to_string(), "#f4f4f5".to_string()),
                ("text-muted".to_string(), "#a1a1aa".to_string()),
                ("accent".to_string(), "#06b6d4".to_string()),
                ("accent-glow".to_string(), "rgba(6, 182, 212, 0.2)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "commons-light".to_string(),
            class_name: Some("theme-commons-light".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#f7f3ea".to_string()),
                ("surface".to_string(), "rgba(255, 252, 247, 0.92)".to_string()),
                ("border".to_string(), "rgba(181, 154, 124, 0.45)".to_string()),
                ("text".to_string(), "#33261d".to_string()),
                ("text-muted".to_string(), "#7a6555".to_string()),
                ("accent".to_string(), "#c26d2c".to_string()),
                ("accent-glow".to_string(), "rgba(194, 109, 44, 0.22)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "forest-ledger".to_string(),
            class_name: Some("theme-forest-ledger".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#0c1612".to_string()),
                ("surface".to_string(), "rgba(16, 33, 26, 0.82)".to_string()),
                ("border".to_string(), "rgba(72, 108, 91, 0.45)".to_string()),
                ("text".to_string(), "#edf6f0".to_string()),
                ("text-muted".to_string(), "#93b3a0".to_string()),
                ("accent".to_string(), "#7dd3a7".to_string()),
                ("accent-glow".to_string(), "rgba(125, 211, 167, 0.24)".to_string()),
            ]),
        },
    ]
}

pub fn resolve_theme(binding: Option<&ThemeBinding>, catalog: &[ThemeDefinition]) -> ResolvedTheme {
    let Some(binding) = binding else {
        return ResolvedTheme::default();
    };

    let mut resolved = ResolvedTheme::default();

    if let Some(theme_id) = binding.theme_id.as_ref() {
        resolved.theme_key = Some(theme_id.clone());

        if let Some(definition) = catalog.iter().find(|theme| theme.id == *theme_id) {
            resolved.class_name = definition.class_name.clone();
            push_stylesheet(&mut resolved.stylesheets, definition.stylesheet_href.clone());
            resolved.tokens.extend(definition.tokens.clone());
        }
    }

    if let Some(class_name) = binding.class_name.clone() {
        resolved.class_name = Some(class_name);
    }

    push_stylesheet(&mut resolved.stylesheets, binding.stylesheet_href.clone());
    resolved.tokens.extend(binding.tokens.clone());
    resolved
}

pub fn render_scope_tokens(selector: &str, theme: &ResolvedTheme) -> Option<String> {
    if theme.tokens.is_empty() {
        return None;
    }

    let mut pairs: Vec<_> = theme.tokens.iter().collect();
    pairs.sort_by(|left, right| left.0.cmp(right.0));

    let mut css = format!("{selector} {{\n");
    for (token, value) in pairs {
        css.push_str("  --qualia-");
        css.push_str(token);
        css.push_str(": ");
        css.push_str(value);
        css.push_str(";\n");
    }
    css.push_str("}\n");
    Some(css)
}

pub fn collect_stylesheets(themes: &[&ResolvedTheme]) -> Vec<String> {
    let mut hrefs = BTreeSet::new();
    for theme in themes {
        for href in theme.stylesheets.iter() {
            if !href.trim().is_empty() {
                hrefs.insert(href.clone());
            }
        }
    }
    hrefs.into_iter().collect()
}

pub fn join_theme_classes(base_class: &str, theme: &ResolvedTheme) -> String {
    match theme.class_name.as_deref() {
        Some(class_name) if !class_name.trim().is_empty() => {
            format!("{base_class} {class_name}")
        }
        _ => base_class.to_string(),
    }
}

fn push_stylesheet(stylesheets: &mut Vec<String>, href: Option<String>) {
    if let Some(href) = href {
        if !href.trim().is_empty() && !stylesheets.iter().any(|existing| existing == &href) {
            stylesheets.push(href);
        }
    }
}
