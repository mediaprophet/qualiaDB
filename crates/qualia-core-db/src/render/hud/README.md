# Qualia Portal HUD

`QualiaHud` is a generic, browser canvas control layer intended to sit above a Portal
WebGPU viewport. The caller supplies a transparent `HtmlCanvasElement` of the same
pixel dimensions as the scene. Qualia draws the menu and performs pointer hit testing
and keyboard focus traversal. The application interprets the returned action IDs.

Call `set_document_json` when presentation data changes, then call `hit_test(x, y)`
for clicks in canvas pixels. Use `focus_next(reverse)` and `focused_action()` for
keyboard navigation. The HUD is independent of any game rules, persistence model,
or document format. Parsing is bounded to 96 KB and 256 items; painting uses the
parsed structure without allocating Rust heap memory.

The document is an object with `items`. Each item has `kind` (`panel`, `label`,
`button`, or `meter`), `x`, `y`, `w`, and `h` in canvas pixels. Buttons use `id`
as the action returned from a hit or focused activation. Labels and buttons use
`text`. Meters use `value` in the range 0–1. `disabled` prevents button hits.

```json
{"items":[
  {"kind":"panel","x":16,"y":16,"w":240,"h":96},
  {"kind":"label","text":"Workshop","x":28,"y":24,"w":200,"h":24},
  {"kind":"button","id":"open","text":"Open","x":28,"y":56,"w":200,"h":36}
]}
```

Future generic additions: editable text controls, screen-reader semantics, pointer
hover and press states, scalable typography, layout constraints, and GPU-composited
HUD rendering. These belong in Qualia rather than individual application forks.
