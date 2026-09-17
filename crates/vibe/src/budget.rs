//! Instruction budget.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Budget {
    pub steps_left: u64,
    pub workspace_left: u64,
}

/// 42 MB Prolog Sentinel ceiling (AGENTS.md).
pub const SENTINEL_BYTES: u64 = 42 * 1024 * 1024;

impl Default for Budget {
    fn default() -> Self {
        Self {
            steps_left: 100_000,
            workspace_left: SENTINEL_BYTES,
        }
    }
}

impl Budget {
    pub fn from_steps(steps: u64) -> Self {
        Self {
            steps_left: steps,
            workspace_left: SENTINEL_BYTES,
        }
    }

    pub fn charge(&mut self, bytes: u64, span: Span) -> Result<(), Diagnostic> {
        if bytes > self.workspace_left {
            return Err(Diagnostic::new(
                DiagCode::E400,
                span,
                format!(
                    "workspace budget exhausted ({bytes} bytes requested, {} left; Sentinel is {SENTINEL_BYTES})",
                    self.workspace_left
                ),
            ));
        }
        self.workspace_left -= bytes;
        Ok(())
    }

    pub fn tick(&mut self, span: Span) -> Result<(), Diagnostic> {
        if self.steps_left == 0 {
            return Err(Diagnostic::new(
                DiagCode::E400,
                span,
                "instruction budget exhausted",
            ));
        }
        self.steps_left -= 1;
        Ok(())
    }
}
