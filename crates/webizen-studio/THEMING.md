# Webizen Studio Theming

Webizen Studio now supports layered themes instead of a single global token dump. A theme can be attached at four scopes:

1. `environment` via `:root`
2. `app` via the studio shell
3. `page` via the active canvas page
4. `module` via an individual pane or Qapp/module wrapper

This keeps the renderer CSS-first and zero-JS while still letting a workspace mix broad visual identity with local overrides.

## Theme Model

The manifest can now carry a theme catalog plus scoped bindings:

```rust
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
    pub theme_tokens: HashMap<String, String>, // legacy environment overrides
    pub themes: Vec<ThemeDefinition>,
    pub environment_theme: ThemeBinding,
    pub app_theme: ThemeBinding,
}

pub struct Page {
    pub theme: ThemeBinding,
}

pub struct PanePlacement {
    pub theme: ThemeBinding,
}
```

`ThemeDefinition` is a reusable preset. `ThemeBinding` is what gets attached to a scope. Bindings can reference a preset with `theme_id`, add local token overrides, and optionally load a stylesheet.

```rust
pub struct ThemeDefinition {
    pub id: String,
    pub stylesheet_href: Option<String>,
    pub class_name: Option<String>,
    pub tokens: HashMap<String, String>,
}

pub struct ThemeBinding {
    pub theme_id: Option<String>,
    pub stylesheet_href: Option<String>,
    pub class_name: Option<String>,
    pub tokens: HashMap<String, String>,
}
```

## How Scoping Works

The renderer emits token blocks for each active scope:

```css
:root { --qualia-bg: #09090b; }
.webizen-studio-shell { --qualia-accent: #06b6d4; }
.webizen-page-shell { --qualia-surface: rgba(24, 24, 27, 0.7); }
.webizen-module-pane[data-pane-index='2'] { --qualia-border: #7dd3a7; }
```

It also annotates the DOM so theme CSS files can target the same scopes safely:

```html
<div class="webizen-studio-shell theme-fiduciary-dark" data-theme-scope="app" data-theme="fiduciary-dark">
<div class="webizen-page-shell report-theme" data-theme-scope="page" data-theme="commons-light">
<div class="webizen-module-pane chart-theme" data-theme-scope="module" data-theme="forest-ledger">
```

## Stylesheet Themes

For whole-theme CSS files, attach a `stylesheet_href` to either a preset or a binding. The studio will load the stylesheet once and let the CSS file decide what to style through the provided classes and `data-theme-*` attributes.

That means a theme file should avoid raw global selectors like `body { ... }` unless it is intentionally an environment-wide theme. Prefer scoped selectors such as:

```css
.webizen-page-shell[data-theme="commons-light"] .ledger-card {
    background: var(--qualia-surface);
    color: var(--qualia-text);
}

.webizen-module-pane[data-theme="forest-ledger"] .chart-title {
    color: var(--qualia-accent);
}
```

## Core Tokens

The current built-in presets share these semantic tokens:

| Token | Purpose |
|---|---|
| `--qualia-bg` | Primary background |
| `--qualia-surface` | Elevated panes/cards |
| `--qualia-border` | Dividers and outlines |
| `--qualia-text` | Main text |
| `--qualia-text-muted` | Secondary text |
| `--qualia-accent` | Action/highlight color |
| `--qualia-accent-glow` | Accent shadow/glow |

## Guidance For Modules

Custom modules should consume tokens rather than hardcoding colors:

```css
.my-custom-pane {
    background: var(--qualia-surface, #111);
    color: var(--qualia-text, #fff);
    border: 1px solid var(--qualia-border, #333);
}

.my-custom-pane:hover {
    box-shadow: 0 0 10px var(--qualia-accent-glow);
}
```

That lets the same module inherit an environment theme, participate in an app theme, and still accept a module-specific override when needed.
