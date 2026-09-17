//! Requests for RDF-Star and ontology authoring surfaces.

use super::helpers::field_value;
use web_sys::Document;

pub(super) fn authoring_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    match mode {
        "rdf-star" => source_request(document, "rdfstar-editor", "rdfstar_resolve"),
        "rdfstar-extract" => {
            let source = required_source(document, "rdfstar-editor")?;
            Ok(("NLP.relation_extract", serde_json::Value::String(source)))
        }
        "ontology-compile" | "ontology-validate" => {
            let source = required_source(document, "onto-editor")?;
            Ok((
                "GraphAuthoring.process",
                serde_json::json!({
                    "mode": mode.replace('-', "_"),
                    "source": source,
                    "format": "turtle",
                    "context": "urn:poet:ontology-workbench",
                    "prefix": field_value(document, "onto-prefix"),
                    "namespace": field_value(document, "onto-namespace")
                }),
            ))
        }
        _ => Err(format!("Unknown authoring request `{mode}`.")),
    }
}

fn source_request(
    document: &Document,
    field: &str,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    Ok((
        "GraphAuthoring.process",
        serde_json::json!({
            "mode": mode,
            "source": required_source(document, field)?,
            "format": "turtle",
            "context": "urn:poet:rdfstar-workbench"
        }),
    ))
}

fn required_source(document: &Document, field: &str) -> Result<String, String> {
    let source = field_value(document, field);
    if source.trim().is_empty() {
        Err("Enter an RDF document before running this operation.".into())
    } else if source.len() > 256 * 1024 {
        Err("The authoring document exceeds the 256 KiB workbench limit.".into())
    } else {
        Ok(source)
    }
}
