//! Cold, bounded explicit-seam disk charts using the canonical harmonic solver.
//! No automatic seams, conformality, distortion ceiling or optimal packing claim.
//! All public outputs remain unchanged on every returned error, including cancel.
mod manifest;
mod packing;
pub use manifest::UV_ATLAS_MANIFEST;
#[cfg(test)]
mod tests;
mod topology;
use super::{harmonic_parameterize_disk, Point3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasOptions {
    pub atlas_size: u32,
    pub gutter_pixels: u32,
    /// Per-chart area density in local source metres; local distortion varies.
    pub texels_per_metre: f64,
    pub iterations: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct AtlasLimits {
    pub vertices: usize,
    pub triangles: usize,
    pub charts: usize,
    pub scratch_bytes: usize,
    pub work_units: u64,
}
impl Default for AtlasLimits {
    fn default() -> Self {
        Self {
            vertices: 65_536,
            triangles: 32_768,
            charts: 4096,
            scratch_bytes: 256 * 1024 * 1024,
            work_units: 100_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AtlasChart {
    pub min_source_triangle: u32,
    pub triangle_count: u32,
    /// Outer pixel rectangle including reserved gutter, x/y/width/height.
    pub rect_pixels: [u32; 4],
    pub surface_area: f64,
    pub uv_area_pixels: f64,
    /// Square root of triangle UV-pixel area / source-metre area. These
    /// measured extrema disclose local distortion; no conformality claim.
    pub min_local_texels_per_metre: f64,
    pub max_local_texels_per_metre: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasCounts {
    pub vertices: usize,
    pub triangles: usize,
    pub charts: usize,
}
pub struct AtlasOutput<'a> {
    pub source_vertices: &'a mut [u32],
    /// Preserves input triangle order and winding.
    pub triangles: &'a mut [[u32; 3]],
    pub triangle_charts: &'a mut [u32],
    pub uv0: &'a mut [[f32; 2]],
    pub charts: &'a mut [AtlasChart],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasError {
    Invalid(&'static str),
    Budget,
    OutputTooSmall,
    Cancelled,
}
type Result<T> = core::result::Result<T, AtlasError>;
fn invalid(message: &'static str) -> AtlasError {
    AtlasError::Invalid(message)
}
fn probe(cancelled: &dyn Fn() -> bool) -> Result<()> {
    if cancelled() {
        Err(AtlasError::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn area3(a: Point3, b: Point3, c: Point3) -> f64 {
    let u = [b.x - a.x, b.y - a.y, b.z - a.z];
    let v = [c.x - a.x, c.y - a.y, c.z - a.z];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    cross.iter().map(|x| x * x).sum::<f64>().sqrt() * 0.5
}
pub(crate) fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
/// Scratch is admitted conservatively before allocation. Work charges include
/// solver O(iterations*corners*faces), topology and the pairwise UV overlap oracle.
/// Cancellation is checked between chart solves and bounded topology/UV passes.
pub fn unwrap_atlas_into(
    positions: &[Point3],
    triangles: &[[u32; 3]],
    seams: &[[u32; 2]],
    options: AtlasOptions,
    limits: AtlasLimits,
    cancelled: &dyn Fn() -> bool,
    output: AtlasOutput<'_>,
) -> Result<AtlasCounts> {
    probe(cancelled)?;
    preflight_counts(
        positions.len(),
        triangles.len(),
        seams.len(),
        options,
        limits,
    )?;
    let n = positions.len();
    let f = triangles.len();
    if positions
        .iter()
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite())
    {
        return Err(invalid("nonfinite source vertex"));
    }
    for t in triangles {
        if t.iter().any(|&v| v as usize >= n) || t[0] == t[1] || t[1] == t[2] || t[2] == t[0] {
            return Err(invalid("invalid triangle indices"));
        }
        let a = area3(
            positions[t[0] as usize],
            positions[t[1] as usize],
            positions[t[2] as usize],
        );
        if !a.is_finite() || a <= 1e-20 {
            return Err(invalid("degenerate source triangle"));
        }
    }
    build(
        positions, triangles, seams, options, limits, cancelled, output,
    )
}
/// Maximum split-vertex count, with all size/options/work charges checked before
/// callers allocate output buffers. This does not certify input topology.
pub fn preflight_counts(
    n: usize,
    f: usize,
    seam_count: usize,
    options: AtlasOptions,
    limits: AtlasLimits,
) -> Result<usize> {
    let corners = f.checked_mul(3).ok_or(AtlasError::Budget)?;
    if n == 0
        || f == 0
        || options.atlas_size == 0
        || options.atlas_size > 16384
        || options.gutter_pixels == 0
        || options.gutter_pixels > options.atlas_size / 4
        || !options.texels_per_metre.is_finite()
        || options.texels_per_metre <= 0.0
        || options.texels_per_metre > 1_000_000.0
        || options.iterations == 0
        || options.iterations > 4096
    {
        return Err(invalid(
            "invalid atlas size, positive gutter/density or iterations",
        ));
    }
    let bytes = n
        .checked_add(corners)
        .and_then(|x| x.checked_mul(1024))
        .and_then(|x| seam_count.checked_mul(64).and_then(|s| x.checked_add(s)))
        .ok_or(AtlasError::Budget)?;
    let work = (f as u64)
        .checked_mul(f as u64)
        .and_then(|x| x.checked_mul((options.iterations as u64) * 12 + 128))
        .and_then(|x| x.checked_add((n as u64) * 8))
        .ok_or(AtlasError::Budget)?;
    if n > limits.vertices
        || corners > limits.vertices
        || f > limits.triangles
        || corners > u32::MAX as usize
        || seam_count > corners
        || bytes > limits.scratch_bytes
        || work > limits.work_units
    {
        return Err(AtlasError::Budget);
    }
    Ok(corners)
}
fn build(
    positions: &[Point3],
    triangles: &[[u32; 3]],
    seams: &[[u32; 2]],
    options: AtlasOptions,
    limits: AtlasLimits,
    cancelled: &dyn Fn() -> bool,
    output: AtlasOutput<'_>,
) -> Result<AtlasCounts> {
    let f = triangles.len();
    let mut cut = topology::cut(triangles, seams, cancelled)?;
    if cut.charts.len() > limits.charts {
        return Err(AtlasError::Budget);
    }
    let counts = AtlasCounts {
        vertices: cut.sources.len(),
        triangles: f,
        charts: cut.charts.len(),
    };
    if output.source_vertices.len() < counts.vertices
        || output.uv0.len() < counts.vertices
        || output.triangles.len() < f
        || output.triangle_charts.len() < f
        || output.charts.len() < counts.charts
    {
        return Err(AtlasError::OutputTooSmall);
    }
    let mut uv = vec![[0.0f64; 2]; counts.vertices];
    let mut reports = Vec::with_capacity(counts.charts);
    for (chart_id, chart) in cut.charts.iter_mut().enumerate() {
        probe(cancelled)?;
        let local_positions: Vec<_> = chart
            .vertices
            .iter()
            .map(|&v| positions[cut.sources[v as usize] as usize])
            .collect();
        let mut local_uv = vec![[0.0; 2]; local_positions.len()];
        // The existing solver initializes each invocation. Run once for all
        // iterations; cancel latency is one explicitly charged chart solve.
        harmonic_parameterize_disk(
            &local_positions,
            &chart.triangles,
            &chart.boundary,
            &mut local_uv,
            options.iterations,
        )
        .map_err(|_| invalid("canonical harmonic disk solver refused chart"))?;
        probe(cancelled)?;
        let mut area = 0.0;
        let mut uv_area = 0.0;
        for t in &chart.triangles {
            let a = area3(
                local_positions[t[0] as usize],
                local_positions[t[1] as usize],
                local_positions[t[2] as usize],
            );
            let u = cross(
                local_uv[t[0] as usize],
                local_uv[t[1] as usize],
                local_uv[t[2] as usize],
            ) * 0.5;
            if !u.is_finite() || u <= 1e-14 {
                return Err(invalid(
                    "folded or unconverged UV chart; add seams or iterations",
                ));
            }
            area += a;
            uv_area += u;
        }
        packing::no_overlap(&local_uv, &chart.triangles, cancelled)?;
        let scale = (area / uv_area).sqrt() * options.texels_per_metre;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(invalid("UV density overflow"));
        }
        for (&v, u) in chart.vertices.iter().zip(&local_uv) {
            uv[v as usize] = [(u[0] + 1.0) * scale, (u[1] + 1.0) * scale];
        }
        let inner = (2.0 * scale).ceil();
        let extent = inner + 2.0 * options.gutter_pixels as f64;
        if extent > options.atlas_size as f64 {
            return Err(invalid("chart exceeds atlas at requested texel density"));
        }
        let mut low = f64::INFINITY;
        let mut high = 0.0f64;
        for t in &chart.triangles {
            let a = area3(
                local_positions[t[0] as usize],
                local_positions[t[1] as usize],
                local_positions[t[2] as usize],
            );
            let u = cross(
                local_uv[t[0] as usize],
                local_uv[t[1] as usize],
                local_uv[t[2] as usize],
            ) * 0.5;
            let density = (u / a).sqrt() * scale;
            low = low.min(density);
            high = high.max(density);
        }
        reports.push(AtlasChart {
            min_source_triangle: chart.faces[0],
            triangle_count: chart.faces.len() as u32,
            rect_pixels: [0, 0, extent as u32, extent as u32],
            surface_area: area,
            uv_area_pixels: uv_area * scale * scale,
            min_local_texels_per_metre: low,
            max_local_texels_per_metre: high,
        });
        debug_assert!(chart
            .faces
            .iter()
            .all(|&f| cut.face_charts[f as usize] == chart_id as u32));
    }
    packing::pack(&mut reports, options.atlas_size)?;
    let mut final_uv = vec![[0.0f32; 2]; counts.vertices];
    for (chart, report) in cut.charts.iter().zip(&mut reports) {
        for &v in &chart.vertices {
            let p = uv[v as usize];
            final_uv[v as usize] = [
                (p[0] + report.rect_pixels[0] as f64 + options.gutter_pixels as f64) as f32
                    / options.atlas_size as f32,
                (p[1] + report.rect_pixels[1] as f64 + options.gutter_pixels as f64) as f32
                    / options.atlas_size as f32,
            ];
        }
        // Recheck the actual f32 delivery stream; quantization must not fold or overlap.
        let mapped: Vec<_> = chart
            .vertices
            .iter()
            .map(|&v| final_uv[v as usize].map(f64::from))
            .collect();
        if chart.triangles.iter().any(|t| {
            cross(
                mapped[t[0] as usize],
                mapped[t[1] as usize],
                mapped[t[2] as usize],
            ) <= 0.0
        }) {
            return Err(invalid("f32 atlas UV precision collapses a triangle"));
        }
        packing::no_overlap(&mapped, &chart.triangles, cancelled)?;
        // Report the actual f32 delivery stream, including its quantization.
        report.uv_area_pixels = 0.0;
        report.min_local_texels_per_metre = f64::INFINITY;
        report.max_local_texels_per_metre = 0.0;
        for (t, &face) in chart.triangles.iter().zip(&chart.faces) {
            let input = triangles[face as usize];
            let source_area = area3(
                positions[input[0] as usize],
                positions[input[1] as usize],
                positions[input[2] as usize],
            );
            let pixel_area = cross(
                mapped[t[0] as usize],
                mapped[t[1] as usize],
                mapped[t[2] as usize],
            ) * 0.5
                * (options.atlas_size as f64).powi(2);
            let density = (pixel_area / source_area).sqrt();
            report.uv_area_pixels += pixel_area;
            report.min_local_texels_per_metre = report.min_local_texels_per_metre.min(density);
            report.max_local_texels_per_metre = report.max_local_texels_per_metre.max(density);
        }
    }
    probe(cancelled)?;
    output.source_vertices[..counts.vertices].copy_from_slice(&cut.sources);
    output.triangles[..f].copy_from_slice(&cut.triangles);
    output.triangle_charts[..f].copy_from_slice(&cut.face_charts);
    output.uv0[..counts.vertices].copy_from_slice(&final_uv);
    output.charts[..counts.charts].copy_from_slice(&reports);
    Ok(counts)
}
