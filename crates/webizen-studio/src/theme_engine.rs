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
                ("bg".to_string(), "#0a1122".to_string()), // Deep navy
                ("surface".to_string(), "rgba(20, 28, 48, 0.7)".to_string()), // Charcoal glass
                ("border".to_string(), "rgba(80, 90, 110, 0.5)".to_string()),
                ("text".to_string(), "#f8f9fb".to_string()),
                ("text-muted".to_string(), "#94a3b8".to_string()),
                ("accent".to_string(), "#f59e0b".to_string()), // Warm gold
                ("accent-glow".to_string(), "rgba(245, 158, 11, 0.18)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 20% 20%, rgba(245,158,11,0.10) 0%, transparent 50%), linear-gradient(160deg, #050a14 0%, #0a1122 100%)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "commons-light".to_string(),
            class_name: Some("theme-commons-light".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#faf9f6".to_string()), // Soft cream
                ("surface".to_string(), "rgba(255, 255, 255, 0.75)".to_string()),
                ("border".to_string(), "rgba(163, 177, 161, 0.4)".to_string()), // Sage-tinted border
                ("text".to_string(), "#2d3748".to_string()),
                ("text-muted".to_string(), "#718096".to_string()),
                ("accent".to_string(), "#4a5568".to_string()), // Accessible slate/sage
                ("accent-glow".to_string(), "rgba(74, 85, 104, 0.15)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(ellipse at 20% 20%, rgba(163,177,161,0.15) 0%, transparent 50%), linear-gradient(160deg, #ffffff 0%, #f4f5f0 100%)".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "sanctuary".to_string(),
            class_name: Some("theme-sanctuary".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#fefeff".to_string()),
                ("surface".to_string(), "rgba(244, 246, 248, 0.95)".to_string()), // High clarity, opaque glass
                ("border".to_string(), "rgba(200, 205, 212, 0.8)".to_string()),
                ("text".to_string(), "#1a202c".to_string()), // High contrast text
                ("text-muted".to_string(), "#4a5568".to_string()),
                ("accent".to_string(), "#2b6cb0".to_string()), // Calm, trustworthy blue
                ("accent-glow".to_string(), "rgba(43, 108, 176, 0.1)".to_string()),
                ("bg-gradient".to_string(), "none".to_string()), // Muted, gentle
                ("motion-duration".to_string(), "0ms".to_string()),
                ("motion-ease".to_string(), "linear".to_string()),
            ]),
        },
        ThemeDefinition {
            id: "infosphere".to_string(),
            class_name: Some("theme-infosphere".to_string()),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), "#050510".to_string()), // Deep space
                ("surface".to_string(), "rgba(18, 15, 38, 0.6)".to_string()),
                ("border".to_string(), "rgba(78, 65, 128, 0.5)".to_string()),
                ("text".to_string(), "#e0def4".to_string()),
                ("text-muted".to_string(), "#908caa".to_string()),
                ("accent".to_string(), "#eb6f92".to_string()), // Soft rose / neural
                ("accent-glow".to_string(), "rgba(235, 111, 146, 0.25)".to_string()),
                ("bg-gradient".to_string(), "radial-gradient(circle at 50% 50%, rgba(235,111,146,0.1) 0%, transparent 40%), radial-gradient(circle at 10% 80%, rgba(156,207,216,0.1) 0%, transparent 30%), linear-gradient(180deg, #020208 0%, #050510 100%)".to_string()),
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

/// Motion, elevation, typography, and focus tokens shared by every QPrime scope.
pub fn qprime_system_token_pairs() -> [(&'static str, &'static str); 12] {
    [
        ("elevation-0", "none"),
        (
            "elevation-1",
            "0 12px 26px rgba(0, 0, 0, 0.18)",
        ),
        (
            "elevation-2",
            "0 22px 50px rgba(0, 0, 0, 0.28)",
        ),
        (
            "elevation-3",
            "0 28px 80px rgba(0, 0, 0, 0.38)",
        ),
        ("motion-duration", "220ms"),
        ("motion-ease", "cubic-bezier(0.22, 1, 0.36, 1)"),
        ("type-scale", "1"),
        ("type-scale-sm", "0.875"),
        ("type-scale-lg", "1.125"),
        (
            "focus-ring",
            "0 0 0 3px var(--qualia-accent-glow, rgba(245, 158, 11, 0.35))",
        ),
        ("focus-ring-color", "var(--qualia-accent, #f59e0b)"),
        ("focus-ring-offset", "2px"),
    ]
}

/// Shoelace design tokens bridged from Qualia accent/surface tokens.
pub fn shoelace_bridge_css(selector: &str, theme: &ResolvedTheme) -> String {
    let accent = theme
        .tokens
        .get("accent")
        .map(String::as_str)
        .unwrap_or("#f59e0b");
    let surface = theme
        .tokens
        .get("surface")
        .map(String::as_str)
        .unwrap_or("rgba(20, 28, 48, 0.7)");
    let text = theme
        .tokens
        .get("text")
        .map(String::as_str)
        .unwrap_or("#f8f9fb");
    format!(
        "{selector} {{
  --sl-color-primary-600: {accent};
  --sl-color-primary-500: {accent};
  --sl-color-primary-400: {accent};
  --sl-color-neutral-0: {surface};
  --sl-color-neutral-50: {surface};
  --sl-color-neutral-900: {text};
  --sl-focus-ring: var(--qualia-focus-ring);
  --sl-transition-fast: var(--qualia-motion-duration);
  --sl-transition-medium: var(--qualia-motion-duration);
}}
"
    )
}

pub fn render_scope_tokens(selector: &str, theme: &ResolvedTheme) -> Option<String> {
    let mut css = format!("{selector} {{\n");
    for (token, value) in qprime_system_token_pairs() {
        css.push_str("  --qualia-");
        css.push_str(token);
        css.push_str(": ");
        css.push_str(value);
        css.push_str(";\n");
    }
    let mut pairs: Vec<_> = theme.tokens.iter().collect();
    pairs.sort_by(|left, right| left.0.cmp(right.0));
    for (token, value) in pairs {
        css.push_str("  --qualia-");
        css.push_str(token);
        css.push_str(": ");
        css.push_str(value);
        css.push_str(";\n");
    }
    css.push_str("}\n");
    css.push_str(&shoelace_bridge_css(selector, theme));
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

/// Human-readable provenance for inspector chips (Phase 2B).
pub fn theme_binding_provenance(binding: &ThemeBinding) -> &'static str {
    if !binding.tokens.is_empty()
        || binding
            .stylesheet_href
            .as_ref()
            .is_some_and(|h| !h.trim().is_empty())
        || binding
            .class_name
            .as_ref()
            .is_some_and(|c| !c.trim().is_empty())
    {
        "Locally overridden"
    } else if binding.theme_id.is_some() {
        "Inherited from preset"
    } else {
        "Workspace default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoelace_bridge_maps_accent() {
        let theme = resolve_theme(
            Some(&ThemeBinding {
                theme_id: Some("fiduciary-dark".to_string()),
                ..Default::default()
            }),
            &builtin_theme_catalog(),
        );
        let css = shoelace_bridge_css(":root", &theme);
        assert!(css.contains("--sl-color-primary-600: #f59e0b"));
    }

    #[test]
    fn sanctuary_zero_motion_override() {
        let theme = builtin_theme_catalog()
            .into_iter()
            .find(|t| t.id == "sanctuary")
            .expect("sanctuary preset");
        assert_eq!(theme.tokens.get("motion-duration").map(String::as_str), Some("0ms"));
    }
}

fn push_stylesheet(stylesheets: &mut Vec<String>, href: Option<String>) {
    if let Some(href) = href {
        if !href.trim().is_empty() && !stylesheets.iter().any(|existing| existing == &href) {
            stylesheets.push(href);
        }
    }
}
