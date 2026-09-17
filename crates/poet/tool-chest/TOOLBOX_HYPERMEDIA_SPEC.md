# Tool-Chest Spec — Hypermedia Asset Toolboxes

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.
**Parent spec:** [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md)
**Core ontology:** [`qualia-ui/ontologies/hypermedia.n3`](../ontologies/hypermedia.n3) (N3 authoring → CBOR-LD runtime)
**Domain ontologies:** `image-editing.n3`, `audio-production.n3`, `video-production.n3`, `spatial-3d.n3`, `interactive-hypermedia.n3`, `portal-worlds.n3`, `production-events.n3`

These seven toolboxes serve hypermedia asset creation, editing, and inspection across image, audio, video, 3D, interactive hypermedia, portals, and live productions. They are **loosely coupled** to containers and may be used across different manifold types. Assets in one domain may reference assets in another (e.g. a 3D material references an image texture, a video references an audio track).

**Nomenclature note:** All toolboxes follow the agent nomenclature isolation rules (see [`agent-nomenclature-rules.md`](../../qualia-db-standards/agent-nomenclature-rules.md)). Tools use computational terms (process, render, enumerate, generate), not mind-dependent terms.

---

## 1. Toolbox: `image` (Raster & Vector Editing)

The `image` toolbox is for creating, editing, and inspecting raster and vector images. It is the native equivalent of Photoshop, Affinity Photo, or GIMP — but built on Vibe, CBOR-LD, and the context graph.

**Ontology:** [`qualia-ui/ontologies/image-editing.n3`](../ontologies/image-editing.n3)

### 1.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `image-canvas` | content | missing | Primary editing surface — pixel or vector canvas with zoom, pan, rulers |
| `layer-panel` | panel | missing | Layer stack — visibility, opacity, blend mode, ordering |
| `brush-palette` | panel | missing | Brush selection, size, opacity, hardness, flow |
| `colour-picker` | panel | missing | Colour selection — swatches, HSL wheel, hex input, sample |
| `histogram` | panel | missing | Luminance and RGB histogram, levels readout |
| `navigator` | panel | missing | Zoomed-out overview, pan indicator, zoom level |

### 1.2 Tool-chains

#### `layers` — layer management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-layer` | Mutate | `image_iri: iri`, `layer_type: LayerType`, `name: string` | Adds a new layer — pixel, adjustment, vector, text, or smart-object |
| `delete-layer` | Mutate | `layer_iri: iri` | Removes a layer from the stack |
| `reorder-layer` | Mutate | `layer_iri: iri`, `new_position: int` | Changes z-order of a layer |
| `merge-layers` | Mutate | `layer_iris: [iri]` | Merges selected layers into a single pixel layer |
| `duplicate-layer` | Mutate | `layer_iri: iri` | Duplicates a layer with all properties |
| `group-layers` | Mutate | `layer_iris: [iri]`, `group_name: string` | Creates a layer group |
| `layer-opacity` | Mutate | `layer_iri: iri`, `opacity: float` | Sets layer opacity (0.0–1.0) |
| `layer-blend-mode` | Mutate | `layer_iri: iri`, `blend_mode: string` | Sets blend mode — normal, multiply, screen, overlay, soft-light, hard-light, colour-dodge, colour-burn, difference, exclusion, hue, saturation, colour, luminosity |

#### `brushes` — brush tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `brush-select` | Mutate | `brush_type: string` | Selects brush type — round, flat, texture, custom |
| `brush-size` | Mutate | `size: float` | Sets brush diameter in pixels |
| `brush-opacity` | Mutate | `opacity: float` | Sets brush opacity (0.0–1.0) |
| `brush-hardness` | Mutate | `hardness: float` | Sets brush edge hardness (0.0 = soft, 1.0 = hard) |
| `brush-flow` | Mutate | `flow: float` | Sets brush flow rate (0.0–1.0) |
| `brush-colour` | Mutate | `colour: string` | Sets active brush colour |
| `eraser` | Mutate | `selection_span: [start, end]` | Erases pixels on active layer |
| `clone-stamp` | Mutate | `source_point: [x, y]`, `target_point: [x, y]` | Clones pixels from source to target |
| `heal-brush` | Mutate | `source_point: [x, y]`, `target_point: [x, y]` | Heals texture from source to target with colour matching |
| `gradient-fill` | Mutate | `start_point: [x, y]`, `end_point: [x, y]`, `gradient_type: string` | Fills with a gradient — linear, radial, conical, reflected |

#### `selection` — selection tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `rect-select` | Mutate | `rect: [x, y, w, h]` | Rectangular selection |
| `lasso-select` | Mutate | `points: [[x, y], ...]` | Freeform polygonal selection |
| `magic-wand` | Mutate | `seed_point: [x, y]`, `tolerance: float` | Selects contiguous pixels within tolerance |
| `vector-path-select` | Mutate | `path_iri: iri` | Selects along a vector path |
| `invert-selection` | Mutate | `image_iri: iri` | Inverts current selection |
| `feather-selection` | Mutate | `radius: float` | Feathers selection edges |
| `refine-edge` | Mutate | `selection_iri: iri`, `smooth: float`, `contrast: float`, `shift-edge: float` | Refines selection edge for complex boundaries |

#### `filters` — image filters

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `gaussian-blur` | Mutate | `layer_iri: iri`, `radius: float` | Applies Gaussian blur |
| `sharpen` | Mutate | `layer_iri: iri`, `amount: float`, `radius: float` | Sharpening filter |
| `colour-balance` | Mutate | `layer_iri: iri`, `shadows: [r,g,b]`, `midtones: [r,g,b]`, `highlights: [r,g,b]` | Colour balance adjustment |
| `hue-saturation` | Mutate | `layer_iri: iri`, `hue: float`, `saturation: float`, `lightness: float` | HSL adjustment |
| `curves` | Mutate | `layer_iri: iri`, `channel: string`, `points: [[x, y], ...]` | Tone curve adjustment |
| `levels` | Mutate | `layer_iri: iri`, `channel: string`, `input: [min, max]`, `output: [min, max]`, `gamma: float` | Levels adjustment |
| `distort` | Mutate | `layer_iri: iri`, `distort_type: string`, `params: CBOR-LD` | Distortion filter — lens, ripple, wave, twirl |
| `noise-reduction` | Mutate | `layer_iri: iri`, `strength: float`, `preserve_details: float` | Noise reduction filter |

#### `masks` — masking tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-layer-mask` | Mutate | `layer_iri: iri`, `mask_type: white|black|reveal-selection` | Adds a layer mask |
| `apply-mask` | Mutate | `mask_iri: iri` | Applies mask to layer permanently |
| `invert-mask` | Mutate | `mask_iri: iri` | Inverts mask values |
| `vector-mask` | Mutate | `layer_iri: iri`, `path_iri: iri` | Adds a vector-based mask |
| `clipping-mask` | Mutate | `layer_iri: iri`, `clip_to: iri` | Clips layer to the shape of another layer |

#### `colour` — colour management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `colour-picker` | Mutate | `point: [x, y]` | Samples colour from canvas |
| `sample-colour` | Query | `point: [x, y]` | Reads colour value at point without setting active |
| `colour-profile` | Mutate | `image_iri: iri`, `profile: string` | Assigns colour profile — sRGB, AdobeRGB, ProPhoto, DCI-P3, Rec.2020 |
| `convert-profile` | Mutate | `image_iri: iri`, `target_profile: string`, `rendering_intent: string` | Converts between colour profiles |
| `gradient-map` | Mutate | `layer_iri: iri`, `gradient_stops: [[pos, colour], ...]` | Maps luminance to gradient colours |
| `posterise` | Mutate | `layer_iri: iri`, `levels: int` | Reduces colour levels per channel |

#### `vector` — vector tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `pen-tool` | Mutate | `points: [[x, y], ...]`, `closed: bool` | Creates a vector path |
| `shape-tool` | Mutate | `shape_type: string`, `rect: [x, y, w, h]` | Creates a vector shape — rectangle, ellipse, polygon, star |
| `path-edit` | Mutate | `path_iri: iri`, `point_index: int`, `new_point: [x, y]` | Edits a path point |
| `text-on-path` | Mutate | `path_iri: iri`, `text: string`, `style: CBOR-LD` | Places text along a vector path |
| `vector-export` | Query | `path_iris: [iri]`, `format: svg|pdf` | Exports vector paths |
| `svg-import` | Mutate | `svg_data: string`, `layer_iri: iri` | Imports SVG into a vector layer |

#### `inspect` — inspection tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `histogram` | Query | `image_iri: iri`, `channel: string` | Displays luminance or channel histogram |
| `colour-sampler` | Query | `points: [[x, y], ...]` | Samples colour at multiple points |
| `info-probe` | Query | `point: [x, y]` | Displays pixel coordinates, colour, layer info |
| `metadata-view` | Query | `image_iri: iri` | Displays asset metadata — resolution, colour space, bit depth, DPI |
| `profile-inspector` | Query | `image_iri: iri` | Displays ICC profile details |

### 1.3 Image manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `image-canvas` | centre | Primary editing surface |
| `layer-panel` | right | Layer stack management |
| `brush-palette` | left | Brush selection and configuration |
| `colour-picker` | left (bottom) | Colour selection |
| `histogram` | bottom | Tonal analysis |
| `navigator` | top-right | Zoom overview |

---

## 2. Toolbox: `audio` (Audio Production, Synth, MIDI, Processing)

The `audio` toolbox is for recording, producing, synthesising, editing, and mixing audio. It is the native equivalent of a DAW (Logic Pro, Ableton Live, Reaper) — but built on Vibe, CBOR-LD, and the context graph.

**Ontology:** [`qualia-ui/ontologies/audio-production.n3`](../ontologies/audio-production.n3)

### 2.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `audio-timeline` | content | missing | Multi-track timeline — audio clips, MIDI clips, automation |
| `mixer` | panel | missing | Channel strips — volume, pan, sends, EQ, dynamics |
| `synth-rack` | panel | missing | Synthesiser rack — oscillators, filters, envelopes, LFOs |
| `midi-editor` | panel | missing | Piano-roll / MIDI event editor |
| `effect-chain` | panel | missing | Effect chain inspector — reorder, bypass, configure |
| `transport` | panel | missing | Play, stop, record, loop, metronome, tempo |

### 2.2 Tool-chains

#### `transport` — playback control

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `play` | Mutate | `from_position: float` | Starts playback from position (seconds) |
| `stop` | Mutate | — | Stops playback and returns to start |
| `record` | Mutate | `track_iri: iri`, `input_channel: int` | Arms track for recording and starts capture |
| `loop-region` | Mutate | `start: float`, `end: float` | Sets loop region |
| `metronome` | Mutate | `enabled: bool`, `bpm: float`, `time_signature: [n, d]` | Toggles metronome |
| `tempo-set` | Mutate | `bpm: float` | Sets project tempo |

#### `tracks` — track management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-audio-track` | Mutate | `name: string`, `channels: int` | Adds an audio track (mono/stereo) |
| `add-midi-track` | Mutate | `name: string`, `instrument_iri: iri` | Adds a MIDI track with instrument |
| `add-bus` | Mutate | `name: string` | Adds a bus track for routing |
| `delete-track` | Mutate | `track_iri: iri` | Removes a track |
| `rename-track` | Mutate | `track_iri: iri`, `name: string` | Renames a track |
| `track-volume` | Mutate | `track_iri: iri`, `volume: float` | Sets track volume (dB) |
| `track-pan` | Mutate | `track_iri: iri`, `pan: float` | Sets track pan (-1.0 left to 1.0 right) |
| `track-routing` | Mutate | `track_iri: iri`, `output: iri` | Sets track output routing (bus, master, or send) |

#### `editing` — clip editing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `split-clip` | Mutate | `clip_iri: iri`, `position: float` | Splits a clip at position |
| `join-clips` | Mutate | `clip_iris: [iri]` | Joins adjacent clips |
| `trim-clip` | Mutate | `clip_iri: iri`, `start: float`, `end: float` | Trims clip boundaries |
| `fade-in` | Mutate | `clip_iri: iri`, `duration: float`, `curve: string` | Adds fade-in |
| `fade-out` | Mutate | `clip_iri: iri`, `duration: float`, `curve: string` | Adds fade-out |
| `crossfade` | Mutate | `clip_a: iri`, `clip_b: iri`, `duration: float` | Creates crossfade between two clips |
| `time-stretch` | Mutate | `clip_iri: iri`, `ratio: float`, `algorithm: string` | Stretches clip time without pitch change |
| `pitch-shift` | Mutate | `clip_iri: iri`, `semitones: float`, `algorithm: string` | Shifts pitch without time change |
| `reverse` | Mutate | `clip_iri: iri` | Reverses clip audio |
| `normalise` | Mutate | `clip_iri: iri`, `target_db: float` | Normalises clip peak to target level |

#### `synthesis` — synthesiser configuration

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `osc-config` | Mutate | `synth_iri: iri`, `osc_id: int`, `waveform: string`, `freq: float`, `detune: float` | Configures oscillator |
| `filter-config` | Mutate | `synth_iri: iri`, `filter_type: string`, `cutoff: float`, `resonance: float` | Configures filter |
| `envelope-config` | Mutate | `synth_iri: iri`, `env_id: int`, `attack: float`, `decay: float`, `sustain: float`, `release: float` | Configures ADSR envelope |
| `lfo-config` | Mutate | `synth_iri: iri`, `lfo_id: int`, `waveform: string`, `rate: float`, `depth: float`, `target: string` | Configures LFO and modulation target |
| `mod-routing` | Mutate | `synth_iri: iri`, `source: string`, `target: string`, `amount: float` | Routes modulation source to target |
| `preset-save` | Mutate | `synth_iri: iri`, `preset_name: string` | Saves synthesiser preset |
| `preset-load` | Mutate | `synth_iri: iri`, `preset_iri: iri` | Loads a preset |
| `wavetable-import` | Mutate | `synth_iri: iri`, `wavetable_iri: iri` | Imports a wavetable |

#### `midi` — MIDI editing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `note-draw` | Mutate | `track_iri: iri`, `note: int`, `start: float`, `duration: float`, `velocity: int` | Draws a MIDI note |
| `note-edit` | Mutate | `note_iri: iri`, `note: int`, `start: float`, `duration: float` | Edits a MIDI note |
| `velocity-edit` | Mutate | `note_iri: iri`, `velocity: int` | Edits note velocity |
| `cc-edit` | Mutate | `track_iri: iri`, `cc_number: int`, `points: [[time, value], ...]` | Edits continuous controller automation |
| `quantise` | Mutate | `track_iri: iri`, `grid: float`, `strength: float` | Quantises MIDI notes to grid |
| `transpose` | Mutate | `track_iri: iri`, `semitones: int` | Transposes all notes |
| `midi-import` | Mutate | `midi_data: bytes`, `track_iri: iri` | Imports MIDI file |

#### `effects` — audio effects

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-effect` | Mutate | `track_iri: iri`, `effect_type: string` | Adds an effect to track chain |
| `reorder-effect` | Mutate | `effect_iri: iri`, `new_position: int` | Changes effect order |
| `remove-effect` | Mutate | `effect_iri: iri` | Removes an effect |
| `reverb-config` | Mutate | `effect_iri: iri`, `room_size: float`, `damping: float`, `wet: float`, `dry: float` | Configures reverb |
| `delay-config` | Mutate | `effect_iri: iri`, `time: float`, `feedback: float`, `wet: float` | Configures delay |
| `compressor-config` | Mutate | `effect_iri: iri`, `threshold: float`, `ratio: float`, `attack: float`, `release: float` | Configures compressor |
| `eq-config` | Mutate | `effect_iri: iri`, `bands: [[freq, gain, q], ...]` | Configures parametric EQ |
| `saturation-config` | Mutate | `effect_iri: iri`, `drive: float`, `mix: float`, `character: string` | Configures saturation/distortion |

#### `mixing` — mixing and bouncing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `set-send` | Mutate | `track_iri: iri`, `bus_iri: iri`, `level: float` | Sets send level to a bus |
| `set-bus` | Mutate | `track_iri: iri`, `bus_iri: iri` | Routes track output to a bus |
| `automation-draw` | Mutate | `track_iri: iri`, `parameter: string`, `points: [[time, value], ...]` | Draws automation envelope |
| `automation-edit` | Mutate | `automation_iri: iri`, `point_index: int`, `new_value: float` | Edits automation point |
| `bounce-track` | Mutate | `track_iri: iri`, `format: string` | Bounces track to audio file |
| `bounce-mix` | Mutate | `project_iri: iri`, `format: string`, `sample_rate: int`, `bit_depth: int` | Bounces full mix to audio file |

#### `inspect` — audio inspection

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `spectrum-analyser` | Query | `track_iri: iri` | Displays frequency spectrum |
| `waveform-view` | Query | `clip_iri: iri` | Displays waveform |
| `phase-meter` | Query | `track_iri: iri` | Displays phase correlation |
| `loudness-meter` | Query | `track_iri: iri` | Displays LUFS integrated and short-term |
| `metadata-view` | Query | `asset_iri: iri` | Displays audio metadata — sample rate, bit depth, channels, duration |

### 2.3 Audio manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `audio-timeline` | centre | Multi-track timeline |
| `mixer` | right | Channel strips |
| `transport` | top | Playback control |
| `midi-editor` | bottom | Piano-roll editor |
| `synth-rack` | left | Synthesiser configuration |
| `effect-chain` | right (bottom) | Effect chain inspector |

---

## 3. Toolbox: `video` (Video Production, Editing, Colour, Transitions)

The `video` toolbox is for editing, colour grading, and producing video content. It is the native equivalent of DaVinci Resolve, Premiere Pro, or Final Cut Pro — but built on Vibe, CBOR-LD, and the context graph.

**Ontology:** [`qualia-ui/ontologies/video-production.n3`](../ontologies/video-production.n3)

### 3.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `video-timeline` | content | missing | Multi-track timeline — video, audio, overlay tracks |
| `preview-monitor` | content | missing | Video preview — full-screen or overlay |
| `colour-scope` | panel | missing | Waveform monitor, vectorscope, histogram |
| `effect-inspector` | panel | missing | Effect parameters, keyframes, masking |
| `asset-bin` | panel | missing | Media bin — clips, generated content, imports |
| `transport` | panel | missing | Play, stop, mark in/out, shuttle, jog |

### 3.2 Tool-chains

#### `transport` — playback control

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `play` | Mutate | `from_frame: int` | Starts playback from frame |
| `stop` | Mutate | — | Stops playback |
| `mark-in` | Mutate | `frame: int` | Sets in-point |
| `mark-out` | Mutate | `frame: int` | Sets out-point |
| `shuttle` | Mutate | `speed: float` | Variable-speed playback (-32x to +32x) |
| `jog` | Mutate | `frame_delta: int` | Steps by frame(s) |

#### `editing` — clip editing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `razor` | Mutate | `clip_iri: iri`, `frame: int` | Cuts clip at frame |
| `splice` | Mutate | `clip_iris: [iri]` | Splices clips together |
| `overwrite` | Mutate | `clip_iri: iri`, `track_iri: iri`, `position: int` | Overwrites at position |
| `insert` | Mutate | `clip_iri: iri`, `track_iri: iri`, `position: int` | Inserts and ripples downstream |
| `ripple-delete` | Mutate | `clip_iri: iri` | Deletes clip and closes gap |
| `lift` | Mutate | `clip_iri: iri` | Removes clip content, leaves gap |
| `append` | Mutate | `clip_iri: iri`, `track_iri: iri` | Appends to end of track |
| `split-clip` | Mutate | `clip_iri: iri`, `frame: int` | Splits clip into two |
| `match-frame` | Query | `clip_iri: iri` | Finds source frame in asset bin |
| `replace` | Mutate | `clip_iri: iri`, `replacement_iri: iri` | Replaces clip with another |

#### `transitions` — video transitions

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `crossfade` | Mutate | `clip_a: iri`, `clip_b: iri`, `duration: float` | Cross-dissolve transition |
| `wipe` | Mutate | `clip_a: iri`, `clip_b: iri`, `wipe_type: string`, `duration: float` | Wipe transition — left, right, up, down, radial |
| `dissolve` | Mutate | `clip_a: iri`, `clip_b: iri`, `duration: float` | Dissolve transition |
| `push` | Mutate | `clip_a: iri`, `clip_b: iri`, `direction: string`, `duration: float` | Push transition |
| `slide` | Mutate | `clip_a: iri`, `clip_b: iri`, `direction: string`, `duration: float` | Slide transition |
| `dip-to-black` | Mutate | `clip_a: iri`, `clip_b: iri`, `duration: float` | Dip to black transition |
| `custom-transition` | Mutate | `clip_a: iri`, `clip_b: iri`, `transition_iri: iri`, `params: CBOR-LD` | Custom transition from template |

#### `colour` — colour grading

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `lift-gamma-gain` | Mutate | `clip_iri: iri`, `lift: [r,g,b]`, `gamma: [r,g,b]`, `gain: [r,g,b]` | Lift/Gamma/Gain colour grading |
| `saturation` | Mutate | `clip_iri: iri`, `saturation: float` | Saturation adjustment |
| `colour-wheels` | Mutate | `clip_iri: iri`, `shadows: [r,g,b]`, `midtones: [r,g,b]`, `highlights: [r,g,b]` | Colour wheel grading |
| `curves` | Mutate | `clip_iri: iri`, `channel: string`, `points: [[x, y], ...]` | Tone curve grading |
| `hue-vs-hue` | Mutate | `clip_iri: iri`, `points: [[hue_in, hue_out], ...]` | Hue vs hue secondary grading |
| `hue-vs-sat` | Mutate | `clip_iri: iri`, `points: [[hue, sat], ...]` | Hue vs saturation secondary grading |
| `lut-apply` | Mutate | `clip_iri: iri`, `lut_iri: iri` | Applies a LUT |
| `lut-export` | Query | `grade_iri: iri`, `format: string` | Exports grade as LUT (.cube, .3dl) |

#### `effects` — video effects

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `stabilise` | Mutate | `clip_iri: iri`, `smoothness: float` | Stabilises shaky footage |
| `speed-change` | Mutate | `clip_iri: iri`, `speed: float`, `interpolation: string` | Changes clip speed |
| `transform` | Mutate | `clip_iri: iri`, `position: [x, y]`, `scale: float`, `rotation: float` | Transform effect |
| `chroma-key` | Mutate | `clip_iri: iri`, `key_colour: string`, `tolerance: float`, `spill: float` | Chroma key (green screen) |
| `mask` | Mutate | `clip_iri: iri`, `mask_type: string`, `shape: CBOR-LD` | Masks part of the frame |
| `motion-track` | Mutate | `clip_iri: iri`, `track_point: [x, y]`, `range: [start, end]` | Tracks motion in footage |
| `add-blur` | Mutate | `clip_iri: iri`, `blur_type: string`, `radius: float` | Adds blur effect |

#### `generators` — content generators

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `title-generator` | Mutate | `text: string`, `style: CBOR-LD`, `duration: float` | Generates a title clip |
| `lower-third` | Mutate | `text: string`, `style: CBOR-LD`, `duration: float` | Generates a lower-third graphic |
| `shape-generator` | Mutate | `shape: string`, `colour: string`, `duration: float` | Generates a shape clip |
| `colour-matte` | Mutate | `colour: string`, `duration: float` | Generates a solid colour clip |
| `noise-generator` | Mutate | `noise_type: string`, `duration: float`, `intensity: float` | Generates a noise clip |

#### `audio-sync` — audio synchronisation

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `sync-from-audio` | Mutate | `video_iri: iri`, `audio_iri: iri` | Syncs video to audio waveform |
| `auto-align` | Mutate | `clip_iris: [iri]` | Auto-aligns clips by waveform |
| `scrub-audio` | Query | `clip_iri: iri`, `frame: int` | Plays audio at frame position |
| `extract-audio` | Mutate | `clip_iri: iri` | Extracts audio from video clip |
| `link-clips` | Mutate | `video_iri: iri`, `audio_iri: iri` | Links video and audio clips |

#### `inspect` — video inspection

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `waveform-monitor` | Query | `clip_iri: iri`, `frame: int` | Displays luminance waveform |
| `vectorscope` | Query | `clip_iri: iri`, `frame: int` | Displays vectorscope |
| `histogram` | Query | `clip_iri: iri`, `frame: int` | Displays RGB histogram |
| `frame-info` | Query | `clip_iri: iri`, `frame: int` | Displays frame metadata — timecode, resolution, codec |
| `metadata-view` | Query | `asset_iri: iri` | Displays video metadata — codec, resolution, frame rate, colour space, bit depth |

#### `render` — rendering and export

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `render-queue` | Mutate | `project_iri: iri`, `output: CBOR-LD` | Queues a render job |
| `render-settings` | Mutate | `render_iri: iri`, `codec: string`, `quality: float`, `resolution: string` | Configures render settings |
| `render-preview` | Query | `clip_iri: iri`, `frame: int` | Renders a preview frame |
| `export-preset` | Mutate | `preset_name: string`, `settings: CBOR-LD` | Saves export preset |
| `batch-render` | Mutate | `render_iris: [iri]` | Batch renders multiple jobs |

### 3.3 Video manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `video-timeline` | bottom | Multi-track timeline |
| `preview-monitor` | centre | Video preview |
| `colour-scope` | right | Waveform, vectorscope |
| `asset-bin` | left | Media bin |
| `transport` | top | Playback control |
| `effect-inspector` | right (bottom) | Effect parameters |

---

**Part 2:** Toolboxes for 3D, interactive hypermedia, portals, and productions are in [`TOOLBOX_HYPERMEDIA_SPEC_2.md`](TOOLBOX_HYPERMEDIA_SPEC_2.md).

---

## 8. Cross-domain references

Assets in one domain may reference assets in another:

| Source domain | Target domain | Reference type | Example |
|:-------------|:-------------|:---------------|:--------|
| 3D | Image | texture | 3D material references image texture |
| Video | Audio | audio-sync | Video clip references audio track with sync offset |
| Hypermedia | Video | stream-overlay | Interactive package references video stream |
| Hypermedia | Audio | stream-overlay | Interactive package references audio stream |
| Productions | Video | projection-content | Projection surface references video content |
| Productions | Audio | show-control | Production references audio for show sync |
| Portals | 3D | scene-embed | Portal world references 3D scene |
| Portals | Audio | ambient-audio | Portal world references ambient audio |

Cross-domain references use `hm:referencesAsset` with `hm:referenceType` and `hm:syncOffset` as defined in `hypermedia.n3`.

---

## 9. Relationship to existing specs

| Document | Relationship |
|:---------|:-------------|
| [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md) | Parent spec — hierarchy, core traits, ontology layer |
| [`TOOLBOX_HYPERMEDIA_SPEC_2.md`](TOOLBOX_HYPERMEDIA_SPEC_2.md) | Part 2 — 3D, interactive, portals, productions toolboxes |
| [`qualia-ui/ontologies/hypermedia.n3`](../ontologies/hypermedia.n3) | Core hypermedia ontology — asset types, provenance, cross-domain references |
| [`qualia-ui/ontologies/image-editing.n3`](../ontologies/image-editing.n3) | Image domain ontology — layers, filters, brushes, masks |
| [`qualia-ui/ontologies/audio-production.n3`](../ontologies/audio-production.n3) | Audio domain ontology — tracks, synthesis, MIDI, effects |
| [`qualia-ui/ontologies/video-production.n3`](../ontologies/video-production.n3) | Video domain ontology — timeline, clips, colour, transitions |
| [`qualia-ui/ontologies/spatial-3d.n3`](../ontologies/spatial-3d.n3) | 3D domain ontology — meshes, rigs, animation, narratives |
| [`qualia-ui/ontologies/interactive-hypermedia.n3`](../ontologies/interactive-hypermedia.n3) | Interactive domain ontology — HbbTV, 2nd screen, social |
| [`qualia-ui/ontologies/portal-worlds.n3`](../ontologies/portal-worlds.n3) | Portal domain ontology — worlds, immersive environments |
| [`qualia-ui/ontologies/production-events.n3`](../ontologies/production-events.n3) | Production domain ontology — DMX, projection, events |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — content/panel/widget kinds |
| [`qualia-ui/ontologies/provenance.n3`](../ontologies/provenance.n3) | Provenance — asset provenance chains |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | UI concepts — manifolds, containers, presentation |
| [`qualia-db-standards/agent-nomenclature-rules.md`](../../qualia-db-standards/agent-nomenclature-rules.md) | Agent nomenclature isolation rules |

---

## 10. Tool count summary (all 7 toolboxes)

| Toolbox | Part | Tool-chains | Tools |
|:--------|:-----|:-----------|:------|
| `image` | 1 | 8 | 55 |
| `audio` | 1 | 8 | 58 |
| `video` | 1 | 9 | 58 |
| `3d` | 2 | 8 | 52 |
| `hypermedia` | 2 | 6 | 37 |
| `portals` | 2 | 6 | 36 |
| `productions` | 2 | 7 | 43 |
| **Total** | | **52** | **339** |
