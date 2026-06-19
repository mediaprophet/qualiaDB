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
            id: "human-warmth".to_string(),
            class_name: Some("theme-human-warmth".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#fbf9f6".to_string()),
                ("surface".to_string(), "rgba(255, 255, 255, 0.72)".to_string()),
                ("border".to_string(), "rgba(220, 210, 200, 0.55)".to_string()),
                ("text".to_string(), "#2d2824".to_string()),
                ("text-muted".to_string(), "#8b8178".to_string()),
                ("accent".to_string(), "#e07a5f".to_string()),
                ("accent-glow".to_string(), "rgba(224, 122, 95, 0.18)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 20% 15%, rgba(240,175,145,0.38) 0%, transparent 55%), radial-gradient(ellipse at 80% 75%, rgba(230,195,155,0.28) 0%, transparent 50%), linear-gradient(160deg, #fdf6f0 0%, #f5e8da 100%)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "twilight-blue".to_string(),
            class_name: Some("theme-twilight-blue".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#1e212b".to_string()),
                ("surface".to_string(), "rgba(39, 43, 56, 0.6)".to_string()),
                ("border".to_string(), "rgba(74, 85, 104, 0.4)".to_string()),
                ("text".to_string(), "#f0f4f8".to_string()),
                ("text-muted".to_string(), "#a0aec0".to_string()),
                ("accent".to_string(), "#4fd1c5".to_string()),
                ("accent-glow".to_string(), "rgba(79, 209, 197, 0.22)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 20% 15%, rgba(79,209,197,0.18) 0%, transparent 50%), radial-gradient(ellipse at 80% 80%, rgba(59,130,246,0.12) 0%, transparent 50%), linear-gradient(160deg, #1a1e2a 0%, #20263a 100%)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "midnight-slate".to_string(),
            class_name: Some("theme-midnight-slate".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#0f111a".to_string()),
                ("surface".to_string(), "rgba(23, 26, 38, 0.65)".to_string()),
                ("border".to_string(), "rgba(45, 51, 74, 0.5)".to_string()),
                ("text".to_string(), "#e2e8f0".to_string()),
                ("text-muted".to_string(), "#94a3b8".to_string()),
                ("accent".to_string(), "#818cf8".to_string()),
                ("accent-glow".to_string(), "rgba(129, 140, 248, 0.18)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 25% 20%, rgba(129,140,248,0.14) 0%, transparent 50%), radial-gradient(ellipse at 75% 80%, rgba(99,102,241,0.10) 0%, transparent 50%), linear-gradient(160deg, #0d0f18 0%, #121420 100%)".to_string()),
            ]),
        },
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
                ("accent-glow".to_string(), "rgba(6, 182, 212, 0.18)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 20% 20%, rgba(6,182,212,0.10) 0%, transparent 50%), linear-gradient(160deg, #07070a 0%, #0b0b0f 100%)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "forest-moss".to_string(),
            class_name: Some("theme-forest-moss".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#f2f5f0".to_string()),
                ("surface".to_string(), "rgba(240, 245, 235, 0.75)".to_string()),
                ("border".to_string(), "rgba(180, 200, 170, 0.5)".to_string()),
                ("text".to_string(), "#1e2d1a".to_string()),
                ("text-muted".to_string(), "#5a7050".to_string()),
                ("accent".to_string(), "#3d8b5e".to_string()),
                ("accent-glow".to_string(), "rgba(61, 139, 94, 0.18)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 20% 15%, rgba(100,180,120,0.22) 0%, transparent 55%), radial-gradient(ellipse at 80% 80%, rgba(60,140,80,0.15) 0%, transparent 50%), linear-gradient(160deg, #eef4e8 0%, #e0edd6 100%)".to_string()),
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
            push_stylesheet(
                &mut resolved.stylesheets,
                definition.stylesheet_href.clone(),
            );
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
