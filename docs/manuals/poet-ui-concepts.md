# Poet UI concepts — manifolds, tool chest, containers

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Status:** product vocabulary for Poet chrome. Classic UI stays default (D7).  
**Instructional mock:** `C:\Projects\NLP\Canvas_Workbench\designnotes.md`

This is how the workbench is named. HTML `<canvas>` / wgpu / Canvas2D stay **canvas** (drawing surfaces).

---

## 1. Four kinds of thing (plus furniture)

See [ADR 0012](adr/0012-construct-is-the-distributable-composition.md). QApp is **not** a runtime type.

| Kind | What it is | Many of… |
|---|---|---|
| **Construct** | **This user’s** Qualia/Webizen/POET environment on their hardware: interconnected manifolds of interactivism, unique to them. Not the infosphere/noosphere (those are broader layers their construct may join). Not an app; not a person. | POET, Health, Research lab, Studio, Rights, Knowledge |
| **Subject** | The *thing under consideration* (plant, place, diegetic world). Aspects become nested manifolds. Declared with `Poet.subject_declare`; stored as `SubjectSeed` (not a construct). | A plant; Star Trek; a catchment |
| **Project** | Shared, time-bound delivery (camping network, SDG programme). Authored *from* constructs; already a POET family. | Camping sites; an SDG programme |
| **Manifold** | A *lens* / work surface. Lives on a construct; may be an aspect of a subject. May be **personal** (one observer) or **social** (many people — especially projects). Anatomy is a manifold, not a construct. | Research, Media, Social, Health, Anatomy, Projects, … |
| **Container** | A typed *occupant* placed on a manifold. May portal to another manifold or construct. | doc, sheet, nested_manifold, construct_shelf, domain_lab, … |
| **Workspace** | Live projection of an open construct onto this machine’s devices. | (one session) |
| **Tool chest** | The *furniture* that holds toolboxes. One chest per workbench, dockable. | (usually one) |
| **Toolbox** | A *themed drawer* inside the chest. | Vibe, Documents, Sheets, Spatial, Social, Rights, Health, Epistemic |
| **Tool** | One *action* in a toolbox (often “place this container”). | + Document, Run cell, Gazetteer, … |

**Library Software** is how you *find* a scope. The **construct shelf** (Settings) is the desk of scopes this observer holds. Stubs stay in the library until they have lenses (manifolds).

A **tool chest** is not a toolbox. The chest is the rack; a toolbox is a drawer; a tool is what you pick up.

Legacy mock folder `toolboxes/` is the chest’s contents. Do not rename HTML/GPU canvas modules here (D14).

---

## 2. Manifolds

Lenses inside a construct — switchable surfaces, like a pager of virtual desktops. A container on one manifold can target another (**nested manifold**). Anatomy is a lens, not a construct.

| Id | Role |
|---|---|
| `research` | GIS, clinical, rights alignment |
| `media` | `.10d` kinematics, grapheme, acoustic |
| `social` | chat graphs, live peers, field notes |
| `settings` | capabilities, fiduciary VM, sub-manifolds |
| `vibe` | VibeScript console + diagnose (human door into Qualia) |

Pager: Overview + numbered surfaces + `+`. Shortcuts later: Alt+1…

The manifold surface is **not a fixed box**. Drag, place, and pan extend it in any direction (including left and up). The viewport is a window onto that world.

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
| `nested_manifold` | live | portal; pager + breadcrumb dive/pop |
| `subject` | live | `Poet.subject_declare` card; `SubjectSeed` |
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
