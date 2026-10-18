// SGGX Anisotropic Microflake Surface BRDF (`sggx_microflake.wgsl`)
//
// Continuous distant ground cover (LOD 3, >80m) optical model:
// - Analytically integrates billions of microscopic grass blade / foliage orientations
//   into a 3D symmetric ellipsoid distribution matrix S (SGGX phase function).
// - Erect high-turgor grass: tangent fiber T = [0, 1, 0] produces velvet-like rim scattering at grazing angles.
// - Wilted or trampled grass: tangent tilts into the horizontal plane, altering specular sheen identically
//   to near-field 3D geometry without rendering sub-pixel blade triangles.

struct SggxParams {
    light_dir: vec4<f32>,       // xyz: normalized light direction towards light, w: light intensity
    view_pos: vec4<f32>,        // xyz: camera world position, w: turgor pressure [0.0, 1.5]
    fiber_tangent: vec4<f32>,   // xyz: macroscopic culm fiber orientation, w: trample matted ratio
    roughness: vec4<f32>,       // x: longitudinal roughness, y: transversal roughness, z: albedo scale, w: pad
};

@group(0) @binding(0) var<uniform> sggx: SggxParams;

const PI: f32 = 3.14159265359;

// Constructs the 3x3 symmetric SGGX matrix S from fiber tangent T and roughness parameters (sigma_v, sigma_t).
// S = sigma_v² (T ⊗ T) + sigma_t² (I - T ⊗ T)
fn compute_sggx_matrix(T: vec3<f32>, sigma_v: f32, sigma_t: f32) -> mat3x3<f32> {
    let sv2 = sigma_v * sigma_v;
    let st2 = sigma_t * sigma_t;
    let t_norm = normalize(T);

    let m00 = sv2 * t_norm.x * t_norm.x + st2 * (1.0 - t_norm.x * t_norm.x);
    let m01 = (sv2 - st2) * t_norm.x * t_norm.y;
    let m02 = (sv2 - st2) * t_norm.x * t_norm.z;

    let m10 = m01;
    let m11 = sv2 * t_norm.y * t_norm.y + st2 * (1.0 - t_norm.y * t_norm.y);
    let m12 = (sv2 - st2) * t_norm.y * t_norm.z;

    let m20 = m02;
    let m21 = m12;
    let m22 = sv2 * t_norm.z * t_norm.z + st2 * (1.0 - t_norm.z * t_norm.z);

    return mat3x3<f32>(
        vec3<f32>(m00, m10, m20),
        vec3<f32>(m01, m11, m21),
        vec3<f32>(m02, m12, m22)
    );
}

// SGGX microflake normal distribution function D(wh) = 1 / (pi * sqrt(det(S)) * (wh^T * S^-1 * wh)^2)
fn sggx_distribution(wh: vec3<f32>, S: mat3x3<f32>) -> f32 {
    let det = S[0][0] * (S[1][1] * S[2][2] - S[1][2] * S[2][1])
            - S[0][1] * (S[1][0] * S[2][2] - S[1][2] * S[2][0])
            + S[0][2] * (S[1][0] * S[2][1] - S[1][1] * S[2][0]);

    let inv_det = 1.0 / max(det, 1e-6);

    // Compute cofactor matrix / inverse
    let c00 = (S[1][1] * S[2][2] - S[1][2] * S[2][1]) * inv_det;
    let c01 = (S[0][2] * S[2][1] - S[0][1] * S[2][2]) * inv_det;
    let c02 = (S[0][1] * S[1][2] - S[0][2] * S[1][1]) * inv_det;

    let c10 = c01;
    let c11 = (S[0][0] * S[2][2] - S[0][2] * S[2][0]) * inv_det;
    let c12 = (S[0][2] * S[1][0] - S[0][0] * S[1][2]) * inv_det;

    let c20 = c02;
    let c21 = c12;
    let c22 = (S[0][0] * S[1][1] - S[0][1] * S[1][0]) * inv_det;

    let S_inv = mat3x3<f32>(
        vec3<f32>(c00, c10, c20),
        vec3<f32>(c01, c11, c21),
        vec3<f32>(c02, c12, c22)
    );

    let s_wh = S_inv * wh;
    let quad = dot(wh, s_wh);
    let denom = PI * sqrt(max(det, 1e-6)) * quad * quad;

    return 1.0 / max(denom, 1e-5);
}

// Projected area in direction V: sigma(V) = sqrt(V^T * S * V)
fn sggx_projected_area(V: vec3<f32>, S: mat3x3<f32>) -> f32 {
    let sv = S * V;
    return sqrt(max(dot(V, sv), 1e-6));
}

// Evaluates SGGX BRDF reflection for distant terrain canopy
fn evaluate_sggx_ground_brdf(
    world_pos: vec3<f32>,
    terrain_normal: vec3<f32>,
    albedo: vec3<f32>,
) -> vec3<f32> {
    let V = normalize(sggx.view_pos.xyz - world_pos);
    let L = normalize(sggx.light_dir.xyz);
    let H = normalize(V + L);

    // Adjust fiber tangent based on turgor pressure and trample matted ratio
    let turgor = sggx.view_pos.w;
    let matted = sggx.fiber_tangent.w;

    var effective_tangent = sggx.fiber_tangent.xyz;
    if (matted > 0.2) {
        // Flattened horizontally by vehicle/feet
        effective_tangent = normalize(vec3<f32>(effective_tangent.x, 0.05, effective_tangent.z));
    } else if (turgor < 0.25) {
        // Wilted droop: isotropic spread
        effective_tangent = normalize(mix(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.7, 0.1, 0.7), 1.0 - turgor * 4.0));
    }

    let sigma_v = sggx.roughness.x; // e.g. 0.15 (smooth along blade)
    let sigma_t = sggx.roughness.y; // e.g. 0.55 (rough across blade)

    let S = compute_sggx_matrix(effective_tangent, sigma_v, sigma_t);

    let D = sggx_distribution(H, S);
    let sigma_V = sggx_projected_area(V, S);
    let sigma_L = sggx_projected_area(L, S);

    // Smith shadowing-masking for microflakes: G = 1 / (sigma_V + sigma_L)
    let G = 1.0 / max(sigma_V + sigma_L, 1e-4);

    let cos_theta_d = dot(L, H);
    let F0 = 0.04; // 4% dielectric Fresnel for plant cuticle wax
    let F = F0 + (1.0 - F0) * pow(1.0 - max(cos_theta_d, 0.0), 5.0);

    let specular_brdf = (D * F * G) * 0.25;
    let diffuse_brdf = albedo * (1.0 / PI) * max(dot(terrain_normal, L), 0.0);

    return (diffuse_brdf + vec3<f32>(specular_brdf)) * sggx.light_dir.w;
}
