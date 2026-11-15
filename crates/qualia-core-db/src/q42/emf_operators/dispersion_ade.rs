//! Auxiliary Differential Equation (ADE) Dispersive Media Updates (Debye, Drude, Lorentz).
//!
//! Delegates time-domain ADE polarization and current density updates and complex
//! frequency-domain permittivity $\varepsilon_{\text{eff}}(\omega)$ transfer-function
//! verification to `etalon_sim::maxwell_emf::dispersion_ade` per C01 ownership.

pub use etalon_sim::maxwell_emf::dispersion_ade::*;
