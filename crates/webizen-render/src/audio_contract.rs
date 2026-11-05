//! Dual-Mode Audio Data Contract for Webizen 10D Architecture.
//!
//! Re-exports the canonical audio contract types from
//! `etalon_render::webizen_core::audio_contract`.

pub use etalon_render::webizen_core::audio_contract::*;

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
