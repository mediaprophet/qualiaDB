# Tool-Chest Spec — Hypermedia Asset Toolboxes (Part 2: 3D, Interactive, Portals, Productions)

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.
**Parent spec:** [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md)
**Part 1:** [`TOOLBOX_HYPERMEDIA_SPEC.md`](TOOLBOX_HYPERMEDIA_SPEC.md) (image, audio, video)
**Core ontology:** [`qualia-ui/ontologies/hypermedia.n3`](../ontologies/hypermedia.n3) (N3 authoring → CBOR-LD runtime)

This is the second half of the hypermedia asset toolbox spec. It covers 3D editing, interactive hypermedia (HbbTV), portals (worlds), and productions (DMX, projection mapping). See Part 1 for image, audio, and video toolboxes, and for the shared header, nomenclature note, and cross-domain references.

---

## 4. Toolbox: `3d` (3D Assets, Animation, Narratives)

The `3d` toolbox is for creating, rigging, animating, and inspecting 3D assets and scenes. It is the native equivalent of Blender, Maya, or Unity — but built on Vibe, CBOR-LD, and the context graph.

**Ontology:** [`qualia-ui/ontologies/spatial-3d.n3`](../ontologies/spatial-3d.n3)

### 4.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `viewport-3d` | content | missing | 3D viewport — perspective, orthographic, wireframe, solid, rendered |
| `outliner` | panel | missing | Scene hierarchy — objects, groups, collections |
| `properties-3d` | panel | missing | Object properties — transform, material, modifiers, constraints |
| `timeline-3d` | panel | missing | Animation timeline — keyframes, dope sheet, graph editor |
| `material-editor` | panel | missing | Material node editor — PBR, custom shader, texture sets |
| `asset-library` | panel | missing | 3D asset library — meshes, materials, HDRI, presets |

### 4.2 Tool-chains

#### `object` — scene object management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-mesh` | Mutate | `mesh_type: string`, `name: string` | Adds a primitive mesh — cube, sphere, cylinder, plane, cone, torus |
| `import-mesh` | Mutate | `file_iri: iri`, `format: string` | Imports a mesh file — glTF, OBJ, FBX, STL |
| `delete-object` | Mutate | `object_iri: iri` | Removes an object from the scene |
| `duplicate-object` | Mutate | `object_iri: iri`, `linked: bool` | Duplicates an object (linked or independent) |
| `parent-object` | Mutate | `child_iri: iri`, `parent_iri: iri` | Parents one object to another |
| `group-objects` | Mutate | `object_iris: [iri]`, `group_name: string` | Groups objects |
| `snap-to-grid` | Mutate | `object_iri: iri`, `grid_size: float` | Snaps object to grid |
| `transform-object` | Mutate | `object_iri: iri`, `position: [x,y,z]`, `rotation: [x,y,z]`, `scale: [x,y,z]` | Sets object transform |

#### `modelling` — mesh editing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `extrude` | Mutate | `mesh_iri: iri`, `selection: [int]`, `distance: float` | Extrudes selected faces/edges/vertices |
| `inset` | Mutate | `mesh_iri: iri`, `selection: [int]`, `depth: float` | Insets selected faces |
| `bevel` | Mutate | `mesh_iri: iri`, `selection: [int]`, `width: float`, `segments: int` | Bevels selected edges/vertices |
| `loop-cut` | Mutate | `mesh_iri: iri`, `edge: int`, `cuts: int`, `position: float` | Adds loop cuts |
| `subdivide` | Mutate | `mesh_iri: iri`, `selection: [int]`, `levels: int` | Subdivides selected geometry |
| `merge-vertices` | Mutate | `mesh_iri: iri`, `vertices: [int]`, `merge_type: string` | Merges vertices — center, first, last, collapse |
| `knife-tool` | Mutate | `mesh_iri: iri`, `cut_points: [[x,y,z], ...]` | Cuts faces along a path |
| `boolean-op` | Mutate | `mesh_a: iri`, `mesh_b: iri`, `operation: string` | Boolean operation — union, difference, intersect |

#### `rigging` — skeletal rigging

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-bone` | Mutate | `rig_iri: iri`, `parent_bone: iri`, `position: [x,y,z]`, `name: string` | Adds a bone to the rig |
| `weight-paint` | Mutate | `mesh_iri: iri`, `bone_iri: iri`, `vertex_weights: [[int, float], ...]` | Sets vertex weights for a bone |
| `inverse-kinematics` | Mutate | `rig_iri: iri`, `chain: [iri]`, `target: iri`, `solver: string` | Sets up IK chain |
| `bone-constraint` | Mutate | `bone_iri: iri`, `constraint_type: string`, `target: iri`, `params: CBOR-LD` | Adds a bone constraint |
| `pose-mirror` | Mutate | `rig_iri: iri`, `axis: string` | Mirrors pose across axis |
| `rig-test` | Query | `rig_iri: iri` | Tests rig for issues — weight gaps, bone conflicts |

#### `animation` — keyframe animation

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `keyframe-insert` | Mutate | `object_iri: iri`, `property: string`, `frame: int`, `value: CBOR-LD` | Inserts a keyframe |
| `keyframe-edit` | Mutate | `keyframe_iri: iri`, `value: CBOR-LD`, `interpolation: string` | Edits a keyframe |
| `dope-sheet` | Query | `object_iris: [iri]` | Displays dope sheet for objects |
| `graph-editor` | Query | `object_iri: iri`, `property: string` | Displays animation curve graph |
| `action-clip` | Mutate | `animation_iri: iri`, `name: string`, `start: int`, `end: int` | Creates a reusable action clip |
| `animation-mixer` | Mutate | `object_iri: iri`, `clips: [[iri, start, blend], ...]` | Mixes action clips on timeline |
| `motion-path` | Query | `object_iri: iri`, `frame_range: [int, int]` | Displays motion path for frame range |

#### `materials` — material and texture editing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-material` | Mutate | `name: string`, `material_type: string` | Creates a material — PBR, unlit, custom-shader |
| `assign-material` | Mutate | `object_iri: iri`, `material_iri: iri` | Assigns material to object |
| `texture-paint` | Mutate | `mesh_iri: iri`, `texture_iri: iri`, `brush: CBOR-LD` | Paints directly on mesh texture |
| `uv-edit` | Mutate | `mesh_iri: iri`, `uv_map: CBOR-LD` | Edits UV mapping |
| `material-node` | Mutate | `material_iri: iri`, `nodes: CBOR-LD` | Edits material node graph |
| `pbr-preset` | Mutate | `material_iri: iri`, `preset: string` | Applies a PBR preset — metal, wood, stone, glass, fabric |

#### `camera-light` — camera and lighting

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-camera` | Mutate | `position: [x,y,z]`, `rotation: [x,y,z]`, `lens: float` | Adds a camera |
| `camera-frame` | Mutate | `camera_iri: iri`, `target_iri: iri` | Frames camera on target |
| `add-light` | Mutate | `light_type: string`, `position: [x,y,z]`, `intensity: float`, `colour: string` | Adds a light — point, directional, spot, area |
| `light-type` | Mutate | `light_iri: iri`, `light_type: string` | Changes light type |
| `hdri-setup` | Mutate | `scene_iri: iri`, `hdri_iri: iri`, `rotation: float`, `intensity: float` | Sets up HDRI environment |
| `shadow-setup` | Mutate | `light_iri: iri`, `shadow_type: string`, `resolution: int`, `bias: float` | Configures shadows |

#### `narrative` — story and narrative tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-scene` | Mutate | `narrative_iri: iri`, `name: string`, `duration: float` | Adds a narrative scene |
| `scene-sequence` | Mutate | `narrative_iri: iri`, `scene_order: [iri]` | Orders scenes in sequence |
| `camera-path` | Mutate | `camera_iri: iri`, `path: [[x,y,z], ...]`, `durations: [float]` | Creates a camera path animation |
| `storyboard-frame` | Mutate | `scene_iri: iri`, `frame_iri: iri`, `description: string` | Adds a storyboard frame |
| `trigger-zone` | Mutate | `scene_iri: iri`, `zone: CBOR-LD`, `action: CBOR-LD` | Adds a trigger zone for interactive narratives |
| `narrative-export` | Query | `narrative_iri: iri`, `format: string` | Exports narrative as storyboard or script |

#### `inspect` — 3D inspection

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `mesh-statistics` | Query | `mesh_iri: iri` | Displays vertex count, polygon count, UV maps |
| `material-inspector` | Query | `material_iri: iri` | Displays material properties and texture channels |
| `rig-inspector` | Query | `rig_iri: iri` | Displays bone hierarchy, constraints, weights |
| `scene-graph` | Query | `scene_iri: iri` | Displays full scene hierarchy |
| `metadata-view` | Query | `asset_iri: iri` | Displays 3D asset metadata — format, vertex count, rig bones, morph targets |

### 4.3 3D manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `viewport-3d` | centre | 3D viewport |
| `outliner` | top-right | Scene hierarchy |
| `properties-3d` | right | Object properties |
| `timeline-3d` | bottom | Animation timeline |
| `material-editor` | left | Material node editor |
| `asset-library` | top-left | 3D asset library |

---

## 5. Toolbox: `hypermedia` (2nd Screen, Interactive, HbbTV)

The `hypermedia` toolbox is for creating interactive hypermedia experiences — second-screen apps, social TV overlays, and HbbTV 2.x packaged content. It bridges broadcast/video with interactive and social layers.

**Ontology:** [`qualia-ui/ontologies/interactive-hypermedia.n3`](../ontologies/interactive-hypermedia.n3)

### 5.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `interactive-timeline` | content | missing | Timeline of triggers, overlays, and sync events |
| `preview-screen` | content | missing | Preview of broadcast + overlay rendering |
| `trigger-panel` | panel | missing | Trigger configuration — conditions, actions |
| `social-panel` | panel | missing | Social layer — chat, poll, reaction, share |
| `package-inspector` | panel | missing | HbbTV package inspector — AIT, DRM, stream config |
| `device-emulator` | panel | missing | Target device emulator for testing |

### 5.2 Tool-chains

#### `interactive` — interactive overlay tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-trigger` | Mutate | `timeline_iri: iri`, `position: float`, `trigger_type: string` | Adds a trigger — time-based, event-based, user-input, sensor |
| `trigger-condition` | Mutate | `trigger_iri: iri`, `condition: CBOR-LD` | Sets trigger condition |
| `trigger-action` | Mutate | `trigger_iri: iri`, `action: CBOR-LD` | Sets trigger action — show overlay, navigate, call API |
| `overlay-create` | Mutate | `name: string`, `template: string`, `position: CBOR-LD` | Creates an overlay |
| `overlay-edit` | Mutate | `overlay_iri: iri`, `content: CBOR-LD` | Edits overlay content |
| `overlay-timeline` | Mutate | `overlay_iri: iri`, `start: float`, `duration: float` | Positions overlay on timeline |
| `interactive-preview` | Query | `project_iri: iri` | Previews interactive experience |
| `interactive-test` | Mutate | `project_iri: iri`, `test_scenario: CBOR-LD` | Runs interactive test scenario |

#### `second-screen` — second screen tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `companion-app-config` | Mutate | `project_iri: iri`, `app_config: CBOR-LD` | Configures companion app |
| `sync-stream-setup` | Mutate | `project_iri: iri`, `stream_iri: iri`, `sync_method: string` | Sets up sync stream |
| `remote-control-map` | Mutate | `project_iri: iri`, `key_map: CBOR-LD` | Maps remote control keys to actions |
| `second-screen-preview` | Query | `project_iri: iri` | Previews second-screen experience |
| `push-notification` | Mutate | `project_iri: iri`, `message: CBOR-LD`, `trigger_iri: iri` | Configures push notification |
| `screen-pairing` | Mutate | `project_iri: iri`, `pairing_method: string` | Configures screen pairing |

#### `social` — social layer tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `chat-overlay` | Mutate | `project_iri: iri`, `config: CBOR-LD` | Adds chat overlay |
| `poll-create` | Mutate | `project_iri: iri`, `question: string`, `options: [string]`, `duration: float` | Creates a poll |
| `reaction-overlay` | Mutate | `project_iri: iri`, `reaction_type: string`, `position: CBOR-LD` | Adds reaction overlay |
| `share-button` | Mutate | `project_iri: iri`, `share_config: CBOR-LD` | Adds share button |
| `co-view-sync` | Mutate | `project_iri: iri`, `sync_config: CBOR-LD` | Configures co-viewing synchronisation |
| `social-moderation` | Mutate | `project_iri: iri`, `moderation_rules: CBOR-LD` | Configures social moderation rules |

#### `packaging` — HbbTV packaging tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `hbbtv-package` | Mutate | `project_iri: iri`, `package_config: CBOR-LD` | Creates HbbTV 2.x package |
| `ait-config` | Mutate | `package_iri: iri`, `ait: CBOR-LD` | Configures Application Information Table |
| `dvb-stream-bind` | Mutate | `package_iri: iri`, `stream_iri: iri` | Binds DVB stream to package |
| `drm-config` | Mutate | `package_iri: iri`, `drm_scheme: string`, `keys: CBOR-LD` | Configures DRM |
| `app-data-bundle` | Mutate | `package_iri: iri`, `app_data: CBOR-LD` | Bundles application data |
| `package-validate` | Query | `package_iri: iri` | Validates package against HbbTV 2.x spec |
| `package-export` | Mutate | `package_iri: iri`, `output_path: string` | Exports package for deployment |

#### `sync` — synchronisation tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `timeline-sync` | Mutate | `project_iri: iri`, `sync_source: string` | Sets timeline sync source |
| `event-sync` | Mutate | `project_iri: iri`, `event_iri: iri`, `sync_action: CBOR-LD` | Configures event-based sync |
| `wall-clock-sync` | Mutate | `project_iri: iri`, `ntp_server: string` | Configures wall-clock sync |
| `scte35-marker` | Mutate | `timeline_iri: iri`, `position: float`, `cue_type: string` | Inserts SCTE-35 ad marker |
| `sync-test` | Query | `project_iri: iri` | Tests sync accuracy |

#### `inspect` — inspection tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `package-inspector` | Query | `package_iri: iri` | Inspects HbbTV package contents |
| `stream-analyser` | Query | `stream_iri: iri` | Analyses stream — bitrate, codec, continuity |
| `device-emulator` | Query | `project_iri: iri`, `device_profile: string` | Emulates target device |
| `bandwidth-simulator` | Query | `project_iri: iri`, `bandwidth: float` | Simulates bandwidth conditions |
| `metadata-view` | Query | `asset_iri: iri` | Displays interactive asset metadata |

### 5.3 Hypermedia manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `interactive-timeline` | bottom | Trigger and overlay timeline |
| `preview-screen` | centre | Broadcast + overlay preview |
| `trigger-panel` | right | Trigger configuration |
| `social-panel` | left | Social layer configuration |
| `package-inspector` | right (bottom) | HbbTV package inspector |
| `device-emulator` | top-right | Device emulator |

---

## 6. Toolbox: `portals` (Web Portals, Worlds, Immersive Environments)

The `portals` toolbox is for creating web portals, virtual worlds, and immersive environments. It is the native equivalent of VRChat, Spatial, or WebXR worlds — but built on Vibe, CBOR-LD, and the context graph.

**Ontology:** [`qualia-ui/ontologies/portal-worlds.n3`](../ontologies/portal-worlds.n3)

### 6.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `portal-viewport` | content | missing | 3D viewport for world editing — terrain, objects, portals |
| `world-outliner` | panel | missing | World hierarchy — objects, portals, spawn points |
| `portal-properties` | panel | missing | Portal and world properties — IRI, capacity, physics |
| `asset-library` | panel | missing | Portal asset library — props, environments, avatars |
| `physics-inspector` | panel | missing | Physics body inspector — colliders, rigidbodies, triggers |
| `portal-preview` | content | missing | Preview mode — first-person walk-through |

### 6.2 Tool-chains

#### `world-building` — world creation and editing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-world` | Mutate | `name: string`, `world_iri: iri`, `environment: string` | Creates a new world |
| `terrain-edit` | Mutate | `world_iri: iri`, `terrain_data: CBOR-LD`, `brush: string` | Edits terrain heightmap |
| `skybox-set` | Mutate | `world_iri: iri`, `skybox_iri: iri` | Sets world skybox |
| `environment-light` | Mutate | `world_iri: iri`, `light_config: CBOR-LD` | Configures environment lighting |
| `spawn-point` | Mutate | `world_iri: iri`, `position: [x,y,z]`, `rotation: [x,y,z]` | Sets spawn point |
| `boundary-set` | Mutate | `world_iri: iri`, `boundary: CBOR-LD` | Sets world boundary |
| `world-save` | Mutate | `world_iri: iri` | Saves world state |
| `world-publish` | Mutate | `world_iri: iri`, `target_iri: iri` | Publishes world to portal server |

#### `objects` — world object placement

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `place-prop` | Mutate | `world_iri: iri`, `prop_iri: iri`, `position: [x,y,z]` | Places a prop in the world |
| `place-furniture` | Mutate | `world_iri: iri`, `furniture_iri: iri`, `position: [x,y,z]` | Places furniture |
| `interactive-object` | Mutate | `world_iri: iri`, `object_iri: iri`, `interaction: CBOR-LD` | Places an interactive object |
| `portal-anchor` | Mutate | `world_iri: iri`, `position: [x,y,z]`, `destination_iri: iri` | Places a portal anchor |
| `object-transform` | Mutate | `object_iri: iri`, `position: [x,y,z]`, `rotation: [x,y,z]`, `scale: [x,y,z]` | Transforms a placed object |
| `object-duplicate` | Mutate | `object_iri: iri` | Duplicates a placed object |
| `object-delete` | Mutate | `object_iri: iri` | Removes a placed object |

#### `portals` — portal link management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-portal` | Mutate | `world_iri: iri`, `name: string`, `destination_iri: iri` | Creates a portal link |
| `portal-link` | Mutate | `portal_iri: iri`, `destination_iri: iri` | Links portal to destination world |
| `portal-destination` | Query | `portal_iri: iri` | Queries portal destination |
| `portal-preview` | Query | `portal_iri: iri` | Previews portal destination |
| `portal-list` | Query | `world_iri: iri` | Lists all portals in a world |
| `portal-test` | Mutate | `portal_iri: iri` | Tests portal traversal |

#### `avatars` — avatar configuration

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `avatar-select` | Mutate | `world_iri: iri`, `avatar_iri: iri` | Sets default avatar for world |
| `avatar-customise` | Mutate | `avatar_iri: iri`, `customisation: CBOR-LD` | Customises avatar appearance |
| `controller-map` | Mutate | `world_iri: iri`, `controller_config: CBOR-LD` | Maps input controllers |
| `avatar-preview` | Query | `avatar_iri: iri` | Previews avatar |
| `avatar-spawn` | Mutate | `world_iri: iri`, `avatar_iri: iri`, `position: [x,y,z]` | Spawns avatar at position |

#### `physics` — physics body management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-collider` | Mutate | `object_iri: iri`, `collider_type: string`, `size: CBOR-LD` | Adds a collider |
| `add-rigidbody` | Mutate | `object_iri: iri`, `mass: float`, `drag: float`, `angular_drag: float` | Adds a rigidbody |
| `add-trigger` | Mutate | `object_iri: iri`, `trigger_event: string` | Adds a trigger volume |
| `physics-material` | Mutate | `object_iri: iri`, `friction: float`, `restitution: float` | Sets physics material |
| `physics-bake` | Mutate | `world_iri: iri` | Bakes physics scene for performance |

#### `inspect` — portal inspection

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `world-statistics` | Query | `world_iri: iri` | Displays world stats — object count, polygon count, portal count |
| `physics-inspector` | Query | `world_iri: iri` | Displays physics body list and properties |
| `portal-graph` | Query | `world_iri: iri` | Displays portal link graph |
| `player-list` | Query | `world_iri: iri` | Lists active players in world |
| `metadata-view` | Query | `asset_iri: iri` | Displays portal/world metadata |

### 6.3 Portal manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `portal-viewport` | centre | World editing viewport |
| `world-outliner` | top-right | World hierarchy |
| `portal-properties` | right | Portal and world properties |
| `asset-library` | left | Portal asset library |
| `physics-inspector` | bottom | Physics body inspector |
| `portal-preview` | tabbed | First-person preview |

---

## 7. Toolbox: `productions` (Events, DMX, Projection Mapping)

The `productions` toolbox is for creating and controlling live productions — events, shows, installations, and broadcasts. It manages DMX lighting, projection mapping, cue stacks, and show control.

**Ontology:** [`qualia-ui/ontologies/production-events.n3`](../ontologies/production-events.n3)

### 7.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `production-timeline` | content | missing | Show control timeline — cues, triggers, SMPTE |
| `dmx-controller` | content | missing | DMX universe controller — channels, fixtures, scenes |
| `projection-canvas` | content | missing | Projection mapping canvas — surfaces, content, calibration |
| `fixture-patch` | panel | missing | Fixture patch panel — universe, channel, address |
| `cue-stack` | panel | missing | Cue stack — cue list, sequence, fades |
| `preview-monitor` | content | missing | Production preview — lighting + projection render |

### 7.2 Tool-chains

#### `dmx` — DMX universe management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-universe` | Mutate | `production_iri: iri`, `universe_id: int` | Adds a DMX universe |
| `patch-fixture` | Mutate | `universe_iri: iri`, `fixture_iri: iri`, `channel: int` | Patches a fixture at a DMX address |
| `unpatch-fixture` | Mutate | `fixture_iri: iri` | Unpatches a fixture |
| `fixture-profile` | Mutate | `fixture_iri: iri`, `profile_iri: iri` | Assigns a fixture profile (channel map) |
| `channel-assign` | Mutate | `fixture_iri: iri`, `channel_map: CBOR-LD` | Assigns custom channel mapping |
| `dmx-monitor` | Query | `universe_iri: iri` | Displays DMX channel values in real-time |
| `dmx-scene` | Mutate | `universe_iri: iri`, `scene_data: CBOR-LD` | Saves a DMX scene |
| `dmx-cue` | Mutate | `cue_stack_iri: iri`, `scene_iri: iri`, `fade_time: float` | Creates a DMX cue from a scene |

#### `fixtures` — fixture placement and management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-moving-head` | Mutate | `production_iri: iri`, `position: [x,y,z]`, `profile: string` | Adds a moving head fixture |
| `add-led-par` | Mutate | `production_iri: iri`, `position: [x,y,z]`, `profile: string` | Adds an LED par fixture |
| `add-laser` | Mutate | `production_iri: iri`, `position: [x,y,z]`, `profile: string` | Adds a laser fixture |
| `add-strobe` | Mutate | `production_iri: iri`, `position: [x,y,z]`, `profile: string` | Adds a strobe fixture |
| `add-hazer` | Mutate | `production_iri: iri`, `position: [x,y,z]`, `profile: string` | Adds a haze machine |
| `fixture-position` | Mutate | `fixture_iri: iri`, `position: [x,y,z]`, `rotation: [x,y,z]` | Sets fixture position and rotation |

#### `lighting` — lighting control

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `colour-mix` | Mutate | `fixture_iris: [iri]`, `colour: [r,g,b]` or `colour: [c,m,y]` | Sets fixture colour mix |
| `beam-shape` | Mutate | `fixture_iris: [iri]`, `zoom: float`, `focus: float`, `iris: float` | Sets beam shape parameters |
| `gobo-select` | Mutate | `fixture_iris: [iri]`, `gobo_index: int`, `rotation: float` | Selects and rotates gobo |
| `position-set` | Mutate | `fixture_iris: [iri]`, `pan: float`, `tilt: float` | Sets pan/tilt position |
| `intensity-set` | Mutate | `fixture_iris: [iri]`, `intensity: float` | Sets fixture intensity (0.0–1.0) |
| `effect-run` | Mutate | `fixture_iris: [iri]`, `effect_type: string`, `speed: float`, `params: CBOR-LD` | Runs a lighting effect — chase, wave, random, ramp |

#### `projection` — projection mapping

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-surface` | Mutate | `production_iri: iri`, `surface_type: string`, `geometry: CBOR-LD` | Adds a projection surface — flat, curved, architectural, volumetric |
| `surface-map` | Mutate | `surface_iri: iri`, `mapping: CBOR-LD` | Maps content to surface geometry |
| `edge-blend` | Mutate | `surface_iri: iri`, `projector_count: int`, `blend_params: CBOR-LD` | Configures edge blending |
| `geometry-correct` | Mutate | `surface_iri: iri`, `correction: CBOR-LD` | Applies geometry correction (keystone, warp) |
| `calibration-point` | Mutate | `surface_iri: iri`, `point: [x, y]`, `reference: [x, y]` | Adds a calibration point |
| `projection-content` | Mutate | `surface_iri: iri`, `content_iri: iri` | Assigns content to projection surface |
| `projection-preview` | Query | `surface_iri: iri` | Previews projection mapping |

#### `cue-stack` — cue management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-cue` | Mutate | `cue_stack_iri: iri`, `cue_data: CBOR-LD`, `position: int` | Adds a cue to the stack |
| `edit-cue` | Mutate | `cue_iri: iri`, `cue_data: CBOR-LD` | Edits a cue |
| `cue-sequence` | Mutate | `cue_stack_iri: iri`, `sequence: [iri]` | Orders cues in sequence |
| `cue-trigger` | Mutate | `cue_iri: iri`, `trigger_type: string`, `trigger_data: CBOR-LD` | Sets cue trigger — manual, time, event, SMPTE |
| `cue-fade` | Mutate | `cue_iri: iri`, `fade_in: float`, `fade_out: float`, `fade_curve: string` | Sets cue fade parameters |
| `cue-go` | Mutate | `cue_stack_iri: iri` | Executes next cue in stack |

#### `show-control` — show control and sync

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `smpte-sync` | Mutate | `production_iri: iri`, `smpte_source: string`, `frame_rate: float` | Configures SMPTE timecode sync |
| `midi-timecode` | Mutate | `production_iri: iri`, `mtc_source: string` | Configures MIDI timecode sync |
| `art-net-config` | Mutate | `production_iri: iri`, `network: CBOR-LD`, `universes: [int]` | Configures Art-Net network |
| `osc-config` | Mutate | `production_iri: iri`, `osc_config: CBOR-LD` | Configures OSC (Open Sound Control) |
| `timeline-trigger` | Mutate | `production_iri: iri`, `timeline_iri: iri`, `trigger_map: CBOR-LD` | Maps timeline positions to triggers |

#### `inspect` — production inspection

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `dmx-monitor` | Query | `universe_iri: iri` | Displays DMX channel values |
| `fixture-report` | Query | `production_iri: iri` | Reports fixture status — patched, online, errors |
| `projection-analyser` | Query | `surface_iri: iri` | Analyses projection — brightness, coverage, overlap |
| `power-calculator` | Query | `production_iri: iri` | Calculates total power draw |
| `metadata-view` | Query | `asset_iri: iri` | Displays production metadata |

### 7.3 Production manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `production-timeline` | bottom | Show control timeline |
| `dmx-controller` | left | DMX universe controller |
| `projection-canvas` | centre | Projection mapping canvas |
| `fixture-patch` | top-right | Fixture patch panel |
| `cue-stack` | right | Cue stack |
| `preview-monitor` | top-centre | Production preview |

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
| [`TOOLBOX_HYPERMEDIA_SPEC.md`](TOOLBOX_HYPERMEDIA_SPEC.md) | Part 1 — image, audio, video toolboxes |
| [`qualia-ui/ontologies/hypermedia.n3`](../ontologies/hypermedia.n3) | Core hypermedia ontology — asset types, provenance, cross-domain references |
| [`qualia-ui/ontologies/spatial-3d.n3`](../ontologies/spatial-3d.n3) | 3D domain ontology — meshes, rigs, animation, narratives |
| [`qualia-ui/ontologies/interactive-hypermedia.n3`](../ontologies/interactive-hypermedia.n3) | Interactive domain ontology — HbbTV, 2nd screen, social |
| [`qualia-ui/ontologies/portal-worlds.n3`](../ontologies/portal-worlds.n3) | Portal domain ontology — worlds, immersive environments |
| [`qualia-ui/ontologies/production-events.n3`](../ontologies/production-events.n3) | Production domain ontology — DMX, projection, events |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — content/panel/widget kinds |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | UI concepts — manifolds, containers, presentation |

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
