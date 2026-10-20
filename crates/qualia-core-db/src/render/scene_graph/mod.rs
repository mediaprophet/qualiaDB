//! Scene graph — generational 10D node pool, prefab delta inheritance, `.10d` `EONT1` living
//! ontology binding, topological PGA [`Motor`] propagation, lights, semantic links, and IK.
//!
//! Preserves the legacy authoring [`SceneGraph`] / [`SceneNode`] types alongside the Tier-1
//! (`HotZeroHeap`) generational [`NodePool`] and [`SceneNodeRecord`] architecture.

pub mod ik;
pub mod light;
pub mod node;
pub mod ontology_binding;
pub mod pool;
pub mod prefab;
pub mod smooth_damp;
pub mod transform_sync;

pub use ik::{ccd_ik, look_at_ik, IkResult};
pub use light::{Light, LightType};
pub use node::{duplicate_node, link_semantic, SceneGraph, SceneNode, SemanticLink};
pub use ontology_binding::{
    apply_ontogenetic_allometry, dispatch_affordance, interpolate_ontogenetic_scales,
    AffordanceDispatchOutcome, DevelopmentalStage, LivingOntologyState,
};
pub use pool::{
    NodeHandle, NodeKind, NodePool, NodePoolSlice, SceneNodeRecord, INVALID_GENERATION,
    MAX_SCENE_ARENA_NODES, VISIBILITY_CAST_SHADOWS, VISIBILITY_HOLOGRAPHIC_INTERFERENCE,
    VISIBILITY_RAYCAST_PICKABLE, VISIBILITY_RECEIVE_SHADOWS, VISIBILITY_VISIBLE,
};
pub use prefab::{
    apply_archetype_to_instance, override_local_motor, override_local_scale, override_material_id,
    override_node_kind, InheritableField, PrefabDeltaMask,
};
pub use smooth_damp::{smooth_damp, smooth_damp_vec3};
pub use transform_sync::{
    bilateral_pull_active_for_node, compute_topological_order, propagate_transforms_zero_heap,
    SyncedHolographicNode, SyncedInstanceTransform, TransformSyncSummary,
};
