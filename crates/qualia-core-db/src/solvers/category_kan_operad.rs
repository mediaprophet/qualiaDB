//! Limits and colimits of finite attributed directed multigraphs.
//! Copyright (c) 2026 Timothy Charles Holborn. CC BY-NC-ND 4.0.
//!
//! Attributes are fixed labels. A graph morphism preserves vertex labels, edge
//! labels and both endpoint maps. All solver and verifier buffers are caller-owned.

pub const MAX_INPUT_VERTICES: usize = 256;
pub const MAX_INPUT_EDGES: usize = 1024;
pub const MAX_OUTPUT_VERTICES: usize = 2048;
pub const MAX_OUTPUT_EDGES: usize = 4096;
const NONE: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Edge {
    pub source: u32,
    pub target: u32,
    pub attribute: u64,
}

#[derive(Clone, Copy)]
pub struct Graph<'a> {
    pub vertices: &'a [u64],
    pub edges: &'a [Edge],
}

#[derive(Clone, Copy)]
pub struct GraphMap<'a> {
    pub vertices: &'a [u32],
    pub edges: &'a [u32],
}

#[derive(Clone, Copy)]
pub struct Diagram<'a> {
    pub base: Graph<'a>,
    pub left: Graph<'a>,
    pub right: Graph<'a>,
    pub left_map: GraphMap<'a>,
    pub right_map: GraphMap<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Pullback,
    Pushout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    SizeLimit,
    InvalidGraph,
    InvalidMap,
    BufferTooSmall,
    InvalidWitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub vertices: usize,
    pub edges: usize,
}

#[derive(Clone, Copy)]
pub struct PairWitness<'a> {
    pub vertices: &'a [[u32; 2]],
    pub edges: &'a [[u32; 2]],
}

pub struct PullbackOutput<'a> {
    pub vertices: &'a mut [u64],
    pub edges: &'a mut [Edge],
    pub vertex_pairs: &'a mut [[u32; 2]],
    pub edge_pairs: &'a mut [[u32; 2]],
}

pub struct PushoutOutput<'a> {
    pub vertices: &'a mut [u64],
    pub edges: &'a mut [Edge],
    /// Source slots are in left-then-right order, separately for vertices and edges.
    pub vertex_images: &'a mut [u32],
    pub edge_images: &'a mut [u32],
}

pub struct QuotientScratch<'a> {
    pub parent: &'a mut [u32],
    pub class: &'a mut [u32],
}

impl Graph<'_> {
    fn validate(self, vertex_cap: usize, edge_cap: usize) -> Result<(), Error> {
        if self.vertices.len() > vertex_cap || self.edges.len() > edge_cap {
            return Err(Error::SizeLimit);
        }
        if self.vertices.contains(&0)
            || self.edges.iter().any(|edge| {
                edge.attribute == 0
                    || edge.source as usize >= self.vertices.len()
                    || edge.target as usize >= self.vertices.len()
            })
        {
            return Err(Error::InvalidGraph);
        }
        Ok(())
    }
}

impl GraphMap<'_> {
    fn validate(self, domain: Graph<'_>, codomain: Graph<'_>) -> Result<(), Error> {
        if self.vertices.len() != domain.vertices.len() || self.edges.len() != domain.edges.len() {
            return Err(Error::InvalidMap);
        }
        for (&attribute, &image) in domain.vertices.iter().zip(self.vertices) {
            if codomain.vertices.get(image as usize) != Some(&attribute) {
                return Err(Error::InvalidMap);
            }
        }
        for (edge, &image) in domain.edges.iter().zip(self.edges) {
            let expected = Edge {
                source: self.vertices[edge.source as usize],
                target: self.vertices[edge.target as usize],
                attribute: edge.attribute,
            };
            if codomain.edges.get(image as usize) != Some(&expected) {
                return Err(Error::InvalidMap);
            }
        }
        Ok(())
    }
}

impl Diagram<'_> {
    fn validate(self, operation: Operation) -> Result<(), Error> {
        for graph in [self.base, self.left, self.right] {
            graph.validate(MAX_INPUT_VERTICES, MAX_INPUT_EDGES)?;
        }
        match operation {
            Operation::Pullback => {
                self.left_map.validate(self.left, self.base)?;
                self.right_map.validate(self.right, self.base)
            }
            Operation::Pushout => {
                self.left_map.validate(self.base, self.left)?;
                self.right_map.validate(self.base, self.right)
            }
        }
    }
}

fn equal_image_pairs<'a>(left: &'a [u32], right: &'a [u32]) -> impl Iterator<Item = [u32; 2]> + 'a {
    left.iter().enumerate().flat_map(move |(a, &image)| {
        right
            .iter()
            .enumerate()
            .filter_map(move |(b, &rhs)| (image == rhs).then_some([a as u32, b as u32]))
    })
}

/// Count the admitted pullback or return the conservative pushout allocation size.
pub fn output_capacity(diagram: Diagram<'_>, operation: Operation) -> Result<Counts, Error> {
    diagram.validate(operation)?;
    let counts = match operation {
        Operation::Pullback => Counts {
            vertices: equal_image_pairs(diagram.left_map.vertices, diagram.right_map.vertices)
                .take(MAX_OUTPUT_VERTICES + 1)
                .count(),
            edges: equal_image_pairs(diagram.left_map.edges, diagram.right_map.edges)
                .take(MAX_OUTPUT_EDGES + 1)
                .count(),
        },
        Operation::Pushout => Counts {
            vertices: diagram.left.vertices.len() + diagram.right.vertices.len(),
            edges: diagram.left.edges.len() + diagram.right.edges.len(),
        },
    };
    if counts.vertices > MAX_OUTPUT_VERTICES || counts.edges > MAX_OUTPUT_EDGES {
        return Err(Error::SizeLimit);
    }
    Ok(counts)
}

fn pair_endpoints(left: Edge, right: Edge, vertices: &[[u32; 2]]) -> Result<Edge, Error> {
    let source = vertices
        .binary_search(&[left.source, right.source])
        .map_err(|_| Error::InvalidWitness)?;
    let target = vertices
        .binary_search(&[left.target, right.target])
        .map_err(|_| Error::InvalidWitness)?;
    Ok(Edge {
        source: source as u32,
        target: target as u32,
        attribute: left.attribute,
    })
}

/// Materialize the graph of pairs whose images agree. The witness lists the
/// two projection maps in lexicographic order and retains parallel edges.
pub fn pullback_into(diagram: Diagram<'_>, out: PullbackOutput<'_>) -> Result<Counts, Error> {
    let count = output_capacity(diagram, Operation::Pullback)?;
    if out.vertices.len() < count.vertices
        || out.vertex_pairs.len() < count.vertices
        || out.edges.len() < count.edges
        || out.edge_pairs.len() < count.edges
    {
        return Err(Error::BufferTooSmall);
    }
    for (slot, pair) in
        equal_image_pairs(diagram.left_map.vertices, diagram.right_map.vertices).enumerate()
    {
        out.vertex_pairs[slot] = pair;
        out.vertices[slot] = diagram.left.vertices[pair[0] as usize];
    }
    for (slot, pair) in
        equal_image_pairs(diagram.left_map.edges, diagram.right_map.edges).enumerate()
    {
        out.edge_pairs[slot] = pair;
        out.edges[slot] = pair_endpoints(
            diagram.left.edges[pair[0] as usize],
            diagram.right.edges[pair[1] as usize],
            &out.vertex_pairs[..count.vertices],
        )?;
    }
    Ok(count)
}

/// Verify an entire pullback witness. The exact ordered correspondence gives
/// one and only one factor for each pair from any cone over this diagram.
pub fn verify_pullback(
    diagram: Diagram<'_>,
    result: Graph<'_>,
    witness: PairWitness<'_>,
) -> Result<(), Error> {
    diagram.validate(Operation::Pullback)?;
    result.validate(MAX_OUTPUT_VERTICES, MAX_OUTPUT_EDGES)?;
    if witness.vertices.len() != result.vertices.len()
        || witness.edges.len() != result.edges.len()
        || !witness.vertices.iter().copied().eq(equal_image_pairs(
            diagram.left_map.vertices,
            diagram.right_map.vertices,
        ))
        || !witness.edges.iter().copied().eq(equal_image_pairs(
            diagram.left_map.edges,
            diagram.right_map.edges,
        ))
    {
        return Err(Error::InvalidWitness);
    }
    for (&[left, _], &attribute) in witness.vertices.iter().zip(result.vertices) {
        if diagram.left.vertices[left as usize] != attribute {
            return Err(Error::InvalidWitness);
        }
    }
    for (&[left, right], &edge) in witness.edges.iter().zip(result.edges) {
        let paired = pair_endpoints(
            diagram.left.edges[left as usize],
            diagram.right.edges[right as usize],
            witness.vertices,
        )?;
        if paired != edge {
            return Err(Error::InvalidWitness);
        }
    }
    Ok(())
}

fn representative(parent: &mut [u32], mut slot: usize) -> usize {
    while parent[slot] as usize != slot {
        parent[slot] = parent[parent[slot] as usize];
        slot = parent[slot] as usize;
    }
    slot
}

fn with_classes(
    count: usize,
    left_count: usize,
    left: &[u32],
    right: &[u32],
    scratch: &mut QuotientScratch<'_>,
    mut visit: impl FnMut(usize, u32, bool) -> Result<(), Error>,
) -> Result<usize, Error> {
    let parent = scratch
        .parent
        .get_mut(..count)
        .ok_or(Error::BufferTooSmall)?;
    let classes = scratch
        .class
        .get_mut(..count)
        .ok_or(Error::BufferTooSmall)?;
    for (i, slot) in parent.iter_mut().enumerate() {
        *slot = i as u32;
    }
    classes.fill(NONE);
    for (&a, &b) in left.iter().zip(right) {
        let a = representative(parent, a as usize);
        let b = representative(parent, left_count + b as usize);
        parent[a.max(b)] = a.min(b) as u32;
    }
    let mut size = 0;
    for slot in 0..count {
        let rep = representative(parent, slot);
        let fresh = classes[rep] == NONE;
        if fresh {
            classes[rep] = size as u32;
            size += 1;
        }
        visit(slot, classes[rep], fresh)?;
    }
    Ok(size)
}

fn sum_vertex(diagram: Diagram<'_>, slot: usize) -> u64 {
    if slot < diagram.left.vertices.len() {
        diagram.left.vertices[slot]
    } else {
        diagram.right.vertices[slot - diagram.left.vertices.len()]
    }
}

fn sum_edge(diagram: Diagram<'_>, slot: usize, images: &[u32]) -> Edge {
    let (edge, offset) = if slot < diagram.left.edges.len() {
        (diagram.left.edges[slot], 0)
    } else {
        (
            diagram.right.edges[slot - diagram.left.edges.len()],
            diagram.left.vertices.len(),
        )
    };
    Edge {
        source: images[offset + edge.source as usize],
        target: images[offset + edge.target as usize],
        attribute: edge.attribute,
    }
}

/// Quotient two disjoint graphs by precisely the identifications generated by
/// the span. Node and edge equivalences are generated separately.
pub fn pushout_into(
    diagram: Diagram<'_>,
    mut scratch: QuotientScratch<'_>,
    out: PushoutOutput<'_>,
) -> Result<Counts, Error> {
    let capacity = output_capacity(diagram, Operation::Pushout)?;
    if out.vertices.len() < capacity.vertices
        || out.vertex_images.len() != capacity.vertices
        || out.edges.len() < capacity.edges
        || out.edge_images.len() != capacity.edges
    {
        return Err(Error::BufferTooSmall);
    }
    let vertices = with_classes(
        capacity.vertices,
        diagram.left.vertices.len(),
        diagram.left_map.vertices,
        diagram.right_map.vertices,
        &mut scratch,
        |slot, image, fresh| {
            out.vertex_images[slot] = image;
            if fresh {
                out.vertices[image as usize] = sum_vertex(diagram, slot);
            }
            Ok(())
        },
    )?;
    let edges = with_classes(
        capacity.edges,
        diagram.left.edges.len(),
        diagram.left_map.edges,
        diagram.right_map.edges,
        &mut scratch,
        |slot, image, fresh| {
            out.edge_images[slot] = image;
            if fresh {
                out.edges[image as usize] = sum_edge(diagram, slot, out.vertex_images);
            }
            Ok(())
        },
    )?;
    Ok(Counts { vertices, edges })
}

/// Verify that the cocone maps preserve incidence and that their classes are
/// exactly the equivalence generated by the span. This rules out extra merges
/// or unrepresented classes, which a commutative-square check alone cannot do.
pub fn verify_pushout(
    diagram: Diagram<'_>,
    result: Graph<'_>,
    witness: GraphMap<'_>,
    mut scratch: QuotientScratch<'_>,
) -> Result<(), Error> {
    let capacity = output_capacity(diagram, Operation::Pushout)?;
    result.validate(MAX_OUTPUT_VERTICES, MAX_OUTPUT_EDGES)?;
    if witness.vertices.len() != capacity.vertices || witness.edges.len() != capacity.edges {
        return Err(Error::InvalidWitness);
    }
    let vertices = with_classes(
        capacity.vertices,
        diagram.left.vertices.len(),
        diagram.left_map.vertices,
        diagram.right_map.vertices,
        &mut scratch,
        |slot, image, _| {
            if witness.vertices[slot] != image
                || result.vertices.get(image as usize) != Some(&sum_vertex(diagram, slot))
            {
                return Err(Error::InvalidWitness);
            }
            Ok(())
        },
    )?;
    let edges = with_classes(
        capacity.edges,
        diagram.left.edges.len(),
        diagram.left_map.edges,
        diagram.right_map.edges,
        &mut scratch,
        |slot, image, _| {
            if witness.edges[slot] != image
                || result.edges.get(image as usize)
                    != Some(&sum_edge(diagram, slot, witness.vertices))
            {
                return Err(Error::InvalidWitness);
            }
            Ok(())
        },
    )?;
    if vertices != result.vertices.len() || edges != result.edges.len() {
        return Err(Error::InvalidWitness);
    }
    Ok(())
}
