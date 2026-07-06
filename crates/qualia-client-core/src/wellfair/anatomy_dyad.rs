//! The maternal–fetal **dyad**: a maternal body coupled to a developmental (fetal) body at a gestational
//! age, positioned at the maternal host structure (the uterus). This is *the consideration made explicit*
//! — the fetus modelled as part of the female-anatomy picture, not scoped out: two coupled bodies across
//! space and the developmental `t`-axis (reproductive-continuum plan §2/§3.3).
//!
//! The maternal body is the HRA female model; the fetal body is a NIH 3D Carnegie stage
//! ([`crate::wellfair::fetal_stages`]). Placement here is **illustrative**: the two meshes come from
//! different sources in different scales, so an anatomically-registered placement needs real-world-scale
//! metadata on both (a follow-up). What this demonstrates is the coupling + a computed transform that
//! seats the embryo inside the uterine frame.

use crate::wellfair::fetal_stages::CarnegieStage;
use serde::{Deserialize, Serialize};
use wellfare_core::anatomy::AnatomyModel;

/// The dyad: which maternal body, which host structure, and the fetal body's point on the `t`-axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaternalFetalDyad {
    pub maternal_model: AnatomyModel,
    /// The maternal host structure the fetal body couples to (the uterus).
    pub host_structure: String,
    pub carnegie_stage: u8,
    pub gestational_age_days: u16,
}

impl MaternalFetalDyad {
    /// A dyad at a given developmental stage. The maternal side is the female model; the host is the uterus.
    pub fn at_stage(stage: &CarnegieStage) -> Self {
        Self {
            maternal_model: AnatomyModel::Female,
            host_structure: "uterus".to_string(),
            carnegie_stage: stage.stage,
            gestational_age_days: stage.postfertilization_days,
        }
    }
}

/// A computed transform seating a (centroid-origin) fetal mesh within a host (uterine) frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DyadPlacement {
    /// Translate the fetal mesh so its centroid sits at the host centroid.
    pub translate: [f32; 3],
    /// Uniform scale so the fetal mesh's largest extent = `fill` × the host's smallest extent.
    pub scale: f32,
}

/// Compute an **illustrative** placement of a fetal mesh within a host frame: centre it at the host
/// centroid, scaled so its largest extent fills `fill` (0..1) of the host's smallest extent (so it sits
/// inside). Pure. Illustrative only — see the module note on scale registration.
pub fn place_within(
    host_min: [f32; 3],
    host_max: [f32; 3],
    fetal_min: [f32; 3],
    fetal_max: [f32; 3],
    fill: f32,
) -> DyadPlacement {
    let mid = |lo: f32, hi: f32| (lo + hi) * 0.5;
    let translate = [
        mid(host_min[0], host_max[0]),
        mid(host_min[1], host_max[1]),
        mid(host_min[2], host_max[2]),
    ];
    let host_min_ext = (host_max[0] - host_min[0])
        .min(host_max[1] - host_min[1])
        .min(host_max[2] - host_min[2])
        .max(1e-6);
    let fetal_max_ext = (fetal_max[0] - fetal_min[0])
        .max(fetal_max[1] - fetal_min[1])
        .max(fetal_max[2] - fetal_min[2])
        .max(1e-6);
    DyadPlacement {
        translate,
        scale: (fill.clamp(0.0, 1.0) * host_min_ext) / fetal_max_ext,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyad_couples_female_uterus_and_stage_and_placement_fits() {
        let stage = CarnegieStage {
            stage: 18,
            postfertilization_days: 44,
            nih3d_entry: "3DPX-016952",
            glb_file_id: 501993,
        };
        let dyad = MaternalFetalDyad::at_stage(&stage);
        assert_eq!(dyad.maternal_model, AnatomyModel::Female);
        assert_eq!(dyad.host_structure, "uterus");
        assert_eq!(dyad.carnegie_stage, 18);
        assert_eq!(dyad.gestational_age_days, 44);

        // Host: a 2×2×2 box centred at (10,0,0). Fetal: a 4-wide box. fill=0.5 → target extent = 1.0;
        // fetal max-extent = 4 → scale 0.25; centred at the host centroid.
        let p = place_within([9.0, -1.0, -1.0], [11.0, 1.0, 1.0], [-2.0, -2.0, -2.0], [2.0, 2.0, 2.0], 0.5);
        assert_eq!(p.translate, [10.0, 0.0, 0.0]);
        assert!((p.scale - 0.25).abs() < 1e-6, "scale {}", p.scale);
    }

    /// Real-asset harness: seat a Carnegie embryo within the actual HRA female uterus (the dyad on real
    /// data). Live network — ignored by default.
    #[test]
    #[ignore = "live network: places a Carnegie embryo within the maternal HRA uterus"]
    fn place_embryo_in_maternal_uterus_from_real_assets() {
        use crate::wellfair::ccf_resolver::{discover_ref_organs, fetch_glb, HRA_SPARQL_ENDPOINT};
        use crate::wellfair::fetal_stages::carnegie_series;
        use qualia_core_db::render::compile_10d::compile_asset;

        let organs = discover_ref_organs(HRA_SPARQL_ENDPOINT).expect("discover HRA");
        let uterus = organs
            .iter()
            .find(|o| o.filename.contains("uterus"))
            .expect("uterus in the HRA female set");
        let u_mesh = compile_asset(
            &fetch_glb(&uterus.glb_url).expect("fetch uterus"),
            Some("glb"),
            "urn:host:uterus",
            "glb",
        )
        .unwrap()
        .mesh;

        let stage = carnegie_series().into_iter().find(|s| s.stage == 18).unwrap();
        let f_mesh = compile_asset(
            &fetch_glb(&stage.glb_url()).expect("fetch embryo"),
            Some("glb"),
            "urn:fetal:s18",
            "glb",
        )
        .unwrap()
        .mesh;

        let dyad = MaternalFetalDyad::at_stage(&stage);
        let p = place_within(u_mesh.min, u_mesh.max, f_mesh.min, f_mesh.max, 0.6);
        let ext = |m: &qualia_core_db::render::assets::Mesh| {
            [m.max[0] - m.min[0], m.max[1] - m.min[1], m.max[2] - m.min[2]]
        };
        eprintln!(
            "DYAD {:?}: Carnegie {} (~{}d) seated in {} — translate {:?}, scale {:.5} · uterus ext {:?} · embryo ext {:?}",
            dyad.maternal_model,
            dyad.carnegie_stage,
            dyad.gestational_age_days,
            dyad.host_structure,
            p.translate,
            p.scale,
            ext(&u_mesh),
            ext(&f_mesh),
        );
        assert!(p.scale > 0.0);
    }
}
