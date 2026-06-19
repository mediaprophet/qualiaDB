//! Bind group management for render pipeline
//!
//! Manages bind groups for view/projection matrices, motor buffers, and epistemic parameters.

use crate::math::buffer_alignment::RenderQuin;
use wgpu;
use wgpu::util::DeviceExt;

/// Epistemic parameters uniform buffer
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct EpistemicParams {
    pub confidence: f32,
    pub intensity: f32,
    pub _pad: [f32; 2],
}

/// Bind group manager for render resources
pub struct BindGroupManager {
    bind_group_layout: wgpu::BindGroupLayout,
    view_projection_buffer: wgpu::Buffer,
    motor_buffer: wgpu::Buffer,
    epistemic_params_buffer: wgpu::Buffer,
}

impl BindGroupManager {
    /// Create a new bind group manager
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("webizen-render-bind-group-layout"),
            entries: &[
                // Binding 0: View/Projection matrix (4x4 = 16 floats = 64 bytes)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: Motor buffer (storage array of Motors)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: Motor index uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 3: Epistemic parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // View/Projection matrix buffer (4x4 = 16 floats = 64 bytes)
        let view_projection_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-view-projection-buffer"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Motor buffer (storage buffer, initially empty)
        let motor_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-motor-buffer"),
            size: 1024, // Initial size, can be resized
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Epistemic parameters buffer (16 bytes, aligned to 16)
        let epistemic_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-epistemic-params-buffer"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            bind_group_layout,
            view_projection_buffer,
            motor_buffer,
            epistemic_params_buffer,
        }
    }

    /// Update view/projection matrix
    pub fn update_view_projection(&self, queue: &wgpu::Queue, matrix: [[f32; 4]; 4]) {
        queue.write_buffer(
            &self.view_projection_buffer,
            0,
            bytemuck::cast_slice(&matrix),
        );
    }

    /// Update motor buffer with RenderQuin data
    pub fn update_motors(&self, queue: &wgpu::Queue, quins: &[RenderQuin]) {
        let bytes = bytemuck::cast_slice(quins);
        if bytes.len() > 1024 {
            // TODO: Resize buffer if needed
            log::warn!("Motor buffer overflow: {} bytes > 1024", bytes.len());
        }
        queue.write_buffer(&self.motor_buffer, 0, &bytes[..bytes.len().min(1024)]);
    }

    /// Update epistemic parameters
    pub fn update_epistemic_params(&self, queue: &wgpu::Queue, params: EpistemicParams) {
        queue.write_buffer(
            &self.epistemic_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );
    }

    /// Get the bind group layout
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Create a bind group for rendering
    pub fn create_bind_group(&self, device: &wgpu::Device, motor_index: u32) -> wgpu::BindGroup {
        // Motor index uniform buffer
        let motor_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("webizen-motor-index-buffer"),
            contents: bytemuck::bytes_of(&motor_index),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("webizen-render-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.view_projection_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.motor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: motor_index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.epistemic_params_buffer.as_entire_binding(),
                },
            ],
        })
    }
}

/// Pre-configured bind groups for rendering
pub struct RenderBindGroups {
    pub primary_bind_group: wgpu::BindGroup,
}

impl RenderBindGroups {
    /// Create render bind groups from a bind group manager
    pub fn new(manager: &BindGroupManager, device: &wgpu::Device, motor_index: u32) -> Self {
        Self {
            primary_bind_group: manager.create_bind_group(device, motor_index),
        }
    }
}
