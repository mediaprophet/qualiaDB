use std::path::{Component, Path};

#[derive(Debug)]
pub enum ArtifactError {
    InvalidLabel,
    InvalidRelativePath,
    BudgetExceeded {
        budget_bytes: u64,
        attempted_bytes: u64,
    },
    TargetExists,
    Io(std::io::Error),
    Cleanup {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    Promotion {
        staging: std::path::PathBuf,
        target: std::path::PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLabel => write!(
                f,
                "artifact run label must be non-empty ASCII [a-zA-Z0-9_-]"
            ),
            Self::InvalidRelativePath => {
                write!(f, "artifact path must be a non-empty relative child path")
            }
            Self::BudgetExceeded {
                budget_bytes,
                attempted_bytes,
            } => write!(
                f,
                "artifact byte budget exceeded: budget={budget_bytes}, attempted={attempted_bytes}"
            ),
            Self::TargetExists => write!(f, "artifact promotion target already exists"),
            Self::Io(source) => write!(f, "artifact I/O: {source}"),
            Self::Cleanup { path, source } => {
                write!(
                    f,
                    "artifact cleanup failed for {}: {source}",
                    path.display()
                )
            }
            Self::Promotion {
                staging,
                target,
                source,
            } => write!(
                f,
                "artifact promotion {} -> {} failed: {source}",
                staging.display(),
                target.display()
            ),
        }
    }
}

impl std::error::Error for ArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) | Self::Cleanup { source, .. } | Self::Promotion { source, .. } => {
                Some(source)
            }
            _ => None,
        }
    }
}

impl From<std::io::Error> for ArtifactError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn validate_relative_artifact_path(path: &Path) -> Result<(), ArtifactError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(ArtifactError::InvalidRelativePath);
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(ArtifactError::InvalidRelativePath),
        }
    }
    Ok(())
}

pub(super) fn validate_label(label: &str) -> Result<(), ArtifactError> {
    if label.is_empty()
        || !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(ArtifactError::InvalidLabel);
    }
    Ok(())
}
