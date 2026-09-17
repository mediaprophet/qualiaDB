//! Map / media / sub-manifold — renderer contract + the kernel that authored it.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::gpu_frame::PoetGpuFrame;
use dioxus::prelude::*;

#[component]
pub fn MapBody() -> Element {
    rsx! {
        SpatialKernel {
            kind: "map",
            invoke_id: "ComputationalGeometry.convex_hull_2",
            sample: HULL,
            function: "hull",
            blurb: "Catchment hull from ComputationalGeometry.convex_hull_2, projected by Manifold.project, drawn by webizen-render.",
        }
    }
}

#[component]
pub fn MediaBody() -> Element {
    rsx! {
        SpatialKernel {
            kind: "media",
            invoke_id: "EngineeringAnalysis.kinematics",
            sample: KIN,
            function: "path",
            blurb: "10d-adjacent kinematics poses become scene nodes. Open /gpu-viewport for the native swapchain.",
        }
    }
}

#[component]
pub fn SubmanifoldBody() -> Element {
    rsx! {
        SpatialKernel {
            kind: "submanifold",
            invoke_id: "Manifold.axes",
            sample: AXES,
            function: "axes",
            blurb: "A nested surface. Axis taxonomy from Manifold.axes; scene is the same contract as the parent desk.",
        }
    }
}

#[component]
fn SpatialKernel(
    kind: &'static str,
    invoke_id: &'static str,
    sample: &'static str,
    function: &'static str,
    blurb: &'static str,
) -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);
    rsx! {
        div { style: "display:grid;gap:8px;",
            p { style: "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;", "{blurb}" }
            PoetGpuFrame { kind: kind, width: 720, height: 320 }
            button {
                disabled: busy(),
                style: crate::components::poet::vibe_console::secondary(),
                onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match engine::eval(sample.to_string(), false, Some(function.to_string())).await {
                            Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                            Err(e) => out.set(e),
                        }
                        busy.set(false);
                    });
                },
                "Run {invoke_id}"
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;max-height:140px;overflow:auto;", "{out}" }
            }
        }
    }
}

const HULL: &str = r#"requires [ capability("capability.invoke") ];
effect fn hull() {
    return capability.invoke("ComputationalGeometry.convex_hull_2", {
        points: [[0.2,0.6],[0.3,0.3],[0.6,0.2],[0.8,0.5],[0.6,0.8],[0.4,0.8],[0.5,0.5]]
    });
}
"#;

const KIN: &str = r#"requires [ capability("capability.invoke") ];
effect fn path() {
    return capability.invoke("EngineeringAnalysis.kinematics", { x0: 0.12, v0: 0.1, a: 0.015, t: [0,1,2,3,4,5] });
}
"#;

const AXES: &str = r#"requires [ capability("capability.invoke") ];
effect fn axes() {
    return capability.invoke("Manifold.axes", null);
}
"#;
