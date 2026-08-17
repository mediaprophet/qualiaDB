//! Typed container bodies. Each file is one occupant kind.

mod doc;
mod health;
mod ontology;
mod sheet;
mod social;
mod spatial;

pub use doc::DocBody;
pub use health::HealthBody;
pub use ontology::OntologyBody;
pub use sheet::SheetBody;
pub use social::SocialBody;
pub use spatial::{MapBody, MediaBody, SubmanifoldBody};
