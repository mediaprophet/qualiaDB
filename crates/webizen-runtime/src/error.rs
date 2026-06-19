use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum RuntimeError {
    AdapterUnavailable,
    ChannelClosed,
    BufferMapFailed(String),
    DeviceRequestFailed(String),
    ComputeFailed(String),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdapterUnavailable => write!(f, "no compatible GPU adapter was found"),
            Self::ChannelClosed => write!(f, "runtime channel closed unexpectedly"),
            Self::BufferMapFailed(err) => write!(f, "failed to map GPU buffer: {err}"),
            Self::DeviceRequestFailed(err) => write!(f, "failed to request GPU device: {err}"),
            Self::ComputeFailed(err) => write!(f, "compute pass failed: {err}"),
        }
    }
}

impl Error for RuntimeError {}
