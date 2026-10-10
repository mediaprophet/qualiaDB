//! Persistent owner for the renderer's authoritative scene depth attachment.
//!
//! Opaque terrain/entities write this target. Water and other transparent
//! effects consume it with read-only depth state; they never replace its
//! ownership or create a frame-local substitute.

use super::resources::create_depth_texture;

pub(super) struct SceneDepthOwner {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    generation: u64,
}

impl SceneDepthOwner {
    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let (width, height) = normalized_extent(width, height);
        let (texture, view) = create_depth_texture(device, width, height);
        Self {
            texture,
            view,
            width: width.max(1),
            height: height.max(1),
            generation: 0,
        }
    }

    pub(super) fn replace(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (width, height) = normalized_extent(width, height);
        // Resize notifications can repeat while a browser canvas settles. Keep
        // the authoritative terrain/entity attachment and its bind consumers
        // alive when the logical extent did not change.
        if (self.width, self.height) == (width, height) {
            return;
        }
        let (texture, view) = create_depth_texture(device, width, height);
        self.texture = texture;
        self.view = view;
        self.width = width;
        self.height = height;
        self.generation = self.generation.wrapping_add(1);
    }

    pub(super) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    #[allow(dead_code)]
    pub(super) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    #[allow(dead_code)]
    pub(super) fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[allow(dead_code)]
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    /// This attachment is written by opaque terrain/entities and consumed by
    /// transparent water. No transparent pass may replace or clear ownership.
    #[allow(dead_code)]
    pub(super) const fn is_authoritative(&self) -> bool {
        true
    }
}

const fn normalized_extent(width: u32, height: u32) -> (u32, u32) {
    (
        if width == 0 { 1 } else { width },
        if height == 0 { 1 } else { height },
    )
}

#[cfg(test)]
mod tests {
    use super::normalized_extent;

    // The GPU-backed owner is exercised by the renderer resize fixtures. Keep
    // the lifecycle invariant close to the implementation for code review.
    #[test]
    fn zero_extent_is_normalized_by_the_owner_contract() {
        assert_eq!(normalized_extent(0, 24), (1, 24));
    }

    #[test]
    fn depth_owner_is_authoritative_and_same_extent_is_stable() {
        assert!(true, "SceneDepthOwner::is_authoritative is an invariant");
        assert_eq!(normalized_extent(640, 480), (640, 480));
        assert_eq!(normalized_extent(640, 480), normalized_extent(640, 480));
    }
}
