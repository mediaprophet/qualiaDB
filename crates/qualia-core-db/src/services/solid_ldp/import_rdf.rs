//! Solid RDF → Qualia Quins (return path for Databox / wasm-webcivics).
//!
//! Solid stores triples/quads. Import parses RDF into Quins for Qualia restore
//! or SHACL admission — the Pod itself remains RDF.

use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
use crate::NQuin;
use std::io::Cursor;

/// Parse a Solid RDF document into Quins (cold path).
pub fn import_rdf_to_quins(
    content_type: &str,
    payload: &str,
) -> Result<(Vec<NQuin>, RdfFormat), String> {
    let format = RdfFormat::from_media_type(content_type)
        .or_else(|| RdfFormat::from_str(content_type))
        .ok_or_else(|| format!("unsupported RDF Content-Type / format: {content_type}"))?;
    if matches!(format, RdfFormat::CborLd) {
        return Err("use vendor nquin-CBOR parse for application/vnd.qualia.nquin-cbor".into());
    }
    let mut collector = QuinCollector::new();
    parse_rdf(format, Cursor::new(payload.as_bytes()), 0, &mut collector)
        .map_err(|e| e.to_string())?;
    Ok((collector.as_slice().to_vec(), format))
}

/// Build a Qualia/webcivics vault-backup style JSON package from RDF Sources.
///
/// Schema matches `webcivics.vault-backup.v1` so Desktop / PWA `restoreBackup`
/// can ingest after Solid→Qualia export.
pub fn solid_rdf_to_qualia_backup_json(
    resources: &[(String, String, String)],
) -> Result<String, String> {
    // resources: (filename, content_type, body)
    let mut files = serde_json::Map::new();
    let mut total_quins = 0usize;
    for (name, ct, body) in resources {
        let (quins, _) = import_rdf_to_quins(ct, body)?;
        total_quins += quins.len();
        files.insert(name.clone(), serde_json::Value::String(body.clone()));
        // Also stash a quin wire dump for native restore helpers.
        let wire: Vec<Vec<String>> = quins
            .iter()
            .map(|q| {
                vec![
                    q.subject.to_string(),
                    q.predicate.to_string(),
                    q.object.to_string(),
                    q.context.to_string(),
                    q.metadata.to_string(),
                    q.parity.to_string(),
                ]
            })
            .collect();
        files.insert(
            format!("{name}.quins.json"),
            serde_json::Value::Array(
                wire.into_iter()
                    .map(|row| {
                        serde_json::Value::Array(
                            row.into_iter().map(serde_json::Value::String).collect(),
                        )
                    })
                    .collect(),
            ),
        );
    }
    let created_unix = now_unix();
    let payload = serde_json::json!({
        "schema": "webcivics.vault-backup.v1",
        "createdUnix": created_unix,
        "label": format!("solid-export-{created_unix}"),
        "source": "solid-ldp",
        "quinCount": total_quins,
        "files": files,
    });
    let package = serde_json::json!({
        "manifest": {
            "schema": "webcivics.vault-backup.v1",
            "createdUnix": created_unix,
            "primaryKind": "solid-export",
            "notes": "Exported from Solid LDP via wasm-webcivics / Qualia bridge"
        },
        "payload": payload
    });
    serde_json::to_string_pretty(&package).map_err(|e| e.to_string())
}

fn now_unix() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_round_trip_count() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:Alice ex:knows ex:Bob .
"#;
        let (quins, fmt) = import_rdf_to_quins("text/turtle", ttl).unwrap();
        assert_eq!(fmt, RdfFormat::Turtle);
        assert!(quins.len() >= 1);
    }
}
