//! Ingestion and model/inference

#![allow(non_snake_case)]

use qualia_client_core::api;
use qualia_client_core::engine::{ingestion, llm_offload};
use qualia_client_core::state::ProgressPayload;
use tauri::command;

// ── Ingest ────────────────────────────────────────────────────────────────────

#[command]
pub async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    api::ingest_pdf(file_name).await
}

#[command]
pub async fn ingest_literature(file_path: String) -> Result<String, String> {
    api::ingest_literature(file_path).await
}

#[command]
pub async fn upsert_cmld_definition(term: String, context_did: String) -> Result<String, String> {
    api::upsert_cmld_definition(term, context_did).await
}

#[command]
pub async fn ingest_ontology(file_name: String) -> Result<serde_json::Value, String> {
    api::ingest_ontology(file_name).await
}

#[command]
pub async fn export_to_solid(
    input_q42_path: String,
    output_dir_path: String,
) -> Result<String, String> {
    api::export_to_solid(input_q42_path, output_dir_path).await
}

#[command]
pub async fn ingest_image(file_path: String) -> Result<serde_json::Value, String> {
    api::ingest_image(file_path).await
}

#[command]
pub async fn ingest_image_async(file_path: String, typology: String) -> Result<(), String> {
    api::ingest_image_async(file_path, typology).await
}

// ── Model / inference ─────────────────────────────────────────────────────────

#[command]
pub async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    api::discover_models().await
}

#[command]
pub async fn download_and_vectorize(
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    api::download_and_vectorize(url, filename, item_id).await
}

#[command]
pub async fn download_model(
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    api::download_model(url, filename, model_id).await
}

#[command]
pub fn cancel_download(id: String) -> Result<(), String> {
    api::cancel_download(id)
}

#[command]
pub fn get_active_model() -> Option<String> {
    api::get_active_model()
}

#[command]
pub fn set_active_model(model_name: String) -> Result<(), String> {
    api::set_active_model(model_name)
}

#[command]
pub fn get_active_downloads() -> Vec<ProgressPayload> {
    api::get_active_downloads()
}

#[command]
pub async fn run_agent_inference(
    prompt: String,
    model_name: String,
    intent_layout: Vec<f64>,
) -> Result<(), String> {
    api::run_agent_inference(prompt, model_name, intent_layout).await
}
