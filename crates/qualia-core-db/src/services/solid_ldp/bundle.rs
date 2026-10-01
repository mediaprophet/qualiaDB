//! Build a Solid-portable leave/migrate bundle from Quins.
//!
//! Solid stores **RDF triples/quads**, not Quins. This module projects filtered
//! Quins into Turtle / N-Quads / JSON-LD plus a WAC ACL and a JSON-LD manifest
//! that Solid-Databox (or any LDP server) can import with ordinary PUT.

use std::collections::HashMap;
use std::io::Write;

use crate::sparql_library::serialisers::rdf_serializers::{
    serialize_to_jsonld_compact, serialize_to_nquads, serialize_to_turtle,
};
use crate::NQuin;

use super::consent::{
    export_outcome_summary, require_sanctuary_choice, SanctuaryExportChoice,
};
use super::filter::{filter_for_egress, EgressStats, SolidEgressPolicy};
use super::terms::{write_iri, write_object};

/// Formats written into a leave-path bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidBundleFormats {
    pub turtle: bool,
    pub nquads: bool,
    pub jsonld: bool,
    pub acl: bool,
}

impl Default for SolidBundleFormats {
    fn default() -> Self {
        Self {
            turtle: true,
            nquads: true,
            jsonld: true,
            acl: true,
        }
    }
}

/// Options for building a migration bundle.
#[derive(Debug, Clone, Default)]
pub struct SolidMigrationOptions {
    /// Required when sanctuary/restricted quins are present.
    pub sanctuary_choice: SanctuaryExportChoice,
    pub formats: SolidBundleFormats,
    /// Optional owner WebID for ACL (default `urn:qualia:owner`).
    pub owner_webid: Option<String>,
    /// When true, emit public `foaf:Agent` Read — only for deliberately public graphs.
    /// Default false (QW-10: never auto permissive-commons).
    pub grant_public_read: bool,
}

impl SolidMigrationOptions {
    fn egress_policy(&self) -> SolidEgressPolicy {
        SolidEgressPolicy {
            include_restricted: self.sanctuary_choice.includes_restricted(),
        }
    }
}

/// In-memory Solid leave bundle (files a Databox can LDP-PUT).
#[derive(Debug, Clone)]
pub struct SolidMigrationBundle {
    pub turtle: Option<String>,
    pub nquads: Option<String>,
    pub jsonld: Option<String>,
    pub acl: Option<String>,
    pub manifest_jsonld: String,
    pub stats: EgressStats,
    pub sanctuary_choice: SanctuaryExportChoice,
    pub outcome_summary: String,
    pub engine_version: &'static str,
}

const ENGINE_VERSION: &str = "0.0.39";

/// Build a portable Solid RDF bundle from Quins (+ optional Q42LEX map).
///
/// Fails with `SANCTUARY_CHOICE_REQUIRED` when sanctuary data is present and
/// `sanctuary_choice` is still `Unset`.
pub fn build_migration_bundle(
    quins: &[NQuin],
    lexicon: Option<&HashMap<u64, String>>,
    opts: &SolidMigrationOptions,
) -> Result<SolidMigrationBundle, String> {
    require_sanctuary_choice(quins, opts.sanctuary_choice)?;
    let policy = opts.egress_policy();
    let mut filtered = Vec::with_capacity(quins.len());
    let stats = filter_for_egress(quins, policy, &mut filtered);

    let turtle = if opts.formats.turtle {
        Some(serialize_turtle_lex(&filtered, lexicon)?)
    } else {
        None
    };
    let nquads = if opts.formats.nquads {
        Some(serialize_nquads_lex(&filtered, lexicon)?)
    } else {
        None
    };
    let jsonld = if opts.formats.jsonld {
        let mut buf = Vec::new();
        serialize_to_jsonld_compact(&mut buf, &filtered)?;
        Some(String::from_utf8(buf).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let acl = if opts.formats.acl {
        Some(render_acl(
            opts.owner_webid
                .as_deref()
                .unwrap_or("urn:qualia:owner"),
            opts.grant_public_read,
        ))
    } else {
        None
    };

    let manifest_jsonld = render_manifest(&stats, opts, turtle.as_deref(), nquads.as_deref());
    let outcome_summary = export_outcome_summary(opts.sanctuary_choice, &stats);

    Ok(SolidMigrationBundle {
        turtle,
        nquads,
        jsonld,
        acl,
        manifest_jsonld,
        stats,
        sanctuary_choice: opts.sanctuary_choice,
        outcome_summary,
        engine_version: ENGINE_VERSION,
    })
}

/// Lexicon-aware Turtle (preferred over demo-only `serialize_to_turtle` for leave).
fn serialize_turtle_lex(
    quins: &[NQuin],
    lex: Option<&HashMap<u64, String>>,
) -> Result<String, String> {
    let mut out = Vec::new();
    writeln!(
        &mut out,
        "@prefix acl: <http://www.w3.org/ns/auth/acl#> ."
    )
    .map_err(|e| e.to_string())?;
    writeln!(&mut out, "@prefix foaf: <http://xmlns.com/foaf/0.1/> .")
        .map_err(|e| e.to_string())?;
    writeln!(
        &mut out,
        "@prefix qualia: <https://webizen.org/ld/vocab/> ."
    )
    .map_err(|e| e.to_string())?;
    writeln!(&mut out).map_err(|e| e.to_string())?;

    // Group by subject for valid Turtle blocks.
    let mut order: Vec<u64> = Vec::new();
    let mut map: HashMap<u64, Vec<&NQuin>> = HashMap::new();
    for q in quins {
        let bucket = map.entry(q.subject).or_default();
        if bucket.is_empty() {
            order.push(q.subject);
        }
        bucket.push(q);
    }
    for subject in order {
        let rows = map.remove(&subject).unwrap_or_default();
        write_iri(subject, lex, &mut out).map_err(|e| e.to_string())?;
        for (i, quin) in rows.iter().enumerate() {
            if i == 0 {
                write!(&mut out, " ").map_err(|e| e.to_string())?;
            } else {
                write!(&mut out, " ;\n    ").map_err(|e| e.to_string())?;
            }
            write_iri(quin.predicate, lex, &mut out).map_err(|e| e.to_string())?;
            write!(&mut out, " ").map_err(|e| e.to_string())?;
            write_object(quin.object, lex, &mut out).map_err(|e| e.to_string())?;
        }
        writeln!(&mut out, " .").map_err(|e| e.to_string())?;
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

fn serialize_nquads_lex(
    quins: &[NQuin],
    lex: Option<&HashMap<u64, String>>,
) -> Result<String, String> {
    // Prefer lexicon-aware lines so named-graph context is preserved for Solid.
    if lex.is_none() {
        let mut buf = Vec::new();
        serialize_to_nquads(&mut buf, quins)?;
        return String::from_utf8(buf).map_err(|e| e.to_string());
    }
    let mut out = Vec::new();
    for q in quins {
        write_iri(q.subject, lex, &mut out).map_err(|e| e.to_string())?;
        write!(&mut out, " ").map_err(|e| e.to_string())?;
        write_iri(q.predicate, lex, &mut out).map_err(|e| e.to_string())?;
        write!(&mut out, " ").map_err(|e| e.to_string())?;
        write_object(q.object, lex, &mut out).map_err(|e| e.to_string())?;
        write!(&mut out, " ").map_err(|e| e.to_string())?;
        write_iri(q.context, lex, &mut out).map_err(|e| e.to_string())?;
        writeln!(&mut out, " .").map_err(|e| e.to_string())?;
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

fn render_acl(owner_webid: &str, grant_public_read: bool) -> String {
    let mut s = String::from(
        "@prefix acl: <http://www.w3.org/ns/auth/acl#> .\n\
         @prefix foaf: <http://xmlns.com/foaf/0.1/> .\n\n\
         # Generated by QualiaDB Solid leave/migrate exporter\n\n\
         <#ownerAccess> a acl:Authorization ;\n",
    );
    s.push_str(&format!("    acl:agent <{owner_webid}> ;\n"));
    s.push_str(
        "    acl:accessTo <./data.ttl> ;\n\
            acl:mode acl:Read, acl:Write, acl:Control .\n",
    );
    if grant_public_read {
        s.push_str(
            "\n<#publicAccess> a acl:Authorization ;\n\
                acl:agentClass foaf:Agent ;\n\
                acl:accessTo <./data.ttl> ;\n\
                acl:mode acl:Read .\n",
        );
    }
    s
}

fn render_manifest(
    stats: &EgressStats,
    opts: &SolidMigrationOptions,
    turtle: Option<&str>,
    nquads: Option<&str>,
) -> String {
    let resources = {
        let mut parts = Vec::new();
        if turtle.is_some() {
            parts.push(r#"{ "@id": "data.ttl", "dcterms:format": "text/turtle" }"#);
        }
        if nquads.is_some() {
            parts.push(r#"{ "@id": "data.nq", "dcterms:format": "application/n-quads" }"#);
        }
        if opts.formats.jsonld {
            parts.push(r#"{ "@id": "data.jsonld", "dcterms:format": "application/ld+json" }"#);
        }
        if opts.formats.acl {
            parts.push(r#"{ "@id": "data.ttl.acl", "dcterms:format": "text/turtle" }"#);
        }
        parts.join(",\n    ")
    };
    format!(
        r#"{{
  "@context": {{
    "schema": "https://schema.org/",
    "dcterms": "http://purl.org/dc/terms/",
    "qualia": "https://webizen.org/ld/vocab/"
  }},
  "@type": "qualia:SolidMigrationBundle",
  "schema:softwareVersion": "{ENGINE_VERSION}",
  "qualia:sourceEngine": "QualiaDB",
  "qualia:target": "Solid LDP (triples/quads; not Quins)",
  "qualia:sanctuaryChoice": "{}",
  "qualia:includeRestricted": {},
  "qualia:grantPublicRead": {},
  "qualia:stats": {{
    "scanned": {},
    "exported": {},
    "redactedClassified": {},
    "redactedRestricted": {},
    "skippedParity": {}
  }},
  "schema:hasPart": [
    {resources}
  ]
}}"#,
        opts.sanctuary_choice.as_str(),
        opts.sanctuary_choice.includes_restricted(),
        opts.grant_public_read,
        stats.scanned,
        stats.exported,
        stats.redacted_classified,
        stats.redacted_restricted,
        stats.skipped_parity,
    )
}

/// Fallback when no custom lex map: use shared Turtle serializer (demo lex).
#[allow(dead_code)]
pub fn serialize_turtle_shared(quins: &[NQuin]) -> Result<String, String> {
    let mut buf = Vec::new();
    serialize_to_turtle(&mut buf, quins)?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    fn alice_knows_bob() -> NQuin {
        let mut q = NQuin {
            subject: q_hash("Alice"),
            predicate: q_hash("knows"),
            object: q_hash("Bob"),
            context: q_hash("http://example.org/graph"),
            metadata: 0,
            parity: 0,
        };
        q.recalculate_parity();
        q
    }

    #[test]
    fn bundle_emits_turtle_and_redacts_classified() {
        let public = alice_knows_bob();
        let mut classified = alice_knows_bob();
        classified.set_sensitivity_byte(NQuin::SENSITIVITY_CLASSIFIED);
        classified.recalculate_parity();

        let bundle = build_migration_bundle(
            &[public, classified],
            None,
            &SolidMigrationOptions::default(),
        )
        .unwrap();
        assert_eq!(bundle.stats.exported, 1);
        assert_eq!(bundle.stats.redacted_classified, 1);
        let ttl = bundle.turtle.as_deref().unwrap();
        assert!(ttl.contains("schema.org/knows") || ttl.contains("quin:hash/"));
        assert!(bundle.manifest_jsonld.contains("SolidMigrationBundle"));
        assert!(!bundle.acl.as_deref().unwrap().contains("foaf:Agent"));
    }

    #[test]
    fn public_read_acl_only_when_requested() {
        let q = alice_knows_bob();
        let mut opts = SolidMigrationOptions::default();
        opts.grant_public_read = true;
        let bundle = build_migration_bundle(&[q], None, &opts).unwrap();
        assert!(bundle.acl.as_deref().unwrap().contains("foaf:Agent"));
    }
}
