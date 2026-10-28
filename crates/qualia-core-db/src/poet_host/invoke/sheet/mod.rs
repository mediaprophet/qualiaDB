//! Future seam: `qualia-sheet`. CML cell→N3 is later; this is numeric grid start.

mod formula;
mod range;
mod set_cell;

pub use range::{stats, sum_range};
pub use formula::evaluate_formula;
pub use set_cell::set_cell;
