//! WebGL2 GLSL ES 3.0 holographic RDF pass. Mobile path: RGBA32F node atlas + instanced cubes.

use js_sys::Float32Array;
use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlTexture, WebGlUniformLocation,
    WebGlVertexArrayObject,
};

use super::webgl2::{compile_shader, link_program};
use super::webgl2_holograph_shaders::{HOLOGRAPH_FRAGMENT, HOLOGRAPH_VERTEX};
use crate::render::gpu::holographic::HolographicNodeGpu;
use crate::render::rdf_holograph::{RdfHolographScene, HOLOGRAPH_TEXELS_PER_NODE};

pub struct WebGl2Holograph {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    dummy: WebGlBuffer,
    texture: WebGlTexture,
    view_projection: WebGlUniformLocation,
    nodes: WebGlUniformLocation,
    eye: WebGlUniformLocation,
    time: WebGlUniformLocation,
    selected: WebGlUniformLocation,
    w_angle: WebGlUniformLocation,
    count: i32,
    internal_format: u32,
}

impl WebGl2Holograph {
    pub fn try_new(gl: &WebGl2RenderingContext) -> Result<Self, JsValue> {
        let vertex = compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, HOLOGRAPH_VERTEX)?;
        let fragment =
            compile_shader(gl, WebGl2RenderingContext::FRAGMENT_SHADER, HOLOGRAPH_FRAGMENT)?;
        let program = link_program(gl, &vertex, &fragment)?;
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_vao"))?;
        let dummy = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_dummy"))?;
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&dummy));
        let zeros = [0.0f32; 36];
        unsafe {
            let view = Float32Array::view(&zeros);
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 1, WebGl2RenderingContext::FLOAT, false, 0, 0);
        gl.bind_vertex_array(None);

        let texture = gl
            .create_texture()
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_texture"))?;
        let view_projection = gl
            .get_uniform_location(&program, "u_view_projection")
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_u_view_projection"))?;
        let nodes = gl
            .get_uniform_location(&program, "u_nodes")
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_u_nodes"))?;
        let eye = gl
            .get_uniform_location(&program, "u_eye")
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_u_eye"))?;
        let time = gl
            .get_uniform_location(&program, "u_time")
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_u_time"))?;
        let selected = gl
            .get_uniform_location(&program, "u_selected")
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_u_selected"))?;
        let w_angle = gl
            .get_uniform_location(&program, "u_w_angle")
            .ok_or_else(|| JsValue::from_str("webgl2_holograph_u_w_angle"))?;

        let _ = gl.get_extension("EXT_color_buffer_float");
        let _ = gl.get_extension("EXT_float_blend");
        let internal_format = probe_node_format(gl, &texture)?;

        Ok(Self {
            program,
            vao,
            dummy,
            texture,
            view_projection,
            nodes,
            eye,
            time,
            selected,
            w_angle,
            count: 0,
            internal_format,
        })
    }

    pub fn count(&self) -> i32 {
        self.count
    }

    pub fn upload_scene(
        &mut self,
        gl: &WebGl2RenderingContext,
        scene: &RdfHolographScene,
    ) -> Result<(), JsValue> {
        self.upload_nodes(gl, scene.nodes())
    }

    pub fn upload_nodes(
        &mut self,
        gl: &WebGl2RenderingContext,
        nodes: &[HolographicNodeGpu],
    ) -> Result<(), JsValue> {
        let zeros = [0.0f32; 24];
        let (width, height, tex): (i32, i32, &[f32]) = if nodes.is_empty() {
            (HOLOGRAPH_TEXELS_PER_NODE as i32, 1, &zeros)
        } else {
            (
                HOLOGRAPH_TEXELS_PER_NODE as i32,
                nodes.len() as i32,
                bytemuck::cast_slice(nodes),
            )
        };
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));
        gl.pixel_storei(WebGl2RenderingContext::UNPACK_ALIGNMENT, 1);
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
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
        unsafe {
            let view = Float32Array::view(tex);
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                self.internal_format as i32,
                width,
                height,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::FLOAT,
                Some(&view),
            )
            .map_err(|_| JsValue::from_str("webgl2_holograph_tex_image"))?;
        }
        self.count = nodes.len() as i32;
        Ok(())
    }

    pub fn render(
        &self,
        gl: &WebGl2RenderingContext,
        view_projection: &[f32],
        eye: [f32; 3],
        time: f32,
        selected: i32,
        w_angle: f32,
    ) {
        if self.count <= 0 {
            return;
        }
        gl.disable(WebGl2RenderingContext::DEPTH_TEST);
        gl.enable(WebGl2RenderingContext::BLEND);
        gl.blend_func_separate(
            WebGl2RenderingContext::SRC_ALPHA,
            WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
            WebGl2RenderingContext::ONE,
            WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
        );
        // Premultiplied-style glass: overlapping crystals composite instead of blowing out.
        gl.depth_mask(false);
        gl.use_program(Some(&self.program));
        gl.bind_vertex_array(Some(&self.vao));
        gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));
        gl.uniform1i(Some(&self.nodes), 0);
        gl.uniform_matrix4fv_with_f32_array(Some(&self.view_projection), false, view_projection);
        gl.uniform3f(Some(&self.eye), eye[0], eye[1], eye[2]);
        gl.uniform1f(Some(&self.time), time);
        gl.uniform1i(Some(&self.selected), selected);
        gl.uniform1f(Some(&self.w_angle), w_angle);
        gl.draw_arrays_instanced(WebGl2RenderingContext::TRIANGLES, 0, 36, self.count);
        gl.bind_vertex_array(None);
        gl.depth_mask(true);
        gl.disable(WebGl2RenderingContext::BLEND);
        gl.enable(WebGl2RenderingContext::DEPTH_TEST);
        let _ = self.dummy;
    }
}

fn probe_node_format(gl: &WebGl2RenderingContext, texture: &WebGlTexture) -> Result<u32, JsValue> {
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
    let probe = [0.0f32; 4];
    unsafe {
        let view = Float32Array::view(&probe);
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA32F as i32,
            1,
            1,
            0,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::FLOAT,
            Some(&view),
        )
        .map_err(|_| JsValue::from_str("webgl2_holograph_rgba32f_unavailable"))?;
    }
    let err = gl.get_error();
    if err == WebGl2RenderingContext::NO_ERROR {
        return Ok(WebGl2RenderingContext::RGBA32F);
    }
    Err(JsValue::from_str(&format!(
        "webgl2_holograph_rgba32f_error_{err:#x}"
    )))
}
