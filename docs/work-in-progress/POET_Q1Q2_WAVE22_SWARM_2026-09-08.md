# Q1/Q2 incorporation wave 22 swarm — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** **Complete (integrated)**  
**Prior:** wave 21 Complete — Audio +9 · Scene +8 · NLP +9 Live; Host CG/Stats +6  
**Backlog (stale file still shows pre-w21):** post-w21 inventory `ALL_BOUND=1062` · `PoetLive=649` · Q2 ≈ 413

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A `Q2-AUDIO` | Exhaust remaining `Audio.*` Q2 (13) | `audio_fx_chain_actions.rs` + chain `audio:fx` |
| B `Q2-SCENE` | Exhaust remaining `Scene.*` Q2 (11) | `scene_graph_chain_actions.rs` + extend `spatial:scene` |
| C `Q2-IMAGE` | `Image.*` Q2 (15) | `image_edit_chain_actions.rs` + chain `image:edit` |
| D `Q1-HOST` | 8 CG predicates/queries | `geometry/wave22_host.rs` + paired catalogs |

### Shared-file rule (avoid merge collisions)

Lanes **must not** edit:

- `crates/poet/src/browser/tool_actions.rs`
- `crates/poet/src/browser/tool_copy.rs`
- `crates/poet/src/browser/registration/mod.rs`

Parent wires `has_live_invoke`, dispatch arms, `tool_copy`, and registration asserts after lanes land.

Lanes **do** edit their own files listed below + `crates/poet/src/browser/mod.rs` (`mod` line only, unique).

**Never `Proficiency::Advanced`.** Dual-path. Exact Host scopes. No Host invent on Live lanes.

---

## Lane A — remaining Audio Live (13)

Already Live (wave 21): `epistemic_temperature_from_q`, `epistemic_fm_index`, `sigma_dominant_frequency`, `parametric_sample`, `bin_to_freq_linear`, `bin_to_freq_log`, `midi_note`, `quantize`, `transpose`.

**Bind these Host ids** (already in `ALL_BOUND`):

| Tool id | Host id | Args (match Host) |
|--------|---------|--------------------|
| `audio:fx_oscillator` | `Audio.oscillator` | `{ waveform, frequency, sample_rate?, n?, gain? }` |
| `audio:fx_envelope` | `Audio.envelope` | `{ attack?, decay?, sustain?, release?, sample_rate?, n?, note_off_samples? }` |
| `audio:fx_filter` | `Audio.filter` | `{ input, filter_type, cutoff, q?, sample_rate? }` |
| `audio:fx_lfo` | `Audio.lfo` | `{ waveform?, frequency, sample_rate?, n?, depth? }` |
| `audio:fx_delay` | `Audio.delay` | `{ input, delay_samples?, feedback?, mix? }` |
| `audio:fx_reverb` | `Audio.reverb` | `{ input, room_size?, damping?, mix?, sample_rate? }` |
| `audio:fx_compressor` | `Audio.compressor` | `{ input, threshold?, ratio?, attack?, release?, sample_rate? }` |
| `audio:fx_eq` | `Audio.eq` | `{ input, low_gain?, mid_gain?, high_gain?, sample_rate? }` |
| `audio:fx_transport` | `Audio.transport` | `{ action, tempo?, sample_rate?, loop_start?, loop_end? }` |
| `audio:fx_waveform_meter` | `Audio.waveform_meter` | `{ input, buckets? }` |
| `audio:fx_phase_meter` | `Audio.phase_meter` | `{ left, right }` |
| `audio:fx_loudness_meter` | `Audio.loudness_meter` | `{ input, sample_rate? }` |
| `audio:fx_spectrum` | `Audio.spectrum` | `{ raster, frame_count, bin_count, sample_rate, hop_size }` |

Host impl: `crates/qualia-core-db/src/poet_host/invoke/audio/dsp.rs` and `audio/mod.rs` (`spectrum`).

Copy `invoke_dual` from `audio_chain_actions.rs`. Local sketches: small bounded buffers (n≤16 for osc/lfo; 8-sample input for filter/eq/delay/reverb/comp/meters). Spectrum: tiny raster default.

Files:

- **create** `crates/poet/src/browser/audio_fx_chain_actions.rs`
- `crates/poet/src/browser/mod.rs` — `mod audio_fx_chain_actions;`
- `crates/poet/src/browser/registration/register_audio_toolbox.rs` — new chain `audio:fx` with 13 tools
- test `local_audio_wave22_fx_sketches` in the new file (known-good scalars: sine n=1 at t=0 is 0; envelope attack>0; filter identity-ish on DC for lowpass)

---

## Lane B — remaining Scene Live (11)

Already Live (wave 21): `lerp_camera`, `camera_frame_node`, `smooth_damp`, `smooth_damp_vec3`, `ik_look_at`, `ik_ccd`, `set_render_budget`, `set_clear_colour`.

**Bind these:**

| Tool id | Host id | Args |
|--------|---------|------|
| `spatial:scene_create` | `Scene.create` | `{ name }` |
| `spatial:scene_add_node` | `Scene.add_node` | `{ id, x?, y?, z? }` |
| `spatial:scene_set_transform` | `Scene.set_transform` | `{ node_id, tx?, ty?, tz?, rx?, ry?, rz?, sx?, sy?, sz? }` |
| `spatial:scene_set_mesh` | `Scene.set_mesh` | `{ node_id, mesh_iri }` |
| `spatial:scene_add_camera` | `Scene.add_camera` | `{ x?, y?, z?, fov? }` |
| `spatial:scene_render` | `Scene.render` | `{ scene, camera_id? }` |
| `spatial:scene_set_viewport` | `Scene.set_viewport` | `{ width, height, format? }` |
| `spatial:scene_capture_frame` | `Scene.capture_frame` | `{ viewport_id? }` |
| `spatial:scene_add_light` | `Scene.add_light` | `{ light_type, colour?, intensity?, position?, direction?, inner_cone?, outer_cone? }` |
| `spatial:scene_link_semantic` | `Scene.link_semantic` | `{ node_id, semantic_iri, link_type?, confidence? }` |
| `spatial:scene_duplicate_node` | `Scene.duplicate_node` | `{ source_id, new_id, parent? }` |

Host: `crates/qualia-core-db/src/poet_host/invoke/render/scene_graph.rs`.

Files:

- **create** `crates/poet/src/browser/scene_graph_chain_actions.rs`
- `mod scene_graph_chain_actions;` in `browser/mod.rs`
- append 11 tools to `scene_tools` in `register_spatial_toolbox.rs`
- test `local_scene_wave22_graph_defaults`

---

## Lane C — Image Live (15)

**Bind all `Image.*` Q2:**

| Tool id | Host id | Args |
|--------|---------|------|
| `image:edit_new` | `Image.new` | `{ id, width?, height? }` |
| `image:edit_add_layer` | `Image.add_layer` | `{ id, name? }` |
| `image:edit_remove_layer` | `Image.remove_layer` | `{ id, index? }` |
| `image:edit_set_pixel` | `Image.set_pixel` | `{ id, x, y, r?, g?, b?, a? }` |
| `image:edit_fill` | `Image.fill` | `{ id, r?, g?, b? }` |
| `image:edit_brush` | `Image.brush` | `{ id, size?, points? }` |
| `image:edit_apply_filter` | `Image.apply_filter` | `{ id, filter, intensity? }` |
| `image:edit_set_opacity` | `Image.set_opacity` | `{ id, opacity }` |
| `image:edit_set_blend_mode` | `Image.set_blend_mode` | `{ id, blend_mode }` |
| `image:edit_set_visible` | `Image.set_visible` | `{ id, visible? }` |
| `image:edit_set_mask` | `Image.set_mask` | `{ id, x?, y?, width?, height? }` |
| `image:edit_clear_mask` | `Image.clear_mask` | `{ id }` |
| `image:edit_composite` | `Image.composite` | `{ id }` |
| `image:edit_add_selection` | `Image.add_selection` | `{ id, selection_id }` |
| `image:edit_clear_selections` | `Image.clear_selections` | `{ id }` |

Host: `crates/qualia-core-db/src/poet_host/invoke/hypermedia/mod.rs`.

Default `id` from selected container `data-image-id` or `"doc-1"`.

Files:

- **create** `crates/poet/src/browser/image_edit_chain_actions.rs`
- `mod image_edit_chain_actions;` in `browser/mod.rs`
- `register_image_toolbox.rs` — new chain `image:edit`
- test `local_image_wave22_edit_defaults`

Do **not** mix with existing ComputerVision `image:histogram` tools.

---

## Lane D — Host CG ×8 (pure CPU)

Wrap existing `specialized_libs::computational_geometry` functions. **No GPU / forge / CUDA / `caps()`.**

| Host id | Function | Args | Out |
|---------|----------|------|-----|
| `ComputationalGeometry.incircle` | `incircle(a,b,c,d) -> Sign` | `{ a,b,c,d: [f64;2] }` | `{ sign: i64 }` (−1/0/1) |
| `ComputationalGeometry.tukey_depth` | `tukey_depth(query, points)` | `{ query:[f64;2], points:[[f64;2]] }` | `{ depth: u64 }` |
| `ComputationalGeometry.directional_width` | `directional_width(points, dir)` | `{ points:[[f64;2]], dir:[f64;2] }` | `{ width: f64 }` |
| `ComputationalGeometry.width` | `width(points)` | `{ points:[[f64;2]] }` | `{ width: f64 }` |
| `ComputationalGeometry.farthest_site_brute` | `farthest_site_brute(sites, q)` | `{ sites:[[f64;2]], q:[f64;2] }` | `{ index: u64 }` |
| `ComputationalGeometry.k_nearest_sites` | `k_nearest_sites(sites, q, k)` | `{ sites:[[f64;2]], q:[f64;2], k:u64 }` | `{ indices: [u64] }` |
| `ComputationalGeometry.is_hull_site` | `is_hull_site(sites, idx)` | `{ sites:[[f64;2]], index:u64 }` | `{ on_hull: bool }` |
| `ComputationalGeometry.centrepoint` | `centrepoint(points)` | `{ points:[[f64;2]] }` | `{ x, y }` or fail if None |

Point cap: 1..=1024. Parse like `wave21_host.rs`.

Files (paired catalogs **must** stay in sync):

- **create** `crates/qualia-core-db/src/poet_host/invoke/geometry/wave22_host.rs`
- `geometry/mod.rs` — `mod wave22_host;` + `pub use` + wasm `geom_stub!` names
- `poet_host/invoke/ids.rs` — consts, `ALL_BOUND` array, family `"geometry"`
- `crates/vibe/src/catalog/ids.rs` — same 8 strings
- `poet_host/invoke/mod.rs` — dispatch arms
- tests named `wave22_*` (8 tests)

Avoid `LinearAlgebra.gemm`.

---

## Parent verify (after integrate)

```
cargo test -p poet --lib wave22
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
cargo test -p qualia-core-db --lib wave22
cargo test -p qualia-core-db --lib vibe_catalog_contains_every_bound_invoke_id
```

Then refresh `scripts/vibe_incorporation_backlog.py` and update register/ledger/handover.
