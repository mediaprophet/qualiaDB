/// Dual-Mode Audio Data Contract for Webizen 10D Architecture
///
/// Supports both generative spectral synthesis (Mode A) and legacy PCM passthrough (Mode B)
/// while maintaining zero-heap compliance and zero-copy transport.
///
/// Zero-Heap Considerations:
/// - Stack-allocated enums and primitive types
/// - References to byte buffers (no heap allocation in view)
/// - Binary IPC optimization using TensorBufferView pattern
use serde::{Deserialize, Serialize};

/// Audio rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioMode {
    /// Mode A: Generative (Tensor Route)
    /// Delivers spectral parameters for real-time synthesis by AudioWorklet
    /// Extremely low bandwidth, infinite resolution
    Generative,

    /// Mode B: PCM Passthrough (Legacy Route)
    /// Delivers raw byte streams for zero-copy playback of heavy assets
    /// Used for legacy media, archival recordings, DCI/IPTV integration
    PCMPassthrough,
}

/// Spectral parameters for generative synthesis (Mode A)
///
/// Maps directly to 10D tensor dimensions for zero-copy transport
///
/// Zero-Heap Consideration: Clone type, heap-allocated Vec for spectral data
/// (Serde doesn't support [f32; 64] serialization, using Vec<f32> instead)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralParams {
    /// σ (Spectral Signature): Frequency bins
    pub sigma: Vec<f32>,
    /// α (Amplitude): Gain staging and volume scaling
    pub alpha: f32,
    /// μ (Modulation): Phase data and oscillator manipulation
    pub mu: f32,
}

impl Default for SpectralParams {
    fn default() -> Self {
        Self {
            sigma: vec![0.0; 64],
            alpha: 1.0,
            mu: 0.0,
        }
    }
}

/// Audio sheet for generative synthesis (Mode A)
///
/// Contains spectral parameters for real-time AudioWorklet synthesis
///
/// Zero-Heap Consideration: Clone type (not Copy due to Vec<f32> in SpectralParams)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeAudioSheet {
    /// Spectral parameters for synthesis
    pub spectral: SpectralParams,
    /// Spatial positioning (x, y, z) for binaural soundfield
    pub position: [f32; 3],
    /// Topological class (v) for track separation
    pub track_index: u8,
    /// Manifold index (w) for environment (acoustic properties)
    pub manifold_index: u8,
}

impl Default for GenerativeAudioSheet {
    fn default() -> Self {
        Self {
            spectral: SpectralParams::default(),
            position: [0.0, 0.0, 0.0],
            track_index: 0,
            manifold_index: 0,
        }
    }
}

/// Audio sheet for PCM passthrough (Mode B)
///
/// Acts as a pointer to raw byte streams for zero-copy playback
///
/// Zero-Heap Consideration: Copy type, references byte buffer (no heap allocation)
#[derive(Debug, Clone, Copy)]
pub struct PCMAudioSheet<'a> {
    /// Raw PCM byte data reference (zero-copy)
    pub pcm_data: &'a [u8],
    /// Sample rate in Hz (e.g., 48000 for DCI standard)
    pub sample_rate: u32,
    /// Bit depth (e.g., 24 for DCI standard)
    pub bit_depth: u8,
    /// Number of channels (e.g., 6 for 5.1 surround)
    pub channels: u8,
}

/// Dual-mode Audio Spectral Sheet
///
/// Supports both generative synthesis and legacy PCM passthrough
///
/// Zero-Heap Consideration: Enum with Copy variants where possible
/// PCM variant uses lifetime to avoid heap allocation
#[derive(Debug, Clone)]
pub enum AudioSpectralSheet<'a> {
    /// Mode A: Generative synthesis from 10D tensor parameters
    Generative(GenerativeAudioSheet),

    /// Mode B: PCM passthrough for legacy media
    PCMPassthrough(PCMAudioSheet<'a>),
}

impl<'a> AudioSpectralSheet<'a> {
    /// Get the audio mode
    #[inline]
    pub fn mode(&self) -> AudioMode {
        match self {
            AudioSpectralSheet::Generative(_) => AudioMode::Generative,
            AudioSpectralSheet::PCMPassthrough(_) => AudioMode::PCMPassthrough,
        }
    }

    /// Create generative audio sheet from 10D tensor
    ///
    /// Zero-Heap Consideration: Stack-allocated conversion
    pub fn from_tensor(
        sigma: [f64; 10],
        alpha: f64,
        mu: f64,
        position: [f64; 3],
        track_index: u8,
        manifold_index: u8,
    ) -> Self {
        // Map 10D tensor sigma to 64-bin spectral signature
        let spectral_sigma = Self::map_tensor_to_spectral(sigma);

        let sheet = GenerativeAudioSheet {
            spectral: SpectralParams {
                sigma: spectral_sigma,
                alpha: alpha as f32,
                mu: mu as f32,
            },
            position: [position[0] as f32, position[1] as f32, position[2] as f32],
            track_index,
            manifold_index,
        };

        AudioSpectralSheet::Generative(sheet)
    }

    /// Map 10D tensor sigma to 64-bin spectral signature
    ///
    /// Zero-Heap Consideration: Stack-allocated array operations, returns Vec<f32> for serde compatibility
    fn map_tensor_to_spectral(tensor_sigma: [f64; 10]) -> Vec<f32> {
        let mut spectral = vec![0.0f32; 64];

        // Simple mapping: interpolate 10 tensor values to 64 frequency bins
        for i in 0..64 {
            let t_idx = (i as f64 / 64.0 * 10.0) as usize;
            let t_idx_next = (t_idx + 1).min(9);
            let t = (i as f64 / 64.0 * 10.0) - t_idx as f64;

            spectral[i] = (tensor_sigma[t_idx] * (1.0 - t) + tensor_sigma[t_idx_next] * t) as f32;
        }

        spectral
    }
}

/// Audio track information for multi-track production
///
/// Zero-Heap Consideration: Clone type (not Copy due to String field)
/// Heap-allocated track_name is optional and only used for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    /// Track identifier (binary index from BinaryNodeRegistry)
    pub track_index: u64,
    /// Track name (optional, heap-allocated only when needed for display)
    pub track_name: Option<String>,
    /// Whether track is muted
    pub muted: bool,
    /// Track volume (0.0..1.0)
    pub volume: f32,
    /// Pan position (-1.0 left, 0.0 center, 1.0 right)
    pub pan: f32,
}

impl Default for AudioTrack {
    fn default() -> Self {
        Self {
            track_index: 0,
            track_name: None,
            muted: false,
            volume: 1.0,
            pan: 0.0,
        }
    }
}

/// Audio scene for multi-track production
///
/// Represents a complete audio session with multiple tracks
///
/// Zero-Heap Consideration: Vec<T> is heap-allocated but necessary for
/// dynamic track management. The per-track data is stack-allocated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioScene {
    /// Audio tracks in the scene
    pub tracks: Vec<AudioTrack>,
    /// Temporal position (seconds)
    pub temporal_position: f64,
    /// Playback state
    pub is_playing: bool,
    /// Master volume (0.0..1.0)
    pub master_volume: f32,
}

impl Default for AudioScene {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            temporal_position: 0.0,
            is_playing: false,
            master_volume: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_params_default() {
        let params = SpectralParams::default();
        assert_eq!(params.sigma.len(), 64);
        assert_eq!(params.alpha, 1.0);
        assert_eq!(params.mu, 0.0);
    }

    #[test]
    fn test_generative_audio_sheet_default() {
        let sheet = GenerativeAudioSheet::default();
        assert_eq!(sheet.position.len(), 3);
        assert_eq!(sheet.track_index, 0);
        assert_eq!(sheet.manifold_index, 0);
    }

    #[test]
    fn test_audio_sheet_from_tensor() {
        let sigma = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let sheet = AudioSpectralSheet::from_tensor(sigma, 0.75, 0.5, [1.0, 2.0, 3.0], 1, 2);

        match sheet {
            AudioSpectralSheet::Generative(gen) => {
                assert_eq!(gen.spectral.alpha, 0.75);
                assert_eq!(gen.spectral.mu, 0.5);
                assert_eq!(gen.position, [1.0, 2.0, 3.0]);
                assert_eq!(gen.track_index, 1);
                assert_eq!(gen.manifold_index, 2);
            }
            _ => panic!("Expected Generative mode"),
        }
    }

    #[test]
    fn test_audio_mode_detection() {
        let gen_sheet = AudioSpectralSheet::Generative(GenerativeAudioSheet::default());
        assert_eq!(gen_sheet.mode(), AudioMode::Generative);

        let pcm_data = [0u8; 1024];
        let pcm_sheet = AudioSpectralSheet::PCMPassthrough(PCMAudioSheet {
            pcm_data: &pcm_data,
            sample_rate: 48000,
            bit_depth: 24,
            channels: 6,
        });
        assert_eq!(pcm_sheet.mode(), AudioMode::PCMPassthrough);
    }

    #[test]
    fn test_audio_track_default() {
        let track = AudioTrack::default();
        assert_eq!(track.track_index, 0);
        assert!(!track.muted);
        assert_eq!(track.volume, 1.0);
        assert_eq!(track.pan, 0.0);
    }

    #[test]
    fn test_audio_scene_default() {
        let scene = AudioScene::default();
        assert!(scene.tracks.is_empty());
        assert_eq!(scene.temporal_position, 0.0);
        assert!(!scene.is_playing);
        assert_eq!(scene.master_volume, 1.0);
    }
}
