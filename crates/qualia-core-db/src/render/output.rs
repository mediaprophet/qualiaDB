//! Versioned scene-linear output transforms and manual exposure controls.

/// The renderer's first shared SDR view transform: scene-linear Rec.709 to display-linear Rec.709.
///
/// Coefficients and operations follow the Khronos PBR Neutral reference algorithm. The
/// destination is still linear here; an sRGB surface performs the display encoding exactly once.
pub const SDR_OUTPUT_TRANSFORM_PBR_NEUTRAL_V1: u32 = 1;

pub const DEFAULT_HDR_EXPOSURE_EV: f32 = 0.070_389_33;
pub const MIN_HDR_EXPOSURE_EV: f32 = -8.0;
pub const MAX_HDR_EXPOSURE_EV: f32 = 8.0;
pub const MIN_WHITE_BALANCE_EV: f32 = -1.0;
pub const MAX_WHITE_BALANCE_EV: f32 = 1.0;

/// Neutral gains for the renderer's Rec.709 temperature/tint control.
pub const DEFAULT_WHITE_BALANCE_GAINS: [f32; 3] = [1.0; 3];

/// Convert bounded exposure compensation, expressed in stops, to a linear scale.
pub fn exposure_scale_from_ev(ev: f32) -> Option<f32> {
    ev.is_finite()
        .then(|| 2.0_f32.powf(ev.clamp(MIN_HDR_EXPOSURE_EV, MAX_HDR_EXPOSURE_EV)))
}

/// Convert warm/cool and green/magenta bias in stops to diagonal Rec.709 gains.
///
/// This control is deliberately a bounded, linear-light camera balance adjustment rather than
/// a Kelvin estimator. Compute the gains once when settings change, then multiply per pixel.
pub fn white_balance_gains_from_stops(temperature_ev: f32, tint_ev: f32) -> Option<[f32; 3]> {
    if !temperature_ev.is_finite() || !tint_ev.is_finite() {
        return None;
    }
    let temperature = temperature_ev.clamp(MIN_WHITE_BALANCE_EV, MAX_WHITE_BALANCE_EV);
    let tint = tint_ev.clamp(MIN_WHITE_BALANCE_EV, MAX_WHITE_BALANCE_EV);
    let magenta_counter = -0.5 * tint;
    Some([
        2.0_f32.powf(temperature + magenta_counter),
        2.0_f32.powf(tint),
        2.0_f32.powf(-temperature + magenta_counter),
    ])
}

/// Map non-negative scene-linear Rec.709 through the versioned PBR Neutral SDR transform.
///
/// Returns display-linear Rec.709 in [0, 1]. Keep the matching implementation in
/// `shaders/viewport/bloom.wgsl` operation-for-operation aligned with this CPU oracle.
pub fn pbr_neutral_v1(color: [f32; 3]) -> [f32; 3] {
    let mut color = color.map(|channel| channel.max(0.0));
    let min_channel = color[0].min(color[1]).min(color[2]);
    let offset = if min_channel < 0.08 {
        min_channel - 6.25 * min_channel * min_channel
    } else {
        0.04
    };
    for channel in &mut color {
        *channel -= offset;
    }

    let peak = color[0].max(color[1]).max(color[2]);
    let start_compression = 0.8 - 0.04;
    if peak < start_compression {
        return color;
    }

    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    for channel in &mut color {
        *channel *= new_peak / peak;
    }
    let desaturation = 0.15;
    let gray_mix = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    color.map(|channel| channel * (1.0 - gray_mix) + new_peak * gray_mix)
}

/// sRGB display encoding for linear UNORM surfaces that lack an sRGB attachment format.
pub fn linear_rec709_to_srgb(color: [f32; 3]) -> [f32; 3] {
    color.map(|channel| {
        let channel = channel.clamp(0.0, 1.0);
        if channel <= 0.003_130_8 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        }
    })
}

/// Apply the v1 SDR output transform and encode the result for an RGBA8 sRGB capture.
pub fn pbr_neutral_v1_srgb(color: [f32; 3]) -> [f32; 3] {
    linear_rec709_to_srgb(pbr_neutral_v1(color))
}

/// Apply exposure and pre-tone-map white balance once before the versioned SDR output transform.
pub fn pbr_neutral_v1_srgb_with_controls(
    color: [f32; 3],
    exposure_ev: f32,
    white_balance_gains: [f32; 3],
) -> Option<[f32; 3]> {
    let exposure = exposure_scale_from_ev(exposure_ev)?;
    if white_balance_gains
        .iter()
        .any(|gain| !gain.is_finite() || *gain < 0.0)
    {
        return None;
    }
    Some(linear_rec709_to_srgb(pbr_neutral_v1(std::array::from_fn(
        |index| color[index] * exposure * white_balance_gains[index],
    ))))
}

#[cfg(test)]
mod tests {
    use super::{
        exposure_scale_from_ev, linear_rec709_to_srgb, pbr_neutral_v1, pbr_neutral_v1_srgb,
        pbr_neutral_v1_srgb_with_controls, white_balance_gains_from_stops, DEFAULT_HDR_EXPOSURE_EV,
        DEFAULT_WHITE_BALANCE_GAINS, SDR_OUTPUT_TRANSFORM_PBR_NEUTRAL_V1,
    };

    #[test]
    fn exposure_stops_map_to_linear_scale_and_clamp() {
        assert_eq!(exposure_scale_from_ev(0.0), Some(1.0));
        assert_eq!(exposure_scale_from_ev(1.0), Some(2.0));
        assert_eq!(exposure_scale_from_ev(-1.0), Some(0.5));
        assert_eq!(exposure_scale_from_ev(80.0), Some(256.0));
        assert_eq!(exposure_scale_from_ev(-80.0), Some(1.0 / 256.0));
        assert!(exposure_scale_from_ev(f32::NAN).is_none());
        assert!(exposure_scale_from_ev(f32::INFINITY).is_none());
    }

    #[test]
    fn default_ev_preserves_the_previous_hdr_exposure_scale() {
        let scale = exposure_scale_from_ev(DEFAULT_HDR_EXPOSURE_EV).unwrap();
        assert!((scale - 1.05).abs() < 0.0001);
    }

    #[test]
    fn output_transform_applies_ev_once_before_tone_mapping() {
        let scene = [0.25, 0.12, 0.06];
        let one_stop = pbr_neutral_v1_srgb_with_controls(scene, 1.0, [1.0; 3]).unwrap();
        let correctly_exposed = pbr_neutral_v1_srgb([0.5, 0.24, 0.12]);
        assert_eq!(one_stop, correctly_exposed);
        let double_exposed = pbr_neutral_v1_srgb([1.0, 0.48, 0.24]);
        assert_ne!(one_stop, double_exposed);
    }

    #[test]
    fn white_balance_zero_is_identity_and_controls_are_bounded() {
        assert_eq!(white_balance_gains_from_stops(0.0, 0.0), Some([1.0; 3]));
        assert_eq!(DEFAULT_WHITE_BALANCE_GAINS, [1.0; 3]);
        assert!(pbr_neutral_v1_srgb_with_controls([0.2; 3], 0.0, [f32::NAN, 1.0, 1.0]).is_none());
        assert_eq!(
            white_balance_gains_from_stops(40.0, -40.0),
            white_balance_gains_from_stops(1.0, -1.0)
        );
        assert!(white_balance_gains_from_stops(f32::NAN, 0.0).is_none());
        let warm = white_balance_gains_from_stops(1.0, 0.0).unwrap();
        assert!(warm[0] > 1.0 && warm[1] == 1.0 && warm[2] < 1.0);
        let green_bias = white_balance_gains_from_stops(0.0, 1.0).unwrap();
        assert!(green_bias[1] > green_bias[0] && green_bias[0] == green_bias[2]);
    }

    #[test]
    fn pbr_neutral_v1_preserves_sdr_and_compresses_highlights() {
        assert_eq!(SDR_OUTPUT_TRANSFORM_PBR_NEUTRAL_V1, 1);
        let mid_gray = pbr_neutral_v1([0.18, 0.18, 0.18]);
        assert!(mid_gray
            .iter()
            .all(|channel| (*channel - 0.14).abs() < 1e-6));
        let mapped = pbr_neutral_v1([4.0, 1.0, 0.25]);
        assert!(mapped.iter().all(|channel| (0.0..=1.0).contains(channel)));
        assert!(mapped[0] < 1.0 && mapped[0] > mapped[1] && mapped[1] > mapped[2]);
    }

    #[test]
    fn pbr_neutral_v1_maps_neutral_highlights_to_neutral_white() {
        let mapped = pbr_neutral_v1([16.0, 16.0, 16.0]);
        assert!(mapped.iter().all(|channel| (0.99..1.0).contains(channel)));
    }

    #[test]
    fn pbr_neutral_v1_srgb_matches_grey_saturated_and_highlight_ramp_goldens() {
        let cases = [
            ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
            ([0.18, 0.18, 0.18], [0.410_020_9, 0.410_020_9, 0.410_020_9]),
            ([0.5, 0.5, 0.5], [0.708_369_7, 0.708_369_7, 0.708_369_7]),
            ([1.0, 0.1, 0.02], [0.943_123_2, 0.324_910_52, 0.133_482_68]),
            ([0.02, 0.4, 3.0], [0.513_250_2, 0.603_086_35, 0.989_646_73]),
            ([16.0, 16.0, 16.0], [0.998_358_3, 0.998_358_3, 0.998_358_3]),
        ];
        for (scene_linear, expected_srgb) in cases {
            let actual = pbr_neutral_v1_srgb(scene_linear);
            for (actual, expected) in actual.into_iter().zip(expected_srgb) {
                assert!(
                    (actual - expected).abs() <= 2e-6,
                    "input {scene_linear:?}: expected {expected}, got {actual}"
                );
            }
        }
    }

    #[test]
    fn linear_fallback_uses_the_srgb_transfer_function_once() {
        let encoded = linear_rec709_to_srgb([0.0, 0.003_130_8, 1.0]);
        assert_eq!(encoded[0], 0.0);
        assert!((encoded[1] - 0.040_449_936).abs() < 1e-6);
        assert!((encoded[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn default_scene_clear_maps_to_the_versioned_sdr_capture_value() {
        let mapped = pbr_neutral_v1_srgb_with_controls(
            [0.03, 0.05, 0.08],
            DEFAULT_HDR_EXPOSURE_EV,
            DEFAULT_WHITE_BALANCE_GAINS,
        )
        .unwrap();
        let rgba8 = mapped.map(|channel| (channel * 255.0).round() as u8);
        assert_eq!(rgba8, [18, 46, 69]);
    }
}
