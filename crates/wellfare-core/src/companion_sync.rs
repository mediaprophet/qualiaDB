//! Phone → desktop health bundle transport (Samsung CSV exports from companion WASM).

use serde::{Deserialize, Serialize};

pub const COMPANION_HEALTH_BUNDLE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionCsvFile {
    pub filename: String,
    pub csv_content: String,
}

/// Versioned bundle produced on the user's phone and ingested on the authoritative desktop host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionHealthBundle {
    pub schema_version: u32,
    pub operation_id: String,
    pub device_id: String,
    pub captured_at_unix: u32,
    pub files: Vec<CompanionCsvFile>,
}

impl CompanionHealthBundle {
    pub fn new(
        device_id: impl Into<String>,
        captured_at_unix: u32,
        files: Vec<CompanionCsvFile>,
    ) -> Self {
        Self {
            schema_version: COMPANION_HEALTH_BUNDLE_SCHEMA,
            operation_id: uuid::Uuid::new_v4().to_string(),
            device_id: device_id.into(),
            captured_at_unix,
            files,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMPANION_HEALTH_BUNDLE_SCHEMA {
            return Err(format!(
                "unsupported schema_version {} (expected {COMPANION_HEALTH_BUNDLE_SCHEMA})",
                self.schema_version
            ));
        }
        if self.device_id.trim().is_empty() {
            return Err("device_id is required".into());
        }
        if self.files.is_empty() {
            return Err("bundle must contain at least one CSV file".into());
        }
        for file in &self.files {
            if file.filename.trim().is_empty() {
                return Err("each file must have a filename".into());
            }
            if file.csv_content.trim().is_empty() {
                return Err(format!("{} has empty CSV content", file.filename));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_nonempty_bundle() {
        let bundle = CompanionHealthBundle::new(
            "phone-abc",
            1_700_000_000,
            vec![CompanionCsvFile {
                filename: "weight.csv".into(),
                csv_content: "uuid,start_time,weight\nx,1,70".into(),
            }],
        );
        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn rejects_empty_files() {
        let bundle = CompanionHealthBundle::new("phone", 0, vec![]);
        assert!(bundle.validate().is_err());
    }
}
