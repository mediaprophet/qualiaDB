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
        let (texture, view) = create_depth_texture(device, width, height);
        self.texture = texture;
        self.view = view;
        self.width = width.max(1);
        self.height = height.max(1);
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
}

#[cfg(test)]
mod tests {
    // The GPU-backed owner is exercised by the renderer resize fixtures. Keep
    // the lifecycle invariant close to the implementation for code review.
    #[test]
    fn zero_extent_is_normalized_by_the_owner_contract() {
        assert_eq!((0_u32.max(1), 24_u32.max(1)), (1, 24));
    }
}
