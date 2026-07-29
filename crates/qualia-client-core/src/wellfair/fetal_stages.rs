//! Developmental (embryonic/fetal) assets — the CC-BY Carnegie human-embryo series from NIH 3D, keyed
//! by gestational age = the `.10d` **`t`-axis** coordinate. This is the first concrete consumer of the
//! t-axis (reproductive-continuum plan §2): geometry as a function of developmental time.
//!
//! Source: **NIH 3D** (`3d.nih.gov`) — the same repository that hosts the HRA adult library
//! ([[hra-sparql-ccf-organ-assets]]). The Carnegie Human Embryo series (author kbrowne, NIH/NIAID) covers
//! Carnegie stages 12–23 (~26–56 postfertilization days — the embryonic period, weeks 4–8). **CC-BY**
//! (attribution) — derivatives and the `.10d` pipeline are permitted (unlike the CC-BY-NC-ND / -NC-SA
//! embryology atlases). Each stage's GLB is fetched via `https://3d.nih.gov/api/files/<fileId>` (raw S3 is
//! access-denied; NIH 3D serves files through that endpoint).
//!
//! Honest scope: this is the **embryonic** period (Carnegie stages, ≤ ~8 weeks). The later *fetal* period
//! (9 weeks → birth) is not covered by a comparably clean CC-BY 3D series and is a separate ⚑ acquisition.

use serde::{Deserialize, Serialize};

/// One Carnegie developmental stage: its gestational-age `t`-coordinate + the NIH 3D GLB to fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarnegieStage {
    /// Carnegie stage number (12–23). Ordering *is* the developmental `t`-axis order.
    pub stage: u8,
    /// Approximate postfertilization age in days — the `.10d` `t`-axis coordinate for this body.
    pub postfertilization_days: u16,
    /// NIH 3D entry id (provenance / attribution).
    pub nih3d_entry: &'static str,
    /// NIH 3D fileId of the GLB (served via `/api/files/<id>`).
    pub glb_file_id: u32,
}

impl CarnegieStage {
    /// The direct GLB download URL (the NIH 3D file endpoint; raw S3 is access-denied).
    pub fn glb_url(&self) -> String {
        format!("https://3d.nih.gov/api/files/{}", self.glb_file_id)
    }

    /// A stable key for the stage (feeds the compile / manifest, like an organ key).
    pub fn key(&self) -> String {
        format!("carnegie-stage-{}", self.stage)
    }
}

/// The curated Carnegie embryo series on NIH 3D (CC-BY), ordered by developmental time.
/// Postfertilization ages are the standard Carnegie-stage values (stage 16 ≈ 39 d is confirmed on NIH 3D).
pub fn carnegie_series() -> Vec<CarnegieStage> {
    vec![
        CarnegieStage {
            stage: 12,
            postfertilization_days: 26,
            nih3d_entry: "3DPX-016955",
            glb_file_id: 565713,
        },
        CarnegieStage {
            stage: 14,
            postfertilization_days: 32,
            nih3d_entry: "3DPX-016954",
            glb_file_id: 502105,
        },
        CarnegieStage {
            stage: 16,
            postfertilization_days: 39,
            nih3d_entry: "3DPX-016953",
            glb_file_id: 508256,
        },
        CarnegieStage {
            stage: 18,
            postfertilization_days: 44,
            nih3d_entry: "3DPX-016952",
            glb_file_id: 501993,
        },
        CarnegieStage {
            stage: 20,
            postfertilization_days: 49,
            nih3d_entry: "3DPX-016951",
            glb_file_id: 501897,
        },
        CarnegieStage {
            stage: 23,
            postfertilization_days: 56,
            nih3d_entry: "3DPX-016950",
            glb_file_id: 502070,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_is_monotonic_in_developmental_time() {
        let s = carnegie_series();
        assert_eq!(s.len(), 6);
        // The t-axis (stage / gestational age) is strictly increasing — the developmental ordering.
        for w in s.windows(2) {
            assert!(w[1].stage > w[0].stage, "stage order");
            assert!(
                w[1].postfertilization_days > w[0].postfertilization_days,
                "t-axis (gestational age) must be monotonic"
            );
        }
        assert_eq!(
            s.iter()
                .find(|x| x.stage == 16)
                .unwrap()
                .postfertilization_days,
            39
        );
        assert_eq!(s[0].glb_url(), "https://3d.nih.gov/api/files/565713");
        assert_eq!(s.last().unwrap().key(), "carnegie-stage-23");
    }

    /// Real-asset harness: fetch + compile the whole NIH 3D Carnegie embryo series into `.10d`, ordered
    /// along the gestational-age `t`-axis. Live network — ignored by default.
    #[test]
    #[ignore = "live network: fetches + compiles the NIH 3D Carnegie embryo series"]
    fn compile_fetal_series_from_nih3d() {
        use crate::wellfair::ccf_resolver::fetch_glb;
        use qualia_core_db::render::compile_10d::{compile_developmental_asset, decode_10d_mesh};

        let mut compiled = 0usize;
        for st in carnegie_series() {
            let bytes = match fetch_glb(&st.glb_url()) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("CARNEGIE {} fetch FAILED: {e}", st.stage);
                    continue;
                }
            };
            let src = bytes.len();
            let uri = format!("urn:qualia:anatomy:fetal:{}", st.key());
            // Compile with the developmental `t`-coordinate (gestational age + stage) bound into the manifest.
            match compile_developmental_asset(
                &bytes,
                Some("glb"),
                &uri,
                "glb",
                st.postfertilization_days,
                st.stage,
            ) {
                Ok(a) => {
                    let mesh = decode_10d_mesh(&a.container_10d).expect("10d round-trip");
                    compiled += 1;
                    eprintln!(
                        "CARNEGIE {:>2} (~{} pf-days, t-axis) · {} verts / {} tris · GLB {} B -> .10d {} B ({:.2}x)",
                        st.stage,
                        st.postfertilization_days,
                        mesh.vertex_count(),
                        mesh.triangle_count(),
                        src,
                        a.container_10d.len(),
                        src as f64 / a.container_10d.len().max(1) as f64,
                    );
                }
                Err(e) => eprintln!("CARNEGIE {} compile FAILED: {e:?}", st.stage),
            }
        }
        assert!(
            compiled >= 5,
            "most of the Carnegie series should compile (got {compiled})"
        );
    }
}
