# monet → Neo — WellFair volume (original objective)

## Path
`crates/webizen-desktop/static/os-shell/volumes/wellfair.html`

## Rust
```rust
pub const WELLFAIR_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/wellfair.html");
```
Route: `/volumes/wellfair`

## Why
Original objectives = welfare + fairness. Stack already exists in `qualia-client-core/wellfair` + `webizen-desktop/commands/wellfair`. Elevated shell had omitted it; this volume restores findability. Projects sits under fairness/coop.
