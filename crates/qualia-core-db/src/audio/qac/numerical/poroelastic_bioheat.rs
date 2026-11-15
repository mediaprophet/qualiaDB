//! Biot Poroelastic Dispersion, 21-Constant Anisotropic Christoffel Solver, Kouyoumjian-Pathak
//! UTD Wedge Diffraction, ISO 532-1 (Zwicker) Sone Loudness, and AIUM/NEMA UD 3 Pennes Bioheat TI.
//!
//! Re-exports the `#![no_std]` continuum/wave and psychoacoustic/bioheat kernels from
//! `etalon_signal::poroelastic_diffraction` and `etalon_signal::bioheat_sones` (`C01` ownership),
//! closing R03 Gap #3 across QAC v1 continuum and ultrasound/perceptual domains.

pub use etalon_signal::bioheat_sones::{
    compute_tib_bone_at_focus, compute_tic_cranial_bone, compute_tis_soft_tissue,
    compute_zwicker_specific_and_total_loudness_into, phons_to_sones_iso532,
    solve_pennes_bioheat_steady_state, sones_to_phons_iso532, spread_zwicker_bark_excitation_into,
    PennesBioheatParams, PennesBioheatResult, ThermalIndexEstimate, ThermalIndexKind,
    ZwickerLoudnessReport, TIB_MODEL_URI, TIC_MODEL_URI, TIS_MODEL_URI,
    ZWICKER_24_BARK_CENTER_HZ, ZWICKER_24_BARK_THRESHOLD_IN_QUIET_DB, ZWICKER_BARK_BANDS,
};
pub use etalon_signal::poroelastic_diffraction::{
    compute_utd_wedge_diffraction, solve_biot_poroelastic_dispersion,
    solve_christoffel_anisotropic_21, utd_fresnel_transition_f, validate_voigt_stiffness_6x6,
    BiotPoroelasticParams, BiotWaveDispersion, ChristoffelSolution,
    Complex64 as PoroelasticComplex64,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poroelastic_christoffel_utd_zwicker_and_pennes_ti_delegation() {
        let biot = BiotPoroelasticParams {
            porosity: 0.22,
            rho_fluid_kg_m3: 1000.0,
            rho_solid_kg_m3: 2650.0,
            tortuosity_inf: 1.8,
            permeability_m2: 1e-12,
            viscosity_pa_s: 1e-3,
            biot_willis_alpha: 0.78,
            biot_modulus_m_pa: 1.1e10,
            frame_bulk_modulus_pa: 1.4e10,
            frame_shear_modulus_pa: 9.5e9,
        };
        let disp = solve_biot_poroelastic_dispersion(&biot, 1e4).unwrap();
        assert!(disp.c_p1_m_s > disp.c_s_m_s && disp.c_s_m_s > disp.c_p2_m_s);

        let mut bands = [-100.0_f64; ZWICKER_BARK_BANDS];
        bands[8] = 40.0;
        let mut n_prime = [0.0_f64; ZWICKER_BARK_BANDS];
        let zw = compute_zwicker_specific_and_total_loudness_into(&bands, &mut n_prime).unwrap();
        assert!((zw.total_loudness_sones - 1.0).abs() < 0.02);

        let tis = compute_tis_soft_tissue(210.0, 1.0).unwrap();
        assert!((tis.thermal_index - 1.0).abs() < 1e-12);
        assert_eq!(tis.model_uri, TIS_MODEL_URI);
    }
}
