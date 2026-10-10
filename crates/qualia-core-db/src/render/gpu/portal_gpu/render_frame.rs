//! Frame rendering execution, temporal resolve handoff, and presentation for PortalGpu.

use super::super::{
    ambient_draw_instances, create_picking_texture, output_pass, portal_bloom_enabled,
    reserve_view_resources, run_bloom_passes, temporal_resolve_gpu, AmbientUniforms, PortalGpu,
    TemporalProducerViews,
};
use crate::render::telemetry::SystemTelemetry;

impl PortalGpu {
    pub fn render(&mut self, time: f32, telemetry: &SystemTelemetry) -> Result<(), String> {
        self.render_internal(time, telemetry, None).map(|_| ())
    }

    /// Render one frame and submit temporal output when all real host producers are available.
    ///
    /// Scene colour and linear depth remain renderer-owned. The host supplies genuine motion
    /// vectors and a reactive mask; an optional host linear-depth view remains accepted for API
    /// compatibility. If the schedule or any producer is unavailable, this method renders the
    /// ordinary output path and returns `Ok(false)`; no synthetic motion/reactive data is created.
    pub fn render_with_temporal(
        &mut self,
        time: f32,
        telemetry: &SystemTelemetry,
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
        producers: TemporalProducerViews<'_>,
    ) -> Result<bool, String> {
        let has_producers = producers.has_required_host_producers();
        let availability = producers.availability(self.temporal_scene_color_view().is_some());
        let renderer_owns_linear_depth = self.temporal_resolve.is_some();
        let eligible = schedule.is_valid()
            && producers.extent == (self.width, self.height)
            && availability.scene_color
            && has_producers
            && (availability.linear_depth || renderer_owns_linear_depth);
        if !eligible {
            return self
                .render_internal(time, telemetry, None)
                .map(|_| false);
        }
        let schedule = self.effective_temporal_schedule(schedule);
        self.render_internal(time, telemetry, Some((schedule, Some(producers))))
    }

    /// Render one frame with automatic temporal history accumulation using renderer-owned
    /// motion vectors and reactive mask producers.
    pub fn render_with_auto_temporal(
        &mut self,
        time: f32,
        telemetry: &SystemTelemetry,
    ) -> Result<bool, String> {
        if self.temporal_producers.is_none() || self.temporal_resolve.is_none() {
            return self.render_internal(time, telemetry, None).map(|_| false);
        }
        let schedule = crate::render::frame_graph::TemporalOutputSchedule {
            enabled: true,
            reset_history: self.temporal_reset_pending,
            reads_history: !self.temporal_reset_pending && self.temporal_history_valid(),
            publish_history: true,
            final_output: true,
        };
        let schedule = self.effective_temporal_schedule(schedule);
        self.render_internal(time, telemetry, Some((schedule, None)))
    }

    pub(super) fn render_internal(
        &mut self,
        time: f32,
        telemetry: &SystemTelemetry,
        temporal: Option<(
            crate::render::frame_graph::TemporalOutputSchedule,
            Option<TemporalProducerViews<'_>>,
        )>,
    ) -> Result<bool, String> {
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            self.sync_bloom_targets();
        }
        // Create the command encoder first — the uniform belt records copy
        // commands into it, so it must exist before any uniform writes.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("portal-viewport-encoder"),
            });

        // VC3: Write all per-frame uniforms through the pre-allocated
        // uniform belt (zero-alloc after warmup). Each write:
        // 1. Get mapped view from belt
        // 2. Copy uniform data into it
        // 3. Drop view (unmaps buffer)
        // 4. Record copy command into encoder
        // 5. Advance belt to next slot (re-maps oldest slot)
        let uniforms = AmbientUniforms {
            time,
            view_width: self.width as f32,
            view_height: self.height as f32,
            _padding: 0.0,
        };
        {
            let bytes = bytemuck::bytes_of(&uniforms);
            self.uniform_belt.write_and_unmap(bytes);
        }
        self.uniform_belt
            .record_copy(&mut encoder, &self.uniform_buf, 0);
        self.uniform_belt.advance(&self.device);

        {
            let bytes = bytemuck::bytes_of(telemetry);
            self.uniform_belt.write_and_unmap(bytes);
        }
        self.uniform_belt
            .record_copy(&mut encoder, &self.telemetry_buf, 0);
        self.uniform_belt.advance(&self.device);

        self.write_camera_uniform(&mut encoder, time);
        self.water.set_time(time);
        self.water.set_viewport(self.width, self.height);
        self.water.write_uniform(&mut self.uniform_belt, &mut encoder);
        self.uniform_belt.advance(&self.device);
        self.write_observer_uniform(&mut encoder);
        self.update_model(&mut encoder, time);
        self.sort_transparent_draws();
        self.write_shadow_uniform(&mut encoder);

        // A browser target acquires a swapchain frame; a native/headless target keeps a reusable
        // COPY_SRC texture. The draw graph below is identical for both.
        let surface_frame = if let Some(surface) = self.surface.as_ref() {
            match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t)
                | wgpu::CurrentSurfaceTexture::Suboptimal(t) => Some(t),
                other => return Err(format!("surface frame unavailable: {other:?}")),
            }
        } else {
            None
        };

        // On the web backend the swapchain texture tracks the canvas backing store, which can
        // diverge from the size our depth/picking/bloom targets were built at (device-pixel ratio,
        // layout settle, a ResizeObserver resizing in CSS pixels without a matching `resize()`).
        // A depth attachment whose dimensions don't match the colour attachment fails render-pass
        // validation, the whole frame is dropped, and the viewport stays black. Reconcile every
        // attachment to the *actual* acquired texture before recording any pass.
        if let Some(frame) = surface_frame.as_ref() {
            let fw = frame.texture.width();
            let fh = frame.texture.height();
            if fw > 0 && fh > 0 && (fw, fh) != (self.width, self.height) {
                let (_, _, frame_target_reservation, _) = reserve_view_resources(fw, fh, false)?;
                let temporal_was_enabled = self.temporal_resolve.take().is_some();
                self.scene_depth.replace(&self.device, fw, fh);
                if temporal_was_enabled {
                    self.temporal_resolve =
                        temporal_resolve_gpu::TemporalResolveGpu::try_new(
                            &self.device,
                            fw,
                            fh,
                            self.color_format,
                            self.color_format,
                        );
                }
                let (picking_texture, picking_view) = create_picking_texture(&self.device, fw, fh);
                self.width = fw;
                self.height = fh;
                if let Some(config) = self.config.as_mut() {
                    config.width = fw;
                    config.height = fh;
                }
                self.picking_texture = picking_texture;
                self.picking_view = picking_view;
                self._frame_target_reservation = frame_target_reservation;
                self.temporal_reset_pending = true;
                self.rebuild_ao_resources();
                self.sync_bloom_targets();
                self.write_camera_uniform(&mut encoder, time);
            }
        }

        let view = if let Some(frame) = surface_frame.as_ref() {
            // Surface textures are acquired per-frame; we must create a view.
            // (Offscreen views are cached — see below.)
            frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        } else {
            // VC3: Use cached offscreen view instead of create_view per frame.
            self.offscreen_view
                .clone()
                .or_else(|| {
                    // Lazily create the view if it was invalidated (e.g. resize).
                    let v = self
                        .offscreen_texture
                        .as_ref()?
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    self.offscreen_view = Some(v.clone());
                    Some(v)
                })
                .ok_or_else(|| "renderer has no output target".to_string())?
        };

        self.update_mesh_instance_visibility();
        self.write_ao_uniform(&mut encoder);
        self.record_ao_pass(&mut encoder);
        self.record_sun_shadow_pass(&mut encoder);
        self.record_picking_pass(&mut encoder);

        let use_bloom = self.hdr_exposure_available();
        let use_hdr_scene = use_bloom
            || self
                .output_chain
                .as_ref()
                .is_some_and(output_pass::OutputChain::uses_hdr_scene);
        let temporal_scene_view = temporal
            .as_ref()
            .and_then(|_| self.temporal_scene_color_view().cloned());
        let mut temporal_recorded = false;

        if use_hdr_scene {
            let scene_view = temporal_scene_view.as_ref().unwrap_or_else(|| {
                self.bloom
                    .as_ref()
                    .map(|bloom| &bloom.hdr_view)
                    .or_else(|| {
                        self.output_chain
                            .as_ref()
                            .map(output_pass::OutputChain::scene_view)
                    })
                    .unwrap_or(&view)
            });
            let ambient_hdr = self.ambient_pipeline_hdr.as_ref().expect("ambient hdr");
            let projector_hdr = self.projector_pipeline_hdr.as_ref().expect("projector hdr");
            let mesh_hdr = self.mesh_pipeline_hdr.as_ref();

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("portal-hdr-scene"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: scene_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: self.clear_color[0],
                                g: self.clear_color[1],
                                b: self.clear_color[2],
                                a: self.clear_color[3],
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: self.scene_depth.view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    multiview_mask: None,
                    timestamp_writes: None,
                });

                if self.sky_enabled {
                    pass.set_pipeline(
                        self.sky_pipeline_hdr
                            .as_ref()
                            .expect("HDR scene has an HDR sky pipeline"),
                    );
                    pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                    pass.draw(0..3, 0..1);
                }

                if let (Some(mesh), Some(mesh_pipe)) = (self.mesh.as_ref(), mesh_hdr) {
                    pass.set_pipeline(mesh_pipe);
                    pass.set_bind_group(0, &self.mesh_frame_bind, &[]);
                    pass.set_bind_group(1, &self.mesh_model_bind, &[]);
                    pass.set_bind_group(3, self.visible_mesh_instances.bind_group(), &[]);
                    pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                    pass.set_vertex_buffer(1, mesh.color_buf.slice(..));
                    pass.set_vertex_buffer(2, mesh.normal_buf.slice(..));
                    pass.set_vertex_buffer(3, mesh.tangent_buf.slice(..));
                    pass.set_vertex_buffer(4, mesh.uv_buf.slice(..));
                    pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                    for draw in &mesh.material_gpu.draws {
                        if draw.opacity_mode == crate::container_10d::OpacityMode::Blend {
                            continue;
                        }
                        pass.set_bind_group(
                            2,
                            &mesh.material_gpu.material_bind_groups[draw.material_index],
                            &[draw.material_offset],
                        );
                        pass.draw_indexed(
                            draw.first_index..draw.first_index + draw.index_count,
                            0,
                            0..self.visible_mesh_instances.count(),
                        );
                    }
                    self.water.record(
                        &mut pass,
                        &self.mesh_frame_bind,
                        &self.mesh_model_bind,
                        true,
                    );
                    if let Some(blend_pipe) = self.mesh_pipeline_hdr_blend.as_ref() {
                        pass.set_pipeline(blend_pipe);
                        for &draw_index in &mesh.material_gpu.transparent_draw_order {
                            let draw = &mesh.material_gpu.draws[draw_index];
                            pass.set_bind_group(
                                2,
                                &mesh.material_gpu.material_bind_groups[draw.material_index],
                                &[draw.material_offset],
                            );
                            pass.draw_indexed(
                                draw.first_index..draw.first_index + draw.index_count,
                                0,
                                0..self.visible_mesh_instances.count(),
                            );
                        }
                    }
                } else {
                    self.water.record(
                        &mut pass,
                        &self.mesh_frame_bind,
                        &self.mesh_model_bind,
                        true,
                    );
                }

                if self.tensor_projection_enabled {
                    if let (Some(tensor_bind), count) =
                        (self.projector_tensor_bind.as_ref(), self.tensor_node_count)
                    {
                        if count > 0 {
                            pass.set_pipeline(projector_hdr);
                            pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                            pass.set_bind_group(1, tensor_bind, &[]);
                            pass.draw(0..6, 0..count);
                        }
                    }
                }

                pass.set_pipeline(ambient_hdr);
                pass.set_bind_group(0, &self.ambient_bind_group, &[]);
                let ambient_draw = ambient_draw_instances(self.particle_count);
                if self.ambient_enabled && ambient_draw > 0 {
                    pass.draw(0..6, 0..ambient_draw);
                }
            }

        } else {
            let scene_target = temporal_scene_view
                .as_ref()
                .or_else(|| self.output_chain.as_ref().map(output_pass::OutputChain::scene_view))
                .unwrap_or(&view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("portal-phenomenal-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color[0],
                            g: self.clear_color[1],
                            b: self.clear_color[2],
                            a: self.clear_color[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.scene_depth.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });

            if self.sky_enabled {
                pass.set_pipeline(&self.sky_pipeline);
                pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                pass.draw(0..3, 0..1);
            }

            if let Some(mesh) = self.mesh.as_ref() {
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_bind_group(0, &self.mesh_frame_bind, &[]);
                pass.set_bind_group(1, &self.mesh_model_bind, &[]);
                pass.set_bind_group(3, self.visible_mesh_instances.bind_group(), &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                pass.set_vertex_buffer(1, mesh.color_buf.slice(..));
                pass.set_vertex_buffer(2, mesh.normal_buf.slice(..));
                pass.set_vertex_buffer(3, mesh.tangent_buf.slice(..));
                pass.set_vertex_buffer(4, mesh.uv_buf.slice(..));
                pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                for draw in &mesh.material_gpu.draws {
                    if draw.opacity_mode == crate::container_10d::OpacityMode::Blend {
                        continue;
                    }
                    pass.set_bind_group(
                        2,
                        &mesh.material_gpu.material_bind_groups[draw.material_index],
                        &[draw.material_offset],
                    );
                    pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        0,
                        0..self.visible_mesh_instances.count(),
                    );
                }
                self.water.record(
                    &mut pass,
                    &self.mesh_frame_bind,
                    &self.mesh_model_bind,
                    false,
                );
                if !mesh.material_gpu.transparent_draw_order.is_empty() {
                    pass.set_pipeline(&self.mesh_pipeline_blend);
                    for &draw_index in &mesh.material_gpu.transparent_draw_order {
                        let draw = &mesh.material_gpu.draws[draw_index];
                        pass.set_bind_group(
                            2,
                            &mesh.material_gpu.material_bind_groups[draw.material_index],
                            &[draw.material_offset],
                        );
                        pass.draw_indexed(
                            draw.first_index..draw.first_index + draw.index_count,
                            0,
                            0..self.visible_mesh_instances.count(),
                        );
                    }
                }
            } else {
                self.water.record(
                    &mut pass,
                    &self.mesh_frame_bind,
                    &self.mesh_model_bind,
                    false,
                );
            }

            if self.tensor_projection_enabled {
                if let (Some(tensor_bind), count) =
                    (self.projector_tensor_bind.as_ref(), self.tensor_node_count)
                {
                    if count > 0 {
                        pass.set_pipeline(&self.projector_pipeline);
                        pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                        pass.set_bind_group(1, tensor_bind, &[]);
                        pass.draw(0..6, 0..count);
                    }
                }
            }

            pass.set_pipeline(&self.ambient_pipeline);
            pass.set_bind_group(0, &self.ambient_bind_group, &[]);
            let ambient_draw = ambient_draw_instances(self.particle_count);
            if self.ambient_enabled && ambient_draw > 0 {
                pass.draw(0..6, 0..ambient_draw);
            }
        }

        if let Some((schedule, host_producers)) = temporal {
            self.record_temporal_producers(&mut encoder);
            if let Some(producers) = host_producers {
                temporal_recorded = self.record_scheduled_temporal_output_from_producers(
                    &mut encoder,
                    producers,
                    &view,
                    schedule,
                );
            } else {
                temporal_recorded = self.record_scheduled_temporal_output_auto(
                    &mut encoder,
                    &view,
                    schedule,
                );
            }
            if temporal_recorded {
                self.temporal_reset_pending = false;
            }
        }

        if !temporal_recorded {
            if use_hdr_scene {
                if use_bloom {
                    run_bloom_passes(
                        &mut encoder,
                        self.bloom.as_ref().expect("active bloom chain"),
                        &self.queue,
                        &self.device,
                        &view,
                        self.clear_color,
                    );
                } else if let Some(output) = self.output_chain.as_ref() {
                    output.composite(&mut encoder, &view);
                }
            } else if let Some(output) = self.output_chain.as_ref() {
                output.composite(&mut encoder, &view);
            }
        }

        self.record_pick_copy(&mut encoder);

        // VC3: Submit the command buffer. The uniform belt's copy commands
        // are already recorded in the encoder. The belt's advance() calls
        // have already re-mapped the oldest slots for the next frame.
        self.queue.submit(std::iter::once(encoder.finish()));

        if let Some(frame) = surface_frame {
            // wgpu 30: SurfaceTexture::present() removed → Queue::present(frame).
            self.queue.present(frame);
        }
        self.previous_camera_view_projection = Some(self.current_camera_view_projection());
        Ok(temporal_recorded)
    }

    /// Record scheduled temporal output using internal renderer-owned motion vectors and reactive mask.
    pub fn record_scheduled_temporal_output_auto(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
    ) -> bool {
        let schedule = self.effective_temporal_schedule(schedule);
        if !schedule.is_valid() {
            return false;
        }
        let Some(temporal_extent) = self.temporal_resolve.as_ref().map(|owner| owner.extent())
        else {
            return false;
        };
        let extent = (self.width, self.height);
        if extent != temporal_extent
            || (schedule.reads_history && !self.temporal_history_valid())
        {
            return false;
        }
        let Some(scene) = self.temporal_scene_color_view().cloned() else {
            return false;
        };
        let Some(temporal) = self.temporal_resolve.as_ref() else {
            return false;
        };
        if !temporal.scheduled_output_is_admissible(
            extent,
            schedule,
            self.hdr_exposure_ev,
            self.white_balance_gains,
        ) {
            return false;
        }
        let Some(temporal) = self.temporal_resolve.as_mut() else {
            return false;
        };
        if !temporal.record_linear_depth_producer(
            &self.device,
            &self.queue,
            encoder,
            self.scene_depth.view(),
            crate::render::camera::CAMERA_NEAR_PLANE,
            crate::render::camera::CAMERA_FAR_PLANE,
        ) {
            return false;
        }
        let current_linear_depth = temporal.linear_depth_view().clone();
        let Some(producers) = self.temporal_producers.as_ref() else {
            return false;
        };
        let inputs = temporal_resolve_gpu::TemporalResolveInputs {
            extent,
            current_scene: &scene,
            current_linear_depth: &current_linear_depth,
            motion_vectors: producers.motion_view(),
            reactive_mask: producers.reactive_view(),
        };
        let Some(temporal) = self.temporal_resolve.as_mut() else {
            return false;
        };
        let recorded = temporal.record_scheduled_output(
            &self.device,
            &self.queue,
            encoder,
            inputs,
            target,
            schedule,
            self.hdr_exposure_ev,
            self.white_balance_gains,
            self.color_format.is_srgb(),
        );
        if recorded {
            self.temporal_reset_pending = false;
        }
        recorded
    }

    /// Record scheduled temporal output using explicit producer views.
    pub fn record_scheduled_temporal_output_from_producers(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        producers: TemporalProducerViews<'_>,
        target: &wgpu::TextureView,
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
    ) -> bool {
        let schedule = self.effective_temporal_schedule(schedule);
        let has_motion = producers.motion_vectors.is_some();
        let has_reactive = producers.reactive_mask.is_some();
        if !schedule.is_valid() || !has_motion || !has_reactive {
            return false;
        }
        let Some(temporal_extent) = self.temporal_resolve.as_ref().map(|owner| owner.extent())
        else {
            return false;
        };
        if producers.extent != temporal_extent
            || (schedule.reads_history && !self.temporal_history_valid())
        {
            return false;
        }
        let Some(scene) = self.temporal_scene_color_view().cloned() else {
            return false;
        };
        let Some(temporal) = self.temporal_resolve.as_ref() else {
            return false;
        };
        if !temporal.scheduled_output_is_admissible(
            producers.extent,
            schedule,
            self.hdr_exposure_ev,
            self.white_balance_gains,
        ) {
            return false;
        }
        if producers.current_linear_depth.is_none() {
            let Some(temporal) = self.temporal_resolve.as_mut() else {
                return false;
            };
            if !temporal.record_linear_depth_producer(
                &self.device,
                &self.queue,
                encoder,
                self.scene_depth.view(),
                crate::render::camera::CAMERA_NEAR_PLANE,
                crate::render::camera::CAMERA_FAR_PLANE,
            ) {
                return false;
            }
        }
        let Some(temporal) = self.temporal_resolve.as_mut() else {
            return false;
        };
        let current_linear_depth = producers
            .current_linear_depth
            .cloned()
            .unwrap_or_else(|| temporal.linear_depth_view().clone());
        let motion_vectors = match producers.motion_vectors {
            Some(v) => v,
            None => return false,
        };
        let reactive_mask = match producers.reactive_mask {
            Some(v) => v,
            None => return false,
        };
        let inputs = temporal_resolve_gpu::TemporalResolveInputs {
            extent: producers.extent,
            current_scene: &scene,
            current_linear_depth: &current_linear_depth,
            motion_vectors,
            reactive_mask,
        };
        let recorded = temporal.record_scheduled_output(
            &self.device,
            &self.queue,
            encoder,
            inputs,
            target,
            schedule,
            self.hdr_exposure_ev,
            self.white_balance_gains,
            self.color_format.is_srgb(),
        );
        if recorded {
            self.temporal_reset_pending = false;
        }
        recorded
    }
}
