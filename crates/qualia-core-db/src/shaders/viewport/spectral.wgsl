// σ → CIE 1931 XYZ → linear sRGB (shared by projector + ambient).
// HDR scene path: no sRGB gamma encode — bloom composite applies Reinhard.

fn sigma_to_cie_xyz(sigma: f32) -> vec3<f32> {
    let s = fract(sigma);
    let lambda = 400.0 + (s * 300.0);

    let x1 = 1.056 * exp(-0.5 * pow((lambda - 599.8) / 43.2, 2.0));
    let x2 = 0.362 * exp(-0.5 * pow((lambda - 442.0) / 32.0, 2.0));
    let x3 = -0.065 * exp(-0.5 * pow((lambda - 501.1) / 20.4, 2.0));
    let X = x1 + x2 + x3;

    let y1 = 0.821 * exp(-0.5 * pow((lambda - 568.8) / 46.9, 2.0));
    let y2 = 0.286 * exp(-0.5 * pow((lambda - 530.9) / 16.3, 2.0));
    let Y = y1 + y2;

    let z1 = 1.217 * exp(-0.5 * pow((lambda - 437.0) / 11.8, 2.0));
    let z2 = 0.681 * exp(-0.5 * pow((lambda - 459.0) / 26.0, 2.0));
    let Z = z1 + z2;

    return vec3<f32>(X, Y, Z);
}

fn xyz_to_linear_srgb(xyz: vec3<f32>) -> vec3<f32> {
    let R = 3.2404542 * xyz.x - 1.5371385 * xyz.y - 0.4985314 * xyz.z;
    let G = -0.9692660 * xyz.x + 1.8760108 * xyz.y + 0.0415560 * xyz.z;
    let B = 0.0556434 * xyz.x - 0.2040259 * xyz.y + 1.0572252 * xyz.z;
    return max(vec3<f32>(R, G, B), vec3<f32>(0.0));
}

fn sigma_to_linear_rgb(sigma: f32) -> vec3<f32> {
    return xyz_to_linear_srgb(sigma_to_cie_xyz(sigma));
}