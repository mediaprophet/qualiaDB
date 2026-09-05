use web_sys::{Document, Element};

use super::super::super::cop_records::CopField;
use super::ledger;

pub fn build_channel_strip_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Channel strips persist EQ/filter/comp settings. Audio.filter / Audio.eq are the live kernels.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (strip)",
            },
            CopField {
                key: "filter_type",
                placeholder: "Filter (lowpass|highpass|bandpass)",
            },
            CopField {
                key: "cutoff",
                placeholder: "Cutoff Hz",
            },
        ],
    )
}

pub fn build_routing_matrix_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Routing matrix records (src → dest). This is session wiring, not a fabricated patchbay graph.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (route)",
            },
            CopField {
                key: "src",
                placeholder: "Source",
            },
            CopField {
                key: "dest",
                placeholder: "Destination",
            },
        ],
    )
}

pub fn build_meter_bridge_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Meter snapshots persist here. Audio.waveform_meter / Audio.loudness_meter require an input buffer.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (meter)",
            },
            CopField {
                key: "peak",
                placeholder: "Peak",
            },
            CopField {
                key: "loudness",
                placeholder: "Loudness",
            },
        ],
    )
}

pub fn build_automation_lanes_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Automation breakpoints persist as session records.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (automation)",
            },
            CopField {
                key: "param",
                placeholder: "Parameter",
            },
            CopField {
                key: "value",
                placeholder: "Value",
            },
        ],
    )
}

pub fn build_spatial_audio_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Spatial audio poses persist here. HRTF decode requires a bound Audio session decoder.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (spatial)",
            },
            CopField {
                key: "azimuth",
                placeholder: "Azimuth",
            },
            CopField {
                key: "elevation",
                placeholder: "Elevation",
            },
        ],
    )
}

pub fn build_hrtf_personalization_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "HRTF profiles persist as records. SOFA import is unbound until a decoder session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (hrtf)",
            },
            CopField {
                key: "sofa",
                placeholder: "SOFA URI",
            },
        ],
    )
}

pub fn build_manifold_transition_audio_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Manifold-transition audio cues persist as session records.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (transition)",
            },
            CopField {
                key: "from",
                placeholder: "From manifold",
            },
            CopField {
                key: "to",
                placeholder: "To manifold",
            },
        ],
    )
}

pub fn build_desk_persistence_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Desk recall snapshots persist on the COP ledger.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (recall)",
            },
            CopField {
                key: "name",
                placeholder: "Snapshot name",
            },
        ],
    )
}

pub fn build_animation_timeline_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_animation",
        "Animation keys persist as session records. Dual Studio evaluates Animation.* presets live.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (key)",
            },
            CopField {
                key: "preset",
                placeholder: "Preset",
            },
            CopField {
                key: "t",
                placeholder: "t",
            },
        ],
    )
}

pub fn build_animation_export_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_animation",
        "Export jobs persist here. Container encode is unbound until an export session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (export)",
            },
            CopField {
                key: "format",
                placeholder: "Format",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
    )
}

pub fn build_asset_library_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_asset",
        "Studio assets persist as records. Mesh upload uses Render.gpu_upload_mesh when Dual Studio holds a surface.",
        &[
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "format",
                placeholder: "Format",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}
