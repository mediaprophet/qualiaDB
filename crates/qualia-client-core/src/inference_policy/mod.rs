//! Privileged local-host installation and admission of Sentinel inference policy.
//! This API is for authenticated application commands. It must not be registered
//! as a remotely callable signing endpoint. Inference never installs a default.
mod actor;
mod configuration;
mod product;
mod registry;
mod store;
pub use actor::{
    create_inference_actor_signer, login_inference_actor, InferenceActorSession,
    SelectedInferenceIdentifier,
};
pub use configuration::{InferenceHostConfiguration, CONFIGURATION_BUDGET_BYTES};
pub(crate) use product::bind_turn;
pub use product::{
    bind_chat_inference_session, bind_poet_inference_session, unbind_chat_inference_session,
    bind_mcp_inference_session, native_product_session,
};
pub use store::{
    admit_inference, configure_inference_authority, disconnect_inference_controller,
    install_evidence_policy, install_policy, load_policy_status, lock_inference_controller_vault,
    replace_inference_admission, revoke_policy,
};

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;
