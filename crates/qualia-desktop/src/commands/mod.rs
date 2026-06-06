use tauri::{State, AppHandle};
use qualia_client_core::api::*;
use qualia_client_core::state::{AppState, APP_STATE};
use qualia_core_db::rpc::TaxRecipientSuite;
use qualia_core_db::ilp_dispatcher::DispatchResult;

pub fn get_invoke_handler() -> impl Fn(tauri::Invoke) {
    tauri::generate_handler![]
}
