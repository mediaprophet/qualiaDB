//! Bounded native CML projection of the existing Studio source model. Uses the
//! existing quick-xml engine; imported text never supplies grants or store stamps.
mod parser;
mod writer;
pub use parser::import_into;
pub use writer::export;

/// Cold XML input/output bounds, selected by the native caller. Defaults are a
/// compatibility profile, not an editable-master fidelity ceiling.
#[derive(Clone, Copy)]
pub struct CmlLimits {
    pub bytes: usize,
    pub fields: usize,
    pub depth: usize,
}
impl Default for CmlLimits {
    fn default() -> Self {
        Self {
            bytes: 32 * 1024 * 1024,
            fields: 16_384,
            depth: 4,
        }
    }
}

#[cfg(test)]
mod tests;
