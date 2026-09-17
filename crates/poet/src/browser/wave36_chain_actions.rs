//! Dual-path Tool Chest actions for Social / Finance / Forensic leftovers (wave 36).

use serde_json::json;
use web_sys::Document;

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(document, &label, &report.message, report.status_kind);
        return;
    }
    super::interactions::show_tool_status(document, &label, &format!("Running {cap_id}…"), "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

pub(super) fn run_social_gini(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Social.gini",
        "Social.gini sketch incomes=[1,2,3,4]".to_string(),
        json!({ "incomes": [1.0, 2.0, 3.0, 4.0] }),
    );
}

pub(super) fn run_social_lorenz(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Social.lorenz",
        "Social.lorenz sketch incomes=[1,2,3,4]".to_string(),
        json!({ "incomes": [1.0, 2.0, 3.0, 4.0] }),
    );
}

pub(super) fn run_social_degree_centrality(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Social.degree_centrality",
        "Social.degree_centrality sketch n=2 path graph".to_string(),
        json!({ "n": 2, "adjacency": [0.0, 1.0, 1.0, 0.0] }),
    );
}

pub(super) fn run_social_lww(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Social.lww",
        "Social.lww sketch two quins (clock 1 vs 2)".to_string(),
        json!({
            "local": { "s": 1, "p": 2, "o": 3, "clock": 1 },
            "remote": { "s": 1, "p": 2, "o": 9, "clock": 2 },
            "selfhood": false
        }),
    );
}

pub(super) fn run_finance_convert_currency(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Finance.convert_currency",
        "Finance.convert_currency sketch amount=100 rate_micros=1500000".to_string(),
        json!({ "amount": 100, "rate_micros": 1_500_000 }),
    );
}

pub(super) fn run_finance_multisig_check(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Finance.multisig_check",
        "Finance.multisig_check sketch 2-of-3".to_string(),
        json!({ "valid_signers": 2, "k": 2 }),
    );
}

pub(super) fn run_finance_ledger_balance(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Finance.ledger_balance",
        "Finance.ledger_balance sketch cash debit 100".to_string(),
        json!({
            "accounts": [{ "id": 1, "type": "asset" }],
            "postings": [{ "entry_id": 1, "account_id": 1, "debit": 100, "credit": 0 }]
        }),
    );
}

pub(super) fn run_forensic_malfeasance_delta(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Forensic.malfeasance_delta",
        "Forensic.malfeasance_delta sketch capital=10 utility=8".to_string(),
        json!({ "capital_allocated": 10.0, "delivered_utility": 8.0, "inverted": false }),
    );
}

pub(super) fn run_forensic_narrative_divergence(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Forensic.narrative_divergence",
        "Forensic.narrative_divergence sketch two 5-D traces".to_string(),
        json!({
            "factual": [[0.0, 0.0, 0.0, 0.0, 0.0]],
            "fantasy": [[1.0, 0.0, 0.0, 0.0, 0.0]]
        }),
    );
}

pub(super) fn run_corpus_load(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Corpus.load",
        "Corpus.load sketch path=/tmp/poet-corpus.txt".to_string(),
        json!({ "path": "/tmp/poet-corpus.txt" }),
    );
}

pub(super) fn run_corpus_parse(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Corpus.parse",
        "Corpus.parse sketch name=demo text=hello".to_string(),
        json!({ "name": "demo", "text": "hello" }),
    );
}

pub(super) fn run_chat_validate_fragment(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ChatGraph.validate_fragment",
        "ChatGraph.validate_fragment sketch public fragment".to_string(),
        json!({
            "session_id": "poet-demo",
            "message_lamport": 1,
            "anchor_start": 0,
            "anchor_end": 5,
            "anchor_text": "hello"
        }),
    );
}

pub(super) fn run_chat_link_reply(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ChatGraph.link_reply",
        "ChatGraph.link_reply sketch parent=aaaaaaaa00000001".to_string(),
        json!({
            "parent_fragment_id": "aaaaaaaa00000001",
            "reply_message_lamport": 2,
            "session_id": "poet-demo"
        }),
    );
}

pub(super) fn run_interactive_add_social_post(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Interactive.add_social_post",
        "Interactive.add_social_post sketch demo post".to_string(),
        json!({ "id": "post-1", "author": "did:q42:demo", "content": "hello" }),
    );
}

pub(super) fn run_interactive_add_trigger(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Interactive.add_trigger",
        "Interactive.add_trigger sketch id=trig-1".to_string(),
        json!({ "id": "trig-1", "timestamp": 1 }),
    );
}

pub(super) fn run_second_screen_sync(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SecondScreen.sync",
        "SecondScreen.sync sketch companion content".to_string(),
        json!({ "id": "screen-1", "content_id": "clip-1" }),
    );
}
