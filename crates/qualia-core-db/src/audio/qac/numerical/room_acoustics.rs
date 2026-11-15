//! Room Acoustics: Sabine / Eyring-Norris / Millington-Sette $T_{60}$ & Schroeder RIR Integration Facade.
//!
//! Re-exports the canonical Sabine, Eyring-Norris, and Millington-Sette $T_{60}$ reverberation
//! time calculators and ISO 3382-1 Schroeder backward integration ($\text{EDT}, T_{20}, T_{30},
//! C_{50}, C_{80}, D_{50}, T_s$) from [`etalon_sim::acoustics_continuum::room_acoustics`].

pub use etalon_sim::acoustics_continuum::room_acoustics::{
    analyze_room_impulse_response, fit_decay_rt60_s, schroeder_backward_integration_db,
    RirAcousticsReport, RoomGeometry, SurfacePatch, RT60_NUMERATOR_55_25,
    RT60_NUMERATOR_EXACT_24LN10,
};
