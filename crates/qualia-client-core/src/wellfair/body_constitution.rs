//! The person's **declared body constitution** — measurements, characteristics, attributes.
//!
//! Forum-internum / Sanctuary-class. Stored next to the physiological-state declaration.
//! Absence means "not declared" — the caller uses an identity fit, never an assumed body.

use std::path::Path;

use wellfare_core::anatomy::{BodyConstitution, BodyFit, PhysiologicalState};

/// Where the constitution lives (prefs-style, under the `wellfair/` prefix).
pub const CONSTITUTION_FILE: &str = "wellfair/body_constitution.json";

pub fn load(storage_root: impl AsRef<Path>) -> Option<BodyConstitution> {
    let path = storage_root.as_ref().join(CONSTITUTION_FILE);
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save(storage_root: impl AsRef<Path>, body: &BodyConstitution) -> Result<(), String> {
    body.validate()?;
    let path = storage_root.as_ref().join(CONSTITUTION_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(body).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn clear(storage_root: impl AsRef<Path>) -> Result<(), String> {
    let path = storage_root.as_ref().join(CONSTITUTION_FILE);
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Fit using the constitution plus a physiological state so pregnancy is one source of truth.
/// A declared pregnant physiological state wins; otherwise the constitution's own hint is used.
pub fn fit_for(constitution: &BodyConstitution, phys: &PhysiologicalState) -> BodyFit {
    let pregnancy = match phys {
        PhysiologicalState::Reproductive(wellfare_core::anatomy::ReproductiveState::Pregnant(
            t,
        )) => Some(*t),
        _ => constitution.attributes.pregnancy,
    };
    constitution.fit_with_pregnancy(pregnancy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wellfare_core::anatomy::{Karyotype, ReproductiveState};

    #[test]
    fn persists_and_clears() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).is_none());
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(1720);
        c.characteristics.karyotype = Some(Karyotype::Xx);
        c.knowledge
            .ethnicities
            .push(wellfare_core::anatomy::EthnicityAffiliation::declared("Ashkenazi").unwrap());
        c.attributes.eye_colour = Some("brown".into());
        c.measurements.sleeve_mm = Some(610);
        c.measurements.foot_left_mm = Some(265);
        save(dir.path(), &c).unwrap();
        let back = load(dir.path()).unwrap();
        assert_eq!(back.measurements.stature_mm, Some(1720));
        assert_eq!(back.knowledge.ethnicities.len(), 1);
        assert_eq!(back.knowledge.ethnicities[0].token, "ashkenazi");
        assert_eq!(back.attributes.eye_colour.as_deref(), Some("brown"));
        assert_eq!(back.measurements.sleeve_mm, Some(610));
        assert_eq!(back.measurements.foot_left_mm, Some(265));
        clear(dir.path()).unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_rejects_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(10);
        assert!(save(dir.path(), &c).is_err());
    }

    #[test]
    fn phys_pregnancy_drives_fit() {
        let c = BodyConstitution::default();
        let phys = PhysiologicalState::Reproductive(ReproductiveState::Pregnant(
            wellfare_core::anatomy::Trimester::Third,
        ));
        let fit = fit_for(&c, &phys);
        assert!(fit.pregnancy_abdomen > 0.3);
    }
}
