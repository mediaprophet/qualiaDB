//! Composed excellence pipelines (orchestration only).

pub mod challenge_pad_from_mesh_trace;
pub mod respiration_monitor;
pub mod self_monitor_pulse;
pub mod self_monitor_pulse_evm;
pub mod biosense_observation_quins;
pub mod sparql_mm_observation_query;

pub use challenge_pad_from_mesh_trace::{
    challenge_pad_from_landmark_frames, challenge_pad_from_mesh_trace,
};
pub use respiration_monitor::{respiration_monitor, respiration_monitor_motion_only};
pub use self_monitor_pulse::self_monitor_pulse;
pub use self_monitor_pulse_evm::{self_monitor_pulse_evm, PulseAbstain, PulseEvmResult};
pub use biosense_observation_quins::compile_hr_observation_quins;
pub use sparql_mm_observation_query::{faces_in_zone_time, ObsQueryResult};
