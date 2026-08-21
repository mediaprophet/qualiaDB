//! U3 AcousticPlane — spectral-first + symbolic sonic tokens.

pub mod acoustic_plane;
pub mod acoustic_sab;
pub mod audio_sidecar_link;
pub mod audio_spectral_sheet;
pub mod cqt_bake;
/// N6: Traditional DAW DSP — oscillator, ADSR, filter, LFO, effects, MIDI, transport, meters.
pub mod dsp;
pub mod dsp_kernel;
pub mod hrtf;
/// AU-CORE-DSP — real inverse STFT (WOLA resynthesis; reuses `qualia-audio`). Native only.
#[cfg(not(target_arch = "wasm32"))]
pub mod istft;
pub mod stft;
pub mod stft_bake;
/// P7.5 — Time-frequency surface view over STFT/CQT rasters.
pub mod tf_surface;
/// P7.6 — Audio edits as geometric surface operations.
pub mod tf_surface_edit;

pub use acoustic_plane::{
    acoustic_enabled_for_mode, acoustic_params_from_tensor, drain_sonic_tokens, pop_sonic_token,
    push_sonic_token, sonify_tensor_node, AcousticParams, AcousticUniform, SonicTokenRing,
    SONIC_RING_CAP,
};
pub use acoustic_sab::{
    init_acoustic_sab, push_token_to_sab, read_uniform_from_sab, write_uniform_to_sab,
    write_uniform_to_sab_with_mirror, AcousticSabHeader, ACOUSTIC_SAB_BYTES, ACOUSTIC_SAB_MAGIC,
};
pub use audio_sidecar_link::{
    bake_audio_sidecar_into, compile_spectral_sheet_quin, enrich_preview_from_sidecar,
    format_sidecar_relpath, link_tensor_audio_sidecar, sidecar_content_hash, SidecarBakeKind,
};
pub use audio_spectral_sheet::{
    parse_sidecar_header, AudioSpectralSheetView, AudioSpectralSidecarHeader,
    SPECTRAL_PREVIEW_BINS, SPECTRAL_SIDECAR_MAGIC,
};
pub use cqt_bake::{bake_cqt_sidecar_from_preview, bake_cqt_sidecar_from_samples, forward_cqt};
pub use dsp_kernel::{epistemic_fm_index, epistemic_temperature_from_q, ParametricVoiceState};
pub use hrtf::{
    binaural_from_position, binaural_render, convolve_fir, set_hrtf_profile, synthesize_hrir,
    BinauralGains, HrtfProfile,
};
pub use stft::{bake_stft_sidecar_from_samples, forward_stft, stft_magnitudes};
pub use stft_bake::{bake_stft_sidecar_from_preview, bake_tensor_stft_sidecar, StftBakeError};
