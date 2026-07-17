//! Native legislation structure parse → hypermedia Library ingest.
//!
//! Ports the structural (no-LLM) path of `tools/legislation-etl/legis2cml.py`:
//! Part / Division / Schedule / Section / Subsection decomposition, then seeds
//! each provision as a findable Library entry under **Work** with purpose
//! `legislation`. Full section body text is always stored (the Python ETL used
//! to drop bodies from N3/JSON-LD — this path does not).
//!
//! PDF → text uses `pdf_extract` (already a client-core dependency).

use std::collections::BTreeMap;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::cml_context::{build_document_context, ContextUnit};
use super::hypermedia_store::{
    CommonsVisibility, HypermediaStore, LibraryEntry, LibrarySection,
};

/// Media type for native legislation provision entries.
pub const LEGISLATION_MEDIA_TYPE: &str = "text/x-legislation-provision";
pub const LEGISLATION_INSTRUMENT_MEDIA: &str = "text/x-legislation-instrument";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provision {
    pub frag: String,
    /// part | division | schedule | section | subsection
    pub kind: String,
    pub number: String,
    pub heading: String,
    pub text: String,
    /// Pre-subsection-split body (containers keep full text).
    pub full_text: String,
    pub start_page: u32,
    pub parent: Option<String>,
}

impl Provision {
    pub fn source_text(&self) -> &str {
        if !self.full_text.trim().is_empty() {
            &self.full_text
        } else {
            &self.text
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegislationInstrument {
    pub title: String,
    pub slug: String,
    pub jurisdiction: String,
    pub register_id: Option<String>,
    pub provisions: Vec<Provision>,
    pub pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegislationIngestReport {
    pub title: String,
    pub slug: String,
    pub sections: usize,
    pub subsections: usize,
    pub structural: usize,
    pub concepts_with_text: usize,
    pub empty_text: usize,
    pub library_entries_written: usize,
    pub coverage_ok: bool,
    /// Proposed CML concepts across all provisions.
    pub cml_concepts: usize,
    pub cml_deontic_norms: usize,
    pub cml_privacy_hits: usize,
    pub cml_rights_hits: usize,
}

fn slugify(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_dash = false;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn looks_like_section_heading(title: &str) -> bool {
    let t = title.trim();
    if t.len() < 2 {
        return false;
    }
    let first = t.chars().next().unwrap_or(' ');
    if first.is_lowercase() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    for bad in [
        "of ", "or ", "and ", "to ", "in ", "for ", "as ", "under ", "made by", "has no effect",
    ] {
        if lower.starts_with(bad) {
            return false;
        }
    }
    if Regex::new(r"^\d{4}$").unwrap().is_match(t) {
        return false;
    }
    let words: Vec<_> = t.split_whitespace().collect();
    if words.len() > 12 {
        return false;
    }
    if t.ends_with('.') && words.len() > 4 {
        return false;
    }
    if Regex::new(r"^(If|Subject|This|When|Where|Unless|Despite|For the purposes)\b")
        .unwrap()
        .is_match(t)
    {
        return false;
    }
    true
}

/// Clean PDF page text into lines (mirrors legis2cml.clean lightly).
fn clean_page(raw: &str) -> String {
    raw.replace('\r', "\n")
        .replace('\u{00a0}', " ")
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse legislation pages into structured provisions (AU-oriented; EU Article/Chapter supported).
pub fn parse_pages(pages: &[(u32, String)], title_hint: Option<&str>) -> LegislationInstrument {
    let re_part = Regex::new(r"^(Part|PART)\s+([0-9IVXLC]+[A-Za-z]?)\b[\s—\-:.]*(.*)$").unwrap();
    let re_div = Regex::new(r"^(Division|DIVISION)\s+([0-9]+[A-Za-z]?)\b[\s—\-:.]*(.*)$").unwrap();
    let re_schedule =
        Regex::new(r"^(Schedule|SCHEDULE)\s+([0-9]+[A-Za-z]?)\s*[—–]\s*(.+)$").unwrap();
    let re_section = Regex::new(r"^(\d+[A-Z]{0,2})\s+([A-Z0-9][^\n]{1,160})$").unwrap();
    let re_hist = Regex::new(r"^(\d+[A-Z]{0,2})\.\s*[—–\-]?\s*([A-Z].{0,100})$").unwrap();
    let re_eu_chapter = Regex::new(r"(?i)^CHAPTER\s+([IVXLC]+)$").unwrap();
    let re_eu_article = Regex::new(r"(?i)^Article\s+(\d+[A-Z]?)$").unwrap();
    let re_enacting = Regex::new(r"(?i)Parliament of Australia enacts|BE IT ENACTED").unwrap();

    let mut frag_counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut unique_frag = |base: String| -> String {
        let n = frag_counts.entry(base.clone()).or_insert(0);
        *n += 1;
        if *n == 1 {
            base
        } else {
            format!("{base}-{n}")
        }
    };

    let cleaned_all: Vec<String> = pages
        .iter()
        .flat_map(|(_, raw)| clean_page(raw).lines().map(|s| s.trim().to_string()).collect::<Vec<_>>())
        .collect();
    let has_enacting = cleaned_all.iter().any(|ln| re_enacting.is_match(ln));
    let has_eu_articles = cleaned_all.iter().any(|ln| re_eu_article.is_match(ln));
    let mut in_body = !(has_enacting || has_eu_articles);

    let mut title = title_hint.unwrap_or("").to_string();
    if title.is_empty() {
        title = infer_title(pages).unwrap_or_else(|| "Untitled Legislative Instrument".into());
    }

    let mut provisions: Vec<Provision> = Vec::new();
    let mut cur: Option<Provision> = None;
    let mut buf: Vec<String> = Vec::new();
    let mut current_schedule: Option<String> = None;

    let flush = |cur: &mut Option<Provision>, buf: &mut Vec<String>, provisions: &mut Vec<Provision>| {
        if let Some(mut p) = cur.take() {
            p.text = buf.join("\n").trim().to_string();
            p.full_text = p.text.clone();
            provisions.push(p);
        }
        buf.clear();
    };

    let is_heading_title = |head: &str| {
        let h = head.trim();
        h.is_empty() || !h.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)
    };

    for (page_no, raw) in pages {
        for ln in clean_page(raw).lines() {
            let s = ln.trim();
            if title.is_empty()
                && !Regex::new(r"^[\d(]").unwrap().is_match(s)
                && Regex::new(r"(?i)\bAct\s+(No\.\s*\d+\s+of\s+)?\d{4}\b")
                    .unwrap()
                    .is_match(s)
            {
                title = s.to_string();
            }
            if !in_body {
                if re_enacting.is_match(s) {
                    in_body = true;
                    continue;
                }
                if re_eu_chapter.is_match(s) || re_eu_article.is_match(s) {
                    in_body = true;
                } else {
                    continue;
                }
            }
            if s.is_empty() {
                if cur.is_some() {
                    buf.push(String::new());
                }
                continue;
            }

            if let Some(m) = re_eu_chapter.captures(s) {
                flush(&mut cur, &mut buf, &mut provisions);
                current_schedule = None;
                let number = m.get(1).unwrap().as_str().to_ascii_uppercase();
                provisions.push(Provision {
                    frag: unique_frag(format!("chapter-{}", slugify(&number))),
                    kind: "part".into(),
                    number: number.clone(),
                    heading: format!("Chapter {number}"),
                    text: String::new(),
                    full_text: String::new(),
                    start_page: *page_no,
                    parent: None,
                });
                continue;
            }
            if let Some(m) = re_eu_article.captures(s) {
                flush(&mut cur, &mut buf, &mut provisions);
                let number = m.get(1).unwrap().as_str().to_ascii_uppercase();
                cur = Some(Provision {
                    frag: unique_frag(format!("article-{}", slugify(&number))),
                    kind: "section".into(),
                    number,
                    heading: String::new(),
                    text: String::new(),
                    full_text: String::new(),
                    start_page: *page_no,
                    parent: None,
                });
                // next non-empty line becomes heading if short — append as body for simplicity
                continue;
            }
            if let Some(m) = re_schedule.captures(s) {
                let sched = slugify(m.get(2).unwrap().as_str());
                if current_schedule.as_deref() != Some(&sched) {
                    flush(&mut cur, &mut buf, &mut provisions);
                    current_schedule = Some(sched.clone());
                    provisions.push(Provision {
                        frag: unique_frag(format!("sch-{sched}")),
                        kind: "schedule".into(),
                        number: m.get(2).unwrap().as_str().into(),
                        heading: m.get(3).unwrap().as_str().trim().into(),
                        text: String::new(),
                        full_text: String::new(),
                        start_page: *page_no,
                        parent: None,
                    });
                }
                continue;
            }
            if let Some(m) = re_part.captures(s) {
                if is_heading_title(m.get(3).map(|x| x.as_str()).unwrap_or("")) {
                    flush(&mut cur, &mut buf, &mut provisions);
                    let num = m.get(2).unwrap().as_str();
                    let head = m.get(3).unwrap().as_str().trim();
                    let base = if let Some(sch) = &current_schedule {
                        format!("sch-{sch}-part-{}", slugify(num))
                    } else {
                        format!("part-{}", slugify(num))
                    };
                    provisions.push(Provision {
                        frag: unique_frag(base),
                        kind: "part".into(),
                        number: num.into(),
                        heading: if head.is_empty() {
                            format!("Part {num}")
                        } else {
                            head.into()
                        },
                        text: String::new(),
                        full_text: String::new(),
                        start_page: *page_no,
                        parent: None,
                    });
                    continue;
                }
            }
            if let Some(m) = re_div.captures(s) {
                if is_heading_title(m.get(3).map(|x| x.as_str()).unwrap_or("")) {
                    flush(&mut cur, &mut buf, &mut provisions);
                    let num = m.get(2).unwrap().as_str();
                    let head = m.get(3).unwrap().as_str().trim();
                    let base = if let Some(sch) = &current_schedule {
                        format!("sch-{sch}-div-{}", slugify(num))
                    } else {
                        format!("div-{}", slugify(num))
                    };
                    provisions.push(Provision {
                        frag: unique_frag(base),
                        kind: "division".into(),
                        number: num.into(),
                        heading: if head.is_empty() {
                            format!("Division {num}")
                        } else {
                            head.into()
                        },
                        text: String::new(),
                        full_text: String::new(),
                        start_page: *page_no,
                        parent: None,
                    });
                    continue;
                }
            }
            if let Some(m) = re_section.captures(s) {
                let head = m.get(2).unwrap().as_str();
                if !s.starts_with('(') && looks_like_section_heading(head) {
                    flush(&mut cur, &mut buf, &mut provisions);
                    let num = m.get(1).unwrap().as_str();
                    let base = if let Some(sch) = &current_schedule {
                        format!("sch-{sch}-sec-{}", slugify(num))
                    } else {
                        format!("sec-{}", slugify(num))
                    };
                    cur = Some(Provision {
                        frag: unique_frag(base),
                        kind: "section".into(),
                        number: num.into(),
                        heading: head.trim().into(),
                        text: String::new(),
                        full_text: String::new(),
                        start_page: *page_no,
                        parent: None,
                    });
                    continue;
                }
            }
            if !has_eu_articles {
                if let Some(m) = re_hist.captures(s) {
                    let head = m.get(2).unwrap().as_str();
                    if !s.starts_with('(')
                        && looks_like_section_heading(head)
                        && re_section.captures(s).is_none()
                    {
                        flush(&mut cur, &mut buf, &mut provisions);
                        let num = m.get(1).unwrap().as_str();
                        let base = if let Some(sch) = &current_schedule {
                            format!("sch-{sch}-sec-{}", slugify(num))
                        } else {
                            format!("sec-{}", slugify(num))
                        };
                        let heading = head.trim();
                        if heading.len() > 90 || heading.starts_with(|c: char| c.is_lowercase()) {
                            cur = Some(Provision {
                                frag: unique_frag(base),
                                kind: "section".into(),
                                number: num.into(),
                                heading: format!("Section {num}"),
                                text: String::new(),
                                full_text: String::new(),
                                start_page: *page_no,
                                parent: None,
                            });
                            buf.push(heading.into());
                        } else {
                            cur = Some(Provision {
                                frag: unique_frag(base),
                                kind: "section".into(),
                                number: num.into(),
                                heading: heading.into(),
                                text: String::new(),
                                full_text: String::new(),
                                start_page: *page_no,
                                parent: None,
                            });
                        }
                        continue;
                    }
                }
            }
            if let Some(c) = cur.as_mut() {
                // EU article: first line after "Article N" is often the short title.
                if c.heading.is_empty() && s.len() < 120 && !s.chars().next().unwrap_or(' ').is_ascii_digit()
                {
                    c.heading = s.to_string();
                } else {
                    buf.push(s.to_string());
                }
            }
        }
    }
    flush(&mut cur, &mut buf, &mut provisions);

    // Subsection split
    provisions = decompose_provisions(provisions);

    let slug = {
        let mut s = slugify(&Regex::new(r"(?i)\s*No\.\s*\d+.*$").unwrap().replace(&title, ""));
        let parts: Vec<_> = s.split('-').take(12).collect();
        s = parts.join("-");
        if s.len() < 3 || s.len() > 90 {
            s = slugify(title_hint.unwrap_or("instrument"));
        }
        s
    };

    LegislationInstrument {
        title,
        slug,
        jurisdiction: "AU".into(),
        register_id: None,
        provisions,
        pages: pages.len(),
    }
}

fn decompose_provisions(mut provisions: Vec<Provision>) -> Vec<Provision> {
    let re_sub = Regex::new(r"^\((\d+[A-Za-z]?)\)\s+(.+)$").unwrap();
    let mut out = Vec::new();
    for mut section in provisions.drain(..) {
        if section.kind == "section" {
            if section.full_text.is_empty() {
                section.full_text = section.text.clone();
            }
            let lines: Vec<&str> = section.text.lines().collect();
            let starts: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter_map(|(i, ln)| re_sub.is_match(ln).then_some(i))
                .collect();
            if starts.len() >= 2 {
                let mut subs = Vec::new();
                for (k, &start) in starts.iter().enumerate() {
                    let end = starts.get(k + 1).copied().unwrap_or(lines.len());
                    let block = lines[start..end].join("\n").trim().to_string();
                    let num = re_sub
                        .captures(lines[start])
                        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                        .unwrap_or_else(|| format!("{k}"));
                    let frag = format!("{}-ss-{}", section.frag, slugify(&num));
                    subs.push(Provision {
                        frag,
                        kind: "subsection".into(),
                        number: format!("{}({num})", section.number),
                        heading: section.heading.clone(),
                        text: block.clone(),
                        full_text: block,
                        start_page: section.start_page,
                        parent: Some(section.frag.clone()),
                    });
                }
                let lead = if starts[0] > 0 {
                    lines[..starts[0]].join("\n").trim().to_string()
                } else {
                    String::new()
                };
                section.text = lead;
                out.push(section);
                out.extend(subs);
                continue;
            }
        }
        out.push(section);
    }
    out
}

fn infer_title(pages: &[(u32, String)]) -> Option<String> {
    let joined = pages
        .iter()
        .take(5)
        .map(|(_, r)| r.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let joined = Regex::new(r"\s+").unwrap().replace_all(&joined, " ");
    if let Some(c) = Regex::new(r"(?i)may be cited as the\s+(.{2,120}?\bAct\b[^.]{0,40}?\d{4})\b")
        .unwrap()
        .captures(&joined)
    {
        return Some(c.get(1).unwrap().as_str().trim().to_string());
    }
    None
}

/// Extract text pages from a PDF file.
pub fn extract_pdf_pages(path: &Path) -> Result<Vec<(u32, String)>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    extract_pdf_pages_from_bytes(&bytes)
}

pub fn extract_pdf_pages_from_bytes(bytes: &[u8]) -> Result<Vec<(u32, String)>, String> {
    // pdf_extract gives whole-doc text; split on form feeds if present, else one page.
    let text = pdf_extract::extract_text_from_mem(bytes).map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    if text.contains('\u{c}') {
        for (i, part) in text.split('\u{c}').enumerate() {
            pages.push((i as u32 + 1, part.to_string()));
        }
    } else {
        // Heuristic: split every ~3500 chars for page-ish markers when no form feed.
        let chunk = 3500usize;
        if text.len() <= chunk {
            pages.push((1, text));
        } else {
            let mut i = 0usize;
            let mut page = 1u32;
            while i < text.len() {
                let end = (i + chunk).min(text.len());
                let mut cut = end;
                if end < text.len() {
                    if let Some(rel) = text[i..end].rfind('\n') {
                        cut = i + rel + 1;
                    }
                }
                if cut <= i {
                    cut = end;
                }
                pages.push((page, text[i..cut].to_string()));
                i = cut;
                page += 1;
            }
        }
    }
    if pages.is_empty() {
        pages.push((1, String::new()));
    }
    Ok(pages)
}

fn fnv60(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

/// Seed a parsed instrument into the hypermedia library (instrument + every provision).
pub fn seed_instrument_into_library(
    store: &HypermediaStore,
    inst: &LegislationInstrument,
    now: u64,
) -> std::io::Result<LegislationIngestReport> {
    let mut entries = store.load()?;
    let mut by_uri: std::collections::HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.asset_uri.clone(), i))
        .collect();

    let base = format!(
        "legislation://{}/{}",
        inst.jurisdiction.to_ascii_lowercase(),
        inst.register_id
            .as_deref()
            .unwrap_or(inst.slug.as_str())
    );

    let mut written = 0usize;
    let mut upsert = |entry: LibraryEntry| {
        if let Some(&idx) = by_uri.get(&entry.asset_uri) {
            let mut e = entry;
            e.ingested_unix = entries[idx].ingested_unix;
            entries[idx] = e;
        } else {
            by_uri.insert(entry.asset_uri.clone(), entries.len());
            entries.push(entry);
        }
        written += 1;
    };

    // Build instrument-wide CML context from all provisions (structure + deontic/privacy/rights).
    let cml_units: Vec<ContextUnit> = inst
        .provisions
        .iter()
        .map(|p| ContextUnit {
            frag: p.frag.clone(),
            kind: p.kind.clone(),
            label: format!("{} {}", p.number, p.heading).trim().to_string(),
            text: p.source_text().to_string(),
            page: Some(p.start_page),
            parent: p.parent.clone(),
        })
        .collect();
    let instrument_graph = build_document_context(&base, &inst.title, &cml_units);

    // Instrument root.
    let root_uri = base.clone();
    let root_excerpt = format!(
        "{} — {} provision(s), {} page(s). Native CML context graph (cml:Proposed): {} concepts, {} deontic norms, {} privacy signals.",
        inst.title,
        inst.provisions.len(),
        inst.pages,
        instrument_graph.concepts.len(),
        instrument_graph.deontic_norms,
        instrument_graph.privacy_hits,
    );
    let mut root_topics = vec![
        "legislation".into(),
        "statute".into(),
        "cml".into(),
        inst.jurisdiction.to_ascii_lowercase(),
        inst.slug.clone(),
    ];
    root_topics.extend(instrument_graph.topics.iter().cloned());
    root_topics.sort();
    root_topics.dedup();
    let mut root_purposes = vec!["legislation".into(), "legal".into(), "work".into()];
    root_purposes.extend(instrument_graph.purposes.iter().cloned());
    root_purposes.sort();
    root_purposes.dedup();
    let root_n3 = if instrument_graph.n3.len() > 64_000 {
        format!(
            "{}…\n# [instrument cml_n3 truncated — per-provision entries hold local graphs]",
            &instrument_graph.n3[..64_000]
        )
    } else {
        instrument_graph.n3.clone()
    };
    let mut root = LibraryEntry {
        asset_uri: root_uri.clone(),
        primary_subject: fnv60(root_uri.as_bytes()),
        media_type: LEGISLATION_INSTRUMENT_MEDIA.into(),
        quins: instrument_graph.quins.clone(),
        topics: root_topics,
        projects: vec![format!("legislation:{}", inst.slug)],
        purposes: root_purposes,
        place: None,
        occurred_at: None,
        lat: None,
        lon: None,
        flags: Vec::new(),
        ingested_unix: now,
        excerpt: root_excerpt.chars().take(400).collect(),
        sensitivity: "public".into(),
        section: LibrarySection::Work.as_str().into(),
        commons_visibility: CommonsVisibility::None,
        cml_signals: instrument_graph.signal_tags.clone(),
        cml_concept_count: instrument_graph.concepts.len() as u32,
        cml_n3: root_n3,
    };
    root.recompute_section();
    upsert(root);

    let mut sections = 0usize;
    let mut subsections = 0usize;
    let mut structural = 0usize;
    let mut with_text = 0usize;
    let mut empty = 0usize;
    let mut total_cml_concepts = instrument_graph.concepts.len();
    let mut total_deontic = instrument_graph.deontic_norms;
    let mut total_privacy = instrument_graph.privacy_hits;
    let mut total_rights = instrument_graph.rights_hits;

    for p in &inst.provisions {
        match p.kind.as_str() {
            "section" => sections += 1,
            "subsection" => subsections += 1,
            "part" | "division" | "schedule" => structural += 1,
            _ => {}
        }
        let body = p.source_text();
        if body.trim().is_empty() {
            empty += 1;
        } else {
            with_text += 1;
        }
        if matches!(
            p.kind.as_str(),
            "section" | "subsection" | "part" | "division" | "schedule"
        ) {
            let uri = format!("{base}#{}", p.frag);
            let label = format!("{} {}", p.number, p.heading).trim().to_string();
            let unit = ContextUnit {
                frag: p.frag.clone(),
                kind: p.kind.clone(),
                label: label.clone(),
                text: body.to_string(),
                page: Some(p.start_page),
                parent: p.parent.clone(),
            };
            let g = build_document_context(&uri, &label, &[unit]);
            total_cml_concepts += g.concepts.len();
            total_deontic += g.deontic_norms;
            total_privacy += g.privacy_hits;
            total_rights += g.rights_hits;

            let mut topics = vec![
                "legislation".into(),
                "cml".into(),
                p.kind.clone(),
                inst.slug.clone(),
                format!("s{}", p.number),
            ];
            if let Some(parent) = &p.parent {
                topics.push(parent.clone());
            }
            topics.extend(g.topics.iter().cloned());
            topics.sort();
            topics.dedup();

            let mut purposes = vec!["legislation".into(), "legal".into(), "work".into()];
            purposes.extend(g.purposes.iter().cloned());
            purposes.sort();
            purposes.dedup();

            let mut excerpt = if body.trim().is_empty() {
                format!("{label} (no body text extracted)")
            } else {
                format!("{label}\n\n{body}")
            };
            if excerpt.len() > 12_000 {
                excerpt = excerpt.chars().take(12_000).collect();
                excerpt.push_str("\n…[truncated]");
            }
            // Prefix with signal chips for UI glance.
            if !g.signal_tags.is_empty() {
                let chips: String = g
                    .signal_tags
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" · ");
                excerpt = format!("[{chips}]\n{excerpt}");
            }

            let cml_n3 = if g.n3.len() > 24_000 {
                format!("{}…\n# [truncated]", &g.n3[..24_000])
            } else {
                g.n3
            };

            let mut entry = LibraryEntry {
                asset_uri: uri.clone(),
                primary_subject: fnv60(uri.as_bytes()),
                media_type: LEGISLATION_MEDIA_TYPE.into(),
                quins: g.quins,
                topics,
                projects: vec![format!("legislation:{}", inst.slug)],
                purposes,
                place: None,
                occurred_at: None,
                lat: None,
                lon: None,
                flags: Vec::new(),
                ingested_unix: now,
                excerpt,
                sensitivity: "public".into(),
                section: LibrarySection::Work.as_str().into(),
                commons_visibility: CommonsVisibility::None,
                cml_signals: g.signal_tags,
                cml_concept_count: g.concepts.len() as u32,
                cml_n3,
            };
            entry.recompute_section();
            upsert(entry);
        }
    }

    store.replace_all(&entries)?;

    let concepts = sections + subsections;
    let coverage_ok = empty == 0 || (with_text as f64 / concepts.max(1) as f64) >= 0.85;

    Ok(LegislationIngestReport {
        title: inst.title.clone(),
        slug: inst.slug.clone(),
        sections,
        subsections,
        structural,
        concepts_with_text: with_text,
        empty_text: empty,
        library_entries_written: written,
        coverage_ok,
        cml_concepts: total_cml_concepts,
        cml_deontic_norms: total_deontic,
        cml_privacy_hits: total_privacy,
        cml_rights_hits: total_rights,
    })
}

/// Parse PDF bytes and seed the library.
pub fn ingest_legislation_pdf_bytes(
    store: &HypermediaStore,
    bytes: &[u8],
    register_id: Option<&str>,
    jurisdiction: &str,
    title_hint: Option<&str>,
) -> Result<LegislationIngestReport, String> {
    let pages = extract_pdf_pages_from_bytes(bytes)?;
    let mut inst = parse_pages(&pages, title_hint);
    inst.jurisdiction = jurisdiction.to_string();
    if let Some(id) = register_id {
        inst.register_id = Some(id.to_string());
        if inst.slug.len() < 3 {
            inst.slug = slugify(id);
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    seed_instrument_into_library(store, &inst, now).map_err(|e| e.to_string())
}

/// Parse plain text (already extracted) as a single-page instrument and seed.
pub fn ingest_legislation_text(
    store: &HypermediaStore,
    text: &str,
    register_id: Option<&str>,
    jurisdiction: &str,
    title_hint: Option<&str>,
) -> Result<LegislationIngestReport, String> {
    let pages = vec![(1u32, text.to_string())];
    let mut inst = parse_pages(&pages, title_hint);
    inst.jurisdiction = jurisdiction.to_string();
    if let Some(id) = register_id {
        inst.register_id = Some(id.to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    seed_instrument_into_library(store, &inst, now).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enacts_skips_contents_and_keeps_body_text() {
        let pages = vec![(
            1u32,
            "Example Amendment Act 2004\nNo. 5, 2004\n\
             Contents\n1  Short title\n2  Commencement\nPart 2—Widgets\n3  Widget duty\n\
             The Parliament of Australia enacts:\n\
             1  Short title\nThis Act may be cited as the Example Amendment Act 2004.\n\
             2  Commencement\nThis Act commences on Royal Assent.\n\
             3  Widget duty\nA person must not widget.\n"
                .into(),
        )];
        let inst = parse_pages(&pages, Some("Example Amendment Act 2004"));
        let sections: Vec<_> = inst
            .provisions
            .iter()
            .filter(|p| p.kind == "section")
            .collect();
        assert_eq!(sections.len(), 3);
        assert!(sections[0].source_text().contains("may be cited"));
        assert!(sections[1].source_text().contains("commences"));
        assert!(sections[2].source_text().contains("must not widget"));
        // No phantom Part from contents.
        assert!(inst.provisions.iter().all(|p| p.kind != "part"));
    }

    #[test]
    fn seed_writes_library_entries_with_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = HypermediaStore::open(dir.path()).unwrap();
        let text = "\
The Parliament of Australia enacts:\n\
1  Short title\nThis Act may be cited as the Demo Act 2020.\n\
2  Commencement\nThis Act commences on the day after Royal Assent.\n";
        let report = ingest_legislation_text(&store, text, Some("C2020A00001"), "AU", Some("Demo Act 2020"))
            .unwrap();
        assert!(report.sections >= 2);
        assert!(report.concepts_with_text >= 2);
        assert!(report.coverage_ok);
        let work = store.by_section(LibrarySection::Work).unwrap();
        assert!(work.len() >= 3); // instrument + 2 sections
        assert!(work.iter().any(|e| e.excerpt.contains("may be cited")));
        assert!(work
            .iter()
            .any(|e| e.purposes.iter().any(|p| p == "legislation")));
        // CML context graph attached (deontic/privacy cues from body text when present).
        assert!(report.cml_concepts > 0);
        assert!(work.iter().any(|e| e.cml_concept_count > 0 || !e.cml_n3.is_empty()));
    }

    #[test]
    fn subsections_preserve_full_text_on_parent() {
        let pages = vec![(
            1u32,
            "The Parliament of Australia enacts:\n\
             5  Offence\n\
             A person commits an offence if:\n\
             (1) the person does X; and\n\
             (2) the person does Y.\n"
                .into(),
        )];
        let inst = parse_pages(&pages, Some("Offence Act"));
        let sec = inst
            .provisions
            .iter()
            .find(|p| p.kind == "section")
            .unwrap();
        let subs: Vec<_> = inst
            .provisions
            .iter()
            .filter(|p| p.kind == "subsection")
            .collect();
        assert_eq!(subs.len(), 2);
        assert!(sec.full_text.contains("(1)"));
        assert!(sec.full_text.contains("(2)"));
        assert!(subs[0].source_text().contains("does X"));
    }
}
