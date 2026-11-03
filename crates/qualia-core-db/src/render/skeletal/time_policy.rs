//! Per-invocation sampling policy; the authored clip's loop declaration and
//! legacy natural-advance behavior are never mutated by an inspection seek.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PoseTimePolicy {
    #[default]
    AuthoredLooping,
    /// Sample the closed authored interval, including its distinct final
    /// contact/state boundary. Required for paused end poses and offline frames.
    ExactClipInterval,
}
