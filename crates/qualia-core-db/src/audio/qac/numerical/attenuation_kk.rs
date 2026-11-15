//! Kramers-Kronig Acoustic Attenuation-Dispersion Consistency Facade.
//!
//! Re-exports the canonical causal power-law attenuation-dispersion model ($0 < y < 3$, including
//! the $y = 1$ logarithmic Szabo/Waters branch) and multi-relaxation atmospheric
//! ($\text{O}_2/\text{N}_2$) and seawater ($\text{B(OH)}_3/\text{MgSO}_4$) models from
//! [`etalon_sim::acoustics_continuum::attenuation_kk`].

pub use etalon_sim::acoustics_continuum::attenuation_kk::{
    MultiRelaxationMedium, PowerLawAttenuation, RelaxationProcess,
};
