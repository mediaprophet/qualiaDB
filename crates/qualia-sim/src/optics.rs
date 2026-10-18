//! Hydrologic Optics, Radiative Transfer & Dielectric BSDF Engine (`optics.rs`)
//!
//! Provides zero-heap physical modeling of Inherent Optical Properties (IOP)
//! and Apparent Optical Properties (AOP) across water volumes and solid dielectrics (glass):
//! - Multi-band spectral absorption $a(\lambda)$ and backscattering $b_b(\lambda)$
//! - CASE-2 Bio-Geo-Optical constituent synthesis:
//!   * CDOM / Tannins: Eucalyptus leaf litter and peat humic/fulvic acids (Dark mirror / blackwater)
//!   * Inorganic Tripton & Colloidal Clay: Historic alluvial gold mining disturbance (Milky ochre veil)
//!   * Armored Stream Bedload: Intact cobble/bedrock with high Brewster-angle substrate visibility
//! - Cauchy spectral chromatic dispersion for solid dielectrics (soda-lime glass)
//! - Beer-Lambert volume extinction and EMF phase velocity shifts
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, deterministic).

/// Wavelength standards in nanometers: Red (650 nm), Green (532 nm), Blue (450 nm).
pub const WAVELENGTH_RED_NM: f32 = 650.0;
pub const WAVELENGTH_GREEN_NM: f32 = 532.0;
pub const WAVELENGTH_BLUE_NM: f32 = 450.0;

/// Pure H2O molecular absorption coefficients in m⁻¹ at 20°C (Smith & Baker, 1981).
pub const PURE_WATER_ABSORPTION_RGB: [f32; 3] = [0.350, 0.045, 0.005];
/// Pure H2O molecular Rayleigh scattering coefficients in m⁻¹.
pub const PURE_WATER_RAYLEIGH_RGB: [f32; 3] = [0.0011, 0.0024, 0.0058];

/// Inherent Optical Properties (IOP) of a dielectric medium or fluid volume.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DielectricVolumeIop {
    /// Spectral Index of Refraction (IOR) [n_red, n_green, n_blue] driving chromatic dispersion.
    pub ior_spectrum: [f32; 3],
    /// Spectral absorption coefficient $a(\lambda)$ in m⁻¹ (photons absorbed by molecules).
    pub absorption_coeff: [f32; 3],
    /// Molecular Rayleigh scattering coefficient $b_r(\lambda)$ in m⁻¹ ($\propto \lambda^{-4}$).
    pub rayleigh_scattering: [f32; 3],
    /// Particulate Mie scattering $[b_r, b_g, b_b, g]$ in m⁻¹ + Henyey-Greenstein anisotropy $g \in [-1, 1]$.
    pub mie_scattering: [f32; 4],
}

impl Default for DielectricVolumeIop {
    fn default() -> Self {
        Self {
            ior_spectrum: [1.331, 1.334, 1.338], // Standard water normal dispersion
            absorption_coeff: PURE_WATER_ABSORPTION_RGB,
            rayleigh_scattering: PURE_WATER_RAYLEIGH_RGB,
            mie_scattering: [0.0, 0.0, 0.0, 0.92], // Forward-peaked default
        }
    }
}

impl DielectricVolumeIop {
    /// Total beam attenuation coefficient $c(\lambda) = a(\lambda) + b(\lambda)$ in m⁻¹.
    #[inline]
    pub fn beam_attenuation(&self) -> [f32; 3] {
        [
            self.absorption_coeff[0] + self.rayleigh_scattering[0] + self.mie_scattering[0],
            self.absorption_coeff[1] + self.rayleigh_scattering[1] + self.mie_scattering[1],
            self.absorption_coeff[2] + self.rayleigh_scattering[2] + self.mie_scattering[2],
        ]
    }

    /// Evaluates Beer-Lambert optical transmittance $T(d) = \exp(-c(\lambda) d)$ over distance $d$ in meters.
    #[inline]
    pub fn transmittance(&self, distance_m: f32) -> [f32; 3] {
        let c = self.beam_attenuation();
        [
            (-c[0] * distance_m).exp(),
            (-c[1] * distance_m).exp(),
            (-c[2] * distance_m).exp(),
        ]
    }

    /// Evaluates continuous EMF phase shift $\Delta \phi = \frac{2\pi}{\lambda} n(\lambda) d$ (in radians).
    #[inline]
    pub fn phase_shift(&self, distance_m: f32) -> [f32; 3] {
        let two_pi = core::f32::consts::PI * 2.0;
        // Wavelengths in meters: 650e-9, 532e-9, 450e-9
        [
            (two_pi / 650e-9) * self.ior_spectrum[0] * distance_m,
            (two_pi / 532e-9) * self.ior_spectrum[1] * distance_m,
            (two_pi / 450e-9) * self.ior_spectrum[2] * distance_m,
        ]
    }

    /// Computes substrate visibility score in [0.0, 1.0] at depth $d$:
    /// Combines two-way transmission down and back ($e^{-2 c d}$) with loss from particulate backscatter veil.
    pub fn substrate_visibility(&self, depth_m: f32) -> f32 {
        let c = self.beam_attenuation();
        // Average attenuation across visible green/blue
        let avg_c = (c[1] + c[2]) * 0.5;
        let direct_trans = (-2.0 * avg_c * depth_m).exp();

        // Particulate backscatter veil obscures substrate
        let backscatter = (self.mie_scattering[1] + self.mie_scattering[2]) * 0.5;
        let veil_penalty = 1.0 / (1.0 + backscatter * depth_m * 1.8);

        (direct_trans * veil_penalty).clamp(0.0, 1.0)
    }
}

/// Ecological and Geomorphic Bio-Geo-Optical constituents (CASE-2 Waters).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BioGeoOpticalConstituents {
    /// CDOM (Chromophoric Dissolved Organic Matter) / Tannin concentration in g/m³.
    /// Leached from eucalyptus leaf detritus and peat bogs.
    pub cdom_tannin_conc: f32,
    /// Suspended inorganic mineral tripton and colloidal clay concentration in g/m³.
    /// Elevated by alluvial gold dredging, hydraulic mining scars, and runoff erosion.
    pub tripton_clay_silt_conc: f32,
    /// Biogenic chlorophyll / phytoplankton concentration in mg/m³.
    pub chlorophyll_conc: f32,
    /// Water salinity in Practical Salinity Units (PSU, 0.0 = fresh, 35.0 = sea).
    pub salinity_psu: f32,
    /// Water temperature in degrees Celsius.
    pub water_temp_c: f32,
}

impl Default for BioGeoOpticalConstituents {
    fn default() -> Self {
        Self {
            cdom_tannin_conc: 0.8,
            tripton_clay_silt_conc: 0.5,
            chlorophyll_conc: 0.3,
            salinity_psu: 0.0,
            water_temp_c: 18.0,
        }
    }
}

impl BioGeoOpticalConstituents {
    /// Ecological Preset: Australian Eucalyptus Billabong / Blackwater Creek.
    /// High CDOM tannins, negligible mineral sediment.
    /// Forms a dark amber "blackwater mirror" with intense sky reflections and amber shallows.
    pub const fn eucalyptus_blackwater() -> Self {
        Self {
            cdom_tannin_conc: 24.0,
            tripton_clay_silt_conc: 0.15,
            chlorophyll_conc: 0.2,
            salinity_psu: 0.0,
            water_temp_c: 21.0,
        }
    }

    /// Ecological Preset: Post-Mining Damaged River (Historic Alluvial Gold Dredging).
    /// Stripped cobble armoring exposing raw dispersive clay subsoils.
    /// Heavy colloidal kaolinite/illite suspension creating an opaque ochre milky veil.
    pub const fn gold_mining_damaged_river() -> Self {
        Self {
            cdom_tannin_conc: 1.2,
            tripton_clay_silt_conc: 48.0,
            chlorophyll_conc: 0.1,
            salinity_psu: 0.0,
            water_temp_c: 16.0,
        }
    }

    /// Ecological Preset: Intact Armored Cobble / Bedrock Mountain Stream.
    /// Sediment locked beneath intact stone armor, low detritus, high oxygenation.
    /// High optical transmittance, Brewster polarization window revealing riverbed.
    pub const fn intact_armored_stream() -> Self {
        Self {
            cdom_tannin_conc: 0.25,
            tripton_clay_silt_conc: 0.08,
            chlorophyll_conc: 0.05,
            salinity_psu: 0.0,
            water_temp_c: 12.0,
        }
    }

    /// Dielectric Material Preset: Standard Soda-Lime Window Glass (4mm).
    /// Rigid silica dielectric with Cauchy dispersion and trace Fe²⁺ iron-oxide green edge tint.
    pub const fn soda_lime_glass() -> DielectricVolumeIop {
        DielectricVolumeIop {
            // Cauchy dispersion: n(lambda) = A + B / lambda² (A=1.5046, B=0.0042 um²)
            ior_spectrum: [1.514, 1.519, 1.526],
            // Fe2+ iron impurity absorbs red strongly (giving characteristic green edge)
            absorption_coeff: [0.085, 0.015, 0.025],
            rayleigh_scattering: [0.0002, 0.0004, 0.0008],
            mie_scattering: [0.0001, 0.0001, 0.0001, 0.95],
        }
    }

    /// Synthesizes the full Inherent Optical Properties (IOP) from constituent concentrations.
    /// Zero allocations, pure analytical stack synthesis.
    pub fn synthesize_iop(&self) -> DielectricVolumeIop {
        // 1. Salinity & temperature modulated baseline refractive index (Millard & Seaver, 1990)
        let n_base = 1.333 + 0.00018 * self.salinity_psu - 0.00008 * (self.water_temp_c - 20.0);
        let ior_spectrum = [
            n_base - 0.0025, // Red (650 nm)
            n_base,          // Green (532 nm)
            n_base + 0.0045, // Blue (450 nm)
        ];

        // 2. CDOM / Tannin exponential blue absorption: a_CDOM(lambda) = a_440 * exp(-S * (lambda - 440))
        // S_CDOM spectral slope factor ≈ 0.015 nm⁻¹
        let a_440 = self.cdom_tannin_conc * 0.18;
        let cdom_abs_red = a_440 * (-0.015 * (WAVELENGTH_RED_NM - 440.0)).exp();
        let cdom_abs_green = a_440 * (-0.015 * (WAVELENGTH_GREEN_NM - 440.0)).exp();
        let cdom_abs_blue = a_440 * (-0.015 * (WAVELENGTH_BLUE_NM - 440.0)).exp();

        // 3. Chlorophyll phytoplankton absorption peaks at 440 nm and 675 nm
        let chl_abs_red = self.chlorophyll_conc * 0.025;
        let chl_abs_green = self.chlorophyll_conc * 0.005;
        let chl_abs_blue = self.chlorophyll_conc * 0.035;

        // Total absorption: Pure water + CDOM tannins + Phytoplankton
        let absorption_coeff = [
            PURE_WATER_ABSORPTION_RGB[0] + cdom_abs_red + chl_abs_red,
            PURE_WATER_ABSORPTION_RGB[1] + cdom_abs_green + chl_abs_green,
            PURE_WATER_ABSORPTION_RGB[2] + cdom_abs_blue + chl_abs_blue,
        ];

        // 4. Mineral tripton / clay Mie scattering: b(lambda) = b_550 * (lambda / 550)^(-gamma)
        // Dispersive alluvial clay has gamma ≈ 0.8
        let b_550 = self.tripton_clay_silt_conc * 0.42;
        let tripton_mie_red = b_550 * (WAVELENGTH_RED_NM / 550.0).powf(-0.8);
        let tripton_mie_green = b_550 * (WAVELENGTH_GREEN_NM / 550.0).powf(-0.8);
        let tripton_mie_blue = b_550 * (WAVELENGTH_BLUE_NM / 550.0).powf(-0.8);

        // Highly forward-peaked particulate asymmetry: g ≈ 0.92 for clay/silt
        let mie_scattering = [
            tripton_mie_red,
            tripton_mie_green,
            tripton_mie_blue,
            0.92,
        ];

        DielectricVolumeIop {
            ior_spectrum,
            absorption_coeff,
            rayleigh_scattering: PURE_WATER_RAYLEIGH_RGB,
            mie_scattering,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eucalyptus_blackwater_optics() {
        let constituents = BioGeoOpticalConstituents::eucalyptus_blackwater();
        let iop = constituents.synthesize_iop();

        // Tannins violently absorb blue while transmitting red/amber
        assert!(iop.absorption_coeff[2] > iop.absorption_coeff[0] * 5.0);
        // Negligible particulate scattering in blackwater
        assert!(iop.mie_scattering[1] < 0.2);

        // At 0.5m depth, red light penetrates while blue light is extinguished
        let trans = iop.transmittance(0.5);
        assert!(trans[0] > trans[2] * 4.0); // Transmits amber/red, extinguishes blue
    }

    #[test]
    fn test_gold_mining_damaged_river_opacity() {
        let constituents = BioGeoOpticalConstituents::gold_mining_damaged_river();
        let iop = constituents.synthesize_iop();

        // High suspended clay produces massive Mie scattering
        assert!(iop.mie_scattering[1] > 10.0);

        // Substrate visibility must be destroyed within 0.3m by colloidal clay veil
        let vis_shallow = iop.substrate_visibility(0.1);
        let vis_deep = iop.substrate_visibility(0.35);

        assert!(vis_shallow < 0.35);
        assert!(vis_deep < 0.01); // Riverbed completely invisible beyond 35cm
    }

    #[test]
    fn test_intact_armored_stream_substrate_visibility() {
        let constituents = BioGeoOpticalConstituents::intact_armored_stream();
        let iop = constituents.synthesize_iop();

        // Intact cobble riverbed has clear water: high visibility past 1 meter
        let vis_1m = iop.substrate_visibility(1.0);
        assert!(vis_1m > 0.65); // Clear view of cobbles and caustics
    }

    #[test]
    fn test_soda_lime_glass_cauchy_dispersion() {
        let glass_iop = BioGeoOpticalConstituents::soda_lime_glass();

        // Normal dispersion: refractive index is higher for shorter wavelengths (blue > red)
        assert!(glass_iop.ior_spectrum[2] > glass_iop.ior_spectrum[1]);
        assert!(glass_iop.ior_spectrum[1] > glass_iop.ior_spectrum[0]);

        // Iron impurity causes higher absorption in red than green (producing green edge)
        assert!(glass_iop.absorption_coeff[0] > glass_iop.absorption_coeff[1]);
    }
}
