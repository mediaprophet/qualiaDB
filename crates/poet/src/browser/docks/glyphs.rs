//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Toolbox and tool glyph maps plus kind badges.

use crate::tool_chest::core::tool::ToolKind;

// ---------------------------------------------------------------------------
// Glyph mapping via Webizen Icon Registry & Fallback Chain
// ---------------------------------------------------------------------------

/// Map a toolbox id to an authoritative PUA glyph or standard fallback.
pub fn toolbox_glyph(id: &str) -> &'static str {
    match id {
        "epistemic" => "🧭",
        "office" | "word_processor" | "tb_word_processor" | "doc" => "📝",
        "sheet" | "tb_spreadsheet" => "📊",
        "image" | "graphics" | "tb_graphics" => "🎨",
        "spatial" | "3d" | "tb_3d_spatial" | "dual_studio" | "studio" => "🧊",
        "audio" | "audio_synth" | "tb_audio_synth" | "audio_session" => "🎙️",
        "code" | "tb_code_ide" => "💻",
        "communication" | "mail" | "tb_mail_publish" => "✉️",
        "erp" | "tb_erp_workstream" => "📅",
        "lab" | "science" | "scientific" | "tb_scientific_lab" => "🔬",
        "ai" | "tb_ai_copilot" => "✨",
        "rights" | "governance" | "tb_governance_rights" => "⚖️",
        "sdn" | "tb_sdn_cooperative" => "🌐",
        "health" => "🩺",
        "solid" | "tb_solid" => "📦",
        _ => "🧩",
    }
}

/// Map a tool icon identifier to a display glyph.
pub fn tool_glyph(icon: &str) -> &'static str {
    match icon {
        "doc" => "📄",
        "ontology" => "📖",
        "slide" => "📊",
        "media" => "🎨",
        "marker" => "📍",
        "heatmap" => "🔥",
        "sheet" => "📊",
        "import" => "📥",
        "map" => "🗺",
        "3d" => "🎯",
        "pin" => "📌",
        "track" => "🔍",
        "social" => "💬",
        "webrtc" => "📷",
        "webview" => "🌐",
        "group" => "👥",
        "sign" => "✍",
        "did" => "🆔",
        "health" => "🩺",
        "pathology" => "🔬",
        "anatomy" => "🫀",
        "vibe" => "⚡",
        "quin" => "🧬",
        "coauthor" => "🧑‍🤝‍🧑",
        "extractor" => "⛏",
        "sentinel" => "🛡",
        "triad" => "🎨",
        "objective" => "📍",
        "subjective" => "🧭",
        "intersubjective" => "🤝",
        "normative" => "⚖",
        _ => "💡",
    }
}

/// Short kind label for the tool button badge.
pub(super) fn kind_label(kind: ToolKind) -> &'static str {
    crate::browser::tool_copy::kind_badge(kind)
}
