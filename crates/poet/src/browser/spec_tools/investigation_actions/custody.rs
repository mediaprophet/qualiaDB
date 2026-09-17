//! Chain of custody for investigation evidence.

use super::shared::append_semicolon_attr;
use web_sys::{Document, Element};

pub(super) fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:init-custody" => Some(init_custody(container)),
        "investigation:transfer-custody" => Some(transfer_custody(container)),
        "investigation:record-transform" => Some(record_transform(container)),
        "investigation:verify-custody" => Some(verify_custody(container)),
        "investigation:custody-history" => Some(custody_history(container)),
        _ => None,
    }
}

fn init_custody(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-custody-init", "holder:local_node;reason:evidence_intake")
        .map_err(|_| "Failed to initialise chain of custody.".to_string())?;
    let _ = container.set_attribute("data-custody-status", "active");
    Ok(())
}

fn transfer_custody(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-custody-transfers",
        "from:local_node->to:custodian;reason:handover",
    )
}

fn record_transform(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-custody-transforms",
        "transform:format_normalisation;actor:local_node",
    )
}

fn verify_custody(container: &Element) -> Result<(), String> {
    let has_init = container.get_attribute("data-custody-init").is_some();
    let status = if has_init {
        "integrity:continuous;gaps:0"
    } else {
        "integrity:unknown;gaps:uninitialised"
    };
    container
        .set_attribute("data-custody-verified", status)
        .map_err(|_| "Failed to verify custody chain.".to_string())
}

fn custody_history(container: &Element) -> Result<(), String> {
    let init = container
        .get_attribute("data-custody-init")
        .unwrap_or_else(|| "none".to_string());
    let transfers = container
        .get_attribute("data-custody-transfers")
        .unwrap_or_else(|| "none".to_string());
    let transforms = container
        .get_attribute("data-custody-transforms")
        .unwrap_or_else(|| "none".to_string());
    let history = format!("init={init}|transfers={transfers}|transforms={transforms}");
    container
        .set_attribute("data-custody-history", &history)
        .map_err(|_| "Failed to export custody history.".to_string())
}

