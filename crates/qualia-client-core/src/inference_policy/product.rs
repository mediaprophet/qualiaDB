//! Explicit local chat routing from an already authenticated actor.
use super::{actor::InferenceActorSession, store};
use qualia_core_db::llm_agent::{
    native_request::{NativeHostCapability, NativePreparationSpec, NativeProductSession},
    LocalLlmAgent,
};
use std::sync::{Arc, Mutex, OnceLock};

const MAX_ROUTES: usize = 16;
type Routes = [Option<(String, Arc<NativeProductSession>)>; MAX_ROUTES];
static ROUTES: OnceLock<Mutex<Routes>> = OnceLock::new();

/// Native host command, never callable through chat or an unauthenticated tool.
/// Preparation is caller selected: a retained file lease or an explicit snapshot
/// byte budget. Neither actor identity nor a permissive policy is synthesized.
pub fn bind_chat_inference_session(
    chat_id: &str,
    actor: &InferenceActorSession,
    preparation: NativePreparationSpec,
) -> Result<(), String> {
    if chat_id.is_empty() || chat_id.len() > 4096 {
        return Err("A bounded chat session identifier is required".into());
    }
    let product = prepare_session(actor, preparation)?;
    let mut routes = ROUTES
        .get_or_init(|| Mutex::new(std::array::from_fn(|_| None)))
        .lock()
        .map_err(|_| "Chat authority routes unavailable")?;
    // Replace only the explicitly selected local chat route.
    let index = routes
        .iter()
        .position(|r| r.as_ref().is_some_and(|(id, _)| id == chat_id))
        .or_else(|| routes.iter().position(Option::is_none))
        .ok_or("Chat authority route capacity exhausted; unbind an old chat")?;
    routes[index] = Some((chat_id.to_owned(), Arc::new(product)));
    Ok(())
}

fn prepare_session(
    actor: &InferenceActorSession,
    preparation: NativePreparationSpec,
) -> Result<NativeProductSession, String> {
    store::with_host(|h, key, _| {
        if !Arc::ptr_eq(&h.owner, actor.owner()) {
            return Err("Actor belongs to a different configured authority".into());
        }
        let capability = NativeHostCapability::from_installed(
            Arc::clone(&h.host),
            key.clone(),
            Arc::clone(&h.owner),
        )
        .map_err(|e| e.to_string())?;
        actor.product_session(Arc::new(capability), preparation)
    })
}

/// Explicit local delegation to MCP. Remote tool arguments cannot select an
/// actor or cause a login. Session revocation remains owned by NativeAuthority.
pub fn bind_mcp_inference_session(
    actor: &InferenceActorSession,
    preparation: NativePreparationSpec,
) -> Result<(), String> {
    let product = prepare_session(actor, preparation)?;
    qualia_core_db::mcp::inference_session::configure(Arc::new(product))
}

/// Retain a route for an embedding's own native session (including the CLI).
pub fn native_product_session(
    actor: &InferenceActorSession,
    preparation: NativePreparationSpec,
) -> Result<Arc<NativeProductSession>, String> {
    prepare_session(actor, preparation).map(Arc::new)
}

/// Explicitly delegate the configured actor to this native host's authenticated
/// PoET transport. Prompt principal/agent fields never select this authority.
pub fn bind_poet_inference_session(
    actor: &InferenceActorSession,
    preparation: NativePreparationSpec,
) -> Result<(), String> {
    let product = prepare_session(actor, preparation)?;
    qualia_core_db::services::poet_llm_authority::configure(Arc::new(product));
    Ok(())
}

pub fn unbind_chat_inference_session(chat_id: &str) {
    if let Some(routes) = ROUTES.get() {
        let mut routes = routes.lock().unwrap_or_else(|e| e.into_inner());
        for route in routes.iter_mut() {
            if route.as_ref().is_some_and(|(id, _)| id == chat_id) {
                *route = None;
            }
        }
    }
}

pub(super) fn clear() {
    qualia_core_db::mcp::inference_session::disconnect();
    qualia_core_db::services::poet_llm_authority::disconnect();
    if let Some(routes) = ROUTES.get() {
        *routes.lock().unwrap_or_else(|e| e.into_inner()) = std::array::from_fn(|_| None);
    }
}

pub(crate) fn bind_turn(chat_id: &str, agent: &LocalLlmAgent) -> Result<(), String> {
    // Reopen configured trust/vault on every turn, even when the retained route
    // remains alive; vault loss must invalidate old worker capabilities.
    store::with_host(|_, _, _| Ok(()))?;
    let product = ROUTES
        .get()
        .ok_or("Log in and bind inference authority for this chat")?
        .lock()
        .map_err(|_| "Chat authority routes unavailable")?
        .iter()
        .flatten()
        .find(|(id, _)| id == chat_id)
        .map(|(_, p)| Arc::clone(p))
        .ok_or("Log in and bind inference authority for this chat")?;
    agent
        .bind_native_governed_request(product.bind().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
