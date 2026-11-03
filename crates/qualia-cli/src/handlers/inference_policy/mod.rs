//! Local operator setup; no HTTP, MCP, Poet or LLM signing registration.
#[cfg(test)]
mod integration_tests;
mod io;
mod profile;
use crate::cli::InferencePolicyAction;
use qualia_client_core::{api::inference_policy as api, inference_policy as policy};
use qualia_core_db::key_vault::{KeyRole, KeyVault};
use std::path::Path;

pub fn handle(
    path: &Path,
    action: &InferencePolicyAction,
) -> Result<(), Box<dyn std::error::Error>> {
    let create = matches!(action, InferencePolicyAction::InitVault);
    let root = profile::bind_child(path, create)?;
    let root_text = root.to_str().ok_or("Profile path is not valid UTF-8")?;
    // Actual protected existing-vault read. Only InitVault permits creation;
    // there is no generic unlock/load_or_generate or plaintext fallback here.
    let vault = KeyVault::open_native_secure(root_text, create)?;
    let public = vault
        .derive_key_for_role(KeyRole::ControllerSigning)
        .verifying_key()
        .to_bytes();
    if matches!(
        action,
        InferencePolicyAction::InitVault | InferencePolicyAction::InstallerKey
    ) {
        println!(
            "{}",
            serde_json::json!({"installer_public_key":public,"creation_requested":create})
        );
        return Ok(());
    }
    qualia_client_core::state::init_inference_host_state(vault)?;
    let value = match action {
        InferencePolicyAction::Status => api::get_inference_policy_status()?,
        InferencePolicyAction::CreateActorKey { label } => {
            serde_json::json!({"actor_public_key":api::create_inference_actor_key(label.clone())?})
        }
        InferencePolicyAction::Configure {
            configuration,
            admission,
            document,
        } => {
            api::configure_inference_host(
                io::json(configuration, 32 * 1024)?,
                io::json(admission, 128 * 1024)?,
                io::json(document, 32 * 1024)?,
            )?;
            serde_json::json!({"configured":true})
        }
        InferencePolicyAction::LoginCheck {
            subject,
            identifier,
            signer_label,
            lifetime_seconds,
        } => {
            let actor = api::login_inference_actor(
                subject.clone(),
                io::json(identifier, 32 * 1024)?,
                signer_label.clone(),
                *lifetime_seconds,
            )?;
            let scope = api::get_inference_actor_scope(&actor)?;
            api::logout_inference_actor(&actor)?;
            serde_json::json!({"actor_scope":scope,"session_persisted":false})
        }
        InferencePolicyAction::Install {
            rules,
            expires_at,
            expected_generation,
        } => api::install_inference_policy(io::policy(rules)?, *expires_at, *expected_generation)?,
        InferencePolicyAction::Revoke {
            expected_generation,
        } => api::revoke_inference_policy(*expected_generation)?,
        InferencePolicyAction::ReplaceAdmission { admission } => {
            api::replace_inference_actor_admission(io::json(admission, 128 * 1024)?)?;
            serde_json::json!({"admission_replaced":true,"fresh_login_required":true})
        }
        InferencePolicyAction::InitVault | InferencePolicyAction::InstallerKey => {
            unreachable!("handled before state initialization")
        }
    };
    println!("{}", serde_json::to_string(&value)?);
    policy::disconnect_inference_controller();
    Ok(())
}
