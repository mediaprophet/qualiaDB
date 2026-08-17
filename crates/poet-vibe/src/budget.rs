//! Instruction budget.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Budget {
    pub steps_left: u64,
    pub workspace_left: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            steps_left: 100_000,
            workspace_left: 16 * 1024 * 1024,
        }
    }
}

impl Budget {
    pub fn from_steps(steps: u64) -> Self {
        Self {
            steps_left: steps,
            workspace_left: 16 * 1024 * 1024,
        }
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
