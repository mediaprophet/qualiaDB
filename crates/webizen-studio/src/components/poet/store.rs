//! HyperCanvas workbench state — same fields as `Canvas_Workbench/js/state.js`.

use super::kinds::{
    CanvasNode, ContainerKind, DimMode, DockPos, Epistemic, ManifoldId, Strata, ToolboxId, Wire,
};
use super::manifolds::{load_manifold, ManifoldSeed};

#[derive(Clone, Debug)]
pub struct Workbench {
    pub active: ManifoldId,
    pub title: String,
    pub graph_iri: String,
    pub dim: DimMode,
    pub strata: Vec<Strata>,
    pub epistemic: Epistemic,
    pub dock: DockPos,
    pub open_box: Option<ToolboxId>,
    pub selected: Option<String>,
    pub selected_wire: Option<String>,
    /// Radial "Connect Wire" armed source container id (two-step connect).
    pub wire_source: Option<String>,
    /// Transient operator feedback surfaced in the status bar.
    pub status_note: Option<String>,
    pub expose: bool,
    pub sidebar: bool,
    pub menu: Option<&'static str>,
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub time_progress: f64,
    pub playing: bool,
    /// Bumped on every play/pause toggle so stale ticker loops exit.
    pub play_epoch: u32,
    pub next_id: u32,
    pub nodes: Vec<CanvasNode>,
    pub wires: Vec<Wire>,
}

impl Workbench {
    pub fn new() -> Self {
        Self::from_seed(load_manifold(ManifoldId::Research))
    }

    pub fn from_seed(seed: ManifoldSeed) -> Self {
        Self {
            active: seed.id,
            title: seed.title.into(),
            graph_iri: seed.graph_iri.into(),
            dim: seed.id.default_dim(),
            strata: seed.strata,
            epistemic: Epistemic::All,
            dock: DockPos::Left,
            open_box: None,
            selected: seed.nodes.first().map(|n| n.id.clone()),
            selected_wire: None,
            wire_source: None,
            status_note: None,
            expose: false,
            sidebar: false,
            menu: None,
            zoom: 0.9,
            pan_x: 70.0,
            pan_y: 40.0,
            time_progress: 0.61,
            playing: false,
            play_epoch: 0,
            next_id: 100,
            nodes: seed.nodes,
            wires: seed.wires,
        }
    }

    pub fn switch(&mut self, id: ManifoldId) {
        *self = Self::from_seed(load_manifold(id));
    }

    pub fn strata_on(&self, s: Strata) -> bool {
        self.strata.contains(&s)
    }

    pub fn toggle_strata(&mut self, s: Strata) {
        if let Some(i) = self.strata.iter().position(|x| *x == s) {
            self.strata.remove(i);
        } else {
            self.strata.push(s);
        }
    }

    pub fn select_all_strata(&mut self) {
        self.strata = Strata::ALL.to_vec();
    }

    pub fn dimmed(&self, node: &CanvasNode) -> bool {
        let strata_off =
            !self.strata.is_empty() && self.strata.len() < 5 && !self.strata.contains(&node.strata);
        let epi_off = self.epistemic != Epistemic::All && node.epistemic != self.epistemic;
        strata_off || epi_off
    }

    pub fn find_smart_placement_slot(&self, width: f64, height: f64) -> (f64, f64) {
        let margin = 24.0;
        let cols = 4;
        let rows = 6;
        let start_x = 80.0;
        let start_y = 60.0;
        let step_x = width + 40.0;
        let step_y = height + 40.0;

        for r in 0..rows {
            for c in 0..cols {
                let test_x = start_x + (c as f64) * step_x;
                let test_y = start_y + (r as f64) * step_y;

                let overlaps = self.nodes.iter().any(|node| {
                    let r1_left = test_x;
                    let r1_right = test_x + width;
                    let r1_top = test_y;
                    let r1_bottom = test_y + height;

                    let r2_left = node.x;
                    let r2_right = node.x + node.width;
                    let r2_top = node.y;
                    let r2_bottom = node.y + node.height;

                    !(r1_right + margin <= r2_left
                        || r1_left >= r2_right + margin
                        || r1_bottom + margin <= r2_top
                        || r1_top >= r2_bottom + margin)
                });

                if !overlaps {
                    return (test_x, test_y);
                }
            }
        }

        // Fallback to primary focus position
        (start_x, start_y)
    }

    pub fn auto_arrange(&mut self) {
        let cols = 3;
        let start_x = 80.0;
        let start_y = 60.0;
        let gap_x = 40.0;
        let gap_y = 40.0;

        for (i, node) in self.nodes.iter_mut().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let target_w = node.width.max(380.0);
            let target_h = node.height.max(260.0);
            node.x = start_x + (col as f64) * (target_w + gap_x);
            node.y = start_y + (row as f64) * (target_h + gap_y);
        }
    }

    pub fn place(&mut self, kind: ContainerKind) {
        let n = self.next_id;
        self.next_id += 1;
        let default_w = 400.0;
        let default_h = 300.0;

        let (slot_x, slot_y) = self.find_smart_placement_slot(default_w, default_h);

        let id = format!("container-{n}");
        let node = CanvasNode {
            id: id.clone(),
            kind,
            title: kind.title().into(),
            x: slot_x,
            y: slot_y,
            width: default_w,
            height: default_h,
            z: 0.0,
            d: 1.0,
            strata: Strata::Technical,
            epistemic: Epistemic::Objective,
        };
        self.nodes.push(node);
        self.select_node(&id);
    }

    pub fn close(&mut self, id: &str) {
        self.nodes.retain(|n| n.id != id);
        self.wires.retain(|w| w.from != id && w.to != id);
        if self.selected.as_deref() == Some(id) {
            self.selected = self.nodes.first().map(|n| n.id.clone());
        }
        if self.wire_source.as_deref() == Some(id) {
            self.wire_source = None;
        }
        if let Some(wid) = self.selected_wire.clone() {
            if !self.wires.iter().any(|w| w.id == wid) {
                self.selected_wire = None;
            }
        }
    }

    /// Select a container, clearing any wire selection.
    pub fn select_node(&mut self, id: &str) {
        self.selected = Some(id.into());
        self.selected_wire = None;
    }

    /// Select a wire, clearing any container selection.
    pub fn select_wire(&mut self, id: &str) {
        self.selected_wire = Some(id.into());
        self.selected = None;
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.selected_wire = None;
    }

    /// Transient operator feedback shown in the status bar.
    pub fn note(&mut self, msg: impl Into<String>) {
        self.status_note = Some(msg.into());
    }

    /// Create a wire between two containers. Rejects self-loops, missing
    /// endpoints, and duplicate ordered pairs. Returns the new wire id.
    pub fn connect_wire(&mut self, from: &str, to: &str) -> Option<String> {
        if from == to
            || self.node(from).is_none()
            || self.node(to).is_none()
            || self.wires.iter().any(|w| w.from == from && w.to == to)
        {
            return None;
        }
        let id = format!("wire-{}", self.next_id);
        self.next_id += 1;
        self.wires.push(Wire {
            id: id.clone(),
            from: from.into(),
            to: to.into(),
            kind: "data-pipe".into(),
            label: "qualia:linksTo".into(),
        });
        self.select_wire(&id);
        Some(id)
    }

    pub fn remove_wire(&mut self, id: &str) {
        self.wires.retain(|w| w.id != id);
        if self.selected_wire.as_deref() == Some(id) {
            self.selected_wire = None;
        }
    }

    pub fn rename_wire(&mut self, id: &str, label: String) {
        if let Some(w) = self.wires.iter_mut().find(|w| w.id == id) {
            w.label = label;
        }
    }

    pub fn wire(&self, id: &str) -> Option<&Wire> {
        self.wires.iter().find(|w| w.id == id)
    }

    /// Wire kind drives the CSS class (`wire-{kind}`) — lane colour.
    pub fn set_wire_kind(&mut self, id: &str, kind: &str) {
        if let Some(w) = self.wires.iter_mut().find(|w| w.id == id) {
            w.kind = kind.into();
        }
    }

    /// Delete the selected wire or container. Returns true when something
    /// was removed.
    pub fn delete_selected(&mut self) -> bool {
        if let Some(id) = self.selected_wire.clone() {
            self.remove_wire(&id);
            return true;
        }
        if let Some(id) = self.selected.clone() {
            self.close(&id);
            return true;
        }
        false
    }

    /// Duplicate the selected container with a snapped +24px offset;
    /// selects the copy. Returns the new container id.
    pub fn duplicate_selected(&mut self) -> Option<String> {
        let src = self
            .selected
            .clone()
            .and_then(|id| self.node(&id).cloned())?;
        let n = self.next_id;
        self.next_id += 1;
        let id = format!("container-{n}");
        self.nodes.push(CanvasNode {
            id: id.clone(),
            title: format!("{} (copy)", src.title),
            x: snap(src.x + 24.0),
            y: snap(src.y + 24.0),
            ..src
        });
        self.select_node(&id);
        Some(id)
    }

    /// Radial "Connect Wire": the first call arms the selected container
    /// as the wire source; a second call with a different container
    /// selected completes the wire. Returns the new wire id on completion.
    pub fn connect_wire_via_selection(&mut self) -> Option<String> {
        match (self.wire_source.clone(), self.selected.clone()) {
            (Some(src), Some(dst)) if src != dst => {
                self.wire_source = None;
                self.connect_wire(&src, &dst)
            }
            (None, Some(sel)) => {
                self.wire_source = Some(sel);
                None
            }
            (Some(src), _) if !self.nodes.iter().any(|n| n.id == src) => {
                self.wire_source = None;
                None
            }
            _ => None,
        }
    }

    /// Serialize the desk (nodes + wires + title) to a `webizen.hcf/1`
    /// JSON envelope — the clipboard/localStorage interchange form.
    pub fn to_hcf_json(&self) -> String {
        let doc = serde_json::json!({
            "format": "webizen.hcf/1",
            "manifold": self.active.id(),
            "title": self.title,
            "graph": self.graph_iri,
            "nodes": self.nodes.iter().map(|n| serde_json::json!({
                "id": n.id, "kind": n.kind.id(), "title": n.title,
                "x": n.x, "y": n.y, "width": n.width, "height": n.height,
                "z": n.z, "d": n.d,
                "strata": n.strata.id(), "epistemic": n.epistemic.id(),
            })).collect::<Vec<_>>(),
            "wires": self.wires.iter().map(|w| serde_json::json!({
                "id": w.id, "from": w.from, "to": w.to,
                "kind": w.kind, "label": w.label,
            })).collect::<Vec<_>>(),
        });
        serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into())
    }

    /// Restore a desk document written by `to_hcf_json`. Replaces nodes,
    /// wires and title; drops wires whose endpoints don't resolve.
    /// Returns false when the payload isn't a `webizen.hcf/1` document.
    pub fn restore_hcf_json(&mut self, json: &str) -> bool {
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(json) else {
            return false;
        };
        if doc.get("format").and_then(|f| f.as_str()) != Some("webizen.hcf/1") {
            return false;
        }
        let mut nodes = Vec::new();
        if let Some(arr) = doc.get("nodes").and_then(|n| n.as_array()) {
            for n in arr {
                let get = |k: &str| n.get(k).and_then(|v| v.as_str());
                let num = |k: &str, d: f64| n.get(k).and_then(|v| v.as_f64()).unwrap_or(d);
                let (Some(id), Some(kind)) =
                    (get("id"), get("kind").and_then(ContainerKind::from_id))
                else {
                    continue;
                };
                nodes.push(CanvasNode {
                    id: id.into(),
                    kind,
                    title: get("title").unwrap_or(kind.title()).into(),
                    x: num("x", 80.0),
                    y: num("y", 60.0),
                    width: num("width", 400.0).max(320.0),
                    height: num("height", 300.0).max(220.0),
                    z: num("z", 0.0),
                    d: num("d", 1.0),
                    strata: get("strata")
                        .and_then(Strata::from_id)
                        .unwrap_or(Strata::Technical),
                    epistemic: get("epistemic")
                        .and_then(Epistemic::from_id)
                        .unwrap_or(Epistemic::Objective),
                });
            }
        }
        let mut wires = Vec::new();
        if let Some(arr) = doc.get("wires").and_then(|w| w.as_array()) {
            for w in arr {
                let get = |k: &str| w.get(k).and_then(|v| v.as_str());
                let (Some(id), Some(from), Some(to)) = (get("id"), get("from"), get("to")) else {
                    continue;
                };
                if !nodes.iter().any(|n| n.id == from) || !nodes.iter().any(|n| n.id == to) {
                    continue;
                }
                wires.push(Wire {
                    id: id.into(),
                    from: from.into(),
                    to: to.into(),
                    kind: get("kind").unwrap_or("data-pipe").into(),
                    label: get("label").unwrap_or("qualia:linksTo").into(),
                });
            }
        }
        if let Some(title) = doc.get("title").and_then(|t| t.as_str()) {
            self.title = title.into();
        }
        self.next_id = nodes
            .iter()
            .map(|n| n.id.as_str())
            .chain(wires.iter().map(|w| w.id.as_str()))
            .filter_map(|id| id.rsplit('-').next().and_then(|s| s.parse::<u32>().ok()))
            .max()
            .map(|m| m + 1)
            .unwrap_or(100)
            .max(100);
        self.nodes = nodes;
        self.wires = wires;
        self.selected = None;
        self.selected_wire = None;
        self.wire_source = None;
        true
    }

    /// Export the desk graph as Turtle — real triples, no mock.
    pub fn to_turtle(&self) -> String {
        let mut out = String::from(
            "@prefix qualia: <https://qualia.dev/ns#> .\n\
             @prefix graph: <https://qualia.dev/graph/> .\n\n",
        );
        out.push_str(&format!(
            "<{}> a qualia:ManifoldDesk ;\n  qualia:hasTitle {:?} ;\n  qualia:nodeCount {} ;\n  qualia:wireCount {} .\n\n",
            self.graph_iri,
            self.title,
            self.nodes.len(),
            self.wires.len()
        ));
        for n in &self.nodes {
            out.push_str(&format!(
                "qualia:{} a qualia:Container ;\n  qualia:hasKind \"{}\" ;\n  qualia:hasTitle {:?} ;\n  qualia:hasStrata \"{}\" ;\n  qualia:hasEpistemic \"{}\" ;\n  qualia:hasX {} ;\n  qualia:hasY {} .\n\n",
                n.id,
                n.kind.id(),
                n.title,
                n.strata.id(),
                n.epistemic.id(),
                n.x,
                n.y
            ));
        }
        for w in &self.wires {
            out.push_str(&format!(
                "qualia:{} {} qualia:{} .\n",
                w.from, w.label, w.to
            ));
        }
        out
    }

    pub fn node(&self, id: &str) -> Option<&CanvasNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut CanvasNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn move_node(&mut self, id: &str, x: f64, y: f64) {
        if let Some(n) = self.node_mut(id) {
            n.x = snap(x).max(0.0);
            n.y = snap(y).max(0.0);
        }
        self.select_node(id);
    }

    pub fn resize_node(&mut self, id: &str, width: f64, height: f64) {
        if let Some(n) = self.node_mut(id) {
            n.width = snap(width).max(320.0);
            n.height = snap(height).max(220.0);
        }
        self.select_node(id);
    }

    pub fn stage_transform(&self) -> String {
        match self.dim {
            DimMode::D2 => format!(
                "translate({}px, {}px) scale({})",
                self.pan_x, self.pan_y, self.zoom
            ),
            DimMode::D3 | DimMode::D4 => format!(
                "translate({}px, {}px) scale({}) rotateX(18deg) rotateY(-12deg)",
                self.pan_x, self.pan_y, self.zoom
            ),
        }
    }
}

fn snap(v: f64) -> f64 {
    (v / 8.0).round() * 8.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_wire_lifecycle() {
        let mut w = Workbench::new();
        w.place(ContainerKind::Doc);
        w.place(ContainerKind::Map);
        let a = w.nodes[w.nodes.len() - 2].id.clone();
        let b = w.nodes[w.nodes.len() - 1].id.clone();

        assert!(w.connect_wire(&a, &a).is_none()); // self-loop rejected
        let wid = w.connect_wire(&a, &b).expect("fresh pair connects");
        assert!(w.connect_wire(&a, &b).is_none()); // duplicate pair rejected
        assert_eq!(w.selected_wire.as_deref(), Some(wid.as_str()));
        assert!(w.selected.is_none());

        w.rename_wire(&wid, "qualia:cites".into());
        assert_eq!(w.wires.last().unwrap().label, "qualia:cites");

        assert!(w.delete_selected()); // deletes the wire first
        assert!(w.wires.iter().all(|x| x.id != wid));

        w.select_node(&a);
        assert!(w.delete_selected()); // then the selected container
        assert!(w.nodes.iter().all(|n| n.id != a));
    }

    #[test]
    fn armed_wire_completes_on_second_call() {
        let mut w = Workbench::new();
        w.place(ContainerKind::Doc);
        w.place(ContainerKind::Map);
        let a = w.nodes[w.nodes.len() - 2].id.clone();
        let b = w.nodes[w.nodes.len() - 1].id.clone();

        w.select_node(&a);
        assert!(w.connect_wire_via_selection().is_none()); // first call arms
        assert_eq!(w.wire_source.as_deref(), Some(a.as_str()));

        w.select_node(&b);
        assert!(w.connect_wire_via_selection().is_some()); // second completes
        assert!(w.wire_source.is_none());
        assert!(w.wires.iter().any(|x| x.from == a && x.to == b));
    }

    #[test]
    fn duplicate_offsets_and_selects_copy() {
        let mut w = Workbench::new();
        w.place(ContainerKind::Sheet);
        let src = w.nodes.last().unwrap().id.clone();
        let dup = w.duplicate_selected().expect("selected node duplicates");
        assert_ne!(dup, src);
        assert_eq!(w.selected.as_deref(), Some(dup.as_str()));
        let (s, d) = (w.node(&src).unwrap(), w.node(&dup).unwrap());
        assert!((d.x - s.x - 24.0).abs() < 8.0);
        assert!(d.title.ends_with(" (copy)"));
    }

    #[test]
    fn hcf_checkpoint_round_trips() {
        let mut w = Workbench::new();
        w.place(ContainerKind::Code);
        let id = w.nodes.last().unwrap().id.clone();
        let before = w.nodes.len();
        let json = w.to_hcf_json();

        let mut other = Workbench::new();
        assert!(other.restore_hcf_json(&json));
        assert_eq!(other.nodes.len(), before);
        assert_eq!(other.node(&id).unwrap().kind, ContainerKind::Code);

        assert!(!other.restore_hcf_json("{\"nope\":true}"));
        assert!(!other.restore_hcf_json("not json"));
    }

    #[test]
    fn turtle_export_is_prefixed_and_typed() {
        let w = Workbench::new();
        let ttl = w.to_turtle();
        assert!(ttl.contains("@prefix qualia:"));
        assert!(ttl.contains("a qualia:ManifoldDesk"));
        assert!(ttl.contains("qualia:hasTitle"));
        // Seed wires carry qualia: predicates.
        assert!(ttl.contains("qualia:groundsGeospatialObservation"));
    }
}
