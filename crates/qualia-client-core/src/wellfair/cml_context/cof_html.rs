//! **COF — Context Optimisation Format** (HTML+RDFa profile).
//!
//! Design (see `tools/legislation-etl/cof.n3` and plan `docs/plans/cof-html-rdfa-etl.md`):
//!
//! - **CML** = TEXT → CONCEPT → LOGIC graph (N3 / NQuin).
//! - **COF** = how that graph is serialised for **agent context windows**: constrained
//!   HTML5 + RDFa (`typeof` / `about` / `property` / `resource` / `rel`) with almost no
//!   layout tokens. Domain meaning stays on `cml:` / `values:` / `skos:`.
//!
//! Profile IRI: `https://ns.webcivics.net/cof/profile/html-rdfa-1`
//!
//! Large instruments are **segmented** at section boundaries so a host can load only the
//! token budget required for a turn (index segment + one body segment).

use serde::{Deserialize, Serialize};

use super::extract::{classify_deontic, extract_privacy_signals};
use super::graph::ContextUnit;

/// Official COF HTML+RDFa profile (must match `cof.n3` / legis2cml).
pub const COF_PROFILE: &str = "https://ns.webcivics.net/cof/profile/html-rdfa-1";
pub const COF_NS: &str = "https://ns.webcivics.net/cof/";
pub const CML_NS: &str = "https://ns.webcivics.net/cml/";
pub const VALUES_NS: &str = "https://ns.webcivics.net/values/";
pub const MEDIA_TYPE_COF: &str =
    "text/html;profile=\"https://ns.webcivics.net/cof/profile/html-rdfa-1\"";

/// Default agent body-segment budget (~6–8k tokens at ~4 chars/token; leave headroom).
pub const DEFAULT_SEGMENT_MAX_CHARS: usize = 24_000;
/// Soft floor: never emit a body segment smaller than this unless it is the only content.
pub const DEFAULT_SEGMENT_MIN_CHARS: usize = 2_000;

/// One COF HTML segment (self-contained HTML document, RDFa-complete).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CofSegment {
    /// 0 = index/TOC; 1..N = body segments.
    pub index: u32,
    pub total: u32,
    pub id: String,
    pub title: String,
    /// Full HTML document for this segment.
    pub html: String,
    pub char_count: usize,
    /// Approximate token estimate (chars / 4).
    pub approx_tokens: usize,
    /// Concept / unit frags included in this segment.
    pub unit_frags: Vec<String>,
    pub is_index: bool,
}

/// A complete COF package: index + ordered body segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CofPackage {
    pub document_uri: String,
    pub title: String,
    pub profile: String,
    pub segment_max_chars: usize,
    pub segments: Vec<CofSegment>,
    pub total_chars: usize,
    pub total_approx_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CofStyle {
    /// Minimal markup for agent windows (no CSS, short header).
    AgentLean,
    /// Thin human CSS + proposed banner (still COF attributes for machines).
    DualSurface,
}

impl Default for CofStyle {
    fn default() -> Self {
        Self::AgentLean
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn approx_tokens(chars: usize) -> usize {
    chars.div_ceil(4)
}

fn prefix_attr() -> String {
    format!(
        "rdf: http://www.w3.org/1999/02/22-rdf-syntax-ns# \
         rdfs: http://www.w3.org/2000/01/rdf-schema# \
         dc: http://purl.org/dc/terms/ \
         skos: http://www.w3.org/2004/02/skos/core# \
         prov: http://www.w3.org/ns/prov# \
         cml: {CML_NS} \
         values: {VALUES_NS} \
         cof: {COF_NS}"
    )
}

fn dual_css() -> &'static str {
    /* Intentionally tiny — COF forbids presentation bloat; this is human affordance only. */
    "body{font:15px/1.45 system-ui,sans-serif;max-width:48rem;margin:1rem auto;padding:0 1rem;color:#111}\
     h1{font-size:1.25rem;margin:0 0 .5rem}h2,h3{font-size:1rem;margin:1rem 0 .35rem}\
     .banner{border-left:3px solid #6a4c93;background:#f4f1f7;padding:.5rem .75rem;margin:.75rem 0;font-size:.85rem}\
     section{margin:.9rem 0;padding-bottom:.5rem;border-bottom:1px solid #eee}\
     .text{white-space:pre-wrap}.meta{color:#666;font-size:.8rem}\
     .sig{display:inline-block;font-size:.7rem;background:#eee;border-radius:999px;padding:.05rem .4rem;margin:.1rem}\
     aside.logic{font:12px ui-monospace,monospace;background:#f7f5fa;border-left:2px solid #6a4c93;padding:.4rem .6rem;margin:.4rem 0}"
}

/// Render one unit as a COF section fragment (no document chrome).
pub fn render_unit_fragment(doc_uri: &str, unit: &ContextUnit, style: CofStyle) -> String {
    let _ = style;
    let frag = esc(&unit.frag);
    let label = esc(&unit.label);
    let kind = esc(&unit.kind);
    let about = esc(&format!("{doc_uri}#{}", unit.frag));
    let resource = about.clone();
    let (deontic, dconf) = if unit.text.trim().is_empty() {
        (super::extract::DeonticClass::Undertaking, 0u8)
    } else {
        classify_deontic(&unit.text)
    };
    let privacy = extract_privacy_signals(&unit.text);
    let signals: Vec<String> = privacy
        .iter()
        .map(|s| format!("privacy:{}", s.signal))
        .chain(std::iter::once(format!("deontic:{}", deontic.as_str())))
        .collect();
    let sig_attr = esc(&signals.join(" "));
    let conf = format!("{:.2}", dconf as f32 / 100.0);
    let page_attr = unit
        .page
        .map(|p| format!(" data-page=\"{p}\""))
        .unwrap_or_default();
    let part_of = unit
        .parent
        .as_ref()
        .map(|p| {
            format!(
                " rel=\"values:partOf\" resource=\"{}\"",
                esc(&format!("{doc_uri}#{p}"))
            )
        })
        .unwrap_or_default();

    let mut sig_chips = String::new();
    for s in &signals {
        sig_chips.push_str(&format!(
            "<span class=\"sig\" property=\"cml:hasSignal\" content=\"{}\">{}</span> ",
            esc(s),
            esc(s)
        ));
    }

    let text_block = if unit.text.trim().is_empty() {
        String::new()
    } else {
        let claim_id = format!("{doc_uri}#{}-claim", unit.frag);
        format!(
            "<div class=\"text\" typeof=\"cof:Block\" property=\"cof:hasBlock values:originalText\" \
             resource=\"{doc_uri}#{frag}-text\"{page_attr}>\
             <span typeof=\"cof:Claim\" property=\"cof:hasClaim\" about=\"{claim}\" \
             resource=\"{claim}\" data-confidence=\"{conf}\" data-deontic=\"{deontic}\" \
             data-signals=\"{sig_attr}\">{text}</span></div>",
            claim = esc(&claim_id),
            deontic = deontic.as_str(),
            text = esc(&unit.text),
        )
    };

    let logic =
        if matches!(deontic, super::extract::DeonticClass::Undertaking) && privacy.is_empty() {
            String::new()
        } else {
            format!(
                "<aside class=\"logic\" typeof=\"cml:LogicApplication\" property=\"cml:asserts\" \
             about=\"{about}-norm\">\
             <span property=\"cml:modality\" resource=\"cml:Deontic\">deontic</span> \
             <span property=\"values:deonticClass\">{deontic}</span> · conf {conf} · \
             <span property=\"cml:curationStatus\" resource=\"cml:Proposed\">cml:Proposed</span>\
             </aside>",
                deontic = deontic.as_str(),
            )
        };

    format!(
        "<section id=\"{frag}\" typeof=\"cml:Concept cof:Section\" about=\"{about}\" \
         resource=\"{resource}\" property=\"cof:hasSection\" data-kind=\"{kind}\"{page_attr}{part_of} \
         data-confidence=\"{conf}\" data-deontic=\"{deontic}\" data-signals=\"{sig_attr}\">\
         <h3><span property=\"skos:prefLabel cof:title\">{label}</span></h3>\
         <div class=\"meta\">{sig_chips}</div>\
         {text_block}{logic}\
         <link rel=\"cml:realizedBy\" href=\"{doc_uri}#{frag}\" />\
         <link rel=\"cml:curationStatus\" href=\"{CML_NS}Proposed\" />\
         </section>\n",
        deontic = deontic.as_str(),
    )
}

fn wrap_document(
    doc_uri: &str,
    title: &str,
    segment: &CofSegmentMeta,
    body_inner: &str,
    style: CofStyle,
) -> String {
    let prefix = prefix_attr();
    let css = match style {
        CofStyle::AgentLean => String::new(),
        CofStyle::DualSurface => format!("<style>{}</style>", dual_css()),
    };
    let banner = match style {
        CofStyle::AgentLean => format!(
            "<div property=\"cml:curationStatus\" resource=\"cml:Proposed\" \
             content=\"cml:Proposed\">cml:Proposed · COF segment {}/{} · profile {COF_PROFILE}</div>",
            segment.index + 1,
            segment.total
        ),
        CofStyle::DualSurface => format!(
            "<div class=\"banner\">⚑ <strong>cml:Proposed</strong> — machine layer only. \
             COF segment <span property=\"cof:segmentIndex\" content=\"{}\">{}/{}</span> · \
             profile <span property=\"cof:profile\" content=\"{COF_PROFILE}\">{COF_PROFILE}</span>. \
             Load only the segments you need (token optimisation).</div>",
            segment.index,
            segment.index + 1,
            segment.total
        ),
    };
    let nav = {
        let mut n = String::new();
        if let Some(p) = &segment.prev_id {
            n.push_str(&format!(
                "<link rel=\"cof:prevSegment\" href=\"{}\" />\n",
                esc(p)
            ));
        }
        if let Some(nx) = &segment.next_id {
            n.push_str(&format!(
                "<link rel=\"cof:nextSegment\" href=\"{}\" />\n",
                esc(nx)
            ));
        }
        n.push_str(&format!(
            "<meta name=\"cof-segment\" content=\"{}\">\n\
             <meta name=\"cof-segment-total\" content=\"{}\">\n\
             <meta name=\"cof-segment-id\" content=\"{}\">\n",
            segment.index,
            segment.total,
            esc(&segment.id)
        ));
        n
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en" prefix="{prefix}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="cof-profile" content="{COF_PROFILE}">
<meta name="cml-schema" content="2">
{nav}{css}
<title>{title_esc} — COF {seg_label}</title>
</head>
<body typeof="cof:Document" about="{doc}" resource="{doc}" vocab="{COF_NS}"
      data-cof-segment="{idx}" data-cof-segment-total="{total}">
<header>
  <h1 property="dc:title cof:title">{title_esc}</h1>
  <div class="meta">HTML+RDFa COF · CML TEXT→CONCEPT→LOGIC · engine=qualia-rust</div>
  <link rel="prov:wasDerivedFrom" href="{doc}" />
  {banner}
</header>
<main property="cof:body">
{body}
</main>
</body>
</html>
"#,
        title_esc = esc(title),
        seg_label = if segment.is_index {
            "index".into()
        } else {
            format!("seg-{}", segment.index)
        },
        doc = esc(doc_uri),
        idx = segment.index,
        total = segment.total,
        body = body_inner,
    )
}

struct CofSegmentMeta {
    index: u32,
    total: u32,
    id: String,
    prev_id: Option<String>,
    next_id: Option<String>,
    is_index: bool,
}

/// Pack units into COF segments under a character budget (section-aligned).
pub fn pack_units_into_segments(
    units: &[ContextUnit],
    max_chars: usize,
) -> Vec<(Vec<usize>, usize)> {
    // Returns list of (unit_indices, estimated_chars).
    let mut packs: Vec<(Vec<usize>, usize)> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cur_chars = 0usize;
    let max_chars = max_chars.max(DEFAULT_SEGMENT_MIN_CHARS);

    for (i, u) in units.iter().enumerate() {
        // Rough size: label + text + markup overhead (~200).
        let unit_chars = u.label.len() + u.text.len() + 200;
        if unit_chars > max_chars {
            // Oversized unit: flush current, then emit alone (caller may further split text).
            if !cur.is_empty() {
                packs.push((cur, cur_chars));
                cur = Vec::new();
                cur_chars = 0;
            }
            packs.push((vec![i], unit_chars));
            continue;
        }
        if !cur.is_empty() && cur_chars + unit_chars > max_chars {
            packs.push((cur, cur_chars));
            cur = Vec::new();
            cur_chars = 0;
        }
        cur.push(i);
        cur_chars += unit_chars;
    }
    if !cur.is_empty() {
        packs.push((cur, cur_chars));
    }
    if packs.is_empty() {
        packs.push((Vec::new(), 0));
    }
    packs
}

/// Build a full COF package (index + body segments) for agent token optimisation.
pub fn build_cof_package(
    doc_uri: &str,
    title: &str,
    units: &[ContextUnit],
    max_chars: usize,
    style: CofStyle,
) -> CofPackage {
    let packs = pack_units_into_segments(units, max_chars);
    let body_count = packs.len() as u32;
    // total segments = 1 index + body_count (even if empty body → index only)
    let total = body_count + 1;

    // --- Index segment (TOC): titles + signals, no full body text ---
    let mut index_body = String::from(
        "<nav typeof=\"cof:Section\" property=\"cof:hasSection\" about=\"#index\" id=\"index\">\n\
         <h2 property=\"cof:title\">Index (token-cheap map)</h2>\n\
         <ol>\n",
    );
    for (seg_i, (idxs, _chars)) in packs.iter().enumerate() {
        let seg_id = format!("{doc_uri}#cof-seg-{}", seg_i + 1);
        index_body.push_str(&format!(
            "<li property=\"cof:hasSegment\" resource=\"{}\">segment {} · {} unit(s)<ul>",
            esc(&seg_id),
            seg_i + 1,
            idxs.len()
        ));
        for &ui in idxs {
            let u = &units[ui];
            let (deontic, _) = classify_deontic(&u.text);
            let priv_n = extract_privacy_signals(&u.text).len();
            index_body.push_str(&format!(
                "<li><a property=\"cof:ref\" href=\"{doc_uri}#{frag}\" resource=\"{doc_uri}#{frag}\">{label}</a> \
                 <span class=\"meta\">deontic:{deontic} · privacy:{priv_n}</span></li>",
                frag = esc(&u.frag),
                label = esc(&u.label),
                deontic = deontic.as_str(),
            ));
        }
        index_body.push_str("</ul></li>\n");
    }
    index_body.push_str("</ol>\n<p class=\"meta\">Load body segments by <code>cof:nextSegment</code> / segment id. Bodies hold <code>values:originalText</code> claims.</p>\n</nav>\n");

    let mut segment_ids: Vec<String> = Vec::with_capacity(total as usize);
    segment_ids.push(format!("{doc_uri}#cof-seg-0"));
    for i in 0..body_count {
        segment_ids.push(format!("{doc_uri}#cof-seg-{}", i + 1));
    }

    let mut segments = Vec::new();

    // Index HTML
    {
        let meta = CofSegmentMeta {
            index: 0,
            total,
            id: segment_ids[0].clone(),
            prev_id: None,
            next_id: segment_ids.get(1).cloned(),
            is_index: true,
        };
        let html = wrap_document(doc_uri, title, &meta, &index_body, style);
        let char_count = html.len();
        segments.push(CofSegment {
            index: 0,
            total,
            id: meta.id,
            title: format!("{title} — index"),
            html,
            char_count,
            approx_tokens: approx_tokens(char_count),
            unit_frags: Vec::new(),
            is_index: true,
        });
    }

    // Body segments
    for (seg_i, (idxs, _)) in packs.iter().enumerate() {
        let mut body = String::new();
        let mut frags = Vec::new();
        // Page markers when present
        let mut last_page: Option<u32> = None;
        for &ui in idxs {
            let u = &units[ui];
            if let Some(p) = u.page {
                if last_page != Some(p) {
                    last_page = Some(p);
                    body.push_str(&format!(
                        "<div class=\"meta\" typeof=\"cof:Page\" property=\"cof:hasPage\" \
                         resource=\"{doc_uri}#page-{p}\" content=\"{p}\">\
                         page <span property=\"cof:pageNumber\">{p}</span></div>\n"
                    ));
                }
            }
            body.push_str(&render_unit_fragment(doc_uri, u, style));
            frags.push(u.frag.clone());
        }
        let idx = (seg_i as u32) + 1;
        let meta = CofSegmentMeta {
            index: idx,
            total,
            id: segment_ids[seg_i + 1].clone(),
            prev_id: Some(segment_ids[seg_i].clone()),
            next_id: segment_ids.get(seg_i + 2).cloned(),
            is_index: false,
        };
        let html = wrap_document(doc_uri, title, &meta, &body, style);
        let char_count = html.len();
        segments.push(CofSegment {
            index: idx,
            total,
            id: meta.id,
            title: format!("{title} — segment {idx}"),
            html,
            char_count,
            approx_tokens: approx_tokens(char_count),
            unit_frags: frags,
            is_index: false,
        });
    }

    let total_chars: usize = segments.iter().map(|s| s.char_count).sum();
    CofPackage {
        document_uri: doc_uri.into(),
        title: title.into(),
        profile: COF_PROFILE.into(),
        segment_max_chars: max_chars,
        total_chars,
        total_approx_tokens: approx_tokens(total_chars),
        segments,
    }
}

/// Single-document COF (no multi-segment) — still valid html-rdfa-1.
pub fn render_cof_document(
    doc_uri: &str,
    title: &str,
    units: &[ContextUnit],
    style: CofStyle,
) -> String {
    let pkg = build_cof_package(doc_uri, title, units, usize::MAX / 4, style);
    // Prefer body-only when one pack; else concatenate is wrong — return first body or index+first.
    if pkg.segments.len() >= 2 {
        pkg.segments[1].html.clone()
    } else {
        pkg.segments[0].html.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_units(n: usize, body_len: usize) -> Vec<ContextUnit> {
        (0..n)
            .map(|i| ContextUnit {
                frag: format!("sec-{i}"),
                kind: "section".into(),
                label: format!("{i} Sample section"),
                text: format!(
                    "Section {i}. The controller shall not process personal data without consent. {}",
                    "word ".repeat(body_len / 5)
                ),
                page: Some((i as u32 / 3) + 1),
                parent: None,
            })
            .collect()
    }

    #[test]
    fn cof_profile_and_rdfa_attributes_present() {
        let units = sample_units(2, 40);
        let html = render_cof_document("urn:doc:t", "Test Act", &units, CofStyle::AgentLean);
        assert!(html.contains(COF_PROFILE));
        assert!(
            html.contains("typeof=\"cof:Document\"")
                || html.contains("typeof=\"cml:Concept cof:Section\"")
        );
        assert!(html.contains("cof:hasSection") || html.contains("property=\"cof:hasSection\""));
        assert!(html.contains("values:originalText") || html.contains("cof:Claim"));
        assert!(html.contains("cml:Proposed"));
        assert!(!html.contains("<style>")); // agent-lean
    }

    #[test]
    fn large_doc_segments_for_token_budget() {
        let units = sample_units(20, 2000);
        let pkg = build_cof_package("urn:doc:big", "Big Act", &units, 8_000, CofStyle::AgentLean);
        assert!(
            pkg.segments.len() >= 3,
            "index + ≥2 body segs, got {}",
            pkg.segments.len()
        );
        assert!(pkg.segments[0].is_index);
        // Each body segment under soft ceiling (markup can exceed pack estimate slightly).
        for s in pkg.segments.iter().filter(|s| !s.is_index) {
            assert!(
                s.char_count < 40_000,
                "segment {} too large: {}",
                s.index,
                s.char_count
            );
            assert!(!s.unit_frags.is_empty());
            assert!(s.html.contains("cof-segment"));
            assert!(s.html.contains("cof:prevSegment") || s.index == 1);
        }
        // Index lists segments without dumping full bodies.
        assert!(pkg.segments[0].html.contains("Index"));
        assert!(pkg.segments[0].html.len() < pkg.total_chars);
    }

    #[test]
    fn dual_surface_has_minimal_css() {
        let units = sample_units(1, 20);
        let html = render_cof_document("urn:doc:h", "Human", &units, CofStyle::DualSurface);
        assert!(html.contains("<style>"));
        assert!(html.contains("cml:Proposed"));
    }
}
