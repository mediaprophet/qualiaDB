//! N-Quads + compact JSON emit. No Host invent; symbols stay Present-only.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::model::WorkGraph;

pub const NS: &str = "https://webizen.org/ns/work-graph#";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const NQ_NAME: &str = "impl-graph.nq";
pub const JSON_NAME: &str = "impl-graph.json";

pub fn write_emit(graph: &WorkGraph, out_dir: &Path) -> io::Result<(std::path::PathBuf, std::path::PathBuf)> {
    fs::create_dir_all(out_dir)?;
    let nq_path = out_dir.join(NQ_NAME);
    let json_path = out_dir.join(JSON_NAME);
    fs::write(&nq_path, to_nquads(graph))?;
    fs::write(&json_path, to_json(graph))?;
    Ok((nq_path, json_path))
}

pub fn to_nquads(graph: &WorkGraph) -> String {
    let mut out = String::new();
    let project = iri("project");
    triple(&mut out, &project, RDF_TYPE, &iri("Project"));
    lit(&mut out, &project, &pred("root"), &graph.project.root);
    lit(&mut out, &project, &pred("tipSha"), &graph.project.tip_sha);
    lit(&mut out, &project, &pred("branch"), &graph.project.branch);
    lit(&mut out, &project, &pred("dirty"), if graph.project.dirty { "true" } else { "false" });
    lit(
        &mut out,
        &project,
        &pred("indexedAt"),
        &graph.project.indexed_at_unix.to_string(),
    );
    lit(&mut out, &project, &pred("emitHonesty"), graph.honesty.this_emit);
    lit(&mut out, &project, &pred("customerName"), "Work graph");
    lit(&mut out, &project, &pred("allBoundSource"), &graph.sources.all_bound);
    if let Some(cat) = &graph.sources.catalog {
        lit(&mut out, &project, &pred("catalogSource"), cat);
    }

    for file in &graph.files {
        let s = file_iri(&file.path);
        triple(&mut out, &s, RDF_TYPE, &iri("File"));
        lit(&mut out, &s, &pred("path"), &file.path);
        lit(&mut out, &s, &pred("lang"), file.lang);
        triple(&mut out, &s, &pred("inProject"), &project);
    }

    for krate in &graph.crates {
        let s = crate_iri(&krate.name);
        triple(&mut out, &s, RDF_TYPE, &iri("Crate"));
        lit(&mut out, &s, &pred("name"), &krate.name);
        triple(&mut out, &s, &pred("inFile"), &file_iri(&krate.manifest));
        triple(&mut out, &s, &pred("inProject"), &project);
    }

    let catalog: std::collections::BTreeSet<&str> =
        graph.catalog_ids.iter().map(String::as_str).collect();

    for id in &graph.invoke_ids {
        let s = invoke_iri(id);
        triple(&mut out, &s, RDF_TYPE, &iri("InvokeId"));
        lit(&mut out, &s, &pred("id"), id);
        if let Some(fam) = crate::extract::family_of(id) {
            triple(&mut out, &s, &pred("inFamily"), &family_iri(fam));
        }
        triple(&mut out, &s, &pred("inFile"), &file_iri(&graph.sources.all_bound));
        if catalog.contains(id.as_str()) {
            triple(&mut out, &s, &pred("catalogMember"), &iri("vibeCatalog"));
        }
        triple(&mut out, &s, &pred("inProject"), &project);
    }

    for fam in &graph.families {
        let s = family_iri(&fam.name);
        triple(&mut out, &s, RDF_TYPE, &iri("Family"));
        lit(&mut out, &s, &pred("name"), &fam.name);
        lit(&mut out, &s, &pred("memberCount"), &fam.member_count.to_string());
        triple(&mut out, &s, &pred("inProject"), &project);
    }

    for path in &graph.doc_pages {
        let s = doc_iri(path);
        triple(&mut out, &s, RDF_TYPE, &iri("DocPage"));
        lit(&mut out, &s, &pred("path"), path);
        triple(&mut out, &s, &pred("inFile"), &file_iri(path));
        triple(&mut out, &s, &pred("inProject"), &project);
    }

    for cite in &graph.doc_cites {
        let doc = doc_iri(&cite.doc_path);
        let about = if cite.kind == "family" {
            family_iri(&cite.about)
        } else {
            invoke_iri(&cite.about)
        };
        triple(&mut out, &about, &pred("documentedBy"), &doc);
    }

    out
}

pub fn to_json(graph: &WorkGraph) -> String {
    let mut w = Vec::new();
    let _ = write_json(&mut w, graph);
    String::from_utf8(w).unwrap_or_else(|_| "{}".into())
}

fn write_json(w: &mut Vec<u8>, graph: &WorkGraph) -> io::Result<()> {
    write!(w, "{{\n")?;
    write!(w, "  \"customerName\": \"Work graph\",\n")?;
    write!(w, "  \"workingId\": \"implementation-graph\",\n")?;
    write!(w, "  \"noHostInvent\": true,\n")?;
    write!(w, "  \"honesty\": {{\n")?;
    write!(w, "    \"vocabulary\": [\"Present\", \"Live\", \"Planned\"],\n")?;
    write!(w, "    \"thisEmit\": \"Present\",\n")?;
    write!(
        w,
        "    \"note\": {}\n",
        json_str(graph.honesty.note)
    )?;
    write!(w, "  }},\n")?;
    write!(w, "  \"project\": {{\n")?;
    write!(w, "    \"root\": {},\n", json_str(&graph.project.root))?;
    write!(w, "    \"tipSha\": {},\n", json_str(&graph.project.tip_sha))?;
    write!(w, "    \"branch\": {},\n", json_str(&graph.project.branch))?;
    write!(w, "    \"dirty\": {},\n", graph.project.dirty)?;
    write!(w, "    \"indexedAtUnix\": {}\n", graph.project.indexed_at_unix)?;
    write!(w, "  }},\n")?;
    write!(w, "  \"sources\": {{\n")?;
    write!(w, "    \"allBound\": {},\n", json_str(&graph.sources.all_bound))?;
    match &graph.sources.catalog {
        Some(c) => write!(w, "    \"catalog\": {}\n", json_str(c))?,
        None => write!(w, "    \"catalog\": null\n")?,
    }
    write!(w, "  }},\n")?;
    write!(w, "  \"counts\": {{\n")?;
    write!(w, "    \"files\": {},\n", graph.files.len())?;
    write!(w, "    \"crates\": {},\n", graph.crates.len())?;
    write!(w, "    \"invokeIds\": {},\n", graph.invoke_ids.len())?;
    write!(w, "    \"catalogIds\": {},\n", graph.catalog_ids.len())?;
    write!(w, "    \"families\": {},\n", graph.families.len())?;
    write!(w, "    \"docPages\": {},\n", graph.doc_pages.len())?;
    write!(w, "    \"docCites\": {}\n", graph.doc_cites.len())?;
    write!(w, "  }},\n")?;
    write!(w, "  \"crates\": [")?;
    for (i, k) in graph.crates.iter().enumerate() {
        if i > 0 {
            write!(w, ", ")?;
        }
        write!(w, "{}", json_str(&k.name))?;
    }
    write!(w, "],\n")?;
    write!(w, "  \"families\": [\n")?;
    for (i, fam) in graph.families.iter().enumerate() {
        if i > 0 {
            write!(w, ",\n")?;
        }
        write!(
            w,
            "    {{\"name\": {}, \"memberCount\": {}}}",
            json_str(&fam.name),
            fam.member_count
        )?;
    }
    write!(w, "\n  ],\n")?;
    write!(w, "  \"invokeIds\": [\n")?;
    for (i, id) in graph.invoke_ids.iter().enumerate() {
        if i > 0 {
            write!(w, ",\n")?;
        }
        write!(w, "    {}", json_str(id))?;
    }
    write!(w, "\n  ],\n")?;
    let missing_cat = graph.bound_missing_from_catalog();
    let extra_cat = graph.catalog_only_ids();
    write!(w, "  \"boundMissingFromCatalog\": [")?;
    for (i, id) in missing_cat.iter().enumerate() {
        if i > 0 {
            write!(w, ", ")?;
        }
        write!(w, "{}", json_str(id))?;
    }
    write!(w, "],\n")?;
    write!(w, "  \"catalogOnlyIds\": [")?;
    for (i, id) in extra_cat.iter().enumerate() {
        if i > 0 {
            write!(w, ", ")?;
        }
        write!(w, "{}", json_str(id))?;
    }
    write!(w, "],\n")?;
    write!(w, "  \"docCites\": [\n")?;
    for (i, cite) in graph.doc_cites.iter().enumerate() {
        if i > 0 {
            write!(w, ",\n")?;
        }
        write!(
            w,
            "    {{\"about\": {}, \"doc\": {}, \"kind\": {}}}",
            json_str(&cite.about),
            json_str(&cite.doc_path),
            json_str(cite.kind)
        )?;
    }
    write!(w, "\n  ]\n")?;
    write!(w, "}}\n")?;
    Ok(())
}

fn iri(local: &str) -> String {
    format!("<{NS}{local}>")
}

fn pred(name: &str) -> String {
    iri(name)
}

fn file_iri(path: &str) -> String {
    iri(&format!("file/{}", encode_seg(path)))
}

fn crate_iri(name: &str) -> String {
    iri(&format!("crate/{}", encode_seg(name)))
}

fn invoke_iri(id: &str) -> String {
    iri(&format!("invoke/{}", encode_seg(id)))
}

fn family_iri(name: &str) -> String {
    iri(&format!("family/{}", encode_seg(name)))
}

fn doc_iri(path: &str) -> String {
    iri(&format!("doc/{}", encode_seg(path)))
}

fn triple(out: &mut String, s: &str, p: &str, o: &str) {
    out.push_str(s);
    out.push(' ');
    out.push_str(p);
    out.push(' ');
    out.push_str(o);
    out.push_str(" .\n");
}

fn lit(out: &mut String, s: &str, p: &str, value: &str) {
    out.push_str(s);
    out.push(' ');
    out.push_str(p);
    out.push(' ');
    out.push_str(&nq_string(value));
    out.push_str(" .\n");
}

fn nq_string(value: &str) -> String {
    let mut s = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            _ => s.push(c),
        }
    }
    s.push('"');
    s
}

fn json_str(value: &str) -> String {
    let mut s = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if c.is_control() => s.push_str(&format!("\\u{:04x}", c as u32)),
            _ => s.push(c),
        }
    }
    s.push('"');
    s
}

fn encode_seg(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CrateNode, DocCite, FamilyNode, FileNode, Honesty, Project, Sources, WorkGraph,
    };

    fn sample() -> WorkGraph {
        WorkGraph {
            honesty: Honesty::presence_index(),
            project: Project {
                root: "/repo".into(),
                tip_sha: "abc123".into(),
                branch: "0.0.38".into(),
                dirty: false,
                indexed_at_unix: 1,
            },
            files: vec![FileNode {
                path: "crates/vibe/Cargo.toml".into(),
                lang: "toml",
            }],
            crates: vec![CrateNode {
                name: "vibe".into(),
                manifest: "crates/vibe/Cargo.toml".into(),
            }],
            invoke_ids: vec!["ClinicalRisk.framingham".into()],
            catalog_ids: vec!["ClinicalRisk.framingham".into()],
            catalog_present: true,
            families: vec![FamilyNode {
                name: "ClinicalRisk".into(),
                member_count: 1,
            }],
            doc_pages: vec!["docs/vibe/SKILL.md".into()],
            doc_cites: vec![DocCite {
                about: "ClinicalRisk.framingham".into(),
                doc_path: "docs/vibe/SKILL.md".into(),
                kind: "invoke",
            }],
            sources: Sources {
                all_bound: crate::ALL_BOUND_REL.into(),
                catalog: Some(crate::CATALOG_REL.into()),
            },
        }
    }

    #[test]
    fn nquads_has_tip_invoke_and_cite() {
        let nq = to_nquads(&sample());
        assert!(nq.contains("abc123"));
        assert!(nq.contains("ClinicalRisk.framingham"));
        assert!(nq.contains("documentedBy"));
        assert!(nq.contains("emitHonesty"));
        assert!(!nq.contains("ImplGraph."));
        assert!(!nq.contains("held"));
    }

    #[test]
    fn json_is_presence_only() {
        let js = to_json(&sample());
        assert!(js.contains("\"thisEmit\": \"Present\""));
        assert!(js.contains("\"noHostInvent\": true"));
        assert!(js.contains("ClinicalRisk.framingham"));
        assert!(!js.contains("ImplGraph."));
    }
}
