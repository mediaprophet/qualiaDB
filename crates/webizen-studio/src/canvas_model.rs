//! Shared workspace canvas types used by the studio editor and render panes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::theme_engine::{ThemeBinding, ThemeDefinition};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LayoutStrategy {
    PointGrid {
        width_points: u16,
        height_points: u16,
        snap_step: u16,
        gutter: u16,
    },
    CssGrid {
        cols: u8,
        rows: u8,
        gap: u8,
    },
    FlexBox,
    Masonry,
}

impl Default for LayoutStrategy {
    fn default() -> Self {
        Self::PointGrid {
            width_points: 96,
            height_points: 64,
            snap_step: 2,
            gutter: 2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PresentationMode {
    GridBound,
    NodeRelational,
    Spatial,
}

impl Default for PresentationMode {
    fn default() -> Self {
        Self::GridBound
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CoordinateSpace {
    GlobalCartesian,
    RelativeAnchored,
}

impl Default for CoordinateSpace {
    fn default() -> Self {
        Self::GlobalCartesian
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UiMode {
    NativeDioxus,
    IFrameSandbox,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LayerBehavior {
    Docked,
    FloatingOverlay,
    ModalOverlay,
    FullCanvas,
}

impl Default for LayerBehavior {
    fn default() -> Self {
        Self::Docked
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PanePlacement {
    pub component_id: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub data_bindings: Vec<String>,
    #[serde(default)]
    pub binds_rpc: Option<String>,
    #[serde(default)]
    pub requires_capability: Vec<String>,
    #[serde(default)]
    pub ui_mode: Option<UiMode>,
    #[serde(default)]
    pub layer: LayerBehavior,
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub min_w_points: u16,
    #[serde(default)]
    pub min_h_points: u16,
    #[serde(default)]
    pub supported_presentations: Vec<PresentationMode>,
    #[serde(default)]
    pub theme: ThemeBinding,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Page {
    pub url_path: String,
    pub name: String,
    #[serde(default)]
    pub layout_strategy: LayoutStrategy,
    pub panes: Vec<PanePlacement>,
    #[serde(default)]
    pub presentation_mode: PresentationMode,
    #[serde(default)]
    pub coordinate_space: CoordinateSpace,
    #[serde(default)]
    pub pan_and_zoom: bool,
    #[serde(default)]
    pub theme: ThemeBinding,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
    #[serde(default)]
    pub theme_tokens: HashMap<String, String>,
    #[serde(default)]
    pub themes: Vec<ThemeDefinition>,
    #[serde(default)]
    pub environment_theme: ThemeBinding,
    #[serde(default)]
    pub app_theme: ThemeBinding,
}