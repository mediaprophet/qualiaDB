use super::{invalid, probe, Result};
#[derive(Clone, Copy)]
struct Edge {
    a: u32,
    b: u32,
    face: usize,
    corner: usize,
}
fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}
fn join(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[a.max(b)] = a.min(b);
}
pub(super) struct Chart {
    pub vertices: Vec<u32>,
    pub faces: Vec<u32>,
    pub triangles: Vec<[u32; 3]>,
    pub boundary: Vec<u32>,
}
pub(super) struct Cut {
    pub sources: Vec<u32>,
    pub triangles: Vec<[u32; 3]>,
    pub face_charts: Vec<u32>,
    pub charts: Vec<Chart>,
}
pub(super) fn cut(
    triangles: &[[u32; 3]],
    seams: &[[u32; 2]],
    cancelled: &dyn Fn() -> bool,
) -> Result<Cut> {
    // Sorted explicit pairs make duplicate/reversed seams an error instead of
    // silently changing authored intent. Source indices are never welded by position.
    if seams.iter().any(|s| s[0] >= s[1]) || seams.windows(2).any(|s| s[0] >= s[1]) {
        return Err(invalid(
            "seams must be unique sorted [smaller,larger] source-index pairs",
        ));
    }
    let mut edges = Vec::with_capacity(triangles.len() * 3);
    for (f, t) in triangles.iter().enumerate() {
        for c in 0..3 {
            edges.push(Edge {
                a: t[c],
                b: t[(c + 1) % 3],
                face: f,
                corner: f * 3 + c,
            });
        }
    }
    edges.sort_unstable_by_key(|e| (e.a.min(e.b), e.a.max(e.b), e.face));
    let mut corners: Vec<_> = (0..edges.len()).collect();
    let mut faces: Vec<_> = (0..triangles.len()).collect();
    let mut matched_seams = 0;
    let mut boundary = Vec::with_capacity(edges.len());
    let mut i = 0;
    while i < edges.len() {
        probe(cancelled)?;
        let e = edges[i];
        let key = [e.a.min(e.b), e.a.max(e.b)];
        let mut j = i + 1;
        while j < edges.len() && [edges[j].a.min(edges[j].b), edges[j].a.max(edges[j].b)] == key {
            j += 1;
        }
        let seam = seams.binary_search(&key).is_ok();
        if seam {
            matched_seams += 1;
        }
        if j - i > 2 {
            return Err(invalid("nonmanifold source edge has more than two faces"));
        }
        if j - i == 2 {
            let b = edges[i + 1];
            if e.a != b.b || e.b != b.a {
                return Err(invalid("source adjacent face winding is inconsistent"));
            }
            if !seam {
                join(&mut faces, e.face, b.face);
                // Union corners across each kept edge, including a nonseparating
                // cut within one face component. A chart ID alone cannot do this.
                join(&mut corners, e.corner, b.face * 3 + (b.corner % 3 + 1) % 3);
                join(&mut corners, e.face * 3 + (e.corner % 3 + 1) % 3, b.corner);
            }
        }
        if seam || j - i == 1 {
            for e in &edges[i..j] {
                boundary.push(*e);
            }
        }
        i = j;
    }
    if matched_seams != seams.len() {
        return Err(invalid(
            "seam references an edge absent from source topology",
        ));
    }
    let mut roots = vec![usize::MAX; corners.len()];
    let mut sources = Vec::with_capacity(corners.len());
    let mut output = vec![[0u32; 3]; triangles.len()];
    for (f, t) in triangles.iter().enumerate() {
        for c in 0..3 {
            let r = root(&corners, f * 3 + c);
            if roots[r] == usize::MAX {
                roots[r] = sources.len();
                sources.push(t[c]);
            }
            output[f][c] = roots[r] as u32;
        }
    }
    let mut chart_roots = vec![usize::MAX; faces.len()];
    let mut face_charts = vec![0u32; faces.len()];
    let mut charts: Vec<Chart> = Vec::new();
    for f in 0..triangles.len() {
        let r = root(&faces, f);
        if chart_roots[r] == usize::MAX {
            chart_roots[r] = charts.len();
            charts.push(Chart {
                vertices: Vec::new(),
                faces: Vec::new(),
                triangles: Vec::new(),
                boundary: Vec::new(),
            });
        }
        let id = chart_roots[r];
        face_charts[f] = id as u32;
        charts[id].faces.push(f as u32);
    }
    let mut local = vec![usize::MAX; sources.len()];
    for (id, chart) in charts.iter_mut().enumerate() {
        probe(cancelled)?;
        for &f in &chart.faces {
            let mut tri = [0u32; 3];
            for c in 0..3 {
                let v = output[f as usize][c] as usize;
                if local[v] == usize::MAX {
                    local[v] = chart.vertices.len();
                    chart.vertices.push(v as u32);
                }
                tri[c] = local[v] as u32;
            }
            chart.triangles.push(tri);
        }
        let nv = chart.vertices.len();
        let mut next = vec![usize::MAX; nv];
        let mut incoming = vec![0usize; nv];
        let mut ne = 0;
        for e in &boundary {
            if face_charts[e.face] != id as u32 {
                continue;
            }
            let a = local[output[e.face][e.corner % 3] as usize];
            let b = local[output[e.face][(e.corner % 3 + 1) % 3] as usize];
            if next[a] != usize::MAX {
                return Err(invalid(
                    "cut vertex has multiple boundary successors; add seams",
                ));
            }
            next[a] = b;
            incoming[b] += 1;
            ne += 1;
        }
        // A connected orientable triangle manifold with one simple boundary
        // and Euler characteristic one is a disk. Reject closed/holey charts.
        if ne < 3
            || incoming.iter().enumerate().any(|(v, &n)| {
                (next[v] != usize::MAX && n != 1) || (next[v] == usize::MAX && n != 0)
            })
        {
            return Err(invalid(
                "chart lacks a simple boundary; mark seams to make disks",
            ));
        }
        let edge_count = (chart.triangles.len() * 3 + ne) / 2;
        if nv + chart.triangles.len() != edge_count + 1 {
            return Err(invalid("chart is not a disk after seams; add cuts"));
        }
        let start = next
            .iter()
            .position(|&v| v != usize::MAX)
            .ok_or(invalid("closed chart needs seams"))?;
        let mut v = start;
        for _ in 0..ne {
            if v == usize::MAX || chart.boundary.contains(&(v as u32)) {
                return Err(invalid("chart has multiple boundary loops"));
            }
            chart.boundary.push(v as u32);
            v = next[v];
        }
        if v != start {
            return Err(invalid("chart boundary does not close"));
        }
        // Interior vertex fan manifold check. Corner unions enforce connected
        // fans; an interior bow tie remains split and fails Euler/boundary above.
        for &v in &chart.vertices {
            local[v as usize] = usize::MAX;
        }
    }
    Ok(Cut {
        sources,
        triangles: output,
        face_charts,
        charts,
    })
}
