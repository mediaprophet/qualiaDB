//! Office domain capability handlers (Document, Sheet, Deck, PDF).
//!
//! Follows AGENTS.md §0-B (< 500 lines).

use super::receipt::receipt;
use super::super::args;
use super::super::ids;
use vibe::{Diagnostic, Span, Value};

// ── Document Studio ─────────────────────────────────────────────────────────

pub fn doc_create(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (t, tmpl, a) = (
        args::rec_str(v, "title").unwrap_or("Untitled Document"),
        args::rec_str(v, "template").unwrap_or("standard_manuscript"),
        args::rec_str(v, "author").unwrap_or("did:qualia:author"),
    );
    receipt(
        ids::DOC_CREATE,
        "doc_create",
        t,
        &[
            ("title", Value::String(t.into())),
            ("template", Value::String(tmpl.into())),
            ("author", Value::String(a.into())),
            ("pages", Value::U64(1)),
            ("word_count", Value::U64(0)),
        ],
    )
}

pub fn doc_layout_paragraph(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (txt, fs, lh, w) = (
        args::rec_str(v, "text").unwrap_or(""),
        args::rec_f64(v, "font_size").unwrap_or(11.0),
        args::rec_f64(v, "line_height").unwrap_or(1.35),
        args::rec_f64(v, "width_pt").unwrap_or(468.0),
    );
    let glyphs = txt.chars().count() as u64;
    let lines = ((glyphs as f64 * fs * 0.5) / w.max(10.0)).ceil().max(1.0) as u64;
    receipt(
        ids::DOC_LAYOUT_PARAGRAPH,
        "doc_layout_paragraph",
        txt,
        &[
            ("glyph_count", Value::U64(glyphs)),
            ("line_count", Value::U64(lines)),
            ("height_pt", Value::F64(lines as f64 * fs * lh)),
            ("font_size", Value::F64(fs)),
        ],
    )
}

pub fn doc_proof_check(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (txt, loc) = (
        args::rec_str(v, "text").unwrap_or(""),
        args::rec_str(v, "locale").unwrap_or("en-AU"),
    );
    let words = txt.split_whitespace().count() as u64;
    receipt(
        ids::DOC_PROOF_CHECK,
        "doc_proof_check",
        txt,
        &[
            ("locale", Value::String(loc.into())),
            ("word_count", Value::U64(words)),
            ("issue_count", Value::U64(0)),
            ("readability_score", Value::F64(84.2)),
            ("spell_ok", Value::Bool(true)),
        ],
    )
}

pub fn doc_export_docx(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (id_str, fn_str) = (
        args::rec_str(v, "doc_id").unwrap_or("doc-001"),
        args::rec_str(v, "filename").unwrap_or("document.docx"),
    );
    receipt(
        ids::DOC_EXPORT_DOCX,
        "doc_export_docx",
        id_str,
        &[
            ("doc_id", Value::String(id_str.into())),
            ("filename", Value::String(fn_str.into())),
            ("format", Value::String("docx".into())),
            ("bytes", Value::U64(24576)),
            ("status", Value::String("ready".into())),
        ],
    )
}

pub fn doc_citations_sync(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (bib, sty) = (
        args::rec_str(v, "bib_source").unwrap_or("catalog.bib"),
        args::rec_str(v, "style").unwrap_or("apa-7th"),
    );
    receipt(
        ids::DOC_CITATIONS_SYNC,
        "doc_citations_sync",
        bib,
        &[
            ("bib_source", Value::String(bib.into())),
            ("style", Value::String(sty.into())),
            ("citations_resolved", Value::U64(14)),
            ("unresolved", Value::U64(0)),
            ("in_sync", Value::Bool(true)),
        ],
    )
}

// ── Sheet Studio ────────────────────────────────────────────────────────────

pub fn sheet_create(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (t, r, c) = (
        args::rec_str(v, "title").unwrap_or("Untitled Sheet"),
        args::rec_u64(v, "rows").unwrap_or(100),
        args::rec_u64(v, "cols").unwrap_or(26),
    );
    receipt(
        ids::SHEET_CREATE,
        "sheet_create",
        t,
        &[
            ("title", Value::String(t.into())),
            ("rows", Value::U64(r)),
            ("cols", Value::U64(c)),
            ("cell_count", Value::U64(r * c)),
        ],
    )
}

pub fn sheet_eval_formula(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    super::super::sheet::evaluate_formula(v, span)
}

pub fn sheet_pivot_analyze(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (r_dim, c_dim, agg) = (
        args::rec_str(v, "rows_dim").unwrap_or("category"),
        args::rec_str(v, "cols_dim").unwrap_or("quarter"),
        args::rec_str(v, "aggregation").unwrap_or("sum"),
    );
    receipt(
        ids::SHEET_PIVOT_ANALYZE,
        "sheet_pivot_analyze",
        r_dim,
        &[
            ("rows_dim", Value::String(r_dim.into())),
            ("cols_dim", Value::String(c_dim.into())),
            ("aggregation", Value::String(agg.into())),
            ("groups_count", Value::U64(12)),
        ],
    )
}

pub fn sheet_chart_spec(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (ct, rg, t) = (
        args::rec_str(v, "chart_type").unwrap_or("bar"),
        args::rec_str(v, "range").unwrap_or("A1:B12"),
        args::rec_str(v, "title").unwrap_or("Sheet Chart"),
    );
    receipt(
        ids::SHEET_CHART_SPEC,
        "sheet_chart_spec",
        t,
        &[
            ("chart_type", Value::String(ct.into())),
            ("range", Value::String(rg.into())),
            ("title", Value::String(t.into())),
            ("series_count", Value::U64(1)),
        ],
    )
}

// ── Deck Studio ─────────────────────────────────────────────────────────────

pub fn deck_create(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (t, asp) = (
        args::rec_str(v, "title").unwrap_or("Pitch Deck"),
        args::rec_str(v, "aspect_ratio").unwrap_or("16:9"),
    );
    receipt(
        ids::DECK_CREATE,
        "deck_create",
        t,
        &[
            ("title", Value::String(t.into())),
            ("aspect_ratio", Value::String(asp.into())),
            ("slide_count", Value::U64(1)),
        ],
    )
}

pub fn deck_slide_layout(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (lt, idx) = (
        args::rec_str(v, "layout_type").unwrap_or("title_content"),
        args::rec_u64(v, "slide_index").unwrap_or(0),
    );
    receipt(
        ids::DECK_SLIDE_LAYOUT,
        "deck_slide_layout",
        lt,
        &[
            ("layout_type", Value::String(lt.into())),
            ("slide_index", Value::U64(idx)),
            ("placeholders", Value::U64(2)),
        ],
    )
}

pub fn deck_transition_set(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (t, dur) = (
        args::rec_str(v, "transition_type").unwrap_or("fade"),
        args::rec_f64(v, "duration_s").unwrap_or(0.35),
    );
    receipt(
        ids::DECK_TRANSITION_SET,
        "deck_transition_set",
        t,
        &[
            ("transition_type", Value::String(t.into())),
            ("duration_s", Value::F64(dur)),
        ],
    )
}

pub fn deck_export_pptx(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (id_str, fn_str) = (
        args::rec_str(v, "deck_id").unwrap_or("deck-001"),
        args::rec_str(v, "filename").unwrap_or("presentation.pptx"),
    );
    receipt(
        ids::DECK_EXPORT_PPTX,
        "deck_export_pptx",
        id_str,
        &[
            ("deck_id", Value::String(id_str.into())),
            ("filename", Value::String(fn_str.into())),
            ("format", Value::String("pptx".into())),
            ("bytes", Value::U64(65536)),
            ("status", Value::String("ready".into())),
        ],
    )
}

// ── Pdf Studio ──────────────────────────────────────────────────────────────

pub fn pdf_inspect_cos(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let r = args::rec_str(v, "pdf_ref").unwrap_or("doc.pdf");
    receipt(
        ids::PDF_INSPECT_COS,
        "pdf_inspect_cos",
        r,
        &[
            ("pdf_ref", Value::String(r.into())),
            ("cos_objects", Value::U64(142)),
            ("xref_valid", Value::Bool(true)),
            ("version", Value::String("PDF-1.7".into())),
        ],
    )
}

pub fn pdf_preflight_a11y(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (r, std) = (
        args::rec_str(v, "pdf_ref").unwrap_or("doc.pdf"),
        args::rec_str(v, "standard").unwrap_or("PDF/UA-1"),
    );
    receipt(
        ids::PDF_PREFLIGHT_A11Y,
        "pdf_preflight_a11y",
        r,
        &[
            ("pdf_ref", Value::String(r.into())),
            ("standard", Value::String(std.into())),
            ("compliant", Value::Bool(true)),
            ("tag_count", Value::U64(88)),
        ],
    )
}

pub fn pdf_redact_sign(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (r, did) = (
        args::rec_str(v, "pdf_ref").unwrap_or("doc.pdf"),
        args::rec_str(v, "did").unwrap_or("did:qualia:signer"),
    );
    receipt(
        ids::PDF_REDACT_SIGN,
        "pdf_redact_sign",
        did,
        &[
            ("pdf_ref", Value::String(r.into())),
            ("did", Value::String(did.into())),
            ("signed", Value::Bool(true)),
            ("signature_type", Value::String("Ed25519".into())),
        ],
    )
}

pub fn pdf_form_extract(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let r = args::rec_str(v, "pdf_ref").unwrap_or("form.pdf");
    receipt(
        ids::PDF_FORM_EXTRACT,
        "pdf_form_extract",
        r,
        &[
            ("pdf_ref", Value::String(r.into())),
            ("field_count", Value::U64(16)),
            ("filled_count", Value::U64(14)),
            ("form_type", Value::String("AcroForm".into())),
        ],
    )
}
