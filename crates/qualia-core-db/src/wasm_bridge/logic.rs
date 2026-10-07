//! WASM-bindgen modal-logic evaluators (UE-020…UE-023).
//!
//! Mirrors the ontology MCP tools in `webizen-lite-wasm` onto the full playground
//! / portal wasm_bridge surface so Civics and Node hosts can call the same
//! kernels without going through MCP JSON-RPC.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn quin_from_arr(arr: &[u64; 6]) -> crate::NQuin {
    crate::NQuin {
        subject: arr[0],
        predicate: arr[1],
        object: arr[2],
        context: arr[3],
        metadata: arr[4],
        parity: arr[5],
    }
}

#[cfg(target_arch = "wasm32")]
fn quin_to_arr(q: &crate::NQuin) -> [u64; 6] {
    [
        q.subject,
        q.predicate,
        q.object,
        q.context,
        q.metadata,
        q.parity,
    ]
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct QuinListParams {
    pub quins: Vec<[u64; 6]>,
    #[serde(default)]
    pub now_unix: Option<u32>,
    #[serde(default)]
    pub agent: Option<u64>,
    #[serde(default)]
    pub world: Option<u64>,
}

/// Evaluate deontic norms in a quin frame (`evaluate_deontic_contract`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_deontic_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::modalities::logic::deontic::{evaluate_deontic_contract, DeonticVerdict};

    let p: QuinListParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid deontic params: {e}")))?;
    let quins: Vec<_> = p.quins.iter().map(quin_from_arr).collect();
    let now = p.now_unix.unwrap_or(0);
    let mut out = vec![DeonticVerdict::default(); quins.len().max(1)];
    let count = evaluate_deontic_contract(&quins, now, &mut out)
        .map_err(|e| JsValue::from_str(&format!("deontic evaluation failed: {e:?}")))?;

    #[derive(Serialize)]
    struct VerdictOut {
        status: String,
        opcode: u8,
        defeat_kind: String,
        norm: [u64; 6],
    }
    #[derive(Serialize)]
    struct DeonticOut {
        verdict_count: usize,
        verdicts: Vec<VerdictOut>,
        engine_version: &'static str,
    }

    let verdicts = out[..count]
        .iter()
        .map(|v| VerdictOut {
            status: format!("{:?}", v.status),
            opcode: v.opcode,
            defeat_kind: format!("{:?}", v.defeat_kind),
            norm: quin_to_arr(&v.norm),
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&DeonticOut {
        verdict_count: count,
        verdicts,
        engine_version: "0.0.39",
    })?)
}

/// Evaluate epistemic claims (`evaluate_epistemic_frame`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_epistemic_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::modalities::epistemic::{
        evaluate_epistemic_frame, EpistemicStatus, EpistemicVerdict,
    };

    let p: QuinListParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid epistemic params: {e}")))?;
    let quins: Vec<_> = p.quins.iter().map(quin_from_arr).collect();
    let agent = p.agent.unwrap_or(0);
    let world = p.world.unwrap_or(0);
    let empty = EpistemicVerdict {
        claim: crate::NQuin::default(),
        status: EpistemicStatus::Skipped,
        certainty: 0,
    };
    let mut out = vec![empty; quins.len().max(1)];
    let count = evaluate_epistemic_frame(&quins, agent, world, &mut out)
        .map_err(|e| JsValue::from_str(&format!("epistemic evaluation failed: {e:?}")))?;

    #[derive(Serialize)]
    struct VerdictOut {
        status: String,
        certainty: u8,
        claim: [u64; 6],
    }
    #[derive(Serialize)]
    struct EpistemicOut {
        verdict_count: usize,
        verdicts: Vec<VerdictOut>,
        engine_version: &'static str,
    }

    let verdicts = out[..count]
        .iter()
        .map(|v| VerdictOut {
            status: format!("{:?}", v.status),
            certainty: v.certainty,
            claim: quin_to_arr(&v.claim),
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&EpistemicOut {
        verdict_count: count,
        verdicts,
        engine_version: "0.0.39",
    })?)
}

/// Route contradictions into an isolated context (`route_paraconsistent`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn route_paraconsistent_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: QuinListParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid paraconsistent params: {e}")))?;
    let quins: Vec<_> = p.quins.iter().map(quin_from_arr).collect();
    let mut consistent = vec![crate::NQuin::default(); quins.len().max(1)];
    let mut isolated = vec![crate::NQuin::default(); quins.len().max(1)];
    let (c_n, i_n) = crate::modalities::paraconsistent::route_paraconsistent(
        &quins,
        &mut consistent,
        &mut isolated,
    )
    .map_err(|e| JsValue::from_str(&format!("paraconsistent routing failed: {e:?}")))?;

    #[derive(Serialize)]
    struct ParaOut {
        consistent: Vec<[u64; 6]>,
        isolated: Vec<[u64; 6]>,
        engine_version: &'static str,
    }

    Ok(serde_wasm_bindgen::to_value(&ParaOut {
        consistent: consistent[..c_n].iter().map(quin_to_arr).collect(),
        isolated: isolated[..i_n].iter().map(quin_to_arr).collect(),
        engine_version: "0.0.39",
    })?)
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct LtlParams {
    pub trace: Vec<[u64; 6]>,
    pub formula: LtlFormulaSpec,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct LtlFormulaSpec {
    pub kind: String,
    #[serde(default)]
    pub property: Option<u64>,
    #[serde(default)]
    pub ante: Option<u64>,
    #[serde(default)]
    pub consequent: Option<u64>,
    #[serde(default)]
    pub trigger: Option<u64>,
    #[serde(default)]
    pub invariant: Option<u64>,
}

/// Evaluate an LTL formula against a quin trace (`evaluate_ltl_trace`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_ltl_trace_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};

    let p: LtlParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid LTL params: {e}")))?;
    let trace: Vec<_> = p.trace.iter().map(quin_from_arr).collect();
    let formula = match p.formula.kind.as_str() {
        "globally" => LtlFormula::Globally(
            p.formula
                .property
                .ok_or_else(|| JsValue::from_str("globally needs property"))?,
        ),
        "finally" => LtlFormula::Finally(
            p.formula
                .property
                .ok_or_else(|| JsValue::from_str("finally needs property"))?,
        ),
        "next" => LtlFormula::Next(
            p.formula
                .property
                .ok_or_else(|| JsValue::from_str("next needs property"))?,
        ),
        "until" => LtlFormula::Until {
            ante: p
                .formula
                .ante
                .ok_or_else(|| JsValue::from_str("until needs ante"))?,
            consequent: p
                .formula
                .consequent
                .ok_or_else(|| JsValue::from_str("until needs consequent"))?,
        },
        "release" => LtlFormula::Release {
            trigger: p
                .formula
                .trigger
                .ok_or_else(|| JsValue::from_str("release needs trigger"))?,
            invariant: p
                .formula
                .invariant
                .ok_or_else(|| JsValue::from_str("release needs invariant"))?,
        },
        other => {
            return Err(JsValue::from_str(&format!(
                "unsupported LTL formula kind: {other}"
            )))
        }
    };

    #[derive(Serialize)]
    struct LtlOut {
        holds: bool,
        engine_version: &'static str,
    }

    Ok(serde_wasm_bindgen::to_value(&LtlOut {
        holds: evaluate_ltl_trace(&trace, &formula),
        engine_version: "0.0.39",
    })?)
}

/// Description-logic subsumption check (`check_subsumption_quin`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn check_subsumption_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct SubParams {
        sub_class: u64,
        super_class: u64,
        tbox: Vec<[u64; 6]>,
    }
    let p: SubParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid subsumption params: {e}")))?;
    let tbox: Vec<_> = p.tbox.iter().map(quin_from_arr).collect();
    let holds = crate::modalities::dl::check_subsumption_quin(p.sub_class, p.super_class, &tbox);

    #[derive(Serialize)]
    struct SubOut {
        holds: bool,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&SubOut {
        holds,
        engine_version: "0.0.39",
    })?)
}

/// Enumerate ASP stable-model world contexts (`enumerate_stable_models`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn enumerate_stable_models_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::modalities::asp::{enumerate_stable_models, MAX_STABLE_MODELS};

    #[derive(Deserialize)]
    struct AspParams {
        base: [u64; 6],
        #[serde(default)]
        rules: Vec<[u64; 6]>,
    }
    let p: AspParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid ASP params: {e}")))?;
    let base = quin_from_arr(&p.base);
    let rules: Vec<_> = p.rules.iter().map(quin_from_arr).collect();
    let mut worlds = [0u64; MAX_STABLE_MODELS];
    let count = enumerate_stable_models(&base, &rules, &mut worlds);

    #[derive(Serialize)]
    struct AspOut {
        model_count: usize,
        world_contexts: Vec<u64>,
        engine_version: &'static str,
        note: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&AspOut {
        model_count: count,
        world_contexts: worlds[..count].to_vec(),
        engine_version: "0.0.39",
        note: "legacy context-bifurcation enumerator; Gelfond-Lifschitz compute_answer_sets remains native/Poet",
    })?)
}

/// Values abuse-check (agency.n3 G1/G1' personhood guard) — WASM surface for Civics.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn values_check_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        #[serde(rename = "agentType", default)]
        agent_type: Option<String>,
        #[serde(rename = "claimsDignityRight", default)]
        claims_dignity_right: bool,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid values_check params: {e}")))?;
    let agent_type_name = p.agent_type.as_deref().unwrap_or("NaturalPerson");
    let agent_type = crate::q_hash(&format!(
        "https://ns.webcivics.net/values/{agent_type_name}"
    ));
    let flagged =
        crate::webizen::check_personhood_category_error(agent_type, p.claims_dignity_right);

    #[derive(Serialize)]
    struct Out {
        flagged: bool,
        flag: &'static str,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        flagged,
        flag: if flagged {
            "values:PersonhoodCategoryError"
        } else {
            ""
        },
        engine_version: "0.0.39",
    })?)
}

/// Consent non-coerced guard (`capacity::detect_duress` inverted).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn values_consent_non_coerced_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        #[serde(default)]
        imbalance: f32,
        #[serde(rename = "explicitThreat", default)]
        explicit_threat: bool,
        #[serde(default = "default_threshold")]
        threshold: f32,
    }
    fn default_threshold() -> f32 {
        0.5
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid consent params: {e}")))?;
    let coerced =
        crate::modalities::capacity::detect_duress(p.imbalance, p.explicit_threat, p.threshold);

    #[derive(Serialize)]
    struct Out {
        non_coerced: bool,
        coerced: bool,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        non_coerced: !coerced,
        coerced,
        engine_version: "0.0.39",
    })?)
}

/// Harm-below-ceiling guard (wasm-safe numeric; CAS marginal-harm stays native).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn values_harm_below_ceiling_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        harm: f64,
        ceiling: f64,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid harm params: {e}")))?;
    let below = p.harm < p.ceiling;

    #[derive(Serialize)]
    struct Out {
        below_ceiling: bool,
        harm: f64,
        ceiling: f64,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        below_ceiling: below,
        harm: p.harm,
        ceiling: p.ceiling,
        engine_version: "0.0.39",
    })?)
}

/// But-for / reachability causation (`causal::caused`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn causal_caused_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        edges: Vec<[u64; 6]>,
        roots: Vec<u64>,
        effect: u64,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid causal params: {e}")))?;
    let edges: Vec<_> = p.edges.iter().map(quin_from_arr).collect();
    let holds = crate::modalities::causal::caused(&edges, &p.roots, p.effect);

    #[derive(Serialize)]
    struct Out {
        caused: bool,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        caused: holds,
        engine_version: "0.0.39",
    })?)
}

/// Fuzzy t-norm (Gödel min / Łukasiewicz / product).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn fuzzy_t_norm_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        a: f32,
        b: f32,
        #[serde(default)]
        family: Option<String>,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid fuzzy params: {e}")))?;
    let family = p.family.as_deref().unwrap_or("godel");
    let degree = match family {
        "lukasiewicz" => crate::modalities::fuzzy::t_norm_lukasiewicz(p.a, p.b),
        "product" => crate::modalities::fuzzy::t_norm_product(p.a, p.b),
        _ => crate::modalities::fuzzy::t_norm_godel(p.a, p.b),
    };

    #[derive(Serialize)]
    struct Out {
        degree: f32,
        family: String,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        degree,
        family: family.into(),
        engine_version: "0.0.39",
    })?)
}

/// STIT: did agent bring about content?
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn stit_brought_about_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        facts: Vec<[u64; 6]>,
        agent: u64,
        content: u64,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid stit params: {e}")))?;
    let facts: Vec<_> = p.facts.iter().map(quin_from_arr).collect();
    let holds = crate::modalities::stit::brought_about(&facts, p.agent, p.content);

    #[derive(Serialize)]
    struct Out {
        brought_about: bool,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        brought_about: holds,
        engine_version: "0.0.39",
    })?)
}

/// Hohfeld correlative position for a jural opcode.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn jural_correlative_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Params {
        position: u8,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid jural params: {e}")))?;
    let corr = crate::modalities::jural::correlative(p.position);

    #[derive(Serialize)]
    struct Out {
        correlative: u8,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        correlative: corr,
        engine_version: "0.0.39",
    })?)
}
