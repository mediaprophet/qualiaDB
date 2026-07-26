//! Bounded temporary artifacts and explicit evidence promotion.

mod budget;
mod cleanup;
mod run_dir;

pub use budget::{validate_relative_artifact_path, ArtifactError};
pub use cleanup::{cleanup_stale_runs, StaleCleanupReport};
pub use run_dir::{
    ArtifactFinish, ArtifactRetention, ArtifactStats, RunArtifactDir, RUN_MARKER_FILE,
};
