//! Hardware WebGL2 fallback for browser Anatomy.
//!
//! This backend exists for browsers which expose accelerated WebGL2 while
//! suppressing every WebGPU adapter. Decoded mesh storage stays in Rust and is
//! uploaded directly into WebGL buffers.

use js_sys::{Float32Array, Uint32Array};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

use crate::render::camera::orbit_view_projection_target;

const VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec4 a_color;
uniform mat4 u_view_projection;
out vec3 v_world;
out vec4 v_color;
void main() {
    v_world = a_position;
    v_color = a_color;
    gl_Position = u_view_projection * vec4(a_position, 1.0);
}
"#;

// Same flat shade as the WebGPU mesh shader (mesh.wgsl): face normal from
// screen derivatives, one sun, ambient, a thin rim. Depth test is on, so
// this is the lit frame, not a painter of flat facets.
const FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec3 v_world;
in vec4 v_color;
uniform vec3 u_sun_dir;
uniform float u_sun_int;
uniform float u_amb;
out vec4 out_color;
void main() {
    vec3 n = normalize(cross(dFdx(v_world), dFdy(v_world)));
    vec3 sun = u_sun_dir;
    if (dot(sun, sun) < 0.01) {
        sun = vec3(0.45, 0.8, 0.55);
    }
    vec3 key = normalize(sun);
    float sun_int = u_sun_int > 0.0 ? u_sun_int : 1.0;
    float amb = u_amb > 0.0 ? u_amb : 0.25;
    float diffuse = clamp(dot(n, key), 0.0, 1.0) * sun_int;
    float facing = clamp(abs(n.z), 0.0, 1.0);
    float rim = pow(1.0 - facing, 2.0);
    vec3 col = v_color.rgb * (amb + diffuse) + vec3(0.10, 0.14, 0.22) * rim;
    out_color = vec4(col, v_color.a);
}
"#;

pub struct AnatomyWebGl2 {
    gl: WebGl2RenderingContext,
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    position_buffer: WebGlBuffer,
    color_buffer: WebGlBuffer,
    index_buffer: WebGlBuffer,
    view_projection: WebGlUniformLocation,
    sun_dir: WebGlUniformLocation,
    sun_int: WebGlUniformLocation,
    amb: WebGlUniformLocation,
    index_count: i32,
    frame_count: u32,
}

impl AnatomyWebGl2 {
    pub fn try_new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"alpha".into(), &false.into())?;
        js_sys::Reflect::set(&options, &"antialias".into(), &true.into())?;
        js_sys::Reflect::set(&options, &"depth".into(), &true.into())?;
        js_sys::Reflect::set(
            &options,
            &"failIfMajorPerformanceCaveat".into(),
            &false.into(),
        )?;
        let gl = canvas
            .get_context_with_context_options("webgl2", &options)?
            .ok_or_else(|| JsValue::from_str("webgl2_context_unavailable"))?
            .dyn_into::<WebGl2RenderingContext>()?;

        let vertex = compile_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, VERTEX_SHADER)?;
        let fragment = compile_shader(
            &gl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            FRAGMENT_SHADER,
        )?;
        let program = link_program(&gl, &vertex, &fragment)?;
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| JsValue::from_str("webgl2_vertex_array_allocation_failed"))?;
        let position_buffer = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("webgl2_position_buffer_allocation_failed"))?;
        let color_buffer = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("webgl2_color_buffer_allocation_failed"))?;
        let index_buffer = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("webgl2_index_buffer_allocation_failed"))?;
        let view_projection = gl
            .get_uniform_location(&program, "u_view_projection")
            .ok_or_else(|| JsValue::from_str("webgl2_view_projection_uniform_missing"))?;
        let sun_dir = gl
            .get_uniform_location(&program, "u_sun_dir")
            .ok_or_else(|| JsValue::from_str("webgl2_sun_uniform_missing"))?;
        let sun_int = gl
            .get_uniform_location(&program, "u_sun_int")
            .ok_or_else(|| JsValue::from_str("webgl2_sun_int_uniform_missing"))?;
        let amb = gl
            .get_uniform_location(&program, "u_amb")
            .ok_or_else(|| JsValue::from_str("webgl2_amb_uniform_missing"))?;

        gl.enable(WebGl2RenderingContext::DEPTH_TEST);
        gl.depth_func(WebGl2RenderingContext::LEQUAL);
        gl.disable(WebGl2RenderingContext::BLEND);

        Ok(Self {
            gl,
            program,
            vao,
            position_buffer,
            color_buffer,
            index_buffer,
            view_projection,
            sun_dir,
            sun_int,
            amb,
            index_count: 0,
            frame_count: 0,
        })
    }

    pub fn upload_mesh(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        indices: &[u32],
    ) -> Result<(), JsValue> {
        if positions.is_empty() || indices.is_empty() || positions.len() != colors.len() {
            return Err(JsValue::from_str("webgl2_body_mesh_empty_or_mismatched"));
        }
        self.index_count = i32::try_from(indices.len())
            .map_err(|_| JsValue::from_str("webgl2_body_index_count_exceeds_i32"))?;
        self.frame_count = 0;

        self.gl.bind_vertex_array(Some(&self.vao));

        self.gl.bind_buffer(
            WebGl2RenderingContext::ARRAY_BUFFER,
            Some(&self.position_buffer),
        );
        unsafe {
            let values = Float32Array::view(bytemuck::cast_slice(positions));
            self.gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &values,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        self.gl.enable_vertex_attrib_array(0);
        self.gl
            .vertex_attrib_pointer_with_i32(0, 3, WebGl2RenderingContext::FLOAT, false, 0, 0);

        self.gl.bind_buffer(
            WebGl2RenderingContext::ARRAY_BUFFER,
            Some(&self.color_buffer),
        );
        unsafe {
            let values = Float32Array::view(bytemuck::cast_slice(colors));
            self.gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &values,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        self.gl.enable_vertex_attrib_array(1);
        self.gl
            .vertex_attrib_pointer_with_i32(1, 4, WebGl2RenderingContext::FLOAT, false, 0, 0);

        self.gl.bind_buffer(
            WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
            Some(&self.index_buffer),
        );
        unsafe {
            let values = Uint32Array::view(indices);
            self.gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
                &values,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        self.gl.bind_vertex_array(None);
        Ok(())
    }

    pub fn render(
        &mut self,
        yaw: f32,
        pitch: f32,
        zoom: f32,
        target: [f32; 3],
        sun_dir: [f32; 3],
        sun_intensity: f32,
        ambient: f32,
        width: u32,
        height: u32,
    ) -> Result<(), JsValue> {
        if self.gl.is_context_lost() {
            return Err(JsValue::from_str("webgl2_context_lost"));
        }
        let width = width.max(1);
        let height = height.max(1);
        self.gl.viewport(0, 0, width as i32, height as i32);
        self.gl.clear_color(0.008, 0.012, 0.025, 1.0);
        self.gl.clear_depth(1.0);
        self.gl.clear(
            WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
        );
        if self.index_count == 0 {
            return Ok(());
        }

        let matrix = orbit_view_projection_target(
            yaw,
            pitch,
            zoom,
            target,
            width as f32 / height as f32,
        );
        let flat: &[f32] = bytemuck::cast_slice(&matrix);
        self.gl.use_program(Some(&self.program));
        self.gl.bind_vertex_array(Some(&self.vao));
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.view_projection), false, flat);
        self.gl.uniform3f(Some(&self.sun_dir), sun_dir[0], sun_dir[1], sun_dir[2]);
        self.gl.uniform1f(Some(&self.sun_int), sun_intensity);
        self.gl.uniform1f(Some(&self.amb), ambient);
        self.gl.draw_elements_with_i32(
            WebGl2RenderingContext::TRIANGLES,
            self.index_count,
            WebGl2RenderingContext::UNSIGNED_INT,
            0,
        );
        self.gl.bind_vertex_array(None);

        let error = self.gl.get_error();
        if error != WebGl2RenderingContext::NO_ERROR {
            return Err(JsValue::from_str(&format!("webgl2_draw_error_{error:#x}")));
        }
        self.frame_count = self.frame_count.saturating_add(1);
        Ok(())
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Same span write the WebGPU mesh uses, so a walk on this path moves
    /// the person instead of leaving a frozen upload.
    pub fn write_vertices(&mut self, start: u32, positions: &[[f32; 3]]) {
        if positions.is_empty() {
            return;
        }
        self.gl.bind_buffer(
            WebGl2RenderingContext::ARRAY_BUFFER,
            Some(&self.position_buffer),
        );
        unsafe {
            let values = Float32Array::view(bytemuck::cast_slice(positions));
            self.gl.buffer_sub_data_with_i32_and_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                (start as i32).saturating_mul(12),
                &values,
            );
        }
    }
}

fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, JsValue> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| JsValue::from_str("webgl2_shader_allocation_failed"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(JsValue::from_str(
            &gl.get_shader_info_log(&shader)
                .unwrap_or_else(|| "webgl2_shader_compile_failed".to_string()),
        ))
    }
}

fn link_program(
    gl: &WebGl2RenderingContext,
    vertex: &WebGlShader,
    fragment: &WebGlShader,
) -> Result<WebGlProgram, JsValue> {
    let program = gl
        .create_program()
        .ok_or_else(|| JsValue::from_str("webgl2_program_allocation_failed"))?;
    gl.attach_shader(&program, vertex);
    gl.attach_shader(&program, fragment);
    gl.link_program(&program);
    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(JsValue::from_str(
            &gl.get_program_info_log(&program)
                .unwrap_or_else(|| "webgl2_program_link_failed".to_string()),
        ))
    }
}
