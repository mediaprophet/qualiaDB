//! Explicit developer layer operations; only Session::step commits full tokens.
use super::*;

impl Session {
    /// Explicit lab operations do not commit a whole model token. Native decode
    /// must use step/step_controlled so its position advances exactly once.
    pub fn attention(&mut self, layer: usize, hidden: &mut [f32]) -> Result<(), SessionError> {
        let position = self.check_lab(layer, hidden)?;
        let guard = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        execute_attention(&self.plan, &mut self.buffers, layer, hidden, position)?;
        guard.commit_partial();
        Ok(())
    }
    pub fn ffn(&mut self, layer: usize, hidden: &mut [f32]) -> Result<(), SessionError> {
        self.check_lab(layer, hidden)?;
        let guard = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        execute_ffn(&self.plan, &mut self.buffers, layer, hidden)?;
        guard.commit_partial();
        Ok(())
    }
    pub fn forward(
        &mut self,
        range: std::ops::Range<usize>,
        hidden: &mut [f32],
    ) -> Result<usize, SessionError> {
        let position = self.check_lab(range.start, hidden)?;
        if range.end > self.plan.layers.len() || range.end < range.start {
            return Err(SessionError::Shape);
        }
        let count = range.len();
        let guard = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        for index in range {
            execute_attention(&self.plan, &mut self.buffers, index, hidden, position)?;
            execute_ffn(&self.plan, &mut self.buffers, index, hidden)?;
        }
        guard.commit_partial();
        Ok(count)
    }
    pub fn output_norm(&mut self, hidden: &mut [f32]) -> Result<(), SessionError> {
        self.check_lab(0, hidden)?;
        let guard = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        let source = DenseSource(&self.plan.pkg);
        normalize(
            &DenseHost { source: &source },
            &self.plan.final_norm,
            hidden,
            &mut self.buffers.norm,
            &mut self.buffers.normalized,
            self.plan.profile.ordinary_norm,
            self.plan.profile.epsilon,
        )?;
        hidden.copy_from_slice(&self.buffers.normalized);
        guard.commit_partial();
        Ok(())
    }
    pub fn argmax(&mut self, hidden: &[f32]) -> Result<(u32, f32), SessionError> {
        self.check_lab(0, hidden)?;
        let guard = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        let result = project_output(&self.plan, &mut self.buffers.logits, hidden)?;
        guard.commit_partial();
        Ok(result)
    }
    fn check_lab(&self, layer: usize, hidden: &[f32]) -> Result<usize, SessionError> {
        let position = self.position()?;
        if layer >= self.plan.layers.len() || hidden.len() != self.plan.profile.hidden {
            return Err(SessionError::Shape);
        }
        if position >= self.plan.profile.max_context {
            return Err(SessionError::ContextFull);
        }
        Ok(position)
    }
}
