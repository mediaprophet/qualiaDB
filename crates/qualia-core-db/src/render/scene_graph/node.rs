//! Scene graph nodes — hierarchy, duplication, semantic links.
//!
//! A simple scene graph with parent-child transforms, node duplication,
//! and semantic link annotations.

use std::collections::BTreeMap;

/// A scene graph node with a local transform and optional children.
#[derive(Debug, Clone)]
pub struct SceneNode {
    /// Unique node identifier (IRI or local name).
    pub id: String,
    /// Optional parent node ID.
    pub parent: Option<String>,
    /// Local translation [x, y, z].
    pub translation: [f32; 3],
    /// Local rotation as quaternion [x, y, z, w].
    pub rotation: [f32; 4],
    /// Local scale [x, y, z].
    pub scale: [f32; 3],
    /// Optional mesh asset IRI.
    pub mesh: Option<String>,
    /// Optional material IRI.
    pub material: Option<String>,
    /// User metadata.
    pub metadata: BTreeMap<String, String>,
    /// Child node IDs.
    pub children: Vec<String>,
}

impl SceneNode {
    /// Create a new node with identity transform.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            parent: None,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0; 3],
            mesh: None,
            material: None,
            metadata: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    /// Set translation.
    pub fn with_translation(mut self, t: [f32; 3]) -> Self {
        self.translation = t;
        self
    }

    /// Set rotation (quaternion).
    pub fn with_rotation(mut self, r: [f32; 4]) -> Self {
        self.rotation = r;
        self
    }

    /// Set scale.
    pub fn with_scale(mut self, s: [f32; 3]) -> Self {
        self.scale = s;
        self
    }

    /// Set mesh.
    pub fn with_mesh(mut self, mesh: &str) -> Self {
        self.mesh = Some(mesh.to_string());
        self
    }

    /// Set material.
    pub fn with_material(mut self, mat: &str) -> Self {
        self.material = Some(mat.to_string());
        self
    }
}

/// A semantic link between a scene node and a semantic entity (IRI).
#[derive(Debug, Clone)]
pub struct SemanticLink {
    /// Scene node ID.
    pub node_id: String,
    /// Semantic entity IRI.
    pub semantic_iri: String,
    /// Link type (e.g. "represents", "annotates", "derives-from").
    pub link_type: String,
    /// Optional confidence [0, 1].
    pub confidence: Option<f32>,
}

/// The scene graph — a collection of nodes with hierarchy.
#[derive(Debug, Clone, Default)]
pub struct SceneGraph {
    /// Nodes keyed by ID.
    pub nodes: BTreeMap<String, SceneNode>,
    /// Semantic links.
    pub links: Vec<SemanticLink>,
    /// Render budget in milliseconds (0 = unlimited).
    pub render_budget_ms: f32,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: SceneNode) {
        if let Some(parent_id) = &node.parent {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                parent.children.push(node.id.clone());
            }
        }
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add a light to the scene (stored as a node with light metadata).
    pub fn add_light(&mut self, light_id: &str, light: &super::Light) {
        let mut node = SceneNode::new(light_id);
        node.translation = light.position;
        node.metadata
            .insert("light_type".to_string(), format!("{:?}", light.light_type));
        node.metadata
            .insert("colour".to_string(), format!("{:?}", light.colour));
        node.metadata
            .insert("intensity".to_string(), light.intensity.to_string());
        node.metadata
            .insert("cast_shadows".to_string(), light.cast_shadows.to_string());
        self.add_node(node);
    }

    /// Duplicate a node and all its children with a new ID prefix.
    pub fn duplicate(
        &mut self,
        source_id: &str,
        new_id: &str,
        parent: Option<&str>,
    ) -> Option<String> {
        let source = self.nodes.get(source_id)?;
        let mut clone = source.clone();
        clone.id = new_id.to_string();
        clone.parent = parent.map(|p| p.to_string());
        clone.children.clear(); // Children will be re-added as duplicates.

        let children_ids = source.children.clone();
        self.add_node(clone);

        // Recursively duplicate children.
        for child_id in &children_ids {
            let new_child_id = format!("{new_id}/{child_id}");
            self.duplicate_recursive(child_id, &new_child_id, new_id);
        }

        Some(new_id.to_string())
    }

    fn duplicate_recursive(&mut self, source_id: &str, new_id: &str, new_parent: &str) {
        if let Some(source) = self.nodes.get(source_id) {
            let mut clone = source.clone();
            clone.id = new_id.to_string();
            clone.parent = Some(new_parent.to_string());
            let children_ids = clone.children.clone();
            clone.children.clear();
            self.add_node(clone);

            for child_id in &children_ids {
                let new_child_id = format!("{new_id}/{child_id}");
                self.duplicate_recursive(child_id, &new_child_id, new_id);
            }
        }
    }

    /// Link a scene node to a semantic entity.
    pub fn link_semantic(&mut self, link: SemanticLink) {
        self.links.push(link);
    }

    /// Set the render budget (milliseconds per frame).
    pub fn set_render_budget(&mut self, budget_ms: f32) {
        self.render_budget_ms = budget_ms;
    }

    /// Get a node by ID.
    pub fn get(&self, id: &str) -> Option<&SceneNode> {
        self.nodes.get(id)
    }

    /// Get a mutable node by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SceneNode> {
        self.nodes.get_mut(id)
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Convenience function: duplicate a node in a scene graph.
pub fn duplicate_node(
    graph: &mut SceneGraph,
    source_id: &str,
    new_id: &str,
    parent: Option<&str>,
) -> Option<String> {
    graph.duplicate(source_id, new_id, parent)
}

/// Convenience function: link a scene node to a semantic entity.
pub fn link_semantic(
    graph: &mut SceneGraph,
    node_id: &str,
    semantic_iri: &str,
    link_type: &str,
    confidence: Option<f32>,
) {
    graph.link_semantic(SemanticLink {
        node_id: node_id.to_string(),
        semantic_iri: semantic_iri.to_string(),
        link_type: link_type.to_string(),
        confidence,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::scene_graph::Light;

    #[test]
    fn scene_graph_add_node() {
        let mut graph = SceneGraph::new();
        graph.add_node(SceneNode::new("root"));
        assert_eq!(graph.len(), 1);
        assert!(graph.get("root").is_some());
    }

    #[test]
    fn scene_graph_hierarchy() {
        let mut graph = SceneGraph::new();
        graph.add_node(SceneNode::new("parent"));
        let child = SceneNode {
            id: "child".to_string(),
            parent: Some("parent".to_string()),
            ..SceneNode::new("child")
        };
        graph.add_node(child);
        // add_node should have registered child in parent's children list.
        assert_eq!(graph.get("parent").unwrap().children.len(), 1);
        assert_eq!(graph.get("parent").unwrap().children[0], "child");
    }

    #[test]
    fn scene_graph_add_light() {
        let mut graph = SceneGraph::new();
        let light = Light::point([1.0, 2.0, 3.0], [1.0; 3], 100.0);
        graph.add_light("light1", &light);
        let node = graph.get("light1").unwrap();
        assert!(node.metadata.contains_key("light_type"));
    }

    #[test]
    fn scene_graph_duplicate() {
        let mut graph = SceneGraph::new();
        graph.add_node(SceneNode::new("original").with_translation([1.0, 2.0, 3.0]));
        let new_id = graph.duplicate("original", "copy", None);
        assert_eq!(new_id, Some("copy".to_string()));
        let copy = graph.get("copy").unwrap();
        assert_eq!(copy.translation, [1.0, 2.0, 3.0]);
        assert_eq!(copy.id, "copy");
    }

    #[test]
    fn scene_graph_duplicate_with_children() {
        let mut graph = SceneGraph::new();
        graph.add_node(SceneNode::new("parent"));
        let mut child = SceneNode::new("child");
        child.parent = Some("parent".to_string());
        graph.add_node(child);
        // Update parent's children
        graph
            .get_mut("parent")
            .unwrap()
            .children
            .push("child".to_string());

        graph.duplicate("parent", "parent_copy", None);
        assert!(graph.get("parent_copy").is_some());
        assert!(graph.get("parent_copy/child").is_some());
    }

    #[test]
    fn scene_graph_link_semantic() {
        let mut graph = SceneGraph::new();
        graph.add_node(SceneNode::new("node1"));
        link_semantic(
            &mut graph,
            "node1",
            "did:qualia:entity1",
            "represents",
            Some(0.9),
        );
        assert_eq!(graph.links.len(), 1);
        assert_eq!(graph.links[0].link_type, "represents");
    }

    #[test]
    fn scene_graph_render_budget() {
        let mut graph = SceneGraph::new();
        graph.set_render_budget(16.6);
        assert!((graph.render_budget_ms - 16.6).abs() < 1e-6);
    }

    #[test]
    fn scene_graph_node_builder() {
        let node = SceneNode::new("test")
            .with_translation([1.0, 2.0, 3.0])
            .with_rotation([0.0, 0.0, 0.0, 1.0])
            .with_scale([2.0; 3])
            .with_mesh("mesh1")
            .with_material("mat1");
        assert_eq!(node.translation, [1.0, 2.0, 3.0]);
        assert_eq!(node.scale, [2.0; 3]);
        assert_eq!(node.mesh, Some("mesh1".to_string()));
        assert_eq!(node.material, Some("mat1".to_string()));
    }

    #[test]
    fn duplicate_nonexistent_returns_none() {
        let mut graph = SceneGraph::new();
        let result = graph.duplicate("nonexistent", "copy", None);
        assert_eq!(result, None);
    }
}
