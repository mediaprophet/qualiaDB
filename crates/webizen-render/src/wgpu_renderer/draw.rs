//! 2D screen-space drawing primitives, epistemic anchor picking, and ambient controls.

#[cfg(not(feature = "qualia"))]
use super::pipelines::AmbientConfig;
use super::pipelines::ScreenVertex;
#[cfg(not(feature = "qualia"))]
use super::vegetation_effects;
use super::{Camera, ScreenPoint, Vec3, WgpuRenderer};

impl<'a> WgpuRenderer<'a> {
    /// Parse CSS color string to RGBA.
    pub(super) fn parse_color(color: &str, alpha: f64) -> [f32; 4] {
        let (r, g, b, a) = if color.starts_with('#') {
            let hex = color.trim_start_matches('#');
            let len = hex.len();
            if len == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b, (alpha * 255.0) as u8)
            } else if len == 3 {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                (r, g, b, (alpha * 255.0) as u8)
            } else {
                (0, 0, 0, (alpha * 255.0) as u8)
            }
        } else if color.starts_with("rgb(") {
            let parts: Vec<&str> = color
                .trim_start_matches("rgb(")
                .trim_start_matches("rgba(")
                .trim_end_matches(')')
                .split(',')
                .collect();
            let r = parts
                .first()
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let g = parts
                .get(1)
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let b = parts
                .get(2)
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let a = if parts.len() > 3 {
                parts
                    .get(3)
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(alpha)
            } else {
                alpha
            };
            (r, g, b, (a * 255.0) as u8)
        } else {
            (0, 0, 0, (alpha * 255.0) as u8)
        };

        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    }

    /// Get current viewport size.
    pub fn viewport(&self) -> (f64, f64) {
        self.viewport_size
    }

    /// Set the active camera.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    /// Configure ambient visualization options.
    #[cfg(not(feature = "qualia"))]
    pub fn set_ambient_config(&mut self, config: AmbientConfig) {
        self.ambient_config = config;
    }

    /// Get current ambient configuration.
    #[cfg(not(feature = "qualia"))]
    pub fn get_ambient_config(&self) -> AmbientConfig {
        self.ambient_config
    }

    /// Queue a bounded ambient effect. Events are admitted without allocation;
    /// the pool replaces the lowest-priority/oldest slot when it is saturated.
    /// The GPU buffer is refreshed only on this event path, never every frame.
    #[cfg(not(feature = "qualia"))]
    pub fn emit_ambient_effect(
        &mut self,
        event: vegetation_effects::EffectEvent,
        time_seconds: f32,
    ) -> vegetation_effects::SpawnOutcome {
        let outcome = self.ambient_effects.emit(event);
        if matches!(outcome, vegetation_effects::SpawnOutcome::Queued) {
            let frame = vegetation_effects::WorldFrame::from_time(
                time_seconds,
                vegetation_effects::DEFAULT_WORLD_SEED,
            );
            let telemetry = vegetation_effects::SystemTelemetry::default();
            self.ambient_effects.update(
                frame,
                vegetation_effects::WindField::from_telemetry(&telemetry),
            );
            if let Some(buffer) = &self.particle_buffer {
                self.queue.write_buffer(
                    buffer,
                    0,
                    bytemuck::cast_slice(self.ambient_effects.particles()),
                );
            }
        }
        outcome
    }

    /// Clear the frame to a solid background color.
    pub fn clear(&self, color: &str) {
        let rgba = Self::parse_color(color, 1.0);
        let frame = match self.begin_frame() {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("webizen-clear-encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&etalon::RenderPassDescriptor {
                label: Some("webizen-clear-pass"),
                color_attachments: &[Some(etalon::RenderPassColorAttachment {
                    view: frame.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: etalon::Operations {
                        load: etalon::LoadOp::Clear(etalon::Color {
                            r: rgba[0] as f64,
                            g: rgba[1] as f64,
                            b: rgba[2] as f64,
                            a: rgba[3] as f64,
                        }),
                        store: etalon::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present(&self.queue);
    }

    /// Project a world-space point to screen space.
    pub fn project(&self, world: Vec3) -> Option<ScreenPoint> {
        self.camera.project(world, self.viewport_size)
    }

    /// Clear tracked node positions (call before each render).
    pub fn clear_node_positions(&mut self) {
        self.node_positions.clear();
    }

    /// Get tracked node positions (for epistemic anchor coordination).
    pub fn get_node_positions(&self) -> &[(usize, ScreenPoint, f64)] {
        &self.node_positions
    }

    /// Hit test: find which node contains the given screen point.
    ///
    /// Returns the binary node index if a hit is found, `None` otherwise.
    /// Zero-heap: uses stack-allocated `f64` values for ray calculations.
    pub fn hit_test(&self, point: ScreenPoint) -> Option<usize> {
        for (index, pos, radius) in self.node_positions.iter().rev() {
            let dx = point.x - pos.x;
            let dy = point.y - pos.y;
            let distance_squared = dx * dx + dy * dy;
            let radius_squared = *radius * *radius;

            if distance_squared <= radius_squared {
                return Some(*index);
            }
        }
        None
    }

    /// Coordinate epistemic anchor at screen coordinates (`x`, `y` in physical pixels).
    pub fn pick_anchor(&self, x: f64, y: f64) -> Option<usize> {
        let point = ScreenPoint { x, y };
        self.hit_test(point)
    }

    /// Legacy method for backward compatibility - use `pick_anchor()` for epistemic anchor coordination.
    #[deprecated(note = "Use pick_anchor() for epistemic anchor coordination")]
    pub fn pick(&self, x: f64, y: f64) -> Option<usize> {
        self.pick_anchor(x, y)
    }

    /// Check if mouse is hovering over epistemic anchor at screen coordinates (`x`, `y` in physical pixels).
    pub fn hover_anchor(&self, x: f64, y: f64) -> Option<usize> {
        let point = ScreenPoint { x, y };
        self.hit_test(point)
    }

    /// Legacy method for backward compatibility - use `hover_anchor()` for epistemic anchor coordination.
    #[deprecated(note = "Use hover_anchor() for epistemic anchor coordination")]
    pub fn hover(&self, x: f64, y: f64) -> Option<usize> {
        self.hover_anchor(x, y)
    }

    /// Draw a screen-space line segment.
    pub fn line(&self, a: ScreenPoint, b: ScreenPoint, color: &str, alpha: f64, width: f64) {
        let rgba = Self::parse_color(color, alpha);

        let width_px = self.viewport_size.0 as f32;
        let height_px = self.viewport_size.1 as f32;

        let x1 = ((a.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
        let y1 = 1.0 - (a.y as f32 / height_px) * 2.0;
        let x2 = ((b.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
        let y2 = 1.0 - (b.y as f32 / height_px) * 2.0;

        let line_width = width as f32 / width_px;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        let nx = -dy / len * line_width;
        let ny = dx / len * line_width;

        let vertices = [
            ScreenVertex {
                position: [x1 + nx, y1 + ny],
                color: rgba,
            },
            ScreenVertex {
                position: [x1 - nx, y1 - ny],
                color: rgba,
            },
            ScreenVertex {
                position: [x2 + nx, y2 + ny],
                color: rgba,
            },
            ScreenVertex {
                position: [x2 - nx, y2 - ny],
                color: rgba,
            },
        ];

        self.render_vertices(&vertices, etalon::PrimitiveTopology::TriangleStrip);
    }

    /// Draw a filled screen-space disc (billboarded point).
    pub fn point(&self, p: ScreenPoint, radius: f64, color: &str, alpha: f64) {
        let rgba = Self::parse_color(color, alpha);

        let width_px = self.viewport_size.0 as f32;
        let height_px = self.viewport_size.1 as f32;

        let cx = ((p.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
        let cy = 1.0 - (p.y as f32 / height_px) * 2.0;
        let r = (radius as f32 / width_px) * 2.0;

        let segments = 16;
        let mut vertices = Vec::with_capacity(segments + 2);
        vertices.push(ScreenVertex {
            position: [cx, cy],
            color: rgba,
        });

        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x = cx + angle.cos() * r;
            let y = cy + angle.sin() * r;
            vertices.push(ScreenVertex {
                position: [x, y],
                color: rgba,
            });
        }

        self.render_vertices(&vertices, etalon::PrimitiveTopology::TriangleList);
    }

    /// Fill a screen-space polygon.
    pub fn fill_polygon(&self, points: &[ScreenPoint], color: &str, alpha: f64) {
        if points.len() < 3 {
            return;
        }

        let rgba = Self::parse_color(color, alpha);

        let width_px = self.viewport_size.0 as f32;
        let height_px = self.viewport_size.1 as f32;

        let vertices: Vec<ScreenVertex> = points
            .iter()
            .map(|p| {
                let x = ((p.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
                let y = 1.0 - (p.y as f32 / height_px) * 2.0;
                ScreenVertex {
                    position: [x, y],
                    color: rgba,
                }
            })
            .collect();

        self.render_vertices(&vertices, etalon::PrimitiveTopology::TriangleList);
    }

    fn render_vertices(&self, vertices: &[ScreenVertex], _topology: etalon::PrimitiveTopology) {
        if vertices.is_empty() || vertices.len() > self.max_vertices {
            return;
        }

        let frame = match self.begin_frame() {
            Ok(frame) => frame,
            Err(_) => return,
        };

        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));

        let mut encoder = self
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("webizen-render-encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&etalon::RenderPassDescriptor {
                label: Some("webizen-render-pass"),
                color_attachments: &[Some(etalon::RenderPassColorAttachment {
                    view: frame.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: etalon::Operations {
                        load: etalon::LoadOp::Load,
                        store: etalon::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present(&self.queue);
    }
}
