//! Prefab Property Inheritance & Delta Mask (`prefab.rs`).
//!
//! Enables `.hmc` / `.10d` variant scene nodes to inherit unmodified properties from a parent
//! archetype [`SceneNodeRecord`] in $O(1)$ time without duplicating untouched fields, while
//! preserving per-instance overrides via [`PrefabDeltaMask`] and [`InheritableField`].

use crate::render::pga::Motor;
use crate::render::scene_graph::pool::{NodeKind, SceneNodeRecord};

/// 64-bit bitmask tracking which fields of a [`SceneNodeRecord`] have been overridden on an instance.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PrefabDeltaMask(pub u64);

impl PrefabDeltaMask {
    pub const NONE: Self = Self(0);

    pub const BIT_LOCAL_MOTOR: u64 = 1 << 0;
    pub const BIT_LOCAL_SCALE: u64 = 1 << 1;
    pub const BIT_TENSOR_SELECTORS: u64 = 1 << 2;
    pub const BIT_EPISTEMIC_Q: u64 = 1 << 3;
    pub const BIT_ALPHA_SALIENCE: u64 = 1 << 4;
    pub const BIT_MU_CARRIER: u64 = 1 << 5;
    pub const BIT_SIGMA_SPECTRAL: u64 = 1 << 6;
    pub const BIT_NODE_KIND: u64 = 1 << 7;
    pub const BIT_MESH_HANDLE: u64 = 1 << 8;
    pub const BIT_PROC_HEADER_HANDLE: u64 = 1 << 9;
    pub const BIT_EONT_HANDLE: u64 = 1 << 10;
    pub const BIT_MATERIAL_ID: u64 = 1 << 11;
    pub const BIT_RIG_HANDLE: u64 = 1 << 12;
    pub const BIT_COLLIDER_HANDLE: u64 = 1 << 13;
    pub const BIT_LIGHT_HANDLE: u64 = 1 << 14;
    pub const BIT_AUDIO_EMITTER_HANDLE: u64 = 1 << 15;
    pub const BIT_VISIBILITY_FLAGS: u64 = 1 << 16;

    /// Returns `true` if `bit` is marked as overridden on the instance.
    #[inline]
    pub const fn is_modified(&self, bit: u64) -> bool {
        (self.0 & bit) != 0
    }

    /// Marks `bit` as overridden.
    #[inline]
    pub fn mark_modified(&mut self, bit: u64) {
        self.0 |= bit;
    }

    /// Clears the override flag for `bit` so it reverts to inheriting from the archetype.
    #[inline]
    pub fn clear_modified(&mut self, bit: u64) {
        self.0 &= !bit;
    }
}

/// Wrapper for an individual inheritable property with $O(1)$ delta-mask tracking.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InheritableField<T: Copy> {
    value: T,
    bit_flag: u64,
    modified: bool,
}

impl<T: Copy> InheritableField<T> {
    /// Creates a new inheritable field initialized from the archetype default.
    #[inline]
    pub const fn new(value: T, bit_flag: u64) -> Self {
        Self {
            value,
            bit_flag,
            modified: false,
        }
    }

    /// Reads the current value of the field.
    #[inline]
    pub const fn get(&self) -> T {
        self.value
    }

    /// Returns `true` if this field has been explicitly overridden on the instance.
    #[inline]
    pub const fn is_modified(&self) -> bool {
        self.modified
    }

    /// Overrides the field value and sets its corresponding bit in `mask`.
    #[inline]
    pub fn set_override(&mut self, new_value: T, mask: &mut PrefabDeltaMask) {
        self.value = new_value;
        self.modified = true;
        mask.mark_modified(self.bit_flag);
    }

    /// Synchronizes this field with `archetype_value` if it has not been overridden.
    #[inline]
    pub fn inherit_from(&mut self, archetype_value: T, mask: PrefabDeltaMask) {
        if !self.modified && !mask.is_modified(self.bit_flag) {
            self.value = archetype_value;
        }
    }

    /// Resets the override flag and restores `archetype_value`.
    #[inline]
    pub fn reset_to_archetype(&mut self, archetype_value: T, mask: &mut PrefabDeltaMask) {
        self.value = archetype_value;
        self.modified = false;
        mask.clear_modified(self.bit_flag);
    }
}

/// Propagates all non-overridden fields from a parent `.10d` `archetype` record into `instance`
/// in $O(1)$ time while respecting `instance.modified_mask`.
#[inline]
pub fn apply_archetype_to_instance(
    archetype: &SceneNodeRecord,
    instance: &mut SceneNodeRecord,
    current_tick: u32,
) {
    let mask = PrefabDeltaMask(instance.modified_mask);

    if !mask.is_modified(PrefabDeltaMask::BIT_LOCAL_MOTOR) {
        instance.local_motor = archetype.local_motor;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_LOCAL_SCALE) {
        instance.local_scale = archetype.local_scale;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_TENSOR_SELECTORS) {
        instance.tensor_q = archetype.tensor_q;
        instance.tensor_v = archetype.tensor_v;
        instance.tensor_w = archetype.tensor_w;
        instance.deontic_lane = archetype.deontic_lane;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_EPISTEMIC_Q) {
        instance.epistemic_q = archetype.epistemic_q;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_ALPHA_SALIENCE) {
        instance.alpha_salience = archetype.alpha_salience;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_MU_CARRIER) {
        instance.mu_carrier = archetype.mu_carrier;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_SIGMA_SPECTRAL) {
        instance.sigma_spectral = archetype.sigma_spectral;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_NODE_KIND) {
        instance.node_kind = archetype.node_kind;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_MESH_HANDLE) {
        instance.mesh_handle = archetype.mesh_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_PROC_HEADER_HANDLE) {
        instance.proc_header_handle = archetype.proc_header_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_EONT_HANDLE) {
        instance.eont_handle = archetype.eont_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_MATERIAL_ID) {
        instance.material_id = archetype.material_id;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_RIG_HANDLE) {
        instance.rig_handle = archetype.rig_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_COLLIDER_HANDLE) {
        instance.collider_handle = archetype.collider_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_LIGHT_HANDLE) {
        instance.light_handle = archetype.light_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_AUDIO_EMITTER_HANDLE) {
        instance.audio_emitter_handle = archetype.audio_emitter_handle;
    }
    if !mask.is_modified(PrefabDeltaMask::BIT_VISIBILITY_FLAGS) {
        instance.visibility_flags = archetype.visibility_flags;
    }

    instance.changed_tick = current_tick;
}

/// Overrides `instance.local_motor` and marks `BIT_LOCAL_MOTOR` in `instance.modified_mask`.
#[inline]
pub fn override_local_motor(instance: &mut SceneNodeRecord, motor: Motor, current_tick: u32) {
    instance.local_motor = motor;
    instance.modified_mask |= PrefabDeltaMask::BIT_LOCAL_MOTOR;
    instance.changed_tick = current_tick;
}

/// Overrides `instance.local_scale` and marks `BIT_LOCAL_SCALE` in `instance.modified_mask`.
#[inline]
pub fn override_local_scale(instance: &mut SceneNodeRecord, scale: [f32; 3], current_tick: u32) {
    instance.local_scale = scale;
    instance.modified_mask |= PrefabDeltaMask::BIT_LOCAL_SCALE;
    instance.changed_tick = current_tick;
}

/// Overrides `instance.material_id` and marks `BIT_MATERIAL_ID` in `instance.modified_mask`.
#[inline]
pub fn override_material_id(instance: &mut SceneNodeRecord, material_id: u16, current_tick: u32) {
    instance.material_id = material_id;
    instance.modified_mask |= PrefabDeltaMask::BIT_MATERIAL_ID;
    instance.changed_tick = current_tick;
}

/// Overrides `instance.node_kind` and marks `BIT_NODE_KIND` in `instance.modified_mask`.
#[inline]
pub fn override_node_kind(instance: &mut SceneNodeRecord, kind: NodeKind, current_tick: u32) {
    instance.node_kind = kind;
    instance.modified_mask |= PrefabDeltaMask::BIT_NODE_KIND;
    instance.changed_tick = current_tick;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::pga::motor_translate;

    #[test]
    fn inheritable_field_respects_override_and_reset() {
        let mut mask = PrefabDeltaMask::NONE;
        let mut mat = InheritableField::new(10u16, PrefabDeltaMask::BIT_MATERIAL_ID);
        assert_eq!(mat.get(), 10);
        assert!(!mat.is_modified());

        // Unmodified field updates when archetype changes.
        mat.inherit_from(25, mask);
        assert_eq!(mat.get(), 25);

        // Overridden field resists archetype updates.
        mat.set_override(99, &mut mask);
        assert!(mat.is_modified());
        assert!(mask.is_modified(PrefabDeltaMask::BIT_MATERIAL_ID));
        mat.inherit_from(42, mask);
        assert_eq!(mat.get(), 99);

        // Reset restores archetype value.
        mat.reset_to_archetype(42, &mut mask);
        assert!(!mat.is_modified());
        assert!(!mask.is_modified(PrefabDeltaMask::BIT_MATERIAL_ID));
        assert_eq!(mat.get(), 42);
    }

    #[test]
    fn prefab_delta_mask_inherits_unmodified_and_preserves_overrides() {
        let mut archetype = SceneNodeRecord::new(0xAAAA, NodeKind::MeshBRep);
        archetype.mesh_handle = 7;
        archetype.material_id = 3;
        archetype.local_scale = [1.5, 2.0, 1.5];
        archetype.sigma_spectral = 0.62;

        let mut instance = SceneNodeRecord::new(0xBBBB, NodeKind::Empty);
        override_material_id(&mut instance, 42, 1);
        override_local_motor(&mut instance, motor_translate([10.0, 0.0, -5.0]), 1);

        apply_archetype_to_instance(&archetype, &mut instance, 2);

        // Inherited from archetype:
        assert_eq!(instance.node_kind, NodeKind::MeshBRep);
        assert_eq!(instance.mesh_handle, 7);
        assert_eq!(instance.local_scale, [1.5, 2.0, 1.5]);
        assert!((instance.sigma_spectral - 0.62).abs() < 1e-6);

        // Preserved instance overrides:
        assert_eq!(instance.material_id, 42);
        assert_eq!(instance.local_motor.translation(), [10.0, 0.0, -5.0]);
        assert_eq!(instance.changed_tick, 2);
    }
}
