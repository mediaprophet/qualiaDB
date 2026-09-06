//! Errors for bounded asset import jobs (AST-02).

use std::io;
use std::path::PathBuf;

/// Fail-closed errors for cold-construction import jobs.
#[derive(Debug)]
pub enum ImportError {
    /// `chunk_byte_budget` is zero or exceeds the 42 MiB Sentinel pass budget.
    ChunkBudgetExceeded,
    /// Job-level byte budget would be exceeded by the next read.
    ByteBudgetExceeded { used: u64, max: u64 },
    /// Job-level record budget would be exceeded.
    RecordBudgetExceeded { used: u64, max: u64 },
    /// Budget fields must be strictly positive.
    InvalidBudgets,
    /// Caller path missing or not a regular file.
    RawNotFound(PathBuf),
    /// I/O while copying, reading, writing, or promoting.
    Io(io::Error),
    /// Job was cancelled before promote.
    Cancelled,
    /// Promote requires a successful completed feed; current state is not succeeded.
    NotSucceeded,
    /// Callback or framework left the job in a failed state.
    Failed(String),
    /// Destination already exists (promote refuses overwrite).
    DestinationExists(PathBuf),
}

impl PartialEq for ImportError {
    fn eq(&self, other: &Self) -> bool {
        use ImportError::*;
        match (self, other) {
            (ChunkBudgetExceeded, ChunkBudgetExceeded)
            | (InvalidBudgets, InvalidBudgets)
            | (Cancelled, Cancelled)
            | (NotSucceeded, NotSucceeded) => true,
            (ByteBudgetExceeded { used: a, max: b }, ByteBudgetExceeded { used: c, max: d }) => {
                a == c && b == d
            }
            (RecordBudgetExceeded { used: a, max: b }, RecordBudgetExceeded { used: c, max: d }) => {
                a == c && b == d
            }
            (RawNotFound(a), RawNotFound(b)) => a == b,
            (DestinationExists(a), DestinationExists(b)) => a == b,
            (Failed(a), Failed(b)) => a == b,
            (Io(a), Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}

impl Eq for ImportError {}

impl From<io::Error> for ImportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChunkBudgetExceeded => {
                write!(f, "chunk_byte_budget exceeds Sentinel pass budget or is zero")
            }
            Self::ByteBudgetExceeded { used, max } => {
                write!(f, "byte budget exceeded (used={used}, max={max})")
            }
            Self::RecordBudgetExceeded { used, max } => {
                write!(f, "record budget exceeded (used={used}, max={max})")
            }
            Self::InvalidBudgets => write!(f, "import budgets must be positive"),
            Self::RawNotFound(p) => write!(f, "raw artifact not found: {}", p.display()),
            Self::Io(e) => write!(f, "import i/o: {e}"),
            Self::Cancelled => write!(f, "import job cancelled"),
            Self::NotSucceeded => write!(f, "promote requires Succeeded status"),
            Self::Failed(msg) => write!(f, "import failed: {msg}"),
            Self::DestinationExists(p) => {
                write!(f, "promote destination exists: {}", p.display())
            }
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
