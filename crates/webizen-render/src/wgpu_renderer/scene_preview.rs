//! Headless native scene preview rendering (`render_preview_png`, `render_scene_png`, etc.).

#[cfg(not(feature = "qualia"))]
use super::pipelines::{AmbientUniforms, ScreenVertex};
#[cfg(not(feature = "qualia"))]
use super::vegetation_effects;
use super::{ScreenPoint, WgpuRenderer};

/// Render a placeholder semantic scene headlessly and return raw PNG bytes.
///
/// This is **blocking** — it builds a GPU device and drives a synchronous readback
/// (`device.poll(Wait)`), so it must be called off the UI thread (e.g. a worker
/// thread or `spawn_blocking`). Intended as the one-call bridge for the native
/// host until a real `SemanticScene` is wired in. Native-only.
pub fn render_preview_png(width: u32, height: u32) -> Option<Vec<u8>> {
    let renderer = pollster::block_on(WgpuRenderer::new_offscreen(width, height)).ok()?;
    renderer.clear("#101820");
    // A simple cyan triangle so there is visible, non-trivial content to confirm.
    let (w, h) = (width as f64, height as f64);
    renderer.fill_polygon(
        &[
            ScreenPoint {
                x: w * 0.50,
                y: h * 0.12,
            },
            ScreenPoint {
                x: w * 0.12,
                y: h * 0.85,
            },
            ScreenPoint {
                x: w * 0.88,
                y: h * 0.85,
            },
        ],
        "#67e8f9",
        1.0,
    );
    renderer.read_png()
}

/// Render a RenderScene contract headlessly and return raw PNG bytes.
///
/// This is **blocking** — it builds a GPU device and drives a synchronous readback
/// (`device.poll(Wait)`), so it must be called off the UI thread (e.g. a worker
/// thread or `spawn_blocking`). Native-only.
pub fn render_scene_png(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    render_scene_png_with_time(scene, width, height, 0.0)
}

/// Render scene with time parameter for animation effects.
/// Time is in seconds, used for pulsing/glowing animations.
pub fn render_scene_png_with_time(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
    time_seconds: f64,
) -> Option<Vec<u8>> {
    render_scene_png_with_time_and_telemetry(
        scene,
        width,
        height,
        time_seconds,
        &crate::scene_contract::SystemTelemetry::default(),
    )
}

/// Render scene with time parameter and telemetry for ambient visualization.
/// Time is in seconds, used for pulsing/glowing animations.
/// Telemetry drives the ambient particle visualization effects.
pub fn render_scene_png_with_time_and_telemetry(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
    time_seconds: f64,
    telemetry: &crate::scene_contract::SystemTelemetry,
) -> Option<Vec<u8>> {
    #[cfg(feature = "qualia")]
    {
        return crate::volumetric::render_scene_png(
            scene,
            width,
            height,
            time_seconds as f32,
            telemetry,
        )
        .ok();
    }

    #[cfg(not(feature = "qualia"))]
    {
        #[inline]
        fn lerp(a: f64, b: f64, t: f64) -> f64 {
            a + (b - a) * t
        }

        let mut renderer = pollster::block_on(WgpuRenderer::new_offscreen(width, height)).ok()?;

        let (w, h) = (width as f64, height as f64);

        // Clear node positions tracking
        renderer.clear_node_positions();

        // Build a lookup map for previous positions if transition is active
        let previous_positions_map: std::collections::HashMap<
            String,
            crate::scene_contract::ScenePoint,
        > = if let Some(ref transition) = scene.transition_state {
            transition.previous_positions.iter().cloned().collect()
        } else {
            std::collections::HashMap::new()
        };

        // Get transition progress (default to 1.0 if no transition)
        let transition_progress = scene
            .transition_state
            .as_ref()
            .map(|t| t.progress)
            .unwrap_or(1.0);

        // Begin single render pass for the entire frame
        let frame = renderer.begin_frame().ok()?;
        let frame_view = frame.view();

        // Parse background color
        let bg_rgba = WgpuRenderer::parse_color(&scene.background, 1.0);

        let mut encoder = renderer
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("scene-render-encoder"),
            });

        // Render ambient visualization first (background layer)
        let ambient_bind_group = if renderer.ambient_config.enabled {
            if let (
                Some(ambient_pipeline),
                Some(particle_buffer),
                Some(ambient_uniform_buffer),
                Some(telemetry_buffer),
            ) = (
                &renderer.ambient_pipeline,
                &renderer.particle_buffer,
                &renderer.ambient_uniform_buffer,
                &renderer.telemetry_buffer,
            ) {
                let uniforms = AmbientUniforms {
                    time: time_seconds as f32,
                    view_width: w as f32,
                    view_height: h as f32,
                    world_frame: vegetation_effects::WorldFrame::from_time(
                        time_seconds as f32,
                        vegetation_effects::DEFAULT_WORLD_SEED,
                    )
                    .index,
                };
                renderer.queue.write_buffer(
                    ambient_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&uniforms),
                );

                renderer
                    .queue
                    .write_buffer(telemetry_buffer, 0, bytemuck::bytes_of(telemetry));

                Some(
                    renderer
                        .device
                        .create_bind_group(&etalon::BindGroupDescriptor {
                            label: Some("ambient-bind-group"),
                            layout: &ambient_pipeline.get_bind_group_layout(0),
                            entries: &[
                                etalon::BindGroupEntry {
                                    binding: 0,
                                    resource: ambient_uniform_buffer.as_entire_binding(),
                                },
                                etalon::BindGroupEntry {
                                    binding: 1,
                                    resource: telemetry_buffer.as_entire_binding(),
                                },
                                etalon::BindGroupEntry {
                                    binding: 2,
                                    resource: particle_buffer.as_entire_binding(),
                                },
                            ],
                        }),
                )
            } else {
                None
            }
        } else {
            None
        };

        {
            let mut render_pass = encoder.begin_render_pass(&etalon::RenderPassDescriptor {
                label: Some("scene-render-pass"),
                color_attachments: &[Some(etalon::RenderPassColorAttachment {
                    view: frame_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: etalon::Operations {
                        load: etalon::LoadOp::Clear(etalon::Color {
                            r: bg_rgba[0] as f64,
                            g: bg_rgba[1] as f64,
                            b: bg_rgba[2] as f64,
                            a: bg_rgba[3] as f64,
                        }),
                        store: etalon::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Render ambient visualization first (background layer)
            if let Some(ref bind_group) = ambient_bind_group {
                if let Some(ambient_pipeline) = &renderer.ambient_pipeline {
                    render_pass.set_pipeline(ambient_pipeline);
                    render_pass.set_bind_group(0, bind_group, &[]);
                    render_pass.draw(0..6, 0..renderer.particle_count as u32);
                }
            }

            // Render faces (filled polygons) first (background layer)
            for face in &scene.faces {
                let vertices: Vec<ScreenPoint> = face
                    .vertices
                    .iter()
                    .map(|v| {
                        let interpolated = if let Some(ref prev_pos) =
                            previous_positions_map.get(&format!(
                                "face_{}",
                                face.vertices.iter().position(|p| p == v).unwrap_or(0)
                            )) {
                            crate::scene_contract::ScenePoint {
                                x: lerp(prev_pos.x, v.x, transition_progress),
                                y: lerp(prev_pos.y, v.y, transition_progress),
                                z: lerp(prev_pos.z, v.z, transition_progress),
                            }
                        } else {
                            *v
                        };
                        ScreenPoint {
                            x: interpolated.x * w,
                            y: interpolated.y * h,
                        }
                    })
                    .collect();

                let width_px = w as f32;
                let height_px = h as f32;
                let clip_vertices: Vec<ScreenVertex> = vertices
                    .iter()
                    .map(|p| {
                        let x = ((p.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
                        let y = 1.0 - (p.y as f32 / height_px) * 2.0;
                        let rgba = WgpuRenderer::parse_color(&face.color, face.alpha);
                        ScreenVertex {
                            position: [x, y],
                            color: rgba,
                        }
                    })
                    .collect();

                if !clip_vertices.is_empty() && clip_vertices.len() <= renderer.max_vertices {
                    renderer.queue.write_buffer(
                        &renderer.vertex_buffer,
                        0,
                        bytemuck::cast_slice(&clip_vertices),
                    );

                    if let Some(projector) = &renderer.projector_pipeline {
                        render_pass.set_pipeline(projector);
                        render_pass
                            .set_vertex_buffer(0, renderer.projector_vertex_buffer.slice(..));
                    } else {
                        render_pass.set_pipeline(&renderer.render_pipeline);
                        render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
                    }

                    render_pass.draw(0..clip_vertices.len() as u32, 0..1);
                }
            }

            // Render edges (lines)
            for edge in &scene.edges {
                let from_interpolated = if let Some(ref prev_pos) =
                    previous_positions_map.get(&format!("edge_from_{}", edge.from.x))
                {
                    crate::scene_contract::ScenePoint {
                        x: lerp(prev_pos.x, edge.from.x, transition_progress),
                        y: lerp(prev_pos.y, edge.from.y, transition_progress),
                        z: lerp(prev_pos.z, edge.from.z, transition_progress),
                    }
                } else {
                    edge.from
                };

                let to_interpolated = if let Some(ref prev_pos) =
                    previous_positions_map.get(&format!("edge_to_{}", edge.to.x))
                {
                    crate::scene_contract::ScenePoint {
                        x: lerp(prev_pos.x, edge.to.x, transition_progress),
                        y: lerp(prev_pos.y, edge.to.y, transition_progress),
                        z: lerp(prev_pos.z, edge.to.z, transition_progress),
                    }
                } else {
                    edge.to
                };

                renderer.line(
                    ScreenPoint {
                        x: from_interpolated.x * w,
                        y: from_interpolated.y * h,
                    },
                    ScreenPoint {
                        x: to_interpolated.x * w,
                        y: to_interpolated.y * h,
                    },
                    &edge.color,
                    edge.width,
                    edge.alpha,
                );
            }

            // Render nodes (vertices) last (foreground layer)
            for (node_index, node) in scene.nodes.iter().enumerate() {
                let interpolated_position =
                    if let Some(ref prev_pos) = previous_positions_map.get(&node.id) {
                        crate::scene_contract::ScenePoint {
                            x: lerp(prev_pos.x, node.position.x, transition_progress),
                            y: lerp(prev_pos.y, node.position.y, transition_progress),
                            z: lerp(prev_pos.z, node.position.z, transition_progress),
                        }
                    } else {
                        node.position
                    };

                let is_selected = scene.selected_node_index == Some(node_index);
                let is_hovered = scene.hovered_node_index == Some(node_index);

                let animated_radius = if node.is_inferencing && node.pulse_rate > 0.0 {
                    let pulse_phase = 2.0 * std::f64::consts::PI * node.pulse_rate * time_seconds;
                    let pulse_factor = 1.0 + 0.3 * pulse_phase.sin();
                    node.radius * pulse_factor.abs()
                } else {
                    node.radius
                };

                let spectral_color = if node.tensor.sigma != 0.0 {
                    node.tensor.spectral_to_color()
                } else {
                    node.color.clone()
                };

                let (final_radius, final_color) = if is_selected {
                    (animated_radius * 1.5, "#ffffff".to_string())
                } else if is_hovered {
                    (animated_radius * 1.2, "#ffff00".to_string())
                } else {
                    (animated_radius, spectral_color)
                };

                let screen_pos = ScreenPoint {
                    x: interpolated_position.x * w,
                    y: interpolated_position.y * h,
                };

                renderer.point(screen_pos, final_radius, &final_color, node.alpha);

                renderer
                    .node_positions
                    .push((node_index, screen_pos, final_radius));
            }
        }

        renderer.queue.submit(Some(encoder.finish()));
        frame.present(&renderer.queue);

        renderer.read_png()
    }
}

/// Same as [`render_scene_png`], returned as a `data:image/png;base64,...`
/// URI for hosts that prefer inline delivery. Blocking; native-only.
pub fn render_scene_data_uri(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
) -> Option<String> {
    use base64::Engine;
    let png = render_scene_png(scene, width, height)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!("data:image/png;base64,{b64}"))
}

/// Same preview as [`render_preview_png`], returned as a `data:image/png;base64,...`
/// URI for hosts that prefer inline delivery. Blocking; native-only.
pub fn render_preview_data_uri(width: u32, height: u32) -> Option<String> {
    use base64::Engine;
    let png = render_preview_png(width, height)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!("data:image/png;base64,{b64}"))
}
