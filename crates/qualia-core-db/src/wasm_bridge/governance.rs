//! Browser adapter for the graph-backed inference guard.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::modalities::epistemic::{
    CERTAINTY_BIT_SHIFT, OP_BELIEVES, OP_COMMON_KNOWLEDGE, OP_KNOWS,
};
#[cfg(target_arch = "wasm32")]
use crate::modalities::inference_guard::{
    evaluate_inference_guard, InferenceGuardAction, InferenceGuardReason,
};
#[cfg(target_arch = "wasm32")]
use crate::modalities::logic::deontic::{OP_FORBID, OP_OBLIGATE, OP_PERMIT};
#[cfg(target_arch = "wasm32")]
use crate::{q_hash, NQuin};

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuardParams {
    #[serde(default)]
    norms: Vec<NormInput>,
    #[serde(default)]
    facts: Vec<GraphInput>,
    #[serde(default)]
    epistemic: Vec<EpistemicInput>,
    #[serde(default)]
    trace: Vec<GraphInput>,
    #[serde(default)]
    actor: String,
    #[serde(default)]
    required_claim: String,
    #[serde(default)]
    temporal_invariant: String,
    #[serde(default)]
    now_unix: u32,
    #[serde(default)]
    non_derogable: bool,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphInput {
    subject: String,
    predicate: String,
    object: String,
    #[serde(default)]
    context: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormInput {
    subject: String,
    action: String,
    kind: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    expiry_unix: u32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EpistemicInput {
    subject: String,
    claim: String,
    kind: String,
    #[serde(default = "default_certainty")]
    certainty: u8,
    #[serde(default)]
    context: String,
}

#[cfg(target_arch = "wasm32")]
fn default_certainty() -> u8 {
    255
}

#[cfg(target_arch = "wasm32")]
fn h(value: &str) -> u64 {
    if value.is_empty() {
        0
    } else {
        q_hash(value)
    }
}

#[cfg(target_arch = "wasm32")]
fn quin(subject: u64, predicate: u64, object: u64, context: u64, metadata: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: subject ^ predicate ^ object ^ context,
    }
}

#[cfg(target_arch = "wasm32")]
impl GraphInput {
    fn to_quin(&self) -> NQuin {
        quin(
            h(&self.subject),
            h(&self.predicate),
            h(&self.object),
            h(&self.context),
            0,
        )
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_inference_guard_wasm(value: JsValue) -> Result<JsValue, JsValue> {
    let params: GuardParams = serde_wasm_bindgen::from_value(value)?;
    let mut norms = Vec::with_capacity(params.norms.len());
    for norm in params.norms {
        let opcode = match norm.kind.as_str() {
            "obligate" => OP_OBLIGATE,
            "permit" => OP_PERMIT,
            "forbid" => OP_FORBID,
            _ => {
                return Err(JsValue::from_str(
                    "norm.kind must be obligate, permit, or forbid",
                ))
            }
        };
        norms.push(quin(
            h(&norm.subject),
            opcode as u64,
            h(&norm.action),
            h(&norm.context),
            norm.expiry_unix as u64,
        ));
    }
    let facts: Vec<NQuin> = params.facts.iter().map(GraphInput::to_quin).collect();
    let trace: Vec<NQuin> = params.trace.iter().map(GraphInput::to_quin).collect();
    let mut epistemic = Vec::with_capacity(params.epistemic.len());
    for claim in params.epistemic {
        let opcode = match claim.kind.as_str() {
            "knows" => OP_KNOWS,
            "believes" => OP_BELIEVES,
            "commonKnowledge" => OP_COMMON_KNOWLEDGE,
            _ => {
                return Err(JsValue::from_str(
                    "epistemic.kind must be knows, believes, or commonKnowledge",
                ))
            }
        };
        epistemic.push(quin(
            h(&claim.subject),
            opcode as u64 | ((claim.certainty as u64) << CERTAINTY_BIT_SHIFT),
            h(&claim.claim),
            h(&claim.context),
            0,
        ));
    }
    let verdict = evaluate_inference_guard(
        &norms,
        &facts,
        &epistemic,
        &trace,
        h(&params.actor),
        h(&params.required_claim),
        h(&params.temporal_invariant),
        params.now_unix,
        params.non_derogable,
    );
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Output {
        action: &'static str,
        reason: &'static str,
        isolated_count: usize,
    }
    let action = match verdict.action {
        InferenceGuardAction::Allow => "allow",
        InferenceGuardAction::Steer => "steer",
        InferenceGuardAction::Deny => "deny",
    };
    let reason = match verdict.reason {
        InferenceGuardReason::None => "none",
        InferenceGuardReason::DeonticViolation => "deonticViolation",
        InferenceGuardReason::TemporalViolation => "temporalViolation",
        InferenceGuardReason::MissingKnowledge => "missingKnowledge",
        InferenceGuardReason::ContradictoryGraph => "contradictoryGraph",
        InferenceGuardReason::GraphCapacityExceeded => "graphCapacityExceeded",
    };
    serde_wasm_bindgen::to_value(&Output {
        action,
        reason,
        isolated_count: verdict.isolated_count,
    })
    .map_err(Into::into)
}
