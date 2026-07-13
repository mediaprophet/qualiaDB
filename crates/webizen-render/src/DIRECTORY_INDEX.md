---
created: 2026-06-30
updated: 2026-07-13
update_scope: Minor
---

# src Index

## Functionality Overview
Comprehensive index of functionality for `src`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[math](math/DIRECTORY_INDEX.md)`
- 📁 `[pipeline](pipeline/DIRECTORY_INDEX.md)`
- 📁 `[shaders](shaders/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `audio_contract.rs`
  - `enum AudioMode`
  - `struct SpectralParams`
  - `impl Default`
  - `fn default`
  - `struct GenerativeAudioSheet`
  - `struct PCMAudioSheet`
  - `enum AudioSpectralSheet`
  - `fn mode`
  - `fn from_tensor`
  - `fn map_tensor_to_spectral`
  - `struct AudioTrack`
  - `struct AudioScene`
  - `fn test_spectral_params_default`
  - `fn test_generative_audio_sheet_default`
  - `fn test_audio_sheet_from_tensor`
  - *(...and 3 more)*
- 📄 `lib.rs`
  - `fn test_motor_size`
  - `fn test_render_quin_size`
  - `fn test_motor_encoder`
  - `fn test_render_quin_creation`
  - `fn test_shaders_exist`
  - `fn offscreen_clear_reads_back_solid_color`
  - `fn offscreen_midtone_passthrough_is_linear`
  - `fn offscreen_encodes_valid_png`
  - `fn preview_data_uri_is_well_formed`
  - `fn render_scene_with_nodes`
  - `fn render_scene_with_edges`
  - `fn render_scene_with_faces`
  - `fn render_scene_mixed`
  - `fn render_scene_data_uri_is_well_formed`
- 📄 `scene.rs`
  - `struct SceneCamera`
  - `impl Default`
  - `fn default`
  - `struct SceneNode`
  - `struct SceneEdge`
  - `struct RenderScene`
  - `fn one`
  - `fn default_background`
  - `fn default_background_marker`
- 📄 `scene_contract.rs`
  - `struct ScenePoint`
  - `struct SceneNode`
  - `impl Default`
  - `fn default`
  - `struct SceneEdge`
  - `struct SceneFace`
  - `struct SceneCamera`
  - `enum EpistemicState`
  - `struct Tensor10DProjection`
  - `impl Tensor10DProjection`
  - `fn spectral_to_color`
  - `fn spectral_to_cie_xyz`
  - `fn cie_xyz_to_srgb`
  - `fn amplitude_to_opacity`
  - `fn has_hidden_metadata`
  - *(...and 21 more)*
- 📄 `telemetry.rs`
  - `struct SystemTelemetry`
  - `impl SystemTelemetry`
  - `fn new`
  - `fn update_memory_pressure`
  - `fn update_network_ripple`
  - `fn update_llm_heat`
  - `fn update_quantum_activity`
  - `fn update_spectral_shift`
  - `fn update_temporal_pulse`
  - `fn update_epistemic_density`
  - `fn update_manifold_pressure`
  - `fn set_memory_pressure`
  - `fn set_network_ripple`
  - `fn set_baking_crystallization`
  - `fn set_logic_flashes`
  - *(...and 16 more)*
- 📄 `volumetric.rs`
  - `struct VolumetricRenderer`
  - `impl VolumetricRenderer`
  - `fn new_offscreen`
  - `fn upload_tensor_buffer`
  - `fn upload_mesh`
  - `fn upload_mesh_colored`
  - `fn set_camera`
  - `fn render`
  - `fn required_rgba8_bytes`
  - `fn read_rgba8_into`
  - `fn resize`
  - `fn render_scene_rgba8_into`
  - `fn render_scene_png`
  - `fn node_tensor`
  - `fn scene_mesh`
  - *(...and 4 more)*
- 📄 `wgpu_renderer.rs`
  - `struct Vec3`
  - `struct ScreenPoint`
  - `struct Camera`
  - `impl Default`
  - `fn default`
  - `impl Camera`
  - `fn project`
  - `fn orbit`
  - `fn zoom`
  - `fn pan`
  - `struct ScreenVertex`
  - `enum RenderTarget`
  - `enum Frame`
  - `impl Frame`
  - `fn view`
  - *(...and 42 more)*

## Changelog
- **2026-07-13**: Recorded the wgpu 30 renderer migration: color-space selection, explicit queue presentation, optional vertex layouts, and a validated projector fragment stage.
- **2026-06-30**: Automated full index generation, extracting code definitions.
