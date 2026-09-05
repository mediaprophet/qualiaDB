//! Facet catalogs and common SPARQL predicates for the search workbench.

pub(super) const ONTOLOGY_PREFIXES: &[(&str, &str)] = &[
    ("ont", "Ontology (ont:)"),
    ("hm", "Hypermedia (hm:)"),
    ("doc", "Document (doc:)"),
    ("prov", "Provenance (prov:)"),
    ("agency", "Agency (agency:)"),
    ("inv", "Investigation (inv:)"),
    ("set", "Settings (set:)"),
    ("soc", "Social (soc:)"),
    ("comm", "Communications (comm:)"),
    ("rights", "Rights (rights:)"),
    ("coop", "Cooperation (coop:)"),
    ("vibe", "VibeScript (vibe:)"),
    ("sanctuary", "Sanctuary (sanctuary:)"),
    ("epi", "Epistemics (epi:)"),
    ("selfhood", "Selfhood (selfhood:)"),
    ("care", "Care (care:)"),
    ("values", "Values (values:)"),
];

pub(super) const ENTITY_TYPES: &[(&str, &str)] = &[
    ("term", "Term"),
    ("entity", "Named Entity"),
    ("claimedFact", "Claimed Fact"),
    ("statement", "Statement"),
    ("statistic", "Statistic"),
    ("citation", "Citation"),
    ("definition", "Definition"),
    ("quote", "Quote"),
];

pub(super) const EPISTEMIC_MODALITIES: &[(&str, &str)] = &[
    ("objective", "Objective"),
    ("subjective", "Subjective"),
    ("intersubjective", "Intersubjective"),
    ("normative", "Normative"),
];

pub(super) const STRATA: &[(&str, &str)] = &[
    ("environmental", "Environmental"),
    ("social", "Social"),
    ("legal", "Legal"),
    ("financial", "Financial"),
    ("technical", "Technical"),
];

pub(super) const HONESTY_LEVELS: &[(&str, &str)] = &[
    ("live", "Live"),
    ("present", "Present"),
    ("partial", "Partial"),
    ("missing", "Missing"),
];

pub(super) const CONTAINER_TYPES: &[(&str, &str)] = &[
    ("doc", "Document"),
    ("sheet", "Sheet"),
    ("code", "Code"),
    ("map", "Map"),
    ("ontology", "Ontology"),
    ("social", "Social"),
    ("graph", "Graph"),
    ("media", "Media"),
    ("3d", "3D"),
    ("webrtc", "WebRTC"),
    ("webview", "WebView"),
    ("vision", "Vision"),
    ("listen", "Listen"),
    ("triad", "Triad"),
    ("library", "Library"),
    ("latex", "LaTeX"),
    ("slide", "Slides"),
    ("finance", "Finance"),
    ("wallet", "Wallet"),
    ("rights", "Rights"),
    ("pulse", "Pulse"),
    ("aura", "Aura"),
];

// ---------------------------------------------------------------------------
// Common predicates for the visual query builder
// ---------------------------------------------------------------------------

pub(super) const COMMON_PREDICATES: &[(&str, &str)] = &[
    ("rdf:type", "rdf:type"),
    ("rdfs:label", "rdfs:label"),
    ("rdfs:comment", "rdfs:comment"),
    ("ont:hasEntity", "ont:hasEntity"),
    ("ont:hasTerm", "ont:hasTerm"),
    ("doc:hasMarkup", "doc:hasMarkup"),
    ("doc:markupType", "doc:markupType"),
    ("doc:appendScope", "doc:appendScope"),
    ("prov:actor", "prov:actor"),
    ("prov:timestamp", "prov:timestamp"),
    ("prov:role", "prov:role"),
    ("prov:contributedTo", "prov:contributedTo"),
    ("prov:derivedFrom", "prov:derivedFrom"),
    ("agency:actor", "agency:actor"),
    ("agency:did", "agency:did"),
    ("inv:hasHypothesis", "inv:hasHypothesis"),
    ("inv:confidence", "inv:confidence"),
    ("epi:modality", "epi:modality"),
    ("selfhood:access", "selfhood:access"),
    ("set:capability", "set:capability"),
];
