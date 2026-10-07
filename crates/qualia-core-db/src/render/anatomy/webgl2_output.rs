//! Budgeted WebGL2 scene target and the shared versioned SDR output transform.

use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, WebGlFramebuffer, WebGlProgram, WebGlRenderbuffer, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

use super::webgl2_output_shaders::{OUTPUT_FRAGMENT, OUTPUT_VERTEX};
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};

struct SceneTarget {
    framebuffer: WebGlFramebuffer,
    color: WebGlTexture,
    depth: WebGlRenderbuffer,
    width: u32,
    height: u32,
    hdr: bool,
    _reservation: VramReservation<'static>,
}

impl SceneTarget {
    fn try_new(gl: &WebGl2RenderingContext, width: u32, height: u32, hdr: bool) -> Option<Self> {
        let width_i32 = i32::try_from(width).ok()?;
        let height_i32 = i32::try_from(height).ok()?;
        let color_bytes = if hdr { 8_u64 } else { 4_u64 };
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))?
            .checked_mul(color_bytes.checked_add(4)?)?;
        let reservation = global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::FrameTarget, bytes)
            .ok()?;
        let framebuffer = gl.create_framebuffer()?;
        let Some(color) = gl.create_texture() else {
            gl.delete_framebuffer(Some(&framebuffer));
            return None;
        };
        let Some(depth) = gl.create_renderbuffer() else {
            gl.delete_framebuffer(Some(&framebuffer));
            gl.delete_texture(Some(&color));
            return None;
        };

        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&color));
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        let allocation = if hdr {
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA16F as i32,
                width_i32,
                height_i32,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::HALF_FLOAT,
                None,
            )
        } else {
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA8 as i32,
                width_i32,
                height_i32,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                None,
            )
        };
        if allocation.is_err() {
            delete_target_objects(gl, framebuffer, color, depth);
            return None;
        }

        gl.bind_renderbuffer(WebGl2RenderingContext::RENDERBUFFER, Some(&depth));
        gl.renderbuffer_storage(
            WebGl2RenderingContext::RENDERBUFFER,
            WebGl2RenderingContext::DEPTH_COMPONENT24,
            width_i32,
            height_i32,
        );
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&framebuffer));
        gl.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT0,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&color),
            0,
        );
        gl.framebuffer_renderbuffer(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::DEPTH_ATTACHMENT,
            WebGl2RenderingContext::RENDERBUFFER,
            Some(&depth),
        );
        let complete = gl.check_framebuffer_status(WebGl2RenderingContext::FRAMEBUFFER)
            == WebGl2RenderingContext::FRAMEBUFFER_COMPLETE;
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        gl.bind_renderbuffer(WebGl2RenderingContext::RENDERBUFFER, None);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
        if !complete {
            delete_target_objects(gl, framebuffer, color, depth);
            return None;
        }
        Some(Self {
            framebuffer,
            color,
            depth,
            width,
            height,
            hdr,
            _reservation: reservation,
        })
    }
}

fn delete_target_objects(
    gl: &WebGl2RenderingContext,
    framebuffer: WebGlFramebuffer,
    color: WebGlTexture,
    depth: WebGlRenderbuffer,
) {
    gl.delete_framebuffer(Some(&framebuffer));
    gl.delete_texture(Some(&color));
    gl.delete_renderbuffer(Some(&depth));
}

fn discard_setup_errors(gl: &WebGl2RenderingContext) {
    // A failed optional float allocation must not poison the following RGBA8
    // fallback or make the caller attribute setup errors to its scene draw.
    for _ in 0..16 {
        if gl.get_error() == WebGl2RenderingContext::NO_ERROR {
            break;
        }
    }
}

pub(super) struct WebGl2Output {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    scene: Option<SceneTarget>,
    hdr_supported: bool,
    exposure: WebGlUniformLocation,
    sampler: WebGlUniformLocation,
    white_balance: WebGlUniformLocation,
    exposure_scale: f32,
    white_balance_gains: [f32; 3],
}

impl WebGl2Output {
    pub(super) fn try_new(gl: &WebGl2RenderingContext) -> Result<Self, JsValue> {
        let vertex = super::webgl2::compile_shader(
            gl,
            WebGl2RenderingContext::VERTEX_SHADER,
            OUTPUT_VERTEX,
        )?;
        let fragment = super::webgl2::compile_shader(
            gl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            OUTPUT_FRAGMENT,
        )?;
        let program = super::webgl2::link_program(gl, &vertex, &fragment)?;
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| JsValue::from_str("webgl2_output_vao_allocation_failed"))?;
        let exposure = gl
            .get_uniform_location(&program, "u_exposure")
            .ok_or_else(|| JsValue::from_str("webgl2_output_exposure_uniform_missing"))?;
        let sampler = gl
            .get_uniform_location(&program, "u_scene")
            .ok_or_else(|| JsValue::from_str("webgl2_output_scene_uniform_missing"))?;
        let white_balance = gl
            .get_uniform_location(&program, "u_white_balance_gains")
            .ok_or_else(|| JsValue::from_str("webgl2_output_white_balance_uniform_missing"))?;
        let hdr_supported = gl.get_extension("EXT_color_buffer_float")?.is_some();
        Ok(Self {
            program,
            vao,
            scene: None,
            hdr_supported,
            exposure,
            sampler,
            white_balance,
            exposure_scale: crate::render::output::exposure_scale_from_ev(
                crate::render::output::DEFAULT_HDR_EXPOSURE_EV,
            )
            .unwrap_or(1.0),
            white_balance_gains: crate::render::output::DEFAULT_WHITE_BALANCE_GAINS,
        })
    }

    pub(super) fn begin_scene(
        &mut self,
        gl: &WebGl2RenderingContext,
        width: u32,
        height: u32,
    ) -> bool {
        let width = width.max(1);
        let height = height.max(1);
        let current_matches = self
            .scene
            .as_ref()
            .is_some_and(|target| target.width == width && target.height == height);
        if !current_matches {
            // The previous-size target cannot service this frame. Release its
            // admission before reserving the replacement, avoiding a transient
            // double-buffer spike at a resize boundary.
            if let Some(previous) = self.scene.take() {
                delete_target_objects(gl, previous.framebuffer, previous.color, previous.depth);
            }
            let mut replacement = if self.hdr_supported {
                let hdr = SceneTarget::try_new(gl, width, height, true);
                if hdr.is_none() {
                    discard_setup_errors(gl);
                }
                hdr
            } else {
                None
            };
            if replacement.is_none() {
                replacement = SceneTarget::try_new(gl, width, height, false);
            }
            if let Some(replacement) = replacement {
                self.scene = Some(replacement);
            } else {
                discard_setup_errors(gl);
            }
        }
        if let Some(target) = self
            .scene
            .as_ref()
            .filter(|target| target.width == width && target.height == height)
        {
            gl.bind_framebuffer(
                WebGl2RenderingContext::FRAMEBUFFER,
                Some(&target.framebuffer),
            );
            true
        } else {
            gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
            false
        }
    }

    pub(super) fn finish_scene(&self, gl: &WebGl2RenderingContext) {
        let Some(scene) = &self.scene else { return };
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        gl.viewport(0, 0, scene.width as i32, scene.height as i32);
        gl.disable(WebGl2RenderingContext::DEPTH_TEST);
        gl.disable(WebGl2RenderingContext::BLEND);
        gl.use_program(Some(&self.program));
        gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&scene.color));
        gl.uniform1i(Some(&self.sampler), 0);
        gl.uniform1f(Some(&self.exposure), self.exposure_scale);
        gl.uniform3f(
            Some(&self.white_balance),
            self.white_balance_gains[0],
            self.white_balance_gains[1],
            self.white_balance_gains[2],
        );
        gl.bind_vertex_array(Some(&self.vao));
        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
        gl.bind_vertex_array(None);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
        gl.enable(WebGl2RenderingContext::DEPTH_TEST);
    }

    pub(super) fn set_exposure(&mut self, ev: f32) -> bool {
        if crate::render::output::exposure_scale_from_ev(ev).is_none() {
            return false;
        }
        self.exposure_scale = crate::render::output::exposure_scale_from_ev(ev).unwrap_or(1.0);
        true
    }

    pub(super) fn set_white_balance(&mut self, gains: [f32; 3]) {
        self.white_balance_gains = gains;
    }

    pub(super) fn transform_available(&self) -> bool {
        self.scene.is_some()
    }

    pub(super) fn hdr_scene_available(&self) -> bool {
        self.scene.as_ref().is_some_and(|scene| scene.hdr)
    }
}
