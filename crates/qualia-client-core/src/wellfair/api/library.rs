//! Hypermedia asset library: ingest, search, CML, COF


use super::*;

impl WebizenHostApi {
    // --- Hypermedia asset library: ingest a document → make it searchable by meaning ---

    fn library(&self) -> Result<super::super::hypermedia_store::HypermediaStore, String> {
        super::super::hypermedia_store::HypermediaStore::open(&self.storage_root).map_err(|e| e.to_string())
    }

    /// **Ingest a text document** into the library: derive its topics + searchable text, bind them into a
    /// hypermedia container, persist it (findable by meaning), and — if `guardian_did` is set (the principal
    /// is under a guardianship relation) and a flag is raised — notify the guardian **and record it in the
    /// tamper-evident ledger**. Returns a summary (topics, flags, any guardian notifications).
    pub fn ingest_document(
        &self,
        uri: &str,
        media_type: &str,
        text: &str,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        self.ingest_bytes(uri, media_type, text.as_bytes(), text, &ManualFacets::default(), guardian_did)
    }

    /// Ingest a text document **with person-authored facets** — an optional date (→ timeline), place
    /// (→ map), project and purpose the person chooses to attach. The document's derived topics still come
    /// from its content; these facets are added on top (the person authoring meaning, not being defined).
    pub fn ingest_document_annotated(
        &self,
        uri: &str,
        media_type: &str,
        text: &str,
        manual: &ManualFacets,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        self.ingest_bytes(uri, media_type, text.as_bytes(), text, manual, guardian_did)
    }

    /// **Ingest any asset bytes** (a document, a **photo**, an audio clip) into the library. The processor
    /// registered for `media_type` derives searchability — a text doc → topics; a **JPEG/PNG → its EXIF
    /// capture time (timeline) + GPS place (map)**; a WAV → duration + dominant frequency — and it all folds
    /// into the container so the original is findable by meaning. `excerpt_source` is a short human string for
    /// the results list (the text for a doc; a caption/filename for binary). Guardianship + ledger hook as
    /// [`Self::ingest_document`].
    pub fn ingest_bytes(
        &self,
        uri: &str,
        media_type: &str,
        bytes: &[u8],
        excerpt_source: &str,
        manual: &ManualFacets,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        use qualia_core_db::hypermedia::processors::processor_for;
        use qualia_core_db::hypermedia::{
            content_digest, descriptors_to_nquins, ingest_with, Descriptors, FlagSeverity, Place,
        };

        let proc = processor_for(media_type)
            .ok_or_else(|| format!("no ingest processor for media type '{media_type}'"))?;
        let digest = content_digest(bytes);
        let out = proc.process(uri, bytes, media_type);
        let mut r = ingest_with(proc.as_ref(), uri, media_type, digest, bytes);
        let now = Self::now_unix();
        let primary_subject = r.container.primary.subject();

        // Merge the person-authored facets as additional descriptor edges on the primary asset. A processor's
        // own derivation (a photo's EXIF) takes precedence for its fields; manual facets fill / extend.
        let manual_place = match (manual.lat, manual.lon) {
            (Some(lat), Some(lon)) => Some(Place {
                label: manual.place_label.clone().unwrap_or_else(|| format!("{lat:.5},{lon:.5}")),
                lat,
                lon,
            }),
            _ => None,
        };
        if !manual.is_empty() {
            let extra = Descriptors {
                occurred_at: manual.occurred_at.filter(|_| out.descriptors.occurred_at.is_none()),
                place: if out.descriptors.place.is_none() { manual_place.clone() } else { None },
                projects: manual.projects.clone(),
                purposes: manual.purposes.clone(),
                ..Default::default()
            };
            let (eq, _lex) = descriptors_to_nquins(primary_subject, &extra);
            r.quins.extend(eq);
        }

        let flags: Vec<super::super::hypermedia_store::LibraryFlag> = out
            .flags
            .iter()
            .map(|f| super::super::hypermedia_store::LibraryFlag {
                kind: f.kind.clone(),
                severity_level: f.severity.level(),
                detail: f.detail.clone(),
            })
            .collect();

        // Effective facets for the entry's display fields: processor-derived first, else the person's.
        let eff_occurred_at = out.descriptors.occurred_at.or(manual.occurred_at);
        let eff_place = out.descriptors.place.clone().or(manual_place);
        let (lat, lon) = eff_place
            .as_ref()
            .map(|p| (Some(p.lat), Some(p.lon)))
            .unwrap_or((None, None));
        let mut projects = out.descriptors.projects.clone();
        projects.extend(manual.projects.iter().cloned());

        let purposes = manual.purposes.clone();
        let sensitivity = super::super::hypermedia_store::normalize_sensitivity(
            manual
                .sensitivity
                .as_deref()
                .unwrap_or("public"),
        );
        let commons = super::super::hypermedia_store::CommonsVisibility::parse(
            manual
                .commons_visibility
                .as_deref()
                .unwrap_or("none"),
        );
        // Rust-native CML context graph for text-like assets (TEXT→CONCEPT→LOGIC, cml:Proposed).
        let mut cml_topics = Vec::new();
        let mut cml_purposes = purposes.clone();
        let mut cml_signals = Vec::new();
        let mut cml_concept_count = 0u32;
        let mut cml_n3 = String::new();
        let mut cml_quins = Vec::new();
        if media_type.starts_with("text/") || media_type.contains("json") || media_type.contains("markdown")
        {
            let text = String::from_utf8_lossy(bytes);
            let units = super::super::cml_context::units_from_headings(&text);
            let g = super::super::cml_context::build_document_context(uri, excerpt_source, &units);
            cml_topics = g.topics.clone();
            for p in &g.purposes {
                if !cml_purposes.iter().any(|x| x == p) {
                    cml_purposes.push(p.clone());
                }
            }
            cml_signals = g.signal_tags.clone();
            cml_concept_count = g.concepts.len() as u32;
            cml_n3 = if g.n3.len() > 48_000 {
                format!("{}…\n# [cml_n3 truncated]", &g.n3[..48_000])
            } else {
                g.n3
            };
            cml_quins = g.quins;
        }

        let mut topics = out.descriptors.topics.clone();
        for t in cml_topics {
            if !topics.iter().any(|x| x == &t) {
                topics.push(t);
            }
        }

        let mut all_quins = r.quins;
        all_quins.extend(cml_quins);

        let mut entry = super::super::hypermedia_store::LibraryEntry {
            asset_uri: uri.to_string(),
            primary_subject,
            media_type: media_type.to_string(),
            quins: all_quins,
            topics,
            projects,
            purposes: cml_purposes,
            place: eff_place.as_ref().map(|p| p.label.clone()),
            occurred_at: eff_occurred_at,
            lat,
            lon,
            flags: flags.clone(),
            ingested_unix: now,
            excerpt: excerpt_source.chars().take(160).collect(),
            sensitivity: sensitivity.clone(),
            section: manual
                .section
                .clone()
                .unwrap_or_else(|| "personal".into()),
            commons_visibility: commons,
            cml_signals,
            cml_concept_count,
            cml_n3,
            cof_html: String::new(),
            cof_segment_count: 0,
            cof_segment_index: 0,
            cof_profile: String::new(),
        };

        // COF HTML+RDFa package (token-bounded segments) for text assets.
        let mut cof_segment_count = 0u32;
        let mut cof_profile = String::new();
        let mut cof_body_segments: Vec<super::super::cml_context::CofSegment> = Vec::new();
        if media_type.starts_with("text/") {
            let text = String::from_utf8_lossy(bytes);
            let units = super::super::cml_context::units_from_headings(&text);
            let pkg = super::super::cml_context::build_cof_package(
                uri,
                excerpt_source,
                &units,
                super::super::cml_context::DEFAULT_SEGMENT_MAX_CHARS,
                super::super::cml_context::CofStyle::AgentLean,
            );
            cof_segment_count = pkg.segments.len() as u32;
            cof_profile = pkg.profile.clone();
            entry.cof_segment_count = cof_segment_count;
            entry.cof_profile = cof_profile.clone();
            if let Some(index_seg) = pkg.segments.iter().find(|s| s.is_index) {
                entry.cof_html = index_seg.html.clone();
                entry.cof_segment_index = 0;
            } else if let Some(first) = pkg.segments.first() {
                entry.cof_html = first.html.clone();
                entry.cof_segment_index = first.index;
            }
            cof_body_segments = pkg
                .segments
                .into_iter()
                .filter(|s| !s.is_index)
                .collect();
        }

        entry.recompute_section();
        // High sensitivity can never be commons.
        if entry.is_secret() {
            entry.commons_visibility = super::super::hypermedia_store::CommonsVisibility::None;
        }
        let section = entry.section.clone();
        let commons_visibility = entry.commons_visibility;
        let entry_topics = entry.topics.clone();
        let entry_purposes = entry.purposes.clone();
        let store = self.library()?;
        store.add(entry).map_err(|e| e.to_string())?;

        // Sibling COF body segments — load only the budget needed for a turn.
        for seg in &cof_body_segments {
            let seg_uri = format!("{uri}#cof-seg-{}", seg.index);
            let mut se = super::super::hypermedia_store::LibraryEntry {
                asset_uri: seg_uri.clone(),
                primary_subject: qualia_core_db::hypermedia::fnv60(seg_uri.as_bytes()),
                media_type: super::super::cml_context::MEDIA_TYPE_COF.into(),
                quins: Vec::new(),
                topics: entry_topics.clone(),
                projects: Vec::new(),
                purposes: entry_purposes.clone(),
                place: None,
                occurred_at: None,
                lat: None,
                lon: None,
                flags: Vec::new(),
                ingested_unix: now,
                excerpt: format!(
                    "COF segment {}/{} · ~{} tokens · units: {}",
                    seg.index + 1,
                    seg.total,
                    seg.approx_tokens,
                    seg.unit_frags.join(", ")
                ),
                sensitivity: sensitivity.clone(),
                section: section.clone(),
                commons_visibility,
                cml_signals: Vec::new(),
                cml_concept_count: seg.unit_frags.len() as u32,
                cml_n3: String::new(),
                cof_html: seg.html.clone(),
                cof_segment_count,
                cof_segment_index: seg.index,
                cof_profile: cof_profile.clone(),
            };
            se.recompute_section();
            store.add(se).map_err(|e| e.to_string())?;
        }

        // Guardianship hook: a flagged ingest under a guardianship relation notifies + records.
        let mut notified = Vec::new();
        if let Some(g) = &guardian_did {
            if !out.flags.is_empty() {
                let ns = super::super::ingest_guardian::guardian_notifications(
                    &out.flags,
                    uri,
                    g,
                    &self.owner_did,
                    FlagSeverity::Notice,
                    now,
                );
                self.record_guardian_notifications(&ns)?;
                notified = ns;
            }
        }
        Ok(serde_json::json!({
            "asset_uri": uri,
            "topics": entry_topics,
            "occurred_at": eff_occurred_at,
            "place": eff_place.as_ref().map(|p| &p.label),
            "lat": lat,
            "lon": lon,
            "flags": flags,
            "guardian_notifications": notified,
            "section": section,
            "sensitivity": sensitivity,
            "commons_visibility": commons_visibility,
            "purposes": entry_purposes,
            "cml_concept_count": cml_concept_count,
            "cof_segment_count": cof_segment_count,
            "cof_profile": cof_profile,
        }))
    }

    /// **Ingest a photo/audio file from hex-encoded bytes** — the boundary form for the desktop, which reads a
    /// picked file and passes its bytes as hex (a JPEG is not valid utf-8, so it cannot come through the text
    /// path). A photo's EXIF capture-time + GPS auto-populate the timeline + map. `caption` is the short
    /// display string. Same derive + persist + guardian hook as [`Self::ingest_bytes`].
    pub fn ingest_file_hex(
        &self,
        uri: &str,
        media_type: &str,
        bytes_hex: &str,
        caption: &str,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        let bytes = decode_hex(bytes_hex).map_err(|e| format!("bad hex: {e}"))?;
        self.ingest_bytes(uri, media_type, &bytes, caption, &ManualFacets::default(), guardian_did)
    }

    /// Search the library by facet (`topic` | `depicts` | `place` | `project` | `purpose`). Returns per-entry
    /// summaries (not the raw quins).
    pub fn search_library(&self, facet: &str, value: &str) -> Result<Vec<serde_json::Value>, String> {
        let entries = self.library()?.search(facet, value).map_err(|e| e.to_string())?;
        Ok(entries.iter().map(library_summary).collect())
    }

    /// The **timeline** query — entries whose event instant falls within `[start, end]` (unix seconds).
    pub fn search_library_time(&self, start: i64, end: i64) -> Result<Vec<serde_json::Value>, String> {
        let entries = self.library()?.search_time_range(start, end).map_err(|e| e.to_string())?;
        Ok(entries.iter().map(library_summary).collect())
    }

    /// Everything in the library (newest first), as summaries.
    /// Optional `section` filters to secret | wellfair | personal | work | commons.
    pub fn list_library(&self) -> Result<Vec<serde_json::Value>, String> {
        self.list_library_section(None)
    }

    pub fn list_library_section(
        &self,
        section: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let store = self.library()?;
        let entries = match section {
            Some(s) if !s.is_empty() && s != "all" => store
                .by_section(super::super::hypermedia_store::LibrarySection::parse(s))
                .map_err(|e| e.to_string())?,
            _ => store.all().map_err(|e| e.to_string())?,
        };
        Ok(entries.iter().map(library_summary).collect())
    }

    /// Free-text search over uri / excerpt / topics / projects / place.
    pub fn search_library_text(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let entries = self.library()?.search_text(query).map_err(|e| e.to_string())?;
        Ok(entries.iter().map(library_summary).collect())
    }

    /// Multi-facet library query with sort. `filter_json` is a [`FacetFilter`] object;
    /// `sort` is newest|oldest|title_asc|title_desc|media_type|category.
    pub fn query_library_faceted(
        &self,
        filter_json: &str,
        sort: &str,
    ) -> Result<serde_json::Value, String> {
        let filter: super::super::hypermedia_store::FacetFilter = if filter_json.trim().is_empty() {
            Default::default()
        } else {
            serde_json::from_str(filter_json).map_err(|e| format!("facet filter json: {e}"))?
        };
        let sort = super::super::hypermedia_store::LibrarySort::parse(sort);
        let store = self.library()?;
        let entries = store
            .query_faceted(&filter, sort)
            .map_err(|e| e.to_string())?;
        let counts = store.facet_counts(&filter).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "entries": entries.iter().map(library_summary).collect::<Vec<_>>(),
            "total": entries.len(),
            "sort": sort.as_str(),
            "filter": filter,
            "facets": counts,
        }))
    }

    /// Facet value counts for chip UI (optionally narrowed by the same filter JSON).
    pub fn library_facet_counts(&self, filter_json: &str) -> Result<serde_json::Value, String> {
        let filter: super::super::hypermedia_store::FacetFilter = if filter_json.trim().is_empty() {
            Default::default()
        } else {
            serde_json::from_str(filter_json).map_err(|e| format!("facet filter json: {e}"))?
        };
        let counts = self
            .library()?
            .facet_counts(&filter)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(counts).map_err(|e| e.to_string())?)
    }

    /// Seed the early studio academic QApp inventory into Library → Software.
    /// Idempotent; returns add/update counts.
    pub fn seed_studio_qapps_library(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let report = super::super::qapp_catalog::seed_studio_qapps_into_library(&store)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Seed perception models + ontology catalogue rows into Library → Software.
    /// Also ensures seed weight files under `{storage}/models/`.
    pub fn seed_perception_library(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let root = self.storage_root();
        let report =
            super::super::perception_catalog::seed_perception_into_library(&store, root)?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Native legislation ingest (structure parse, no Ollama): PDF bytes → Work shelf
    /// entries for the instrument and every Part/Section/Subsection with full body text.
    pub fn ingest_legislation_pdf_hex(
        &self,
        hex_bytes: &str,
        register_id: Option<&str>,
        jurisdiction: Option<&str>,
        title_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let bytes = decode_hex(hex_bytes)?;
        let store = self.library()?;
        let report = super::super::legislation_ingest::ingest_legislation_pdf_bytes(
            &store,
            &bytes,
            register_id,
            jurisdiction.unwrap_or("AU"),
            title_hint,
        )?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Native legislation ingest from plain text (already extracted PDF text or HTML).
    pub fn ingest_legislation_text(
        &self,
        text: &str,
        register_id: Option<&str>,
        jurisdiction: Option<&str>,
        title_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let report = super::super::legislation_ingest::ingest_legislation_text(
            &store,
            text,
            register_id,
            jurisdiction.unwrap_or("AU"),
            title_hint,
        )?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Build a Rust-native CML context graph for arbitrary text (no Python).
    /// Returns concepts, signal tags, N3, and deontic/privacy counts — does not persist.
    pub fn build_cml_context_graph(
        &self,
        uri: &str,
        title: &str,
        text: &str,
    ) -> Result<serde_json::Value, String> {
        let units = super::super::cml_context::units_from_headings(text);
        let g = super::super::cml_context::build_document_context(uri, title, &units);
        Ok(serde_json::json!({
            "document_uri": g.document_uri,
            "title": g.title,
            "concepts": g.concepts,
            "signal_tags": g.signal_tags,
            "topics": g.topics,
            "purposes": g.purposes,
            "deontic_norms": g.deontic_norms,
            "privacy_hits": g.privacy_hits,
            "rights_hits": g.rights_hits,
            "quin_count": g.quins.len(),
            "n3": g.n3,
            "curation": "cml:Proposed",
            "engine": "qualia-client-core::wellfair::cml_context",
        }))
    }

    /// Build a **COF HTML+RDFa** package (token-bounded segments) without persisting.
    /// `max_chars` defaults to 24000 when zero/None.
    pub fn build_cof_html_package(
        &self,
        uri: &str,
        title: &str,
        text: &str,
        max_chars: Option<usize>,
        dual_surface: bool,
    ) -> Result<serde_json::Value, String> {
        let units = super::super::cml_context::units_from_headings(text);
        let style = if dual_surface {
            super::super::cml_context::CofStyle::DualSurface
        } else {
            super::super::cml_context::CofStyle::AgentLean
        };
        let max = max_chars
            .filter(|n| *n >= 2000)
            .unwrap_or(super::super::cml_context::DEFAULT_SEGMENT_MAX_CHARS);
        let pkg = super::super::cml_context::build_cof_package(uri, title, &units, max, style);
        Ok(serde_json::json!({
            "document_uri": pkg.document_uri,
            "title": pkg.title,
            "profile": pkg.profile,
            "segment_max_chars": pkg.segment_max_chars,
            "total_chars": pkg.total_chars,
            "total_approx_tokens": pkg.total_approx_tokens,
            "segments": pkg.segments.iter().map(|s| serde_json::json!({
                "index": s.index,
                "total": s.total,
                "id": s.id,
                "title": s.title,
                "char_count": s.char_count,
                "approx_tokens": s.approx_tokens,
                "unit_frags": s.unit_frags,
                "is_index": s.is_index,
                "html": s.html,
            })).collect::<Vec<_>>(),
            "how": [
                "Load segment 0 (index) for a token-cheap map of the instrument.",
                "Load only the body segment(s) whose unit_frags match the query.",
                "RDFa attributes carry CML edges; do not strip typeof/property/resource.",
            ],
        }))
    }

    /// Re-run CML context enrichment on an existing library entry's excerpt/text fields.
    pub fn enrich_library_entry_cml(&self, asset_uri: &str) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let mut entries = store.load().map_err(|e| e.to_string())?;
        let e = entries
            .iter_mut()
            .find(|x| x.asset_uri == asset_uri)
            .ok_or_else(|| format!("unknown asset '{asset_uri}'"))?;
        let text = if e.excerpt.len() > 40 {
            e.excerpt.clone()
        } else {
            return Err("entry has no usable text in excerpt to enrich".into());
        };
        let units = super::super::cml_context::units_from_headings(&text);
        let g = super::super::cml_context::build_document_context(&e.asset_uri, &e.asset_uri, &units);
        for t in &g.topics {
            if !e.topics.iter().any(|x| x == t) {
                e.topics.push(t.clone());
            }
        }
        for p in &g.purposes {
            if !e.purposes.iter().any(|x| x == p) {
                e.purposes.push(p.clone());
            }
        }
        e.cml_signals = g.signal_tags.clone();
        e.cml_concept_count = g.concepts.len() as u32;
        e.cml_n3 = if g.n3.len() > 48_000 {
            format!("{}…", &g.n3[..48_000])
        } else {
            g.n3.clone()
        };
        e.quins.extend(g.quins);
        e.recompute_section();
        let out = library_summary(e);
        store.replace_all(&entries).map_err(|e| e.to_string())?;
        Ok(out)
    }

    /// List catalogue categories (for Software shelf UI without seeding first).
    pub fn list_qapp_catalog_categories(&self) -> Result<serde_json::Value, String> {
        let cats: Vec<serde_json::Value> = super::super::qapp_catalog::catalogue_categories()
            .into_iter()
            .map(|slug| {
                serde_json::json!({
                    "slug": slug,
                    "label": super::super::qapp_catalog::category_label(slug),
                    "count": super::super::qapp_catalog::STUDIO_QAPP_CATALOG
                        .iter()
                        .filter(|e| e.category == slug)
                        .count(),
                })
            })
            .collect();
        Ok(serde_json::json!({
            "total": super::super::qapp_catalog::STUDIO_QAPP_CATALOG.len(),
            "categories": cats,
        }))
    }

    /// Aggregate library stats for the UI header (includes section counts).
    pub fn library_stats(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let s = store.stats().map_err(|e| e.to_string())?;
        let sections = store.section_counts().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "total": s.total,
            "with_date": s.with_date,
            "with_place": s.with_place,
            "flags": s.flags,
            "quins": s.quins,
            "topics": s.topics,
            "projects": s.projects,
            "sections": sections,
        }))
    }

    /// Set commons / peer visibility (refuses Secret).
    pub fn set_library_commons_visibility(
        &self,
        asset_uri: &str,
        visibility: &str,
    ) -> Result<serde_json::Value, String> {
        let vis = super::super::hypermedia_store::CommonsVisibility::parse(visibility);
        let e = self
            .library()?
            .set_commons_visibility(asset_uri, vis)
            .map_err(|e| e.to_string())?;
        Ok(library_summary(&e))
    }

    /// Build a **permissive commons share card** for social networking (no secret payloads).
    /// Returns metadata peers can list; raw content stays on-device until a fuller mesh transfer.
    pub fn library_commons_share_card(&self, asset_uri: &str) -> Result<serde_json::Value, String> {
        let entries = self.library()?.all().map_err(|e| e.to_string())?;
        let e = entries
            .iter()
            .find(|x| x.asset_uri == asset_uri)
            .ok_or_else(|| format!("unknown asset '{asset_uri}'"))?;
        if e.is_secret() {
            return Err("secret items cannot be offered to the commons".into());
        }
        if e.commons_visibility == super::super::hypermedia_store::CommonsVisibility::None {
            return Err(
                "set commons visibility to peers or commons before sharing".into(),
            );
        }
        Ok(serde_json::json!({
            "qualia_library_commons": "1",
            "asset_uri": e.asset_uri,
            "media_type": e.media_type,
            "topics": e.topics,
            "projects": e.projects,
            "purposes": e.purposes,
            "excerpt": e.excerpt,
            "section": e.section,
            "commons_visibility": e.commons_visibility,
            "how": [
                "Host: Keep → Library → Commons → Share to peers.",
                "Peer: accept via Talk social connection; request content over mesh when available.",
            ],
            "note": "Card is metadata only — not the secret body. High-sensitivity items never appear here.",
        }))
    }

    /// Remove one library entry by asset URI.
    pub fn remove_library_entry(&self, asset_uri: &str) -> Result<serde_json::Value, String> {
        let ok = self.library()?.remove(asset_uri).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "removed": ok, "asset_uri": asset_uri }))
    }

    /// Export the full hypermedia graph mass (quin count + optional dump for inject).
    /// Returns `{ quin_count, entries }` — the live graph inject seam for daemon/MCP.
    pub fn export_library_graph(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let entries = store.all().map_err(|e| e.to_string())?;
        let quins = store.all_quins().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "quin_count": quins.len(),
            "entry_count": entries.len(),
            "message": "Hypermedia edge-graph ready for daemon /query inject. Quins are the searchable semantic form.",
            "sample_subjects": entries.iter().take(8).map(|e| e.primary_subject).collect::<Vec<_>>(),
        }))
    }

}

// ── Vault-free path helpers (AppState storage_path; no Sanctuary HostApi) ─────
//
// The hypermedia shelf is a JSON index under `{storage}/wellfair/`. Reading and
// seeding catalogue rows must work **before** the person unlocks Sanctuary —
// otherwise Library looks permanently empty after a perception seed.

use super::super::hypermedia_store::{
    FacetFilter, HypermediaStore, LibrarySection, LibrarySort,
};
use super::library_summary;
use std::path::Path;

fn open_store(storage_root: &Path) -> Result<HypermediaStore, String> {
    HypermediaStore::open(storage_root).map_err(|e| e.to_string())
}

/// List library entries at `storage_root` (newest-first store order). Optional section filter.
pub fn list_library_section_at(
    storage_root: &Path,
    section: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    let store = open_store(storage_root)?;
    let entries = match section {
        Some(s) if !s.is_empty() && s != "all" => store
            .by_section(LibrarySection::parse(s))
            .map_err(|e| e.to_string())?,
        _ => store.all().map_err(|e| e.to_string())?,
    };
    Ok(entries.iter().map(library_summary).collect())
}

/// Faceted query + facet counts at `storage_root` (same JSON shape as HostApi).
pub fn query_library_faceted_at(
    storage_root: &Path,
    filter_json: &str,
    sort: &str,
) -> Result<serde_json::Value, String> {
    let filter: FacetFilter = if filter_json.trim().is_empty() {
        Default::default()
    } else {
        serde_json::from_str(filter_json).map_err(|e| format!("facet filter json: {e}"))?
    };
    let sort = LibrarySort::parse(sort);
    let store = open_store(storage_root)?;
    let entries = store
        .query_faceted(&filter, sort)
        .map_err(|e| e.to_string())?;
    let counts = store.facet_counts(&filter).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "entries": entries.iter().map(library_summary).collect::<Vec<_>>(),
        "total": entries.len(),
        "sort": sort.as_str(),
        "filter": filter,
        "facets": counts,
    }))
}

/// Aggregate stats at `storage_root` (header chips + section counts).
pub fn library_stats_at(storage_root: &Path) -> Result<serde_json::Value, String> {
    let store = open_store(storage_root)?;
    let s = store.stats().map_err(|e| e.to_string())?;
    let sections = store.section_counts().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "total": s.total,
        "with_date": s.with_date,
        "with_place": s.with_place,
        "flags": s.flags,
        "quins": s.quins,
        "topics": s.topics,
        "projects": s.projects,
        "sections": sections,
    }))
}

/// Facet search (`topic` | `depicts` | `place` | `project` | `purpose`) without vault.
pub fn search_library_at(
    storage_root: &Path,
    facet: &str,
    value: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let store = open_store(storage_root)?;
    let entries = store.search(facet, value).map_err(|e| e.to_string())?;
    Ok(entries.iter().map(library_summary).collect())
}

/// Free-text search without vault.
pub fn search_library_text_at(
    storage_root: &Path,
    query: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let store = open_store(storage_root)?;
    let entries = store.search_text(query).map_err(|e| e.to_string())?;
    Ok(entries.iter().map(library_summary).collect())
}

/// Timeline range search without vault.
pub fn search_library_time_at(
    storage_root: &Path,
    start: i64,
    end: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let store = open_store(storage_root)?;
    let entries = store
        .search_time_range(start, end)
        .map_err(|e| e.to_string())?;
    Ok(entries.iter().map(library_summary).collect())
}

#[cfg(test)]
mod vault_free_tests {
    use super::*;
    use crate::wellfair::hypermedia_store::{CommonsVisibility, LibraryEntry};

    #[test]
    fn vault_free_list_and_stats_see_seeded_rows() {
        let dir = tempfile::tempdir().unwrap();
        let store = HypermediaStore::open(dir.path()).unwrap();
        let entry = LibraryEntry {
            asset_uri: "model://test-vision".into(),
            primary_subject: 42,
            media_type: "application/x-webizen-model".into(),
            quins: vec![],
            topics: vec!["perception".into(), "computer_vision".into()],
            projects: vec!["perception:vision".into()],
            purposes: vec!["model".into()],
            place: None,
            occurred_at: None,
            lat: None,
            lon: None,
            flags: vec![],
            ingested_unix: 1_700_000_000,
            excerpt: "test model row".into(),
            sensitivity: "public".into(),
            section: "software".into(),
            commons_visibility: CommonsVisibility::None,
            cml_signals: vec![],
            cml_concept_count: 0,
            cml_n3: String::new(),
            cof_html: String::new(),
            cof_segment_count: 0,
            cof_segment_index: 0,
            cof_profile: String::new(),
        };
        store.add(entry).unwrap();

        let listed = list_library_section_at(dir.path(), Some("software")).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["asset_uri"], "model://test-vision");

        let stats = library_stats_at(dir.path()).unwrap();
        assert_eq!(stats["total"], 1);
        assert_eq!(stats["sections"]["software"], 1);

        let faceted = query_library_faceted_at(dir.path(), r#"{"section":"software"}"#, "newest")
            .unwrap();
        assert_eq!(faceted["total"], 1);
        assert_eq!(faceted["entries"].as_array().unwrap().len(), 1);
    }
}