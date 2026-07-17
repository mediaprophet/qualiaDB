pub mod ensemble_respiration;
pub mod respiration_from_motion;
pub mod respiration_rate_from_motion_trace;
pub mod rr_estimate;

pub use ensemble_respiration::ensemble_respiration;
pub use respiration_from_motion::respiration_from_motion;
pub use respiration_rate_from_motion_trace::{
    respiration_rate_from_motion_trace, spectral_rr_peak,
};
pub use rr_estimate::{
    RrEstimate, RR_F_HI_HZ, RR_F_LO_HZ, RR_MIN_SAMPLES, RR_MIN_SNR_DEFAULT, RR_SNR_FOR_FULL_CONF,
    RR_SPECTRAL_STEPS,
};
