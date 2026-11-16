//! P11: replacement-group coverage, seam/border identities, and a CPU legal-cut
//! reference. Coverage indexes an external unique source-primitive ordering.
//! This adapter admits trees/forests only (strict subset of DAG).

use crate::container_10d::cluster_dag_section::{
    MeshletClusterNode, MAX_CLUSTER_DAG_NODES, MAX_MESHLET_TRIANGLES, MAX_MESHLET_VERTICES,
    project_sphere_error_px,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;
use crate::specialized_libs::computational_geometry::production_contracts::ContentId;

use super::RuntimeError;

/// Explicit replacement-group edge and source-primitive coverage interval.
/// Coverage is not stored on the 64-byte CDAG node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementLink {
    pub parent: Option<u32>,
    pub children_start: u32,
    pub children_count: u32,
    pub coverage_start: u32,
    pub coverage_count: u32,
    pub border_identity: ContentId,
}

/// Parent/child dependency identity for one admitted cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyIdentity {
    pub node: u32,
    pub parent: Option<u32>,
    pub border: ContentId,
    pub source: ContentId,
    pub coverage_start: u32,
    pub coverage_count: u32,
}

pub struct ValidatedHierarchy<'a> {
    pub(crate) nodes: &'a [MeshletClusterNode],
    pub(crate) links: &'a [ReplacementLink],
    pub(crate) children: &'a [u32],
    pub(crate) roots: &'a [u32],
    pub(crate) source: ContentId,
    pub(crate) coverage: u32,
}

impl<'a> ValidatedHierarchy<'a> {
    pub fn admit(
        nodes: &'a [MeshletClusterNode],
        links: &'a [ReplacementLink],
        children: &'a [u32],
        roots: &'a [u32],
        source: ContentId,
        source_primitive_order: &[u32],
        seen: &mut [bool],
        budget: &mut ProductionBudget<'_>,
    ) -> Result<Self, RuntimeError> {
        if nodes.is_empty()
            || nodes.len() != links.len()
            || nodes.len() > MAX_CLUSTER_DAG_NODES
            || nodes.len() > u32::MAX as usize
            || source_primitive_order.is_empty()
            || source_primitive_order.len() > u32::MAX as usize
            || seen.len() < source_primitive_order.len()
        {
            return Err(RuntimeError::InvalidHierarchy);
        }
        let count = source_primitive_order.len();
        budget.checkpoint(count as u64)?;
        seen[..count].fill(false);
        for &primitive in source_primitive_order {
            let slot = seen
                .get_mut(primitive as usize)
                .filter(|_| (primitive as usize) < count)
                .ok_or(RuntimeError::InvalidHierarchy)?;
            if *slot {
                return Err(RuntimeError::InvalidHierarchy);
            }
            *slot = true;
        }
        let mut root_cursor = 0usize;
        let mut root_coverage = 0u32;
        let mut edges = 0usize;
        for (index, (node, link)) in nodes.iter().zip(links.iter()).enumerate() {
            budget.checkpoint(1)?;
            if node.vertex_count == 0
                || node.vertex_count > MAX_MESHLET_VERTICES
                || node.triangle_count == 0
                || node.triangle_count > MAX_MESHLET_TRIANGLES
                || !node.bounding_sphere.iter().all(|x| x.is_finite())
                || node.bounding_sphere[3] <= 0.0
                || !node.lod_error_self.is_finite()
                || node.lod_error_self < 0.0
                || link.coverage_count == 0
                || link
                    .coverage_start
                    .checked_add(link.coverage_count)
                    .filter(|end| *end <= count as u32)
                    .is_none()
                || (link.children_count == 0) != (node.child_group_first == 0xFFFF)
            {
                return Err(RuntimeError::InvalidHierarchy);
            }
            match link.parent {
                Some(parent) => {
                    if parent as usize >= index {
                        return Err(RuntimeError::InvalidHierarchy);
                    }
                    let p = &nodes[parent as usize];
                    if p.lod_level <= node.lod_level
                        || p.lod_error_self < node.lod_error_self
                        || node.lod_error_parent != p.lod_error_self
                        || node.parent_error_sphere != p.bounding_sphere
                    {
                        return Err(RuntimeError::InvalidHierarchy);
                    }
                    let dist = (f64::from(p.bounding_sphere[0]) - f64::from(node.bounding_sphere[0]))
                        .hypot(f64::from(p.bounding_sphere[1]) - f64::from(node.bounding_sphere[1]))
                        .hypot(f64::from(p.bounding_sphere[2]) - f64::from(node.bounding_sphere[2]));
                    if dist + f64::from(node.bounding_sphere[3]) > f64::from(p.bounding_sphere[3]) {
                        return Err(RuntimeError::InvalidBounds);
                    }
                }
                None => {
                    if roots.get(root_cursor) != Some(&(index as u32))
                        || link.coverage_start != root_coverage
                        || node.lod_error_parent != f32::INFINITY
                        || node.parent_error_sphere != node.bounding_sphere
                    {
                        return Err(RuntimeError::InvalidHierarchy);
                    }
                    root_cursor += 1;
                    root_coverage = root_coverage
                        .checked_add(link.coverage_count)
                        .ok_or(RuntimeError::InvalidHierarchy)?;
                }
            }
            let start = link.children_start as usize;
            let end = start
                .checked_add(link.children_count as usize)
                .ok_or(RuntimeError::InvalidHierarchy)?;
            let group = children.get(start..end).ok_or(RuntimeError::InvalidHierarchy)?;
            budget.checkpoint(group.len() as u64)?;
            let mut cursor = link.coverage_start;
            let mut previous = None;
            for &child in group {
                if child as usize <= index
                    || child as usize >= nodes.len()
                    || previous.is_some_and(|p| child <= p)
                {
                    return Err(RuntimeError::InvalidHierarchy);
                }
                previous = Some(child);
                let child_link = &links[child as usize];
                if child_link.parent != Some(index as u32)
                    || child_link.coverage_start != cursor
                    || child_link.border_identity != link.border_identity
                {
                    return Err(RuntimeError::InvalidHierarchy);
                }
                cursor = cursor
                    .checked_add(child_link.coverage_count)
                    .ok_or(RuntimeError::InvalidHierarchy)?;
            }
            if !group.is_empty() && cursor != link.coverage_start + link.coverage_count {
                return Err(RuntimeError::InvalidHierarchy);
            }
            edges = edges
                .checked_add(group.len())
                .ok_or(RuntimeError::InvalidHierarchy)?;
        }
        if root_cursor != roots.len()
            || root_coverage != count as u32
            || edges != nodes.len() - roots.len()
        {
            return Err(RuntimeError::InvalidHierarchy);
        }
        Ok(Self {
            nodes,
            links,
            children,
            roots,
            source,
            coverage: count as u32,
        })
    }

    pub fn nodes(&self) -> &[MeshletClusterNode] {
        self.nodes
    }
    pub fn links(&self) -> &[ReplacementLink] {
        self.links
    }
    pub fn roots(&self) -> &[u32] {
        self.roots
    }
    pub fn source(&self) -> ContentId {
        self.source
    }
    pub fn coverage(&self) -> u32 {
        self.coverage
    }
    pub fn children_of(&self, index: usize) -> &[u32] {
        let link = &self.links[index];
        &self.children[link.children_start as usize
            ..(link.children_start + link.children_count) as usize]
    }
    pub fn parent_of(&self, index: usize) -> Option<u32> {
        self.links[index].parent
    }
    pub fn dependency_identity(&self, index: usize) -> DependencyIdentity {
        let link = &self.links[index];
        DependencyIdentity {
            node: index as u32,
            parent: link.parent,
            border: link.border_identity,
            source: self.source,
            coverage_start: link.coverage_start,
            coverage_count: link.coverage_count,
        }
    }

    /// Canonical cuts are ordered by coverage interval, never independent masks.
    pub fn validate_cut(
        &self,
        cut: &[u32],
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        let mut covered = 0u32;
        for &index in cut {
            budget.checkpoint(1)?;
            let link = self.links.get(index as usize).ok_or(RuntimeError::InvalidCut)?;
            if link.coverage_start != covered {
                return Err(RuntimeError::InvalidCut);
            }
            covered = covered
                .checked_add(link.coverage_count)
                .ok_or(RuntimeError::InvalidCut)?;
        }
        if covered != self.coverage {
            return Err(RuntimeError::InvalidCut);
        }
        Ok(())
    }

    /// Forest walk: replace a node with all children, never a sibling subset.
    /// `refine(index)` true means the complete child group is required.
    pub fn cpu_cut_by_refine(
        &self,
        mut refine: impl FnMut(usize) -> Result<bool, RuntimeError>,
        stack: &mut [u32],
        out_cut: &mut [u32],
        budget: &mut ProductionBudget<'_>,
    ) -> Result<usize, RuntimeError> {
        if stack.len() < self.nodes.len() || out_cut.len() < self.nodes.len() {
            return Err(RuntimeError::Capacity);
        }
        let mut top = 0usize;
        for &root in self.roots.iter().rev() {
            stack[top] = root;
            top += 1;
        }
        let mut written = 0usize;
        while top > 0 {
            budget.checkpoint(1)?;
            top -= 1;
            let index = stack[top] as usize;
            let children = self.children_of(index);
            if !children.is_empty() && refine(index)? {
                for &child in children.iter().rev() {
                    if top >= stack.len() {
                        return Err(RuntimeError::Capacity);
                    }
                    stack[top] = child;
                    top += 1;
                }
            } else if written >= out_cut.len() {
                return Err(RuntimeError::Capacity);
            } else {
                out_cut[written] = index as u32;
                written += 1;
            }
        }
        self.validate_cut(&out_cut[..written], budget)?;
        Ok(written)
    }

    /// CPU cut reference using world-space projected error vs `error_threshold_px`.
    pub fn cpu_cut_reference(
        &self,
        camera_pos: [f32; 3],
        proj_scale: f32,
        error_threshold_px: f32,
        stack: &mut [u32],
        out_cut: &mut [u32],
        budget: &mut ProductionBudget<'_>,
    ) -> Result<usize, RuntimeError> {
        if !proj_scale.is_finite()
            || proj_scale <= 0.0
            || !error_threshold_px.is_finite()
            || error_threshold_px < 0.0
            || !camera_pos.iter().all(|x| x.is_finite())
        {
            return Err(RuntimeError::InvalidView);
        }
        self.cpu_cut_by_refine(
            |index| {
                let node = &self.nodes[index];
                let error = project_sphere_error_px(
                    node.bounding_sphere,
                    node.lod_error_self,
                    camera_pos,
                    proj_scale,
                );
                Ok(error > error_threshold_px)
            },
            stack,
            out_cut,
            budget,
        )
    }
}
