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
    pub expose: bool,
    pub sidebar: bool,
    pub menu: Option<&'static str>,
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub time_progress: f64,
    pub playing: bool,
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
            expose: false,
            sidebar: false,
            menu: None,
            zoom: 0.9,
            pan_x: 70.0,
            pan_y: 40.0,
            time_progress: 0.61,
            playing: false,
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

        let node = CanvasNode {
            id: format!("container-{n}"),
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
        self.selected = Some(node.id.clone());
        self.nodes.push(node);
    }

    pub fn close(&mut self, id: &str) {
        self.nodes.retain(|n| n.id != id);
        self.wires.retain(|w| w.from != id && w.to != id);
        if self.selected.as_deref() == Some(id) {
            self.selected = self.nodes.first().map(|n| n.id.clone());
        }
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
        self.selected = Some(id.into());
    }

    pub fn resize_node(&mut self, id: &str, width: f64, height: f64) {
        if let Some(n) = self.node_mut(id) {
            n.width = snap(width).max(320.0);
            n.height = snap(height).max(220.0);
        }
        self.selected = Some(id.into());
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
