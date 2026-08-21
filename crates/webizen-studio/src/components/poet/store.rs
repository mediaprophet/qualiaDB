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

    pub fn place(&mut self, kind: ContainerKind) {
        let n = self.next_id;
        self.next_id += 1;
        let node = CanvasNode {
            id: format!("container-{n}"),
            kind,
            title: kind.title().into(),
            x: 180.0 + (n as f64 % 5.0) * 36.0,
            y: 140.0 + (n as f64 % 4.0) * 28.0,
            width: 380.0,
            height: 260.0,
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
