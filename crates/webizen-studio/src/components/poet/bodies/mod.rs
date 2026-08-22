//! Typed container bodies. Each file is one occupant kind.

mod bookmarks;
mod codecs;
mod doc;
mod domains;
mod economics;
mod git;
mod health;
mod ide;
mod job_center;
mod ontology;
mod shaders;
mod sheet;
mod social;
mod solid;
mod spatial;

pub use bookmarks::BookmarksBody;
pub use codecs::CodecsBody;
pub use doc::DocBody;
pub use domains::DomainsBody;
pub use economics::EconomicsBody;
pub use git::GitBody;
pub use health::HealthBody;
pub use ide::IdeBody;
pub use job_center::JobCenterBody;
pub use ontology::OntologyBody;
pub use shaders::ShadersBody;
pub use sheet::SheetBody;
pub use social::SocialBody;
pub use solid::SolidBody;
pub use spatial::{MapBody, MediaBody, SubmanifoldBody};
