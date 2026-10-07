//! Fullscreen analytic sky for the WebGL2 fallback renderer.

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlUniformLocation, WebGlVertexArrayObject};

use crate::render::camera::{orbit_forward, CAMERA_TAN_HALF_FOV};

const SKY_VERTEX: &str = r#"#version 300 es
precision highp float;
void main() {
    vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    gl_Position = vec4(p * 2.0 - 1.0, 1.0, 1.0);
}
"#;

const SKY_FRAGMENT: &str = r#"#version 300 es
precision highp float;
uniform vec2 u_resolution;
uniform float u_tan_half_fov;
uniform vec3 u_forward;
uniform vec3 u_right;
uniform vec3 u_up;
uniform vec3 u_sun_dir;
uniform float u_sun_int;
uniform float u_amb;
out vec4 out_color;
void main() {
    vec2 ndc = vec2(
        2.0 * gl_FragCoord.x / u_resolution.x - 1.0,
        2.0 * gl_FragCoord.y / u_resolution.y - 1.0
    );
    float aspect = u_resolution.x / u_resolution.y;
    vec3 ray = normalize(u_forward
        + u_right * (ndc.x * aspect * u_tan_half_fov)
        + u_up * (ndc.y * u_tan_half_fov));
    vec3 sun = dot(u_sun_dir, u_sun_dir) > 1e-10
        ? normalize(u_sun_dir) : vec3(0.0, 1.0, 0.0);
    float sun_radiance = max(u_sun_int, 0.0);
    float ambient = clamp(u_amb, 0.0, 1.0);
    float daylight = smoothstep(0.12, 0.38, ambient);
    float dusk = daylight * (1.0 - smoothstep(0.18, 0.62, sun.y));
    vec3 night_zenith = vec3(0.008, 0.018, 0.055);
    vec3 day_zenith = vec3(0.055, 0.22, 0.52);
    vec3 dusk_zenith = vec3(0.12, 0.16, 0.30);
    vec3 night_horizon = vec3(0.035, 0.060, 0.14);
    vec3 day_horizon = vec3(0.58, 0.73, 0.88);
    vec3 dusk_horizon = vec3(0.92, 0.34, 0.14);
    vec3 zenith = mix(mix(night_zenith, day_zenith, daylight), dusk_zenith, dusk);
    vec3 horizon = mix(mix(night_horizon, day_horizon, daylight), dusk_horizon, dusk);
    float horizon_weight = smoothstep(-0.16, 0.56, ray.y);
    float below_horizon = 1.0 - smoothstep(-0.52, -0.015, ray.y);
    vec3 color = mix(mix(horizon, zenith, horizon_weight), mix(vec3(0.035, 0.040, 0.045), horizon, 0.5), below_horizon);
    float sun_cos = dot(ray, sun);
    float disk = smoothstep(cos(0.010), cos(0.004), sun_cos) * smoothstep(-0.08, 0.02, sun.y);
    float halo = pow(max(sun_cos, 0.0), 48.0) * 0.055;
    vec3 sun_color = mix(mix(vec3(0.48, 0.62, 1.0), vec3(1.0, 0.84, 0.64), daylight), vec3(1.0, 0.34, 0.12), dusk);
    color += sun_color * (disk * sun_radiance + halo * min(sun_radiance, 1.5));
    out_color = vec4(color, 1.0);
}
"#;

pub(super) struct WebGl2Sky {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    resolution: WebGlUniformLocation,
    tan_half_fov: WebGlUniformLocation,
    forward: WebGlUniformLocation,
    right: WebGlUniformLocation,
    up: WebGlUniformLocation,
    sun_dir: WebGlUniformLocation,
    sun_int: WebGlUniformLocation,
    ambient: WebGlUniformLocation,
}

impl WebGl2Sky {
    pub fn try_new(gl: &WebGl2RenderingContext) -> Result<Self, JsValue> {
        let vertex =
            super::webgl2::compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, SKY_VERTEX)?;
        let fragment = super::webgl2::compile_shader(
            gl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            SKY_FRAGMENT,
        )?;
        let program = super::webgl2::link_program(gl, &vertex, &fragment)?;
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| JsValue::from_str("webgl2_sky_vao_allocation_failed"))?;
        let uniform = |name: &str| {
            gl.get_uniform_location(&program, name)
                .ok_or_else(|| JsValue::from_str(&format!("webgl2_sky_uniform_missing_{name}")))
        };
        let resolution = uniform("u_resolution")?;
        let tan_half_fov = uniform("u_tan_half_fov")?;
        let forward = uniform("u_forward")?;
        let right = uniform("u_right")?;
        let up = uniform("u_up")?;
        let sun_dir = uniform("u_sun_dir")?;
        let sun_int = uniform("u_sun_int")?;
        let ambient = uniform("u_amb")?;
        Ok(Self {
            program,
            vao,
            resolution,
            tan_half_fov,
            forward,
            right,
            up,
            sun_dir,
            sun_int,
            ambient,
        })
    }

    pub fn render(
        &self,
        gl: &WebGl2RenderingContext,
        yaw: f32,
        pitch: f32,
        sun_dir: [f32; 3],
        sun_intensity: f32,
        ambient: f32,
        width: u32,
        height: u32,
    ) -> Result<(), JsValue> {
        let forward = orbit_forward(yaw, pitch);
        let right = normalize(cross(forward, [0.0, 1.0, 0.0]));
        let up = normalize(cross(right, forward));
        gl.disable(WebGl2RenderingContext::DEPTH_TEST);
        gl.use_program(Some(&self.program));
        gl.bind_vertex_array(Some(&self.vao));
        gl.uniform2f(
            Some(&self.resolution),
            width.max(1) as f32,
            height.max(1) as f32,
        );
        gl.uniform1f(Some(&self.tan_half_fov), CAMERA_TAN_HALF_FOV);
        gl.uniform3f(Some(&self.forward), forward[0], forward[1], forward[2]);
        gl.uniform3f(Some(&self.right), right[0], right[1], right[2]);
        gl.uniform3f(Some(&self.up), up[0], up[1], up[2]);
        gl.uniform3f(Some(&self.sun_dir), sun_dir[0], sun_dir[1], sun_dir[2]);
        gl.uniform1f(Some(&self.sun_int), sun_intensity);
        gl.uniform1f(Some(&self.ambient), ambient);
        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
        gl.bind_vertex_array(None);
        gl.enable(WebGl2RenderingContext::DEPTH_TEST);
        let error = gl.get_error();
        if error != WebGl2RenderingContext::NO_ERROR {
            return Err(JsValue::from_str(&format!(
                "webgl2_sky_draw_error_{error:#x}"
            )));
        }
        Ok(())
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length > 1e-8 {
        [v[0] / length, v[1] / length, v[2] / length]
    } else {
        [1.0, 0.0, 0.0]
    }
}
