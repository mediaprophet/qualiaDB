//! Multi-observer entity-view kernel: entity ids, observer social schema, rights filter,
//! fragment attribution, bifurcated packages, projection layout.
//!
//! No I/O. Host crates compose storage + session.
//! Product language: **whole Qualia/Webizen is mindware** (prosthetic extension of self);
//! this module is only the multi-observer projection kernel - not "the mindware product."

pub mod attribution;
pub mod capability_report;
pub mod circumstance;
pub mod entity_id;
pub mod observer;
pub mod package;
pub mod projection;
pub mod rights_filter;

pub use attribution::{edges_for_subject, AttributionEdge, AttributionRel};
pub use capability_report::entity_view_capability_report;
pub use circumstance::{Circumstance, EnvironmentKind, EvaluatoryFocus};
pub use entity_id::{EntityId, EntityKind};
pub use observer::{
    AffordanceBits, EntityViewMeta, ObserverStatus, RepresentationWing, SensitivityClass,
};
pub use package::{BifurcatedPackage, PackageWing};
pub use projection::{
    layout_scene_nodes, FlatCard, LayoutInput, PresentationLevel, ProjectionResult, SceneNodeProj,
};
pub use rights_filter::{decide_view, filter_visible, ViewDecision};
