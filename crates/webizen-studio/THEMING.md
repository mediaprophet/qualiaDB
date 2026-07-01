# Webizen Studio Theming (QPrime Design System)

Webizen Studio implements a layered, human-centric theme engine. Instead of a single global token dump, themes can be attached at four scopes:

1. `environment` via `:root`
2. `app` via the studio shell
3. `page` via the active canvas page
4. `module` via an individual pane or QAPP/module wrapper

This keeps the renderer CSS-first and zero-JS while allowing a workspace to mix a broad visual identity with local overrides.

## Human-Centric Token Architecture

The aesthetic and semantic architecture reinforce each other. The interface is an extension of human agency, dignity, and curiosity.

### Core Semantic Tokens

| Token | Purpose & Semantic Meaning |
|---|---|
| `--qualia-bg` | Primary background. Establishes the core mood and spatial void. |
| `--qualia-surface` | Elevated panes/cards. Opacity dictates the "glassmorphism" clarity. |
| `--qualia-border` | Dividers. Represents consent boundaries and structural edges. |
| `--qualia-text` | Main text. High legibility and contrast for accessibility. |
| `--qualia-text-muted` | Secondary text for provenance and metadata. |
| `--qualia-accent` | Action/highlight color. Represents warmth, human presence, or agency. |
| `--qualia-accent-glow` | Accent shadow/glow. Emphasizes focus and semantic relationships. |
| `--qualia-bg-gradient` | Premium dynamic background gradient for environmental depth. |

### Semantic Depth & Elevation (Z-Space)

Elevation in QPrime maps directly to *semantic depth*:
*   **Base Level (Z=0)**: The infosphere itself (background).
*   **Level 1 (Z=10, Blur=12px)**: Knowledge layers and standard panes (`--qualia-surface`).
*   **Level 2 (Z=20, Blur=24px)**: Provenance overlays, consent boundaries, and Episteme prompts.
*   **Level 3 (Z=30, Blur=32px)**: Active agency affordances (drag operations, focused editors).

*Note: Glassmorphism is achieved via `backdrop-filter: blur(X)` combined with semi-transparent `rgba()` values on `--qualia-surface`.*

## Built-In Premium Presets

The engine provides 4 built-in presets designed for specific cognitive states:

1.  **Fiduciary Dark** (Default)
    *   *Palette*: Deep navy/charcoal glass (`#0a1122`), warm gold accents (`#f59e0b`).
    *   *Vibe*: Conveys trust, depth, and careful stewardship.
2.  **Commons Light**
    *   *Palette*: Soft cream (`#faf9f6`), sage-tinted borders, warm accessible slate (`#4a5568`).
    *   *Vibe*: Collaborative, open, daylight feel.
3.  **Sanctuary Mode**
    *   *Palette*: Muted, high-clarity (`#fefeff`), calm trustworthy blue (`#2b6cb0`).
    *   *Vibe*: For sensitive work, wellbeing review, or when the user needs calm (higher contrast, reduced motion).
4.  **Infosphere**
    *   *Palette*: Deep space (`#050510`), soft rose/neural accents (`#eb6f92`).
    *   *Vibe*: Experimental, semantic exploration.

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

## How Scoping Works

The renderer emits token blocks for each active scope:

```css
:root { --qualia-bg: #0a1122; }
.webizen-studio-shell { --qualia-accent: #f59e0b; }
.webizen-page-shell { --qualia-surface: rgba(20, 28, 48, 0.7); }
.webizen-module-pane[data-pane-index='2'] { --qualia-border: #7dd3a7; }
```

It also annotates the DOM so theme CSS files can target the same scopes safely:

```html
<div class="webizen-studio-shell theme-fiduciary-dark" data-theme-scope="app" data-theme="fiduciary-dark">
<div class="webizen-page-shell report-theme" data-theme-scope="page" data-theme="commons-light">
<div class="webizen-module-pane chart-theme" data-theme-scope="module" data-theme="sanctuary">
```

## Guidance For Modules

Custom modules should consume tokens rather than hardcoding colors:

```css
.my-custom-pane {
    background: var(--qualia-surface, #111);
    color: var(--qualia-text, #fff);
    border: 1px solid var(--qualia-border, #333);
    /* Glassmorphism */
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
}

.my-custom-pane:hover {
    box-shadow: 0 0 10px var(--qualia-accent-glow);
    /* Micro-animation */
    transition: box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
```

That lets the same module inherit an environment theme, participate in an app theme, and still accept a module-specific override when needed.
