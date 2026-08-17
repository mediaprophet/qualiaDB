//! Future seam: `qualia-logic` (deontic, epistemic, LTL, DL, paraconsistent, ASP).

mod asp;
mod causal;
mod deontic;
mod epistemic;
mod ltl;
mod paraconsistent;
mod subsumption;

pub use asp::enumerate as asp_enumerate;
pub use causal::{caused, t_norm};
pub use deontic::evaluate as deontic_evaluate;
pub use epistemic::evaluate as epistemic_evaluate;
pub use ltl::{finally as ltl_finally, globally as ltl_globally};
pub use paraconsistent::route as paraconsistent_route;
pub use subsumption::check as subsumption_check;
