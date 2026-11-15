// SPDX-License-Identifier: MIT OR Apache-2.0
//! Re-exports spherical atmospheric limb & aerial perspective, Gerstner waves, hypsometric tinting,
//! and GPU picking from `etalon_render::geo_viewport::sky_limb`.

pub use etalon_render::geo_viewport::sky_limb::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[inline]
    fn norm3(a: [f32; 3]) -> f32 {
        (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
    }

    #[test]
    fn test_atmosphere_params_size() {
        assert_eq!(core::mem::size_of::<AtmosphereParams>(), 64);
    }

    #[test]
    fn test_sky_limb_and_aerial_perspective() {
        let params = AtmosphereParams {
            planet_radius_m: 6_371_000.0,
            atmosphere_top_m: 100_000.0,
            rayleigh_scale_h_m: 8_000.0,
            mie_scale_h_m: 1_200.0,
            beta_rayleigh: [5.8e-6, 13.5e-6, 33.1e-6],
            mie_g: 0.76,
            beta_mie: [21.0e-6, 21.0e-6, 21.0e-6],
            sun_intensity: 20.0,
            sun_dir: [0.0, 1.0, 0.0],
            ozone_peak_h_m: 25_000.0,
        };

        let (zenith_rad, zenith_trans) =
            evaluate_sky_limb_and_aerial_perspective(&params, 10.0, [0.0, 1.0, 0.0], 100_000.0, 16);
        assert!(
            zenith_rad[2] > zenith_rad[0],
            "Zenith scattered light must be blue dominant"
        );
        assert!(zenith_trans[0] > 0.0 && zenith_trans[0] <= 1.0);

        let mut sunset_params = params;
        sunset_params.sun_dir = [1.0, 0.0, 0.0];
        let (_, sunset_trans) = evaluate_sky_limb_and_aerial_perspective(
            &sunset_params,
            10.0,
            [1.0, 0.05, 0.0],
            80_000.0,
            16,
        );
        assert!(
            sunset_trans[0] > sunset_trans[2],
            "Sunset horizon transmittance preserves red over blue"
        );

        let (_, near_trans) =
            evaluate_sky_limb_and_aerial_perspective(&params, 10.0, [1.0, 0.0, 0.0], 1_000.0, 8);
        let (_, far_trans) =
            evaluate_sky_limb_and_aerial_perspective(&params, 10.0, [1.0, 0.0, 0.0], 50_000.0, 8);
        assert!(
            near_trans[0] > far_trans[0],
            "Aerial perspective transmittance decays over distance"
        );
    }

    #[test]
    fn test_gerstner_waves_and_foam() {
        let sample = evaluate_water_wave_and_foam([15.0, 25.0], 2.0, 50.0, [1.0, 0.0], 1.8, 0.6);
        let n_len = norm3(sample.normal_xyz);
        assert!((n_len - 1.0).abs() < 1e-4, "Wave normal must be normalized");

        let grazing = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 50.0, [1.0, 0.0], 1.0, 0.0);
        assert!(
            (grazing.fresnel_reflectance - 1.0).abs() < 1e-3,
            "Grazing Fresnel must approach 1.0"
        );
        let normal_inc = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 50.0, [1.0, 0.0], 1.0, 1.0);
        assert!(
            (normal_inc.fresnel_reflectance - 0.02).abs() < 1e-3,
            "Normal incidence must match R0"
        );

        let shallow = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 0.5, [1.0, 0.0], 2.0, 0.5);
        let deep = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 100.0, [1.0, 0.0], 2.0, 0.5);
        assert!(
            shallow.shore_foam_weight > 0.4,
            "Shallow breaking water generates foam"
        );
        assert!(
            deep.shore_foam_weight < 0.05,
            "Deep water has negligible shore foam"
        );
    }

    #[test]
    fn test_hypsometric_contours_and_picking() {
        let dem = [50.0, 400.0, 1000.0, 1000.0];
        let slope = [0.05, 0.05, 0.05, 0.05];
        let mut out = [[0u8; 4]; 4];
        shade_hypsometric_and_contours_into(&dem, &slope, 0.0, 2000.0, 100.0, 5, &mut out).unwrap();
        assert_eq!(out[0][3], 255);
        assert!(out[2][0] > 0);

        let mut id_buf = [0u64; 25];
        let mut depth_buf = [1.0f32; 25];
        id_buf[2 * 5 + 2] = (12u64 << 32) | 101u64;
        depth_buf[2 * 5 + 2] = 0.6;
        id_buf[2 * 5 + 3] = (14u64 << 32) | 202u64;
        depth_buf[2 * 5 + 3] = 0.3;

        let hit = resolve_gpu_picking_id_aperture(&id_buf, &depth_buf, 5, 5, 2, 2, 1);
        assert_eq!(
            hit,
            Some((202, 14, 3, 2, 0.3)),
            "Must pick closest depth within aperture"
        );
        let miss = resolve_gpu_picking_id_aperture(&id_buf, &depth_buf, 5, 5, 0, 0, 0);
        assert_eq!(miss, None);
    }
}
