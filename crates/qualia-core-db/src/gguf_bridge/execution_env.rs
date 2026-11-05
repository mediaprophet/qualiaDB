//! Native prepared configuration or unchanged browser process configuration.
#![allow(unused_imports)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use crate::inference::inference_bench::prepared_settings::{
    env_var as var, env_var_os as var_os,
};

#[cfg(target_arch = "wasm32")]
pub(crate) use std::env::{var, var_os};
