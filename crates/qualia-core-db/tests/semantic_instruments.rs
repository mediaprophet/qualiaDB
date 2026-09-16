//! SI-01 contract tests for semantic-instrument ontology, SHACL and fixtures.
//!
//! Cold path: N3Parser (same engine front-end as cml.n3 / agency guards) plus a
//! closed-world field checker that mirrors the named PropertyShapes. This is
//! not the generic runner and not a package ABI.

use std::collections::{BTreeMap, BTreeSet};

use qualia_core_db::modalities::logic::n3_parser::{N3Event, N3Parser, Term};

const ONTOLOGY: &str = include_str!("../../../core-ontologies/semantic-instruments.n3");
const SHAPES: &str = include_str!("../../../core-ontologies/semantic-instrument-shapes.n3");
const PROFILE: &str =
    include_str!("../../../core-ontologies/semantic-instrument-capability-profile.n3");

const POS_SMALL: &str =
    include_str!("../../../core-ontologies/fixtures/semantic-instruments/positive-small.n3");
const POS_ICON: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/positive-icon-dependency.n3"
);
const POS_RPL: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/positive-rpl-requirement.n3"
);

const NEG_DIGEST: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-missing-content-digest.n3"
);
const NEG_HONESTY: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-missing-honesty-policy.n3"
);
const NEG_CITE: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-missing-citation-applicability.n3"
);
const NEG_ICON: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-undigested-icon.n3"
);
const NEG_COLLAPSE: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-who-tool-claim-collapse.n3"
);
const NEG_CAP_PERM: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-capability-as-permission.n3"
);
const NEG_DEP: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-unresolved-required-dependency.n3"
);
const NEG_REVOKED: &str = include_str!(
    "../../../core-ontologies/fixtures/semantic-instruments/negative-revoked-presented-as-active.n3"
);

const REQUIRED_CLASSES: &[&str] = &[
    "si:SemanticInstrument",
    "si:InstrumentRelease",
    "si:InstrumentPackage",
    "si:EntryPoint",
    "si:DependencyRequirement",
    "si:BadgePresentation",
    "si:BadgeAsset",
    "si:Agent",
    "si:CapacityGrant",
    "si:HostCapabilityGrant",
    "si:Capability",
    "si:CapabilityRequirement",
    "si:CapabilityEvidence",
    "si:CapabilityCredential",
    "si:GapReport",
    "si:ExecutionEvent",
    "si:ExecutionReceipt",
    "si:HonestyPolicy",
    "si:Applicability",
    "si:ResourceLimits",
    "si:ContentCategory",
    "si:Demo",
];

const REQUIRED_PREDICATES: &[&str] = &[
    "si:authoredBy",
    "si:contributedBy",
    "si:technicallyReviewedBy",
    "si:professionallyReviewedBy",
    "si:endorsedBy",
    "si:publishedBy",
    "si:derivedFrom",
    "si:withdrawnBy",
    "si:supersedes",
    "si:contentDigest",
    "si:honestyPolicy",
    "si:citation",
    "si:applicability",
    "si:identifiesRelease",
    "si:assessmentInstrument",
    "si:unresolvedEvidence",
    "si:HostCapabilityGrant",
    "si:contentCategory",
];

const RELEASE_REQUIRED: &[&str] = &[
    "si:instrument",
    "si:releaseId",
    "si:version",
    "si:purpose",
    "si:prohibitedInterpretation",
    "si:domain",
    "si:ontologyReference",
    "si:entryPoint",
    "si:citation",
    "si:applicability",
    "si:honestyPolicy",
    "si:resourceLimits",
    "si:testManifest",
    "si:badgePresentation",
    "si:contentDigest",
    "si:authoredBy",
    "si:licence",
    "si:contentCategory",
];

#[derive(Default)]
struct Graph {
    triples: Vec<(String, String, String)>,
    by_spo: BTreeMap<(String, String), Vec<String>>,
}

impl Graph {
    fn add(&mut self, s: String, p: String, o: String) {
        self.by_spo
            .entry((s.clone(), p.clone()))
            .or_default()
            .push(o.clone());
        self.triples.push((s, p, o));
    }

    fn typed(&self, class: &str) -> Vec<String> {
        self.triples
            .iter()
            .filter(|(_, p, o)| p == "a" && o == class)
            .map(|(s, _, _)| s.clone())
            .collect()
    }

    fn has(&self, s: &str, p: &str) -> bool {
        self.by_spo
            .get(&(s.to_string(), p.to_string()))
            .is_some_and(|v| !v.is_empty())
    }

    fn objects(&self, s: &str, p: &str) -> Vec<String> {
        self.by_spo
            .get(&(s.to_string(), p.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    fn first(&self, s: &str, p: &str) -> Option<String> {
        self.objects(s, p).into_iter().next()
    }

    fn is_true(&self, s: &str, p: &str) -> bool {
        self.objects(s, p).iter().any(|v| v == "true")
    }
}

fn term_str(term: Term<'_>) -> String {
    let raw = match term {
        Term::Uri(s) | Term::Literal(s) | Term::Variable(s) | Term::Formula(s) => s,
    };
    raw.trim_matches(|c| c == '<' || c == '>').to_string()
}

fn parse_graph(n3: &str, label: &str) -> Graph {
    let mut g = Graph::default();
    let mut parser = N3Parser::new(n3);
    parser
        .parse_all(|event| {
            if let N3Event::StaticTriple(t) = event {
                g.add(
                    term_str(t.subject),
                    term_str(t.predicate),
                    term_str(t.object),
                );
            }
            Ok(())
        })
        .unwrap_or_else(|e| panic!("{label} must parse: {e}"));
    g
}

fn parse_must_succeed(n3: &str, label: &str) {
    let mut parser = N3Parser::new(n3);
    let mut triples = 0usize;
    let mut rules = 0usize;
    parser
        .parse_all(|event| {
            match event {
                N3Event::StaticTriple(_) => triples += 1,
                N3Event::LogicRule(_) => rules += 1,
                _ => {}
            }
            Ok(())
        })
        .unwrap_or_else(|e| panic!("{label} must parse through N3Parser: {e}"));
    assert!(
        triples > 0,
        "{label} produced no static triples (parser regression)"
    );
    let _ = rules;
}

fn require_missing(g: &Graph, s: &str, p: &str, code: &str, out: &mut BTreeSet<String>) {
    if !g.has(s, p) {
        out.insert(code.to_string());
    }
}

fn violations(g: &Graph) -> BTreeSet<String> {
    let mut v = BTreeSet::new();

    for release in g.typed("si:InstrumentRelease") {
        let codes = [
            ("si:instrument", "si:MissingInstrument"),
            ("si:releaseId", "si:MissingReleaseId"),
            ("si:version", "si:MissingVersion"),
            ("si:purpose", "si:MissingPurpose"),
            (
                "si:prohibitedInterpretation",
                "si:MissingProhibitedInterpretation",
            ),
            ("si:domain", "si:MissingDomain"),
            ("si:ontologyReference", "si:MissingOntologyReference"),
            ("si:entryPoint", "si:MissingEntryPoint"),
            ("si:citation", "si:MissingCitation"),
            ("si:applicability", "si:MissingApplicability"),
            ("si:honestyPolicy", "si:MissingHonestyPolicy"),
            ("si:resourceLimits", "si:MissingResourceLimits"),
            ("si:testManifest", "si:MissingTestManifest"),
            ("si:badgePresentation", "si:MissingBadgePresentation"),
            ("si:contentDigest", "si:MissingContentDigest"),
            ("si:authoredBy", "si:MissingAuthorship"),
            ("si:licence", "si:MissingLicence"),
            ("si:contentCategory", "si:MissingContentCategory"),
        ];
        for (p, code) in codes {
            require_missing(g, &release, p, code, &mut v);
        }

        for ep in g.objects(&release, "si:entryPoint") {
            require_missing(g, &ep, "si:entryPointName", "si:MissingEntryPoint", &mut v);
            require_missing(
                g,
                &ep,
                "si:inputShape",
                "si:MissingEntryPointShapes",
                &mut v,
            );
            require_missing(
                g,
                &ep,
                "si:outputShape",
                "si:MissingEntryPointShapes",
                &mut v,
            );
        }
        for honesty in g.objects(&release, "si:honestyPolicy") {
            if !g.has(&honesty, "si:incompleteInputBehaviour")
                || !g.has(&honesty, "si:resultKind")
                || !g.has(&honesty, "si:requiredNotice")
                || !g.has(&honesty, "si:signatureIsNotTruth")
            {
                v.insert("si:MissingHonestyPolicy".into());
            }
        }
        for badge in g.objects(&release, "si:badgePresentation") {
            if !g.has(&badge, "si:identifiesRelease") || !g.has(&badge, "si:badgeAsset") {
                v.insert("si:InaccessibleOrUndigestedIcon".into());
            }
            for asset in g.objects(&badge, "si:badgeAsset") {
                if !g.has(&asset, "si:assetLocator")
                    || !g.has(&asset, "si:assetDigest")
                    || !g.has(&asset, "si:accessibleText")
                    || !g.has(&asset, "si:mediaType")
                {
                    v.insert("si:InaccessibleOrUndigestedIcon".into());
                }
            }
        }
        for dep in g.objects(&release, "si:dependency") {
            let required = g.is_true(&dep, "si:required");
            let unresolved = g.first(&dep, "si:resolutionState").as_deref()
                == Some("si:Unresolved")
                || !g.has(&dep, "si:expectedDigest");
            let active = g.first(&release, "si:activationState").as_deref() == Some("si:Active")
                || g.is_true(&release, "si:presentedAsActive");
            if required && unresolved && active {
                v.insert("si:UnresolvedRequiredDependency".into());
            }
        }
        let publication = g.first(&release, "si:publicationState").unwrap_or_default();
        let stopped = matches!(
            publication.as_str(),
            "si:Revoked" | "si:Withdrawn" | "si:Superseded"
        );
        let presented = g.first(&release, "si:activationState").as_deref() == Some("si:Active")
            || g.is_true(&release, "si:presentedAsActive");
        if stopped && presented {
            v.insert("si:RevokedReleasePresentedAsActive".into());
        }
    }

    let tool_classes = [
        "si:SemanticInstrument",
        "si:InstrumentRelease",
        "si:InstrumentPackage",
        "si:ResultClaim",
        "si:BadgePresentation",
    ];
    for person in g.typed("values:NaturalPerson") {
        for class in tool_classes {
            if g.typed(class).iter().any(|s| s == &person) {
                v.insert("si:WhoToolClaimCollapse".into());
            }
        }
        for other in g.objects(&person, "owl:sameAs") {
            for class in tool_classes {
                if g.typed(class).iter().any(|s| s == &other || s == &person) {
                    v.insert("si:WhoToolClaimCollapse".into());
                }
            }
        }
    }

    for cap in g.typed("cap:Capability") {
        if g.typed("si:HostCapabilityGrant").iter().any(|s| s == &cap)
            || g.typed("si:CapacityGrant").iter().any(|s| s == &cap)
        {
            v.insert("si:CapabilityTreatedAsRuntimePermission".into());
        }
    }

    v
}

#[test]
fn ontology_files_parse_and_declare_required_terms() {
    parse_must_succeed(ONTOLOGY, "semantic-instruments.n3");
    parse_must_succeed(SHAPES, "semantic-instrument-shapes.n3");
    parse_must_succeed(PROFILE, "semantic-instrument-capability-profile.n3");

    let vocab = format!("{ONTOLOGY}\n{SHAPES}\n{PROFILE}");
    for class in REQUIRED_CLASSES {
        assert!(
            vocab.contains(class),
            "missing required class declaration: {class}"
        );
    }
    for pred in REQUIRED_PREDICATES {
        assert!(
            vocab.contains(pred),
            "missing required predicate or class token: {pred}"
        );
    }
    for path in RELEASE_REQUIRED {
        assert!(
            SHAPES.contains(path),
            "shapes file must constrain release field {path}"
        );
    }
    assert!(
        ONTOLOGY.contains("si:signatureIsNotTruth"),
        "origin-not-truth must be a first-class honesty property"
    );
    assert!(
        PROFILE.contains("si:assessmentInstrument"),
        "capability credentials must reference the assessment instrument"
    );
    assert!(
        !vocab.contains("qualia.id/ns"),
        "namespace regression: residual qualia.id/ns"
    );
}

#[test]
fn shapes_target_instrument_release_and_named_property_shapes() {
    let g = parse_graph(SHAPES, "shapes");
    let targets = g.objects("si:InstrumentReleaseShape", "sh:targetClass");
    assert!(
        targets.iter().any(|t| t == "si:InstrumentRelease"),
        "InstrumentReleaseShape must target si:InstrumentRelease, got {targets:?}"
    );
    assert!(
        g.has("si:InstrumentRelease-contentDigest", "sh:path"),
        "content digest must be a named property shape"
    );
    assert_eq!(
        g.first("si:InstrumentRelease-contentDigest", "sh:path")
            .as_deref(),
        Some("si:contentDigest")
    );
}

fn assert_positive(n3: &str, label: &str) {
    parse_must_succeed(n3, label);
    let g = parse_graph(n3, label);
    let found = violations(&g);
    assert!(
        found.is_empty(),
        "{label} should conform; violations: {found:?}"
    );
    assert!(
        !g.typed("si:InstrumentRelease").is_empty(),
        "{label} must contain an InstrumentRelease"
    );
    assert!(
        g.typed("si:InstrumentRelease")
            .iter()
            .all(|r| g.first(r, "si:contentCategory").as_deref() == Some("si:Demo")),
        "{label} seed fixtures must be si:Demo"
    );
}

fn assert_negative(n3: &str, label: &str, expected: &[&str]) {
    parse_must_succeed(n3, label);
    let g = parse_graph(n3, label);
    let found = violations(&g);
    for code in expected {
        assert!(
            found.contains(*code),
            "{label} should fail for {code}; got {found:?}"
        );
    }
}

#[test]
fn positive_small_instrument_conforms() {
    assert_positive(POS_SMALL, "positive-small");
}

#[test]
fn positive_icon_and_ontology_dependency_conforms() {
    assert_positive(POS_ICON, "positive-icon-dependency");
    let g = parse_graph(POS_ICON, "positive-icon-dependency");
    assert!(
        !g.typed("si:DependencyRequirement").is_empty(),
        "icon/dependency fixture must declare a dependency"
    );
    assert!(
        g.typed("si:BadgeAsset")
            .iter()
            .any(|a| g.has(a, "si:assetDigest")),
        "icon must be digested"
    );
}

#[test]
fn positive_rpl_capability_requirement_conforms() {
    assert_positive(POS_RPL, "positive-rpl-requirement");
    let g = parse_graph(POS_RPL, "positive-rpl-requirement");
    assert!(!g.typed("si:CapabilityRequirement").is_empty());
    assert!(!g.typed("si:GapReport").is_empty());
    assert!(!g.typed("si:CapabilityCredential").is_empty());
    let cred = &g.typed("si:CapabilityCredential")[0];
    assert!(
        g.has(cred, "si:assessmentInstrument"),
        "capability credential must reference the assessment instrument"
    );
    assert!(
        g.typed("si:SemanticInstrument")
            .iter()
            .all(|inst| inst != cred),
        "credential IRI must not be the instrument IRI"
    );
    let report = &g.typed("si:GapReport")[0];
    assert!(
        g.has(report, "si:unresolvedEvidence"),
        "RPL fixture must show unavailable evidence as unresolved"
    );
}

#[test]
fn missing_content_digest_fails() {
    assert_negative(NEG_DIGEST, "missing-digest", &["si:MissingContentDigest"]);
}

#[test]
fn missing_honesty_policy_fails() {
    assert_negative(NEG_HONESTY, "missing-honesty", &["si:MissingHonestyPolicy"]);
}

#[test]
fn missing_citation_and_applicability_fails() {
    assert_negative(
        NEG_CITE,
        "missing-citation-applicability",
        &["si:MissingCitation", "si:MissingApplicability"],
    );
}

#[test]
fn inaccessible_or_undigested_icon_fails() {
    assert_negative(
        NEG_ICON,
        "undigested-icon",
        &["si:InaccessibleOrUndigestedIcon"],
    );
}

#[test]
fn who_tool_claim_identity_collapse_fails() {
    assert_negative(
        NEG_COLLAPSE,
        "who-tool-collapse",
        &["si:WhoToolClaimCollapse"],
    );
}

#[test]
fn capability_incorrectly_treated_as_runtime_permission_fails() {
    assert_negative(
        NEG_CAP_PERM,
        "capability-as-permission",
        &["si:CapabilityTreatedAsRuntimePermission"],
    );
}

#[test]
fn unresolved_required_dependency_fails() {
    assert_negative(
        NEG_DEP,
        "unresolved-dependency",
        &["si:UnresolvedRequiredDependency"],
    );
}

#[test]
fn revoked_release_presented_as_active_fails() {
    assert_negative(
        NEG_REVOKED,
        "revoked-active",
        &["si:RevokedReleasePresentedAsActive"],
    );
    let g = parse_graph(NEG_REVOKED, "revoked-active");
    assert!(
        !g.typed("si:ExecutionReceipt").is_empty(),
        "revocation must not erase historical execution receipts"
    );
    assert!(
        !g.typed("values:NaturalPerson").is_empty(),
        "revocation must not erase the author"
    );
}
