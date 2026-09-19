# monet → Neo — Projects volume (collaborative Workstream A)

**Tip parent:** after push on `0.0.40-webizen-ui`

## Path
`crates/webizen-desktop/static/os-shell/volumes/projects.html`
(+ updated `volumes.css`, `index.html` constellation)

## Rust
```rust
pub const PROJECTS_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/projects.html");
```
Route: `/volumes/projects`

## Why
Collaborative ERP/PM (Workstream A) lived only in `crates/poet/` — not findable on elevated Desktop. This volume surfaces Phase-1 containers (kanban, sheet, budget, …) as Live chrome; deeper engine bind Planned.
