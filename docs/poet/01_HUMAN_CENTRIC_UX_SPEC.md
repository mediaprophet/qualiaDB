# POET Human-Centric UX & Web Standards Specification

**Document ID:** `POET-SPEC-001`  
**Status:** Canonical UX Specification  
**Scope:** Frontend design system, interaction patterns, accessibility, and human-usability standards across all POET surfaces.

---

## 1. Principles of Human-Centric Design in POET

The primary failure mode of automated agent development is creating interfaces that serve code tests rather than human cognition. POET is strictly governed by the following UX mandates:

1. **Domain-Appropriate Surfaces over Generic CRUD:**  
   A project task belongs on an interactive Kanban board; an audio track belongs on a fader strip; an agreement belongs in a clause editor. Generic key-value tables and raw JSON textareas are strictly prohibited as primary end-user interfaces.
2. **Semantic & Intuitive Inputs:**  
   Never force a human user to type ISO timestamps, raw hex hashes, boolean integer flags (`0`/`1`), or JSON strings into text inputs. Every field must use domain-appropriate controls: visual date/time pickers, range sliders with live readouts, toggle switches, searchable dropdowns, and chip tag selectors.
3. **Continuous Visual & Tactile Feedback:**  
   Every user action must provide immediate visual feedback (optimistic UI state, subtle micro-animations, loading skeletons, and clear toast notifications). Operations must never leave the user wondering if a button click registered.
4. **Honest, Plain-Language Error Reporting:**  
   Errors must explain **what happened**, **why it failed**, and **the exact human action required to resolve it**, rather than dumping technical Rust stack traces or raw HTTP 500 status codes.

---

## 2. Design System & CSS Token Architecture

POET uses a unified Vanilla CSS design token system embedded in [`crates/poet/src/browser/css.rs`](file:///c:/Projects/qualia-27062026/crates/poet/src/browser/css.rs):

```css
:root {
  /* Spatial Depth & Canvas */
  --poet-bg-deep: #060913;
  --poet-bg-surface: #0e1424;
  --poet-bg-card: #151d33;
  --poet-bg-glass: rgba(21, 29, 51, 0.75);
  
  /* Accent & Modality Themes */
  --poet-accent-cyan: #00d2ff;
  --poet-accent-purple: #9d4edd;
  --poet-accent-emerald: #10b981;
  --poet-accent-amber: #f59e0b;
  --poet-accent-rose: #f43f5e;
  
  /* Typography Scale */
  --poet-font-sans: 'Inter', system-ui, -apple-system, sans-serif;
  --poet-font-mono: 'JetBrains Mono', 'Fira Code', monospace;
  --poet-text-xs: 0.75rem;
  --poet-text-sm: 0.875rem;
  --poet-text-base: 1.0rem;
  --poet-text-lg: 1.125rem;
  --poet-text-xl: 1.25rem;
  --poet-text-2xl: 1.5rem;
  
  /* Elevation & Shadows */
  --poet-shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
  --poet-shadow-md: 0 4px 12px rgba(0, 0, 0, 0.4);
  --poet-shadow-glow: 0 0 20px rgba(0, 210, 255, 0.25);
  --poet-border-subtle: rgba(255, 255, 255, 0.08);
  --poet-border-focus: #00d2ff;
}
```

---

## 3. Core Interaction Patterns

### 3.1 Forms & Input Components
- **Text Inputs:** Floating labels, placeholder hints, inline clear buttons (`✕`), and instant validation error messages below the field.
- **Number Inputs:** Increment/decrement buttons with step support, unit badges (e.g., `USD`, `ms`, `Hz`, `kg`), and min/max clamping.
- **Selects & Comboboxes:** Searchable dropdowns with fuzzy filtering, keyboard navigation (`↑`/`↓`/`Enter`), and clear empty states.
- **Date & Time Pickers:** Visual calendar grid, relative presets ("Today", "In 1 week", "End of Sprint"), and local timezone awareness.
- **Color Pickers:** Pre-defined harmonious palette swatches plus a fine hex/HSV picker.

### 3.2 Modals, Drawers & Popovers
- Modal dialogs with dark backdrop blur (`backdrop-filter: blur(8px)`), smooth fade-and-scale entrance, focus trapping, and `Escape` key dismissal.
- Slide-over contextual drawers for detailed record inspection and side-by-side editing.
- Contextual radial popovers on right-click or long-press on canvas objects.

### 3.3 Keyboard Shortcuts & Accessibility
- **Global Hotkeys:**
  - `Ctrl + K` / `Cmd + K`: Open Global Command Palette & Semantic Search.
  - `Alt + A`: Auto-arrange and tidy canvas containers.
  - `Ctrl + Z` / `Ctrl + Y`: Undo / Redo spatial and data operations.
  - `Space + Drag`: Pan the infinite Chora canvas.
  - `Ctrl + Scroll`: Zoom in/out smoothly toward cursor position.
- **Accessibility:** Full WCAG 2.1 AA compliance, high contrast focus rings, proper ARIA landmarks (`role="region"`, `role="dialog"`), and screen-reader announcements on live updates.

---

## 4. UX Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-UX-001` | **No Raw Technical Inputs** | Eliminate all raw JSON string inputs and hex hashes from primary user forms; replace with typed, human-friendly input controls. | `crates/poet/src/browser` |
| `POET-UX-002` | **Design Token Harmonization** | Standardize CSS variables for colors, typography, elevation, spacing, and glassmorphism across all workspaces. | `css.rs`, `theme.rs` |
| `POET-UX-003` | **Visual Feedback & Micro-Animations** | Provide instant visual hover states, button press transforms, loading spinners, and toast notifications on all operations. | `topbar.rs`, `docks.rs` |
| `POET-UX-004` | **Accessible Form Validation** | Real-time inline field validation with human-readable error messages and clear required field indicators. | `project_views`, `studio_views` |
| `POET-UX-005` | **Infinite Canvas Navigation** | Smooth 60 FPS panning, zooming, minimap overview, and grid snapping on the Chora spatial canvas. | `chora_canvas.rs`, `interactions.rs` |
| `POET-UX-006` | **Global Command Palette** | `Ctrl+K` searchable palette for rapid navigation, action execution, and semantic term lookup. | `command_palette.rs` |
| `POET-UX-007` | **Keyboard Shortcut System** | Full keyboard shortcut engine supporting navigation, tool switching, auto-arrange, and modal dismissals. | `interactions.rs` |
| `POET-UX-008` | **WCAG 2.1 AA Compliance** | High-contrast text ratios (min 4.5:1), visible focus rings, ARIA roles, and keyboard focus trapping. | Whole UI |
| `POET-UX-009` | **Responsive Docking & Layouts** | 4-way docking (Left, Top, Bottom, Right) with persistent layout storage and collapsible sidebars. | `docks.rs`, `tool_widgets.rs` |
| `POET-UX-010` | **Optimistic State Updates** | Immediately reflect user edits in UI state with background synchronization and rollbacks on network failure. | `native_daemon.rs` |
