//! Turtle catalog of live 0.1 bindings + capability.invoke ids.
//! Agents query this; they must not invent names absent from it.

use super::catalog::VIBE_0_1;
use super::invoke::ids::{self, ALL_BOUND};

const PREAMBLE: &str = r#"@prefix vibe: <https://qualiadb.org/schema/vibe#> .
@prefix pulse: <https://qualiadb.org/schema/pulse#> .
@prefix aura: <https://qualiadb.org/schema/aura#> .
@prefix poet: <https://qualiadb.org/schema/poet#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# Generated from VIBE_0_1 + ids::ALL_BOUND. Not a second language spec.
# Language: vibe-0.1. Quin overlay literals are illegal.
"#;

pub fn vibe_catalog_ttl() -> String {
    let mut out = String::from(PREAMBLE);
    for b in VIBE_0_1 {
        out.push_str(&format!(
            "\nvibe:{iri} a vibe:BuiltinFunction ;\n    rdfs:label \"{id}\" ;\n    vibe:namespace \"{ns}\" ;\n    vibe:required {req} ;\n    vibe:honesty \"{hon}\" ;\n    vibe:seam \"binding\" .\n",
            iri = safe_local(b.id),
            id = b.id,
            ns = ns_of(b.id),
            req = if b.required { "true" } else { "false" },
            hon = b.honesty,
        ));
    }
    for id in ALL_BOUND {
        out.push_str(&format!(
            "\nvibe:{iri} a vibe:InvokeId ;\n    rdfs:label \"{id}\" ;\n    vibe:seam \"{seam}\" ;\n    vibe:via \"capability.invoke\" .\n",
            iri = safe_local(id),
            id = id,
            seam = ids::seam_for(id),
        ));
    }
    out
}

fn ns_of(id: &str) -> &str {
    id.split('.').next().unwrap_or(id)
}

fn safe_local(id: &str) -> String {
    id.replace('.', "_").replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_teaches_legal_names_only() {
        let ttl = vibe_catalog_ttl();
        assert!(ttl.contains("pulse.publish"));
        assert!(ttl.contains("aura.validate"));
        assert!(ttl.contains("quin.statement"));
        assert!(ttl.contains("CapabilityDiscovery.list"));
        assert!(ttl.contains("GraphDatabase.sparql"));
        assert!(!ttl.contains("pulse.broadcast"));
        assert!(!ttl.contains("apply_schema"));
    }
}
