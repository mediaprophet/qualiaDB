//! S5.7 — the **render surface** for the 3D Anatomy Qapp (interim visual).
//!
//! Builds a [`RenderScene`] representing the whole body as coloured regions per body system, painted by
//! the accumulated burden (σ → RGBA via [`AnatomyViewReport::system_percepts`]). This is the **interim
//! visual** noted in the progress log: a headless whole-body percept snapshot that lets Timothy *see* the
//! body coloured by burden without needing the ~200–290 MB live CCF/HRA GLB download (the cache + real
//! mesh path remains open). The orbit camera (azimuth/elevation) is driven by the caller so the Studio UI
//! can spin the body.
//!
//! The 17 body systems are placed at anatomically meaningful positions on a normalized body silhouette.
//! Each system renders as a coloured node whose radius grows with its burden (a bigger region = more
//! accumulated adverse load) and whose colour is the σ-derived RGBA from the percept. Distributed-overlay
//! systems (ECS / ENS / glymphatic) are rendered as translucent overlays over their host regions. The
//! scene is consumed by the headless `webizen_render::render_scene_png` pipeline — no browser WebGPU
//! required.

use webizen_render::scene_contract::{
    EpistemicState, RenderScene, SceneCamera, SceneNode, ScenePoint,
};

use super::anatomy_view::{AnatomyViewReport, SystemPercept};

/// The approximate anatomical position of each body system on a normalized body silhouette
/// (x: 0..1 left→right, y: 0..1 top→bottom, z: 0..1 front→back). These are illustrative — the real
/// 3D body (when the GLB cache lands) replaces this silhouette with organ meshes.
fn system_position(system_id: &str) -> ScenePoint {
    let p = match system_id {
        // Head / neck.
        "nervous" => (0.50, 0.12, 0.50),
        "sensory" => (0.50, 0.10, 0.55),
        "endocrine" => (0.50, 0.20, 0.50),
        // Chest.
        "respiratory" => (0.50, 0.28, 0.50),
        "circulatory" => (0.50, 0.30, 0.45),
        "immune" => (0.45, 0.32, 0.50),
        // Abdomen.
        "digestive" => (0.50, 0.45, 0.50),
        "urinary" => (0.50, 0.50, 0.55),
        "reticuloendothelial" => (0.55, 0.42, 0.50),
        "hematopoietic" => (0.50, 0.50, 0.40),
        // Pelvis.
        "reproductive" => (0.50, 0.58, 0.55),
        // Whole-body / distributed.
        "musculoskeletal" => (0.50, 0.50, 0.50),
        "integumentary" => (0.50, 0.50, 0.60),
        "thermoregulatory" => (0.50, 0.50, 0.62),
        // Distributed overlays — placed at their primary host region.
        "ens" => (0.50, 0.45, 0.52),        // enteric → digestive
        "glymphatic" => (0.50, 0.12, 0.52), // brain cleanup → nervous
        "ecs" => (0.50, 0.50, 0.50),        // endocannabinoid → whole-body
        _ => (0.50, 0.50, 0.50),
    };
    ScenePoint {
        x: p.0,
        y: p.1,
        z: p.2,
    }
}

/// Whether a system is a distributed overlay (rendered translucently over its hosts).
fn is_overlay(system_id: &str) -> bool {
    matches!(system_id, "ens" | "glymphatic" | "ecs")
}

/// The base radius for a system region (pixels). Distributed overlays are larger (they cover a region,
/// not a point); discrete systems are smaller and grow with burden.
fn base_radius(system_id: &str) -> f64 {
    if is_overlay(system_id) {
        28.0
    } else {
        10.0
    }
}

/// Convert a percept's normalized linear RGBA [0..1]⁴ to a CSS colour string for the renderer.
fn rgba_to_css(rgba: [f32; 4]) -> String {
    let r = (rgba[0] * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (rgba[1] * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (rgba[2] * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Build a whole-body [`RenderScene`] from an [`AnatomyViewReport`], coloured by accumulated burden and
/// viewed from `(azimuth, elevation)` in degrees. `azimuth` 0..360 rotates around the body; `elevation`
/// -90..90 looks up→down. The camera orbits at a fixed radius around the body centre.
pub fn body_scene(report: &AnatomyViewReport, azimuth_deg: f64, elevation_deg: f64) -> RenderScene {
    body_scene_with_fit(
        report,
        azimuth_deg,
        elevation_deg,
        &wellfare_core::anatomy::BodyFit::identity(),
    )
}

/// Like [`body_scene`], but stretches the silhouette by the person's declared stature / torso / legs.
pub fn body_scene_with_fit(
    report: &AnatomyViewReport,
    azimuth_deg: f64,
    elevation_deg: f64,
    fit: &wellfare_core::anatomy::BodyFit,
) -> RenderScene {
    let percepts = report.system_percepts();
    let mut scene = RenderScene {
        background: "#0a0f14".to_string(),
        camera: orbit_camera(azimuth_deg, elevation_deg),
        epistemic_filter: EpistemicState::Collapsed,
        ..Default::default()
    };

    // If there are no percepts, render the settled baseline for every system so the body is still
    // visible (not a blank screen).
    let all_systems = [
        "nervous",
        "sensory",
        "endocrine",
        "respiratory",
        "circulatory",
        "immune",
        "digestive",
        "urinary",
        "reticuloendothelial",
        "hematopoietic",
        "reproductive",
        "musculoskeletal",
        "integumentary",
        "thermoregulatory",
        "ens",
        "glymphatic",
        "ecs",
    ];

    for &sys in all_systems.iter() {
        let percept = percepts
            .iter()
            .find(|p| p.system_id == sys)
            .cloned()
            .unwrap_or_else(|| SystemPercept {
                system_id: sys.to_string(),
                level: wellfare_core::anatomy::WellbeingLevel::Settled,
                sigma: wellfare_core::anatomy::burden_to_sigma(0),
                rgba: [0.29, 0.62, 0.36, 1.0], // settled green
                frequency_hz: 0.0,
            });
        let pos = fit_silhouette_point(system_position(sys), fit);
        let overlay = is_overlay(sys);
        // Radius grows with burden: settled = base, under_strain = base × 2.2.
        let burden_scale = match percept.level {
            wellfare_core::anatomy::WellbeingLevel::UnderStrain => 2.2,
            wellfare_core::anatomy::WellbeingLevel::WorthWatching => 1.5,
            wellfare_core::anatomy::WellbeingLevel::Settled => 1.0,
        };
        let radius = base_radius(sys) * burden_scale;
        let alpha = if overlay { 0.35 } else { 0.92 };
        scene.add_node(SceneNode {
            id: sys.to_string(),
            position: pos,
            color: rgba_to_css(percept.rgba),
            radius,
            alpha,
            is_inferencing: percept.level != wellfare_core::anatomy::WellbeingLevel::Settled,
            pulse_rate: if percept.level == wellfare_core::anatomy::WellbeingLevel::UnderStrain {
                1.2
            } else {
                0.0
            },
            tensor: Default::default(),
            epistemic_state: EpistemicState::Collapsed,
            version: 0.0,
            entity_id: 0,
            affordance_bits: 0,
        });
    }

    scene
}

fn fit_silhouette_point(
    p: ScenePoint,
    fit: &wellfare_core::anatomy::BodyFit,
) -> ScenePoint {
    // Silhouette y is top→bottom (0 head, 1 feet) — invert for the CCF-style fit bands.
    let y_up = 1.0 - p.y as f32;
    let y_seg = if y_up < fit.pelvis_y_norm {
        fit.leg_scale_y
    } else {
        fit.torso_scale_y
    };
    let y_from_feet = (1.0 - p.y) * (y_seg as f64) * (fit.stature_scale as f64);
    let cx = 0.50;
    ScenePoint {
        x: cx + (p.x - cx) * (fit.arm_span_scale_x as f64) * (fit.shoulder_scale_x as f64),
        y: (1.0 - y_from_feet).clamp(0.02, 0.98),
        z: p.z,
    }
}

/// The orbit camera for `(azimuth, elevation)` in degrees, looking at the body centre `[0.5, 0.5, 0]`
/// from a fixed radius.
fn orbit_camera(azimuth_deg: f64, elevation_deg: f64) -> SceneCamera {
    let az = azimuth_deg.to_radians();
    let el = elevation_deg.clamp(-89.0, 89.0).to_radians();
    let radius = 1.8;
    let x = 0.5 + radius * el.cos() * az.sin();
    let y = 0.5 + radius * el.sin();
    let z = radius * el.cos() * az.cos();
    SceneCamera {
        position: [x, y, z],
        target: [0.5, 0.5, 0.0],
        fov: 50.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wellfare_core::anatomy::{PhysiologicalState, RecordRef};

    #[test]
    fn body_scene_has_a_node_per_system_and_orbits() {
        let refs = vec![RecordRef::new("r:htn", "condition", "Hypertension")];
        let report = super::super::anatomy_view::build_report(
            refs,
            wellfare_core::anatomy::Lens::Person,
            2,
            PhysiologicalState::Baseline,
        );
        let scene = body_scene(&report, 0.0, 0.0);
        // All 17 systems are represented (even unburdened ones → settled baseline).
        assert_eq!(scene.nodes.len(), 17, "every body system is rendered");
        // The circulatory system (hypertension) is under strain → bigger + pulsing.
        let circ = scene.nodes.iter().find(|n| n.id == "circulatory").unwrap();
        assert!(
            circ.radius > 10.0,
            "burdened circulatory is enlarged: {}",
            circ.radius
        );
        assert!(circ.is_inferencing, "burdened system is inferencing");
        assert!(circ.pulse_rate > 0.0, "under-strain pulses");
        // A settled system (e.g. respiratory) is calm.
        let resp = scene.nodes.iter().find(|n| n.id == "respiratory").unwrap();
        assert!(!resp.is_inferencing, "settled system is not inferencing");
        assert_eq!(resp.pulse_rate, 0.0, "settled does not pulse");
        // Distributed overlays are translucent.
        let ens = scene.nodes.iter().find(|n| n.id == "ens").unwrap();
        assert!(ens.alpha < 0.5, "overlay is translucent: {}", ens.alpha);
    }

    #[test]
    fn orbit_camera_rotates_with_azimuth() {
        let front = orbit_camera(0.0, 0.0);
        let side = orbit_camera(90.0, 0.0);
        // At azimuth 0 the camera is in front (z > 0); at azimuth 90 it's to the side (x shifts).
        assert!(
            front.position[2] > side.position[2],
            "front view has more z"
        );
        assert!(
            (side.position[0] - 0.5).abs() > 0.01,
            "side view shifts x off-centre"
        );
        // Elevation tilts the camera up/down.
        let up = orbit_camera(0.0, 45.0);
        assert!(
            up.position[1] > front.position[1],
            "elevation raises the camera"
        );
    }

    #[test]
    fn rgba_to_css_round_trips_primary_channels() {
        assert_eq!(rgba_to_css([1.0, 0.0, 0.0, 1.0]), "#ff0000");
        assert_eq!(rgba_to_css([0.0, 1.0, 0.0, 1.0]), "#00ff00");
        assert_eq!(rgba_to_css([0.0, 0.0, 1.0, 1.0]), "#0000ff");
        // Clamping.
        assert_eq!(rgba_to_css([2.0, -1.0, 0.5, 1.0]), "#ff0080");
    }
}
