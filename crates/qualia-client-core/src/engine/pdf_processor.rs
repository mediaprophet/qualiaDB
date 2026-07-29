use pdf_extract::extract_text;
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Extracts text from a PDF file and stores it into the local library,
/// supporting underlying systems like LLMs, graph schemas, and ontological CML assertions.
pub async fn ingest_pdf_to_library(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    // 1. Parse document using pdf_extract
    let text = extract_text(path).map_err(|e| format!("Failed to extract PDF text: {}", e))?;

    // 2. Store to library
    let config_path = crate::state::config_file_path();
    let storage_path = if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<crate::state::AgentConfig>(&config_str) {
            config.storage_path
        } else {
            crate::state::dirs_default_path()
        }
    } else {
        crate::state::dirs_default_path()
    };

    let library_dir = std::path::PathBuf::from(&storage_path).join("library");
    if !library_dir.exists() {
        let _ = fs::create_dir_all(&library_dir);
    }

    let file_id = Uuid::new_v4().to_string();
    let txt_path = library_dir.join(format!("{}.txt", file_id));

    fs::write(&txt_path, &text).map_err(|e| format!("Failed to write text to library: {}", e))?;

    Ok(format!(
        "PDF ingested successfully. Text saved to {:?}",
        txt_path
    ))
}
