use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Knowledge & Semantics â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "ontology-builder",
            name: "Ontology Builder",
            tagline: "Knowledge Engineering",
            desc: "Interactive ontology workbench: define SHACL shapes, link OWL concepts, compile \
                   N3 rules to SlgOpcode bytecode, and seed finished ontologies to the WebTorrent DHT as .c.q42 artifacts.",
            icon: "node-plus",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Knowledge,
        },
        QApp {
            id: "sparql-explorer",
            name: "SPARQL Explorer",
            tagline: "Graph Queries",
            desc: "Federated SPARQL console over the local Qualia daemon (port 4242). Supports AS OF \
                   temporal queries, GeoSPARQL spatial filters, SHACL validation, and multi-modal results.",
            icon: "code-slash",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Knowledge,
        },
        QApp {
            id: "n3-logic-studio",
            name: "N3 Logic Studio",
            tagline: "Rule Engineering",
            desc: "Author and test N3Logic rules that feed the Webizen VM. Compile to SlgOpcode \
                   bytecode, run against the 42 MB SLG Arena, and inspect derivation traces step-by-step.",
            icon: "braces",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Knowledge,
        },
        QApp {
            id: "rdf-star-editor",
            name: "RDF-Star Editor",
            tagline: "Triple Annotations",
            desc: "Edit and query RDF-Star nested triples. Maps annotations to NQuin provenance bits; \
                   supports PROV-O, DC Terms, and ODRL metadata inline on any statement.",
            icon: "diagram-2",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Knowledge,
        },
        QApp {
            id: "solid-browser",
            name: "Solid LDP Browser",
            tagline: "Solid Protocol",
            desc: "Browse Solid Pods via the local LDP proxy. Read/write Turtle and JSON-LD resources, \
                   manage WebACL permissions, and import pod data into the Q42 semantic graph.",
            icon: "folder-symlink",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Knowledge,
        },
    ]
}
