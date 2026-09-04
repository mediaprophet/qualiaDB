use web_sys::{Document, Element};

use super::ledger;
use super::super::super::cop_records::CopField;

pub fn build_scene_graph_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Scene graph nodes persist as session records. Scene.add_node is the live capability.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (node)",
            },
            CopField {
                key: "id",
                placeholder: "Node id",
            },
            CopField {
                key: "x",
                placeholder: "x",
            },
            CopField {
                key: "y",
                placeholder: "y",
            },
            CopField {
                key: "z",
                placeholder: "z",
            },
        ],
    )
}

pub fn build_material_editor_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Material records for the Scene session. GPU material compile stays unbound until Dual Studio holds a surface.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (material)",
            },
            CopField {
                key: "albedo",
                placeholder: "Albedo",
            },
            CopField {
                key: "roughness",
                placeholder: "Roughness",
            },
        ],
    )
}

pub fn build_lighting_editor_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Lights persist on the Scene session. Scene.add_light is the live capability.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (light)",
            },
            CopField {
                key: "intensity",
                placeholder: "Intensity",
            },
            CopField {
                key: "colour",
                placeholder: "Colour",
            },
        ],
    )
}

pub fn build_shadow_settings_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Shadow settings persist on the Scene session. Mapping requires a GPU surface.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (shadow)",
            },
            CopField {
                key: "mode",
                placeholder: "Mode",
            },
        ],
    )
}

pub fn build_lod_chain_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "LOD chain records. compile_10d stays unbound until a render session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (lod)",
            },
            CopField {
                key: "level",
                placeholder: "Level",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
        ],
    )
}

pub fn build_gis_maps_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "GIS map records for the Scene session. GeoSPARQL query is unbound until a graph endpoint is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (gis)",
            },
            CopField {
                key: "crs",
                placeholder: "CRS",
            },
            CopField {
                key: "extent",
                placeholder: "Extent",
            },
        ],
    )
}

pub fn build_ragdoll_skin_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_animation",
        "Skeleton/skin records. Joint physics is unbound until a Scene physics session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (skin)",
            },
            CopField {
                key: "joints",
                placeholder: "Joint count",
            },
        ],
    )
}

pub fn build_tensor_inspector_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Tensor inspector records. Render.gpu_upload_tensor is live when Dual Studio has a GPU surface.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (tensor)",
            },
            CopField {
                key: "shape",
                placeholder: "Shape",
            },
        ],
    )
}

pub fn build_spatial_10d_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "10D manifold pose records. Live axes come from Manifold.axes, not fabricated HUD numbers.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (manifold)",
            },
            CopField {
                key: "d0",
                placeholder: "D0 epistemic",
            },
            CopField {
                key: "d5",
                placeholder: "D5 temporal",
            },
        ],
    )
}

pub fn build_desk_surface_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Mixer desk is an Audio session surface. DSP plugins persist as records; AudioWorklet playback is unbound in this shell.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (desk)",
            },
            CopField {
                key: "channels",
                placeholder: "Channel count",
            },
        ],
    )
}

