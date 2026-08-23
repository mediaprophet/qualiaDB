//! CSS design tokens and styles for the browser UI shell.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! Adapted from Canvas_Workbench/styles/ theme tokens —
//! cyber-semantic glassmorphism with strata, social, health & epistemic modalities.

pub const CSS: &str = r#"
:root {
  /* Canvas Backgrounds & Depths */
  --canvas-bg: #07090e;
  --canvas-grid-line: rgba(0, 210, 255, 0.08);
  --canvas-grid-line-major: rgba(0, 210, 255, 0.2);

  /* Surfaces & Glass */
  --surface-base: #0c1017;
  --surface-panel: #131822;
  --surface-panel-elevated: #1a2230;
  --surface-glass: rgba(19, 24, 34, 0.82);
  --surface-glass-heavy: rgba(12, 16, 23, 0.92);
  --surface-glass-light: rgba(0, 210, 255, 0.04);

  /* Borders & Glows */
  --border-subtle: #1e2838;
  --border-medium: #2b384e;
  --border-bright: #3e5170;
  --border-active: #00d2ff;

  /* Typography */
  --text-primary: #f0f4fc;
  --text-secondary: #9cb1d1;
  --text-muted: #5e7394;
  --text-inverse: #07090e;

  /* Brand Accents */
  --accent-cyan: #00d2ff;
  --accent-cyan-glow: rgba(0, 210, 255, 0.35);
  --accent-emerald: #00f2a9;
  --accent-emerald-glow: rgba(0, 242, 169, 0.35);
  --accent-amber: #ffb834;
  --accent-amber-glow: rgba(255, 184, 52, 0.35);
  --accent-violet: #a855f7;
  --accent-violet-glow: rgba(168, 85, 247, 0.35);
  --accent-rose: #ff4d6d;
  --accent-rose-glow: rgba(255, 77, 109, 0.35);

  /* Epistemic Modality Colors */
  --modality-objective: #00d2ff;
  --modality-subjective: #ec4899;
  --modality-intersubjective: #a855f7;
  --modality-normative: #ffb834;

  /* Container Type Semantic Colors */
  --color-doc: #38bdf8;
  --color-code: #00f2a9;
  --color-3d: #a855f7;
  --color-sheet: #ffb834;
  --color-portal: #ec4899;
  --color-media: #f97316;
  --color-ai: #06b6d4;
  --color-ontology: #8b5cf6;
  --color-map: #10b981;
  --color-social: #6366f1;
  --color-webrtc: #ef4444;
  --color-health: #14b8a6;
  --color-webview: #0ea5e9;
  --color-rights: #f43f5e;

  /* Strata Colors */
  --strata-env: #10b981;
  --strata-social: #38bdf8;
  --strata-legal: #ec4899;
  --strata-fin: #f59e0b;
  --strata-tech: #8b5cf6;

  /* Fonts */
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', Consolas, monospace;

  /* Shadows */
  --shadow-sm: 0 2px 6px rgba(0, 0, 0, 0.4);
  --shadow-md: 0 6px 20px rgba(0, 0, 0, 0.6);
  --shadow-lg: 0 12px 40px rgba(0, 0, 0, 0.8);
  --shadow-container: 0 10px 30px rgba(0, 0, 0, 0.7), 0 0 1px 1px var(--border-subtle);
  --shadow-container-active: 0 12px 35px rgba(0, 0, 0, 0.8), 0 0 0 1.5px var(--accent-cyan), 0 0 20px var(--accent-cyan-glow);

  /* Radius */
  --radius-xs: 4px;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-full: 9999px;

  /* Transitions */
  --trans-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
  --trans-smooth: 250ms cubic-bezier(0.4, 0, 0.2, 1);
}

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { width: 100%; height: 100%; overflow: hidden; background: var(--canvas-bg); color: var(--text-primary); font-family: var(--font-sans); font-size: 13px; user-select: none; -webkit-font-smoothing: antialiased; }

/* Scrollbars */
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border-medium); border-radius: var(--radius-full); }
::-webkit-scrollbar-thumb:hover { background: var(--border-bright); }

/* App Root */
.app { display: flex; flex-direction: column; width: 100vw; height: 100vh; overflow: hidden; background: var(--canvas-bg); position: relative; }

/* === Top Menu Bar === */
.top-menubar {
  height: 32px; background: var(--surface-base); border-bottom: 1px solid var(--border-subtle);
  display: flex; align-items: center; justify-content: space-between; padding: 0 10px; font-size: 12px; z-index: 500;
}
.menu-items-group { display: flex; align-items: center; gap: 2px; }
.brand-icon { font-size: 14px; margin-right: 6px; display: flex; align-items: center; gap: 4px; }
.brand-text { font-weight: 700; color: var(--accent-emerald); font-size: 12px; }
.menu-btn {
  background: transparent; border: none; color: var(--text-secondary);
  padding: 4px 8px; border-radius: var(--radius-xs); cursor: pointer;
  font-size: 12px; transition: var(--trans-fast); position: relative;
}
.menu-btn:hover, .menu-btn.active { background: var(--surface-panel-elevated); color: var(--text-primary); }

/* === Menu Dropdowns === */
.menu-dropdown { position: relative; }
.menu-dropdown-content {
  display: none; position: absolute; top: 100%; left: 0; min-width: 220px;
  background: var(--surface-glass-heavy); backdrop-filter: blur(20px);
  border: 1px solid var(--border-medium); border-radius: var(--radius-sm);
  box-shadow: var(--shadow-lg); z-index: 1000; padding: 4px;
  flex-direction: column; gap: 1px;
}
.menu-dropdown.open .menu-dropdown-content { display: flex; }
.menu-dropdown-item {
  display: flex; align-items: center; gap: 8px; padding: 6px 10px;
  background: transparent; border: none; border-radius: var(--radius-xs);
  color: var(--text-secondary); font-size: 12px; cursor: pointer;
  transition: var(--trans-fast); text-align: left; width: 100%;
}
.menu-dropdown-item:hover {
  background: var(--surface-panel-elevated); color: var(--text-primary);
}
.menu-dropdown-item-icon { font-size: 12px; width: 18px; text-align: center; flex-shrink: 0; }
.menu-dropdown-item-label { flex: 1; }
.menu-dropdown-separator {
  height: 1px; background: var(--border-subtle); margin: 2px 4px;
}
.fiduciary-badge {
  font-size: 9px; font-weight: 700; padding: 2px 8px; border-radius: var(--radius-full);
  background: rgba(0, 242, 169, 0.15); color: var(--accent-emerald); border: 1px solid var(--accent-emerald);
}
.version-badge {
  font-size: 9px; font-weight: 600; padding: 2px 6px; border-radius: var(--radius-xs);
  background: var(--surface-panel); color: var(--text-muted); border: 1px solid var(--border-subtle);
}

/* === Canvas Control Bar (Manifold Pager) === */
.canvas-control-bar {
  height: 42px; background: var(--surface-glass-heavy); backdrop-filter: blur(16px);
  border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center;
  justify-content: flex-start; padding: 0 12px; gap: 10px; position: relative; z-index: 400;
}
.canvas-title-box { display: flex; align-items: center; gap: 6px; }
.canvas-title-input {
  background: transparent; border: 1px solid transparent; border-radius: var(--radius-xs);
  color: var(--text-primary); font-size: 13px; font-weight: 700; padding: 3px 6px; outline: none; transition: var(--trans-fast);
}
.canvas-title-input:hover, .canvas-title-input:focus { border-color: var(--border-medium); background: var(--surface-panel); }
.graph-address-badge {
  font-family: var(--font-mono); font-size: 10px; padding: 2px 8px; border-radius: var(--radius-full);
  background: var(--surface-panel-elevated); border: 1px solid var(--border-subtle); color: var(--accent-cyan);
}

/* Virtual Desktop Pager */
.virtual-desktop-pager {
  display: flex; align-items: center; gap: 4px; background: var(--surface-base);
  border: 1px solid var(--border-medium); border-radius: var(--radius-sm); padding: 2px 4px;
}
.pager-desktops-list { display: flex; align-items: center; gap: 2px; }
.desktop-tab-btn {
  background: transparent; border: 1px solid transparent; color: var(--text-muted);
  font-size: 10px; font-weight: 600; padding: 3px 8px; border-radius: var(--radius-xs);
  cursor: pointer; display: flex; align-items: center; gap: 4px; transition: var(--trans-fast);
}
.desktop-tab-btn:hover { color: var(--text-primary); background: var(--surface-panel); }
.desktop-tab-btn.active {
  background: var(--surface-panel-elevated); border-color: var(--accent-cyan);
  color: #fff; box-shadow: 0 0 8px var(--accent-cyan-glow);
}
.desktop-num {
  font-family: var(--font-mono); font-size: 9px; background: var(--canvas-bg);
  padding: 1px 4px; border-radius: 2px; color: var(--accent-cyan);
}
.desktop-add-btn {
  font-size: 14px; font-weight: 700; padding: 3px 10px;
  color: var(--accent-emerald); border: 1px dashed var(--border-subtle);
}
.desktop-add-btn:hover {
  border-color: var(--accent-emerald); color: var(--accent-emerald);
  background: rgba(0,255,170,0.05);
}

/* === Socket-Case Pods (Top Control Bar) === */
.top-control-pods-bar {
  display: flex; align-items: center; gap: 6px; margin-left: 12px;
}
.top-pod-btn {
  display: flex; align-items: center; gap: 4px; padding: 4px 10px;
  background: var(--surface-panel); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm); cursor: pointer; transition: var(--trans-fast);
  font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);
}
.top-pod-btn:hover {
  background: var(--surface-panel-elevated); border-color: var(--border-medium);
  color: var(--text-primary);
}
.pod-icon { font-size: 12px; }
.pod-label { font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
.pod-value { font-size: 11px; font-weight: 600; }
.pod-chevron { font-size: 10px; color: var(--text-muted); margin-left: 2px; }

/* === Pod Drop Tray === */
.top-pod-drop-tray {
  position: absolute; top: 100%; left: 0; right: 0; z-index: 500;
  background: var(--surface-glass-heavy); backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px); border: 1px solid var(--border-medium);
  border-radius: var(--radius-sm); padding: 16px 20px; gap: 16px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.5); flex-wrap: wrap;
}
.tray-title {
  font-size: 10px; font-weight: 700; color: var(--accent-cyan);
  text-transform: uppercase; letter-spacing: 1px; width: 100%; margin-bottom: 4px;
}
.tray-checkbox-item {
  display: flex; align-items: center; gap: 8px; padding: 6px 0;
  font-size: 12px; color: var(--text-secondary); cursor: pointer; width: 100%;
}
.tray-checkbox-item input { accent-color: var(--accent-cyan); }
.tray-radio-item {
  display: flex; align-items: center; gap: 8px; padding: 6px 12px;
  background: transparent; border: 1px solid transparent; border-radius: var(--radius-xs);
  color: var(--text-secondary); font-size: 12px; cursor: pointer; transition: var(--trans-fast);
  width: 100%; text-align: left;
}
.tray-radio-item:hover { background: var(--surface-panel-elevated); color: var(--text-primary); }
.tray-button-group { display: flex; flex-direction: column; gap: 6px; min-width: 160px; }
.tray-group-label {
  font-size: 9px; font-weight: 600; color: var(--text-muted);
  text-transform: uppercase; letter-spacing: 0.5px;
}
.tray-btn-row { display: flex; gap: 4px; flex-wrap: wrap; }
.tray-toggle-btn {
  padding: 4px 10px; background: var(--surface-panel); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs); color: var(--text-muted); font-size: 11px;
  cursor: pointer; transition: var(--trans-fast);
}
.tray-toggle-btn:hover { color: var(--text-primary); border-color: var(--border-medium); }
.tray-toggle-btn.active {
  background: var(--surface-panel-elevated); color: var(--accent-cyan);
  border-color: var(--accent-cyan); box-shadow: 0 1px 4px rgba(0,0,0,0.4);
}

/* === Editable Title Input === */
.canvas-title-input {
  background: transparent; border: 1px solid transparent; border-radius: var(--radius-xs);
  color: var(--text-primary); font-size: 13px; font-weight: 600; padding: 2px 6px;
  font-family: var(--font-mono); outline: none; transition: var(--trans-fast);
}
.canvas-title-input:hover { border-color: var(--border-subtle); }
.canvas-title-input:focus { border-color: var(--accent-cyan); background: var(--surface-panel); }

/* === Main Workspace === */
.main-workspace { flex: 1; display: flex; position: relative; overflow: hidden; }

/* === Toolbox Dock (Sidebar) === */
.toolbox-dock {
  width: 180px; background: var(--surface-glass-heavy); backdrop-filter: blur(20px);
  border-right: 1px solid var(--border-subtle); display: flex; flex-direction: column;
  padding: 6px 4px; gap: 2px; flex-shrink: 0; z-index: 300; overflow-y: auto;
}

/* === Family Sections === */
.dock-family-section {
  display: flex; flex-direction: column; gap: 1px;
}
.dock-family-header {
  display: flex; align-items: center; gap: 6px; padding: 6px 8px;
  background: transparent; border: 1px solid transparent; border-radius: var(--radius-xs);
  color: var(--text-muted); font-size: 10px; font-weight: 700;
  text-transform: uppercase; letter-spacing: 0.5px; cursor: pointer;
  transition: var(--trans-fast); width: 100%; text-align: left;
}
.dock-family-header:hover {
  background: var(--surface-panel-elevated); color: var(--text-primary);
  border-color: var(--border-subtle);
}
.dock-family-icon { font-size: 14px; flex-shrink: 0; }
.dock-family-label { flex: 1; }
.dock-family-chevron { font-size: 10px; transition: transform var(--trans-fast); }
.dock-family-section:has(.dock-family-children:not(.expanded)) .dock-family-chevron {
  transform: rotate(-90deg);
}
.dock-family-children {
  display: none; flex-direction: row; gap: 3px; padding: 4px 6px;
  flex-wrap: wrap; align-items: center;
}
.dock-family-children.expanded { display: flex; }

/* === Toolbox Dock Buttons === */
.toolbox-dock-btn {
  width: 36px; height: 36px; background: transparent; border: 1px solid transparent;
  border-radius: var(--radius-sm); color: var(--text-secondary); font-size: 16px;
  display: flex; align-items: center; justify-content: center; cursor: pointer;
  transition: var(--trans-fast); position: relative; flex-shrink: 0;
}
.toolbox-dock-btn:hover { background: var(--surface-panel-elevated); color: var(--text-primary); border-color: var(--border-subtle); }
.toolbox-dock-btn.active {
  background: var(--surface-panel-elevated); color: var(--accent-cyan);
  border-color: var(--accent-cyan); box-shadow: 0 0 12px var(--accent-cyan-glow);
}
.dock-divider { height: 1px; width: 100%; background: var(--border-subtle); margin: 2px 0; }
.dock-tooltip {
  position: absolute; left: 100%; top: 50%; transform: translateY(-50%); margin-left: 8px;
  background: var(--surface-panel-elevated); border: 1px solid var(--border-medium);
  border-radius: var(--radius-xs); padding: 3px 8px; font-size: 11px; color: var(--text-primary);
  white-space: nowrap; pointer-events: none; opacity: 0; transition: opacity var(--trans-fast); z-index: 1000;
  box-shadow: var(--shadow-md);
}
.toolbox-dock-btn:hover .dock-tooltip { opacity: 1; }

/* === Toolbox Flyout (tool-chains + tools) === */
.toolbox-flyout {
  position: fixed; left: 180px; top: 48px; width: 320px; max-height: calc(100vh - 110px);
  background: var(--surface-glass-heavy); backdrop-filter: blur(24px);
  border: 1px solid var(--border-medium); border-radius: var(--radius-md);
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45); z-index: 400; overflow-y: auto;
  padding: 10px; display: flex; flex-direction: column; gap: 8px;
  animation: flyoutSlideIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}
@keyframes flyoutSlideIn {
  from { opacity: 0; transform: translateX(-8px); }
  to { opacity: 1; transform: translateX(0); }
}
.toolbox-flyout-header {
  display: flex; align-items: flex-start; justify-content: space-between;
  padding: 4px 6px 8px; border-bottom: 1px solid var(--border-subtle); margin-bottom: 2px;
}
.flyout-header-left { display: flex; align-items: center; gap: 8px; }
.flyout-header-icon { font-size: 20px; flex-shrink: 0; }
.flyout-title-wrap { display: flex; flex-direction: column; }
.flyout-title-text { font-size: 13px; font-weight: 700; color: var(--text-primary); }
.flyout-desc-text { font-size: 10px; color: var(--text-muted); line-height: 1.3; }
.flyout-header-right { display: flex; align-items: center; gap: 6px; }
.flyout-ont-badge {
  font-family: var(--font-mono); font-size: 9px; font-weight: 600;
  color: var(--accent-cyan); background: rgba(0, 210, 255, 0.1);
  padding: 1px 5px; border-radius: var(--radius-xs); border: 1px solid rgba(0, 210, 255, 0.25);
}
.flyout-close-btn {
  background: transparent; border: none; color: var(--text-muted);
  font-size: 13px; cursor: pointer; padding: 2px 5px; border-radius: var(--radius-xs);
  transition: var(--trans-fast);
}
.flyout-close-btn:hover { color: var(--text-primary); background: rgba(255, 255, 255, 0.1); }
.toolbox-flyout-body { display: flex; flex-direction: column; gap: 8px; }

.toolchain-group {
  display: flex; flex-direction: column; gap: 6px;
  background: rgba(255, 255, 255, 0.02); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm); padding: 8px;
}
.toolchain-label {
  font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px;
  color: var(--accent-cyan); display: flex; align-items: center; gap: 6px; cursor: grab;
  transition: var(--trans-fast); border-radius: var(--radius-xs); padding: 2px 4px;
}
.toolchain-label:hover { color: var(--text-primary); background: rgba(0, 210, 255, 0.08); }
.toolchain-label:active { cursor: grabbing; }
.toolchain-label.selected {
  color: var(--accent-cyan); background: rgba(0, 210, 255, 0.12);
  border-left: 2px solid var(--accent-cyan);
}
.toolchain-label-icon { font-size: 11px; opacity: 0.7; }
.toolchain-label-text { flex: 1; font-family: var(--font-mono); }
.toolchain-label-hint { font-size: 9px; color: var(--text-muted); text-transform: none; font-weight: 400; }

.toolchain-widgets-container { display: flex; flex-direction: column; gap: 6px; margin-top: 2px; }

/* Control Widgets */
.tool-widget-control { display: flex; flex-direction: column; gap: 3px; }
.tool-widget-label {
  font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.4px;
  color: var(--text-muted); font-family: var(--font-mono);
}
.tool-widget-select {
  width: 100%; font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);
  background: var(--surface-panel-elevated); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs); padding: 4px 6px; outline: none; cursor: pointer;
  transition: var(--trans-fast);
}
.tool-widget-select:hover, .tool-widget-select:focus {
  border-color: var(--accent-cyan); box-shadow: 0 0 0 1px rgba(0, 210, 255, 0.2);
}

.tool-widget-color-header, .tool-widget-slider-header {
  display: flex; align-items: center; justify-content: space-between;
}
.tool-widget-color-input {
  -webkit-appearance: none; border: none; width: 22px; height: 18px;
  border-radius: 3px; cursor: pointer; background: transparent; padding: 0;
}
.tool-widget-swatches { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; margin-top: 2px; }
.tool-color-swatch {
  width: 16px; height: 16px; border-radius: 3px; border: 1px solid rgba(255, 255, 255, 0.2);
  cursor: pointer; transition: transform var(--trans-fast), border-color var(--trans-fast);
}
.tool-color-swatch:hover { transform: scale(1.2); border-color: #fff; }

.tool-widget-val-display {
  font-family: var(--font-mono); font-size: 9px; font-weight: 600; color: var(--accent-cyan);
}
.tool-widget-range-input {
  -webkit-appearance: none; width: 100%; height: 4px; border-radius: 2px;
  background: var(--surface-panel-elevated); outline: none; margin: 4px 0;
}
.tool-widget-range-input::-webkit-slider-thumb {
  -webkit-appearance: none; width: 12px; height: 12px; border-radius: 50%;
  background: var(--accent-cyan); cursor: pointer; box-shadow: 0 0 6px var(--accent-cyan);
}

.tool-toggle-buttons { display: flex; align-items: center; gap: 2px; background: var(--surface-panel-elevated); padding: 2px; border-radius: var(--radius-xs); border: 1px solid var(--border-subtle); }
.tool-toggle-btn {
  flex: 1; background: transparent; border: 1px solid transparent; border-radius: 2px;
  color: var(--text-secondary); font-size: 11px; font-weight: 600; font-family: var(--font-mono);
  padding: 3px 6px; cursor: pointer; text-align: center; transition: var(--trans-fast);
}
.tool-toggle-btn:hover { color: var(--text-primary); background: rgba(255, 255, 255, 0.05); }
.tool-toggle-btn.active {
  color: var(--accent-cyan); background: rgba(0, 210, 255, 0.15); border-color: rgba(0, 210, 255, 0.3);
}

.tool-btn {
  display: flex; align-items: center; gap: 8px; padding: 5px 8px;
  background: var(--surface-panel-elevated); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);
  color: var(--text-secondary); font-size: 11px; cursor: pointer;
  transition: var(--trans-fast); text-align: left; width: 100%;
}
.tool-btn:hover {
  background: rgba(0, 210, 255, 0.08); color: var(--text-primary);
  border-color: var(--accent-cyan);
}
.tool-btn-icon { font-size: 13px; width: 16px; text-align: center; flex-shrink: 0; }
.tool-btn-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; }
.tool-btn-kind {
  font-size: 8px; text-transform: uppercase; color: var(--text-muted); font-family: var(--font-mono);
  padding: 1px 4px; border: 1px solid var(--border-subtle); border-radius: 2px;
}

/* === Contextual Ribbon === */
.contextual-instrument-panel {
  display: flex; align-items: center; gap: 4px; padding: 6px 12px;
  background: var(--surface-glass-heavy); backdrop-filter: blur(20px);
  border-bottom: 1px solid var(--border-medium); z-index: 250;
  overflow-x: auto; white-space: nowrap;
}
.instrument-panel-context-label {
  font-size: 10px; font-weight: 600; color: var(--text-muted);
  text-transform: uppercase; letter-spacing: 0.5px; margin-right: 8px;
  flex-shrink: 0;
}
.instrument-panel-tool-btn {
  display: flex; align-items: center; gap: 4px; padding: 4px 8px;
  background: transparent; border: 1px solid transparent; border-radius: var(--radius-xs);
  color: var(--text-secondary); font-size: 11px; cursor: pointer;
  transition: var(--trans-fast); flex-shrink: 0;
}
.instrument-panel-tool-btn:hover {
  background: var(--surface-panel-elevated); color: var(--text-primary);
  border-color: var(--border-subtle);
}
.instrument-panel-tool-icon {
  font-size: 11px; font-weight: 700; color: var(--accent-cyan);
  min-width: 16px; text-align: center;
}
.instrument-panel-tool-label { font-size: 11px; }
.instrument-panel-close-btn {
  margin-left: auto; background: transparent; border: none; color: var(--text-muted);
  font-size: 14px; cursor: pointer; padding: 4px 8px; border-radius: var(--radius-xs);
  flex-shrink: 0;
}
.instrument-panel-close-btn:hover { color: var(--text-primary); background: var(--surface-panel-elevated); }

/* === Canvas Viewport === */
.canvas-viewport-container { flex: 1; position: relative; overflow: hidden; background: var(--canvas-bg); }
.canvas-grid-svg { position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; }

/* === Canvas Containers (Glassmorphism) === */
.canvas-container-node {
  position: absolute; background: var(--surface-glass); backdrop-filter: blur(18px);
  border: 1px solid var(--border-medium); border-radius: var(--radius-md);
  box-shadow: var(--shadow-container); display: flex; flex-direction: column;
  min-width: 280px; min-height: 180px; z-index: 20; overflow: hidden;
  transition: box-shadow var(--trans-fast), border-color var(--trans-fast);
}
.canvas-container-node.selected {
  border-color: var(--accent-cyan); box-shadow: var(--shadow-container-active); z-index: 30;
}

/* Filter-hidden containers — dimmed and non-interactive */
.canvas-container-node.strata-hidden,
.canvas-container-node.epistemic-hidden {
  opacity: 0.18; pointer-events: none; filter: grayscale(1);
}

/* Container Header */
.container-header {
  height: 38px; background: var(--surface-panel); border-bottom: 1px solid var(--border-subtle);
  border-radius: var(--radius-md) var(--radius-md) 0 0; display: flex; align-items: center;
  justify-content: space-between; padding: 0 12px; cursor: grab; user-select: none;
}
.container-header:active { cursor: grabbing; }
.container-title-group { display: flex; align-items: center; gap: 6px; }
.container-type-tag {
  font-size: 10px; font-weight: 700; text-transform: uppercase; padding: 2px 6px; border-radius: var(--radius-xs);
}
.tag-social { background: rgba(99, 102, 241, 0.18); color: var(--color-social); border: 1px solid var(--color-social); }
.tag-doc { background: rgba(56, 189, 248, 0.15); color: var(--color-doc); border: 1px solid var(--color-doc); }
.tag-code { background: rgba(0, 242, 169, 0.15); color: var(--color-code); border: 1px solid var(--color-code); }
.tag-map { background: rgba(16, 185, 129, 0.18); color: var(--color-map); border: 1px solid var(--color-map); }
.tag-media { background: rgba(249, 115, 22, 0.15); color: var(--color-media); border: 1px solid var(--color-media); }
.tag-ontology { background: rgba(139, 92, 246, 0.18); color: var(--color-ontology); border: 1px solid var(--color-ontology); }
.tag-webrtc { background: rgba(239, 68, 68, 0.18); color: var(--color-webrtc); border: 1px solid var(--color-webrtc); }
.tag-portal { background: rgba(236, 72, 153, 0.15); color: var(--color-portal); border: 1px solid var(--color-portal); }
.tag-default { background: rgba(107, 118, 137, 0.18); color: var(--text-muted); border: 1px solid var(--border-medium); }

.container-title { font-size: 12px; font-weight: 600; color: var(--text-primary); }

/* Honesty Badges */
.honesty-badge {
  font-family: var(--font-mono); font-size: 8px; font-weight: 700; padding: 1px 5px;
  border-radius: var(--radius-xs); text-transform: lowercase; letter-spacing: 0.3px;
}
.honesty-live { background: rgba(0, 242, 169, 0.18); color: var(--accent-emerald); border: 1px solid var(--accent-emerald); }
.honesty-partial { background: rgba(56, 189, 248, 0.18); color: var(--accent-cyan); border: 1px solid var(--accent-cyan); }
.honesty-present { background: rgba(255, 184, 52, 0.18); color: var(--accent-amber); border: 1px solid var(--accent-amber); }
.honesty-missing { background: rgba(239, 68, 68, 0.18); color: var(--accent-rose); border: 1px solid var(--accent-rose); }

/* Strata Badges */
.strata-badge {
  font-size: 9px; font-weight: 700; padding: 1px 6px; border-radius: var(--radius-xs);
  text-transform: uppercase; letter-spacing: 0.3px;
}
.strata-social { background: rgba(56, 189, 248, 0.18); color: var(--strata-social); border: 1px solid var(--strata-social); }
.strata-legal { background: rgba(236, 72, 153, 0.18); color: var(--strata-legal); border: 1px solid var(--strata-legal); }

/* Epistemic Modality Badges */
.modality-badge {
  font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: var(--radius-xs);
  display: inline-flex; align-items: center; gap: 3px;
}
.modality-objective { background: rgba(0, 210, 255, 0.15); color: var(--modality-objective); border: 1px solid var(--modality-objective); }
.modality-subjective { background: rgba(236, 72, 153, 0.18); color: var(--modality-subjective); border: 1px solid var(--modality-subjective); }
.modality-intersubjective { background: rgba(168, 85, 247, 0.18); color: var(--modality-intersubjective); border: 1px solid var(--modality-intersubjective); }
.modality-normative { background: rgba(255, 184, 52, 0.18); color: var(--modality-normative); border: 1px solid var(--modality-normative); }

/* Container Actions */
.container-actions { display: flex; align-items: center; gap: 6px; }
.container-action-btn {
  background: transparent; border: none; color: var(--text-muted); cursor: pointer;
  padding: 2px; border-radius: var(--radius-xs); display: flex; align-items: center; justify-content: center;
}
.container-action-btn:hover { color: var(--text-primary); background: var(--surface-glass-light); }
.container-action-btn.delete-btn:hover { color: var(--accent-rose); background: rgba(239, 68, 68, 0.1); }

/* Container Body */
.container-body { flex: 1; padding: 12px; overflow: auto; position: relative; user-select: text; display: flex; flex-direction: column; }
.container-placeholder { color: var(--text-muted); font-size: 11px; text-align: center; padding: 20px; }

/* Connection Ports */
.container-port {
  position: absolute; top: 50%; width: 12px; height: 12px; border-radius: 50%;
  background: var(--surface-panel-elevated); border: 2px solid var(--accent-cyan);
  transform: translateY(-50%); cursor: crosshair; z-index: 50;
  transition: transform var(--trans-fast), background var(--trans-fast);
}
.container-port:hover { transform: translateY(-50%) scale(1.4); background: var(--accent-cyan); box-shadow: 0 0 10px var(--accent-cyan-glow); }
.port-in { left: -6px; }
.port-out { right: -6px; }

/* Resize Handle */
.container-resizer {
  position: absolute; right: 0; bottom: 0; width: 14px; height: 14px; cursor: se-resize;
  border-bottom-right-radius: var(--radius-md); background: linear-gradient(135deg, transparent 50%, var(--border-bright) 50%);
}

/* === Social Chat Graph === */
.chat-message {
  display: flex; gap: 8px; padding: 8px 10px; border-radius: var(--radius-sm);
  margin-bottom: 6px; background: var(--surface-panel); border: 1px solid var(--border-subtle);
}
.chat-message.ai-msg { border-left: 3px solid var(--color-ai); }
.chat-message.human-msg { border-left: 3px solid var(--accent-emerald); }
.chat-avatar { width: 28px; height: 28px; border-radius: var(--radius-full); display: flex; align-items: center; justify-content: center; font-size: 14px; flex-shrink: 0; }
.chat-avatar.ai-avatar { background: rgba(6, 182, 212, 0.2); }
.chat-avatar.human-avatar { background: rgba(0, 242, 169, 0.2); }
.chat-content { flex: 1; display: flex; flex-direction: column; gap: 3px; }
.chat-sender { font-size: 11px; font-weight: 600; color: var(--text-primary); display: flex; align-items: center; gap: 6px; }
.chat-text { font-size: 11px; color: var(--text-secondary); line-height: 1.5; }
.chat-time { font-family: var(--font-mono); font-size: 9px; color: var(--text-muted); }
.chat-meta-row { display: flex; align-items: center; gap: 4px; }

/* === Connection Request Card === */
.cr-card {
  background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-sm);
  padding: 10px; margin-bottom: 8px; transition: border-color var(--trans-fast);
}
.cr-card:hover { border-color: var(--border-medium); }
.cr-card .cr-header { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
.cr-card .cr-sender { font-size: 11px; font-weight: 600; color: var(--text-primary); }
.cr-card .cr-did { font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); }
.cr-card .cr-status {
  font-size: 9px; padding: 1px 6px; border-radius: var(--radius-xs); text-transform: uppercase; font-weight: 700;
}
.cr-status-pending { background: rgba(255, 184, 52, 0.15); color: var(--accent-amber); border: 1px solid var(--accent-amber); }
.cr-status-verifying { background: rgba(0, 210, 255, 0.15); color: var(--accent-cyan); border: 1px solid var(--accent-cyan); }
.cr-status-accepted { background: rgba(0, 242, 169, 0.15); color: var(--accent-emerald); border: 1px solid var(--accent-emerald); }
.cr-status-blocked { background: rgba(239, 68, 68, 0.15); color: var(--accent-rose); border: 1px solid var(--accent-rose); }
.cr-card .cr-meta { font-size: 10px; color: var(--text-muted); margin-bottom: 4px; }
.cr-card .cr-actions { display: flex; gap: 6px; margin-top: 8px; }
.cr-btn {
  font-size: 10px; padding: 4px 12px; border: 1px solid var(--border-medium);
  background: var(--surface-panel-elevated); color: var(--text-primary); border-radius: var(--radius-xs); cursor: pointer; transition: var(--trans-fast);
}
.cr-btn:hover { border-color: var(--accent-emerald); color: var(--accent-emerald); }
.cr-btn.danger:hover { border-color: var(--accent-rose); color: var(--accent-rose); }

/* Risk Indicators */
.risk-indicator {
  display: inline-block; font-size: 9px; padding: 1px 5px; border-radius: var(--radius-xs);
  margin-right: 4px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); font-weight: 600;
}
.risk-moderate { border-color: var(--accent-amber); color: var(--accent-amber); }
.risk-high { border-color: var(--accent-rose); color: var(--accent-rose); }
.risk-low { border-color: var(--accent-emerald); color: var(--accent-emerald); }
.risk-critical { border-color: var(--accent-rose); color: var(--accent-rose); background: rgba(239, 68, 68, 0.1); }

/* === Protection Policy Card === */
.pp-card {
  background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-sm);
  padding: 10px; margin-bottom: 8px; transition: border-color var(--trans-fast);
}
.pp-card:hover { border-color: var(--border-medium); }
.pp-card .pp-category { font-size: 11px; font-weight: 700; color: var(--accent-emerald); margin-bottom: 6px; display: flex; align-items: center; gap: 6px; }
.pp-card .pp-row { font-size: 10px; color: var(--text-muted); margin: 3px 0; display: flex; gap: 4px; }
.pp-card .pp-row .pp-key { color: var(--text-secondary); font-weight: 600; min-width: 90px; }
.pp-card .pp-row .pp-val { color: var(--text-primary); }
.pp-card .pp-mandatory {
  font-size: 9px; padding: 1px 5px; border-radius: var(--radius-xs);
  background: rgba(239, 68, 68, 0.1); color: var(--accent-rose); border: 1px solid var(--accent-rose);
}
.pp-card .pp-optin {
  font-size: 9px; padding: 1px 5px; border-radius: var(--radius-xs);
  background: rgba(255, 184, 52, 0.1); color: var(--accent-amber); border: 1px solid var(--accent-amber);
}

/* === Right Dock (Aura Tray + Pulse + Job Center) === */
.right-dock {
  width: 280px; background: var(--surface-glass-heavy); backdrop-filter: blur(20px);
  border-left: 1px solid var(--border-subtle); display: flex; flex-direction: column; flex-shrink: 0; z-index: 300;
}
.right-dock-content {
  display: flex; flex-direction: column; height: 100%; overflow: hidden;
}
.dock-panel { border-bottom: 1px solid var(--border-subtle); display: flex; flex-direction: column; transition: flex var(--trans-fast); }
.dock-panel.collapsed { flex: 0 0 auto !important; }
.dock-panel.collapsed .dock-panel-body { display: none !important; }

.dock-panel-header {
  height: 32px; padding: 0 12px; display: flex; align-items: center; justify-content: space-between;
  font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-muted);
  font-weight: 700; background: var(--surface-panel); cursor: pointer; user-select: none;
  transition: background var(--trans-fast), color var(--trans-fast);
}
.dock-panel-header:hover {
  background: var(--surface-panel-elevated); color: var(--text-primary);
}
.dock-panel-chevron {
  display: inline-flex; align-items: center; justify-content: center; width: 12px; font-size: 9px;
  color: var(--text-muted); transition: transform var(--trans-fast);
}
.dock-panel-badge {
  font-size: 9px; font-weight: 600; padding: 1px 6px; border-radius: var(--radius-xs);
  background: var(--surface-panel-elevated); color: var(--text-secondary); border: 1px solid var(--border-subtle); text-transform: none;
}
.dock-panel-body { padding: 10px 12px; font-size: 11px; color: var(--text-secondary); }
.dock-panel-body .shacl-valid {
  display: inline-flex; align-items: center; gap: 4px; padding: 2px 8px; border-radius: var(--radius-xs);
  background: rgba(0, 242, 169, 0.15); color: var(--accent-emerald); border: 1px solid var(--accent-emerald); font-size: 10px; font-weight: 600;
}
.dock-panel-body .shacl-warn {
  display: inline-flex; align-items: center; gap: 4px; padding: 2px 8px; border-radius: var(--radius-xs);
  background: rgba(255, 184, 52, 0.15); color: var(--accent-amber); border: 1px solid var(--accent-amber); font-size: 10px; font-weight: 600;
}

/* Sub-trays inside Aura Tray */
.dock-subtray {
  margin-bottom: 8px; border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);
  background: rgba(255, 255, 255, 0.015); overflow: hidden;
}
.dock-subtray:last-child { margin-bottom: 0; }
.dock-subtray-header {
  padding: 4px 8px; font-size: 9.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--text-muted); background: var(--surface-panel); display: flex; align-items: center; justify-content: space-between;
  cursor: pointer; user-select: none; transition: background var(--trans-fast), color var(--trans-fast);
}
.dock-subtray-header:hover {
  background: var(--surface-panel-elevated); color: var(--text-primary);
}
.dock-subtray-chevron {
  font-size: 8px; width: 10px; display: inline-flex; align-items: center; justify-content: center; color: var(--text-muted);
}
.dock-subtray-body {
  padding: 6px 8px; font-size: 10px; font-family: var(--font-mono);
}
.dock-subtray.collapsed .dock-subtray-body {
  display: none !important;
}

/* Pulse Stream */
.pulse-entry {
  display: flex; align-items: flex-start; gap: 6px; padding: 4px 0; border-bottom: 1px solid var(--border-subtle);
  font-family: var(--font-mono); font-size: 10px; color: var(--text-muted);
}
.pulse-entry:last-child { border-bottom: none; }
.pulse-dot { width: 6px; height: 6px; border-radius: 50%; margin-top: 4px; flex-shrink: 0; }
.pulse-dot.notification { background: var(--accent-cyan); }
.pulse-dot.telemetry { background: var(--accent-amber); }
.pulse-dot.alert { background: var(--accent-rose); }
.pulse-dot.agent { background: var(--color-ai); }
.pulse-text { flex: 1; line-height: 1.4; }
.pulse-time { color: var(--text-muted); font-size: 9px; }

/* === Bottom Status Bar === */
.bottom-statusbar {
  height: 26px; background: var(--surface-base); border-top: 1px solid var(--border-subtle);
  display: flex; align-items: center; justify-content: space-between; padding: 0 14px;
  font-size: 11px; font-family: var(--font-mono); color: var(--text-muted); z-index: 500;
}
.statusbar-section { display: flex; align-items: center; gap: 12px; }
.statusbar-item { display: flex; align-items: center; gap: 4px; }
.statusbar-label { color: var(--text-muted); }
.statusbar-value { color: var(--text-secondary); }
.statusbar-gas { color: var(--accent-emerald); }
.statusbar-gas-medium { color: var(--accent-amber); }

/* === Canvas Grid Background === */
.canvas-grid-svg {
  background-image:
    linear-gradient(var(--canvas-grid-line) 1px, transparent 1px),
    linear-gradient(90deg, var(--canvas-grid-line) 1px, transparent 1px);
  background-size: 40px 40px;
}
.canvas-grid-svg::before {
  content: ''; position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  background-image:
    linear-gradient(var(--canvas-grid-line-major) 1px, transparent 1px),
    linear-gradient(90deg, var(--canvas-grid-line-major) 1px, transparent 1px);
  background-size: 200px 200px;
  pointer-events: none;
}

/* === Canvas Stage (pan/zoom transform layer) === */
.canvas-stage {
  position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  transform-origin: 0 0; transition: transform 0.05s linear;
}

/* === Connection Wires (SVG) === */
.wire-overlay {
  position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  pointer-events: none; z-index: 10;
}
.wire-overlay path { pointer-events: stroke; fill: none; stroke-width: 2; cursor: pointer; }
.wire-active { stroke: var(--accent-cyan); stroke-dasharray: 6 4; animation: wireFlow 1.5s linear infinite; }
.wire-event { stroke: var(--accent-amber); stroke-dasharray: 4 4; }
.wire-ontology { stroke: var(--color-ontology); stroke-dasharray: 8 4; }
.wire-subjective { stroke: var(--modality-subjective); stroke-dasharray: 3 5; }
.wire-objective { stroke: var(--modality-objective); stroke-dasharray: 6 3; }
.wire-selected { stroke-width: 3; filter: drop-shadow(0 0 6px var(--accent-cyan-glow)); }
.wire-pulsing {
  stroke-width: 3.5px !important;
  filter: drop-shadow(0 0 10px var(--accent-cyan-glow)) drop-shadow(0 0 4px var(--accent-emerald)) !important;
  animation: wireFlow 0.5s linear infinite, wirePulseGlow 1s ease-in-out infinite alternate !important;
}
@keyframes wirePulseGlow {
  from { stroke-width: 2.5px; opacity: 0.85; }
  to { stroke-width: 4px; opacity: 1.0; }
}
.wire-drag-overlay { animation: wireFlow 0.8s linear infinite; }
@keyframes wireFlow { to { stroke-dashoffset: -20; } }
.wire-label-text {
  font-family: var(--font-mono); font-size: 10px; fill: var(--accent-cyan);
  text-anchor: middle; pointer-events: all; cursor: pointer;
}

/* === Container Dragging State === */
.canvas-container-node.dragging {
  opacity: 0.85; cursor: grabbing; box-shadow: var(--shadow-lg), 0 0 20px var(--accent-cyan-glow);
  z-index: 100;
  transition: none !important;
}

/* === Manifold Auto-Arranging & Smart Placement Animations === */
.canvas-container-node.manifold-rearranging {
  transition: left 0.45s cubic-bezier(0.16, 1, 0.3, 1),
              top 0.45s cubic-bezier(0.16, 1, 0.3, 1),
              transform 0.45s cubic-bezier(0.16, 1, 0.3, 1),
              box-shadow 0.3s ease !important;
}

.canvas-container-node.newly-placed {
  animation: container-pop-in 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

@keyframes container-pop-in {
  0% {
    opacity: 0;
    transform: scale(0.92) translateY(-14px);
    box-shadow: 0 0 35px var(--accent-cyan-glow);
  }
  65% {
    opacity: 1;
    transform: scale(1.02) translateY(0);
  }
  100% {
    opacity: 1;
    transform: scale(1.0) translateY(0);
  }
}

/* === VibeScript Console === */
.vibe-console {
  display: flex; flex-direction: column; gap: 8px; font-family: var(--font-mono); font-size: 11px;
}
.vibe-editor {
  background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);
  padding: 10px; color: var(--accent-emerald); min-height: 120px; white-space: pre-wrap; line-height: 1.6;
}
.vibe-editor .vibe-keyword { color: var(--accent-cyan); font-weight: 700; }
.vibe-editor .vibe-string { color: var(--accent-amber); }
.vibe-editor .vibe-comment { color: var(--text-muted); font-style: italic; }
.vibe-output {
  background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);
  padding: 10px; color: var(--text-secondary); min-height: 60px; max-height: 120px; overflow-y: auto;
}
.vibe-output .vibe-out-line { padding: 1px 0; }
.vibe-toolbar {
  display: flex; gap: 4px; padding-bottom: 4px; border-bottom: 1px solid var(--border-subtle);
}
.vibe-run-btn {
  background: rgba(0, 242, 169, 0.15); border: 1px solid var(--accent-emerald); color: var(--accent-emerald);
  font-size: 10px; font-weight: 700; padding: 3px 10px; border-radius: var(--radius-xs); cursor: pointer;
  transition: var(--trans-fast);
}
.vibe-run-btn:hover { background: rgba(0, 242, 169, 0.25); box-shadow: 0 0 8px var(--accent-emerald-glow); }

/* === GIS Map Container === */
.gis-map-svg {
  width: 100%; height: 100%; background: radial-gradient(ellipse at 40% 35%, #0a1520 0%, #050810 100%);
  border-radius: var(--radius-xs);
}
.gis-layer-bar {
  display: flex; gap: 3px; background: rgba(12, 16, 23, 0.85); border-radius: var(--radius-xs);
  padding: 2px 4px; border: 1px solid var(--border-subtle); margin-bottom: 6px;
}
.gis-layer-btn {
  background: transparent; border: 1px solid transparent; color: var(--text-muted); font-size: 9px;
  padding: 2px 6px; border-radius: 2px; cursor: pointer; transition: var(--trans-fast);
}
.gis-layer-btn:hover, .gis-layer-btn.active { color: var(--accent-emerald); border-color: var(--accent-emerald); }
.agent-pin-marker { cursor: pointer; }
.agent-pin-marker:hover circle { r: 8; }

/* === 3D Media Viewport === */
.media-3d-viewport {
  flex: 1; display: flex; align-items: center; justify-content: center;
  background: radial-gradient(ellipse at 50% 50%, #0a1420 0%, #050810 100%);
  border-radius: var(--radius-xs); position: relative; min-height: 200px;
}
.media-3d-placeholder { color: var(--text-muted); font-size: 12px; text-align: center; }
.media-3d-cube {
  width: 60px; height: 60px; border: 2px solid var(--accent-violet); border-radius: 4px;
  margin: 10px auto; animation: spin3d 4s linear infinite;
}
@keyframes spin3d { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

/* === Ontology Tree View === */
.ontology-tree { font-family: var(--font-mono); font-size: 11px; }
.ontology-tree-node { padding: 2px 0; display: flex; align-items: center; gap: 4px; }
.ontology-tree-node .ot-prefix { color: var(--accent-violet); font-weight: 700; }
.ontology-tree-node .ot-class { color: var(--accent-cyan); }
.ontology-tree-node .ot-prop { color: var(--accent-amber); }
.ontology-tree-children { padding-left: 16px; border-left: 1px solid var(--border-subtle); margin-left: 6px; }

/* Rights / Wallet cards */
.cr-card { background: var(--glass-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 10px 12px; }
.cr-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
.cr-name { font-size: 12px; font-weight: 600; color: var(--text-primary); }
.cr-meta { font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); }

/* Resize handle cursor */
.resize-handle { cursor: nwse-resize; }

/* Command palette */
.cmd-palette-item.selected { background: rgba(0,255,170,0.08); border-left: 2px solid var(--accent-emerald); }
.cmd-palette-item:hover { background: rgba(0,255,170,0.05); }

/* === Exposé Overview === */
.expose-card:hover {
  border-color: var(--accent-cyan) !important;
  box-shadow: 0 0 20px var(--accent-cyan-glow);
  transform: translateY(-2px);
}

/* === Tool Notification === */
.tool-notification {
  animation: slideInRight 0.2s ease-out;
}
@keyframes slideInRight {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

/* === Wire Inspector === */
.wire-inspector-btn {
  flex: 1; padding: 4px 8px; background: var(--surface-panel);
  border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);
  color: var(--text-secondary); font-size: 10px; cursor: pointer; transition: var(--trans-fast);
}
.wire-inspector-btn:hover {
  border-color: var(--accent-cyan); color: var(--accent-cyan);
  background: var(--surface-panel-elevated);
}

/* === Top Actions Shelf === */
.top-actions-shelf {
  display: flex; gap: 6px; margin-left: auto;
}
.top-action-btn {
  display: flex; align-items: center; gap: 4px; padding: 4px 8px;
  background: var(--surface-panel); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm); color: var(--text-secondary); font-size: 11px;
  cursor: pointer; transition: var(--trans-fast);
}
.top-action-btn:hover {
  background: var(--surface-panel-elevated); border-color: var(--border-medium);
  color: var(--text-primary);
}

/* === Search Workbench === */
.search-workbench-btn {
  background: rgba(0,255,170,0.08) !important;
  border: 1px solid var(--accent-cyan) !important;
  color: var(--accent-cyan) !important;
  font-weight: 600;
}
.search-workbench-btn:hover {
  background: rgba(0,255,170,0.18) !important;
}
.logic-workbench-btn {
  background: rgba(139,92,246,0.08) !important;
  border: 1px solid var(--accent-violet) !important;
  color: var(--accent-violet) !important;
  font-weight: 600;
}
.logic-workbench-btn:hover {
  background: rgba(139,92,246,0.18) !important;
}
.logic-tool-tab:hover {
  background: var(--surface-panel-elevated);
}
.logic-tool-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.search-mode-tab:hover {
  background: var(--surface-panel-elevated);
}
.facet-chip:hover {
  border-color: var(--accent-cyan) !important;
  color: var(--accent-cyan) !important;
}
.search-mode-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* === CML HyperDoc & Context Markup === */
.q-doc-container {
  font-family: var(--font-sans);
  outline: none;
}
.q-doc-container:focus {
  border-color: var(--accent-cyan) !important;
  box-shadow: 0 0 12px var(--accent-cyan-glow);
}
.cml-entity-tag {
  border-bottom: 2px solid var(--accent-primary, #6366f1);
  background: rgba(255, 255, 255, 0.05);
  border-radius: 3px;
  padding: 1px 4px;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  cursor: pointer;
  transition: var(--trans-fast);
}
.cml-entity-tag:hover {
  background: rgba(255, 255, 255, 0.12);
  transform: translateY(-1px);
}
.cml-entity-badge {
  font-size: 9px;
  padding: 1px 3px;
  border-radius: 2px;
  color: #000;
  font-weight: 700;
  line-height: 1;
}
.cml-popover-cat-btn:hover {
  background: var(--surface-panel-elevated) !important;
  border-color: var(--accent-cyan) !important;
  color: var(--text-primary) !important;
}
.q-aura-tray {
  backdrop-filter: blur(12px);
  transition: var(--trans-smooth);
}
.q-aura-tray:hover {
  border-color: var(--border-medium);
}

/* === 4-Way Docking Architecture === */
.toolbox-dock.dock-pos-left {
  width: 180px;
  border-right: 1px solid var(--border-subtle);
  flex-direction: column;
}
.toolbox-dock.dock-pos-right {
  width: 180px;
  order: 3;
  border-left: 1px solid var(--border-subtle);
  border-right: none;
  flex-direction: column;
}
.toolbox-dock.dock-pos-top {
  width: 100%;
  height: 52px;
  border-bottom: 1px solid var(--border-subtle);
  flex-direction: row;
  order: -1;
  overflow-x: auto;
  overflow-y: hidden;
  align-items: center;
}
.toolbox-dock.dock-pos-bottom {
  width: 100%;
  height: 52px;
  border-top: 1px solid var(--border-subtle);
  flex-direction: row;
  order: 5;
  overflow-x: auto;
  overflow-y: hidden;
  align-items: center;
}
.dock-pos-btn:hover {
  border-color: var(--accent-cyan) !important;
  color: var(--accent-cyan) !important;
}

/* === 8-Sector Radial Action Ring === */
#radial-action-ring {
  user-select: none;
  animation: radialPop 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}
@keyframes radialPop {
  from { transform: translate(-50%, -50%) scale(0.6); opacity: 0; }
  to { transform: translate(-50%, -50%) scale(1); opacity: 1; }
}
.radial-sector-group {
  transition: transform 0.15s ease-out;
}
.radial-sector-group:hover {
  filter: drop-shadow(0 0 8px rgba(0, 210, 255, 0.4));
}

/* === Webizen Unicode & PUA Icon System (.wi) === */
@font-face {
  font-family: 'Webizen Icons';
  src: local('Webizen Icons'), local('Segoe UI Emoji'), local('Apple Color Emoji'), local('Noto Color Emoji');
  unicode-range: U+E000-U+E1FF;
}

.wi {
  font-family: 'Webizen Icons', 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', system-ui, sans-serif;
  font-style: normal;
  font-weight: normal;
  font-variant: normal;
  text-transform: none;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  vertical-align: middle;
  user-select: none;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* Sizing Utilities */
.wi-xs { font-size: 10px; width: 12px; height: 12px; }
.wi-sm { font-size: 14px; width: 16px; height: 16px; }
.wi-md { font-size: 18px; width: 20px; height: 20px; }
.wi-lg { font-size: 24px; width: 28px; height: 28px; }
.wi-xl { font-size: 32px; width: 36px; height: 36px; }

/* Dynamic Interactive States */
.wi-hover {
  transition: transform 0.15s ease, filter 0.15s ease;
}
.wi-hover:hover {
  transform: scale(1.12);
  filter: drop-shadow(0 0 6px var(--accent-cyan-glow, rgba(0, 210, 255, 0.4)));
}

.wi-active {
  color: var(--accent-cyan, #00d2ff);
  filter: drop-shadow(0 0 8px var(--accent-cyan-glow, rgba(0, 210, 255, 0.5)));
}

.wi-disabled {
  opacity: 0.35;
  filter: grayscale(1);
  pointer-events: none;
}

.wi-spin {
  animation: wiSpin 1.2s cubic-bezier(0.4, 0, 0.2, 1) infinite;
}
@keyframes wiSpin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.wi-pulse {
  animation: wiPulse 1.8s ease-in-out infinite;
}
@keyframes wiPulse {
  0%, 100% { transform: scale(1); opacity: 0.85; filter: drop-shadow(0 0 2px rgba(168, 85, 247, 0.3)); }
  50% { transform: scale(1.15); opacity: 1; filter: drop-shadow(0 0 10px rgba(168, 85, 247, 0.7)); }
}

.wi-breathe {
  animation: wiBreathe 3s ease-in-out infinite;
}
@keyframes wiBreathe {
  0%, 100% { opacity: 0.7; transform: scale(0.98); }
  50% { opacity: 1; transform: scale(1.04); filter: drop-shadow(0 0 8px rgba(0, 242, 169, 0.5)); }
}

.wi-error {
  color: var(--accent-rose, #ff4d6d);
  animation: wiErrorShake 0.4s ease-in-out;
}
@keyframes wiErrorShake {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-3px); }
  75% { transform: translateX(3px); }
}
"#;
