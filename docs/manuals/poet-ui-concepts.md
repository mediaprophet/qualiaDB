# Poet UI concepts — manifolds, tool chest, containers

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Status:** product vocabulary for Poet chrome. Classic UI stays default (D7).  
**Instructional mock:** `C:\Projects\NLP\Canvas_Workbench\designnotes.md`

This is how the workbench is named. HTML `<canvas>` / wgpu / Canvas2D stay **canvas** (drawing surfaces).

---

## 1. Three kinds of thing (many types of each)

| Kind | What it is | Many of… |
|---|---|---|
| **Manifold** | A work *surface* (virtual desktop). Same word as the engine 10D manifold. | Research, Media, Social, Settings, Vibe, … |
| **Container** | A typed *occupant* placed on a manifold. | doc, sheet, code, map, social, health, … |
| **Tool chest** | The *furniture* that holds toolboxes. One chest per workbench, dockable. | (usually one) |
| **Toolbox** | A *themed drawer* inside the chest. | Vibe, Documents, Sheets, Spatial, Social, Rights, Health, Epistemic |
| **Tool** | One *action* in a toolbox (often “place this container”). | + Document, Run cell, Gazetteer, … |

A **tool chest** is not a toolbox. The chest is the rack; a toolbox is a drawer; a tool is what you pick up.

Legacy mock folder `toolboxes/` is the chest’s contents. Do not rename HTML/GPU canvas modules here (D14).

---

## 2. Manifolds

Switchable surfaces, like a pager of virtual desktops. A container on one manifold can target another (**sub-manifold**).

| Id | Role |
|---|---|
| `research` | GIS, clinical, rights alignment |
| `media` | `.10d` kinematics, grapheme, acoustic |
| `social` | chat graphs, live peers, field notes |
| `settings` | capabilities, fiduciary VM, sub-manifolds |
| `vibe` | VibeScript console + diagnose (human door into Qualia) |

Pager: Overview + numbered surfaces + `+`. Shortcuts later: Alt+1… 

---

## 3. Containers

Placed by tools. Each type may be implemented at different honesty:

| Kind | Honesty today | Notes |
|---|---|---|
| `code` | live | Vibe eval / diagnose / `Render.scene` |
| `doc` | live (gazetteer) | year-one NLP, not FrameNet |
| `sheet` | live (range stats via invoke) | CML cell→N3 later |
| `ontology` | partial | `SHACL.extensions` live; no shape-IRI registry yet |
| `map` | live | hull + `Manifold.project` + `webizen-render` |
| `media` | live | kinematics poses → `Render.scene`; swapchain on `/gpu-viewport` |
| `social` | live | LWW + peer hash; ring is the renderer contract (no fake chat) |
| `health` | live | Framingham on a **reference adult profile**, not a named person |
| `submanifold` | partial | nested `Manifold.axes` + same scene contract |
| `webview` `webrtc` `portal` | present | not placed yet; do not fake streams |

---

## 4. Tool chest

Dock: left / right / top / bottom (left first). Each toolbox icon opens that drawer. Tools **place containers** or **run Vibe** — they do not `alert()`.

Honesty labels on tools that are not wired (Present / Partial / Missing).

---

## 5. Vibe’s place

Vibe is how humans and apps *reach* Qualia from a **code** container (and the Vibe manifold). It is not a manifold type of its own forever — it is a container kind that can sit on any manifold.

NLP is a **doc** tool, not a Vibe keyword.

---

## 6. Implementation home

Chrome is the **HyperCanvas** from `C:\Projects\NLP\Canvas_Workbench` (menubar, pager, strata, epistemic lens, 2D/3D/4D, time ribbon, spatial stage, wires, 4-way tool chest, status). `/poet` is **not** inside Classic studio chrome.

| Concept | Path |
|---|---|
| Instructional mock | `C:\Projects\NLP\Canvas_Workbench/` |
| CSS (copied) | `crates/webizen-studio/src/components/poet/css/` |
| Registry | `…/poet/kinds.rs` |
| Manifold seeds | `…/poet/manifolds.rs` |
| Shell | `…/poet/workbench.rs` + `chrome.rs` + `stage.rs` + `chest.rs` |
| Container bodies | `…/poet/bodies/` |
| Vibe console | `…/poet/vibe_console.rs` |
| GPU frame | `…/poet/gpu_frame.rs` |
