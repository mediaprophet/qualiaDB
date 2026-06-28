//! Comprehensive graph-level SHACL validation.
//!
//! [`ShaclCompiler`](super::shacl_compiler) lowers a shape to `SlgOpcode`s for the
//! firewall (LLM-rule gating). This module is the complementary **data validator**:
//! it interprets the full [`ShaclConstraint`](super::shacl_types::ShaclConstraint)
//! vocabulary against a graph of [`NQuin`]s and produces a real
//! [`ValidationReport`] — the path a "validate this data against this shape"
//! request (e.g. the docs playground, ingestion-time conformance) actually takes.
//!
//! ## Values are hashes — string constraints need a resolver
//!
//! Quin object fields hold either an **inline-typed literal** (numeric/boolean,
//! decodable here via [`frame_layout`]) or the **`q_hash` of a lexical value**
//! (IRIs and strings — one-way). Numeric/cardinality/value-type/structural
//! constraints are therefore fully enforceable on the bare graph. String-shaped
//! constraints (`sh:pattern`, `sh:minLength`/`maxLength`, `sh:languageIn`,
//! `sh:uniqueLang`) need the original lexical form, so the caller supplies a
//! `resolve: hash -> Option<String>`. When a value cannot be resolved, a
//! string-shaped constraint reports a `Violation` (the value cannot be shown to
//! conform) rather than silently passing — fail closed.
//!
//! Cold path: `ValidationReport` is `Vec`-backed (allocation is expected here, as
//! in the existing `sparql_shacl` validator); the zero-heap invariant covers the
//! hot path, not shape validation.

use super::shacl_types::{
    CompiledShape, NodeKindType, PropertyPath, ShaclConstraint, ShaclSeverity, ValidationReport,
    ValidationResult,
};
use crate::frame_layout::{
    object_tag, unpack_float_object, INLINE_TAG_BOOLEAN, INLINE_TAG_DECIMAL, INLINE_TAG_FLOAT,
    INLINE_TAG_INTEGER, INLINE_VALUE_MASK, MSB_FLAG,
};
use crate::{q_hash, NQuin};

/// A lexical-value resolver: maps a value hash back to its original string when
/// available (used only by string-shaped constraints).
pub type Resolver<'r> = &'r dyn Fn(u64) -> Option<String>;

/// Decode an inline-typed object field to an `f64`, or `None` if the object is a
/// pointer / IRI hash / un-typed (i.e. not a comparable number).
pub fn object_as_f64(object: u64) -> Option<f64> {
    if object & MSB_FLAG != 0 {
        return None; // topological pointer, not a literal
    }
    match object_tag(object) {
        INLINE_TAG_FLOAT => Some(unpack_float_object(object) as f64),
        INLINE_TAG_INTEGER => {
            let mut n = (object & INLINE_VALUE_MASK) as i64;
            if n & (1i64 << 59) != 0 {
                n |= !((1i64 << 60) - 1); // sign-extend 60-bit two's complement
            }
            Some(n as f64)
        }
        INLINE_TAG_DECIMAL => {
            let mut raw = (object & INLINE_VALUE_MASK) as i64;
            if raw & (1i64 << 59) != 0 {
                raw |= !((1i64 << 60) - 1);
            }
            Some(raw as f64 / 1_000_000.0) // fixed-point ×10⁶
        }
        INLINE_TAG_BOOLEAN => Some(if object & 1 != 0 { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// The SHACL node kind of an object value (0=BlankNode, 1=IRI, 2=Literal).
fn node_kind_of(object: u64) -> u8 {
    if object & MSB_FLAG != 0 {
        1 // did:q42 topological pointer — an IRI
    } else if object_tag(object) != 0 {
        2 // inline-typed literal
    } else {
        1 // plain IRI hash
    }
}

fn node_kind_matches(object: u64, want: NodeKindType) -> bool {
    let k = node_kind_of(object); // 0=blank,1=iri,2=literal
    match want {
        NodeKindType::BlankNode => k == 0,
        NodeKindType::Iri => k == 1,
        NodeKindType::Literal => k == 2,
        NodeKindType::BlankNodeOrIri => k == 0 || k == 1,
        NodeKindType::BlankNodeOrLiteral => k == 0 || k == 2,
        NodeKindType::IriOrLiteral => k == 1 || k == 2,
    }
}

/// `sh:datatype` IRI → inline tag.
fn datatype_tag(dt: &str) -> Option<u64> {
    let h = q_hash(dt);
    if h == q_hash("xsd:string") {
        Some(0)
    } else if h == q_hash("xsd:integer") {
        Some(INLINE_TAG_INTEGER)
    } else if h == q_hash("xsd:decimal") {
        Some(INLINE_TAG_DECIMAL)
    } else if h == q_hash("xsd:float") || h == q_hash("xsd:double") {
        Some(INLINE_TAG_FLOAT)
    } else if h == q_hash("xsd:boolean") {
        Some(INLINE_TAG_BOOLEAN)
    } else {
        None
    }
}

/// A readable label for a node: the resolved lexical value, else the decoded
/// inline numeric/boolean value, else a hash tag.
fn label(node: u64, resolve: Resolver) -> String {
    if let Some(s) = resolve(node) {
        return s;
    }
    if node & MSB_FLAG == 0 {
        match object_tag(node) {
            INLINE_TAG_BOOLEAN => return (node & 1 != 0).to_string(),
            INLINE_TAG_FLOAT | INLINE_TAG_INTEGER | INLINE_TAG_DECIMAL => {
                if let Some(n) = object_as_f64(node) {
                    return if n.fract() == 0.0 && n.abs() < 1e15 {
                        (n as i64).to_string()
                    } else {
                        n.to_string()
                    };
                }
            }
            _ => {}
        }
    }
    format!("node:{node:016x}")
}

const RDF_TYPE_KEYS: [&str; 2] = ["rdf:type", "a"];

/// Engine binding shapes + graph for a validation pass.
pub struct ShaclEngine<'a> {
    pub quins: &'a [NQuin],
    /// Registry used to resolve `sh:node` / `and` / `or` / `not` / `xone` references
    /// (referenced shapes are looked up by their `shape_class`).
    pub shapes: &'a [CompiledShape],
}

impl<'a> ShaclEngine<'a> {
    pub fn new(quins: &'a [NQuin], shapes: &'a [CompiledShape]) -> Self {
        Self { quins, shapes }
    }

    /// Validate every shape against its target nodes.
    pub fn validate(&self, resolve: Resolver) -> ValidationReport {
        let mut results = Vec::new();
        for shape in self.shapes {
            for focus in self.target_nodes(shape) {
                self.validate_focus(focus, shape, resolve, &mut results);
            }
        }
        ValidationReport {
            conforms: !results
                .iter()
                .any(|r| r.severity == ShaclSeverity::Violation),
            results,
        }
    }

    /// Validate a single focus node against one shape.
    pub fn validate_focus(
        &self,
        focus: u64,
        shape: &CompiledShape,
        resolve: Resolver,
        out: &mut Vec<ValidationResult>,
    ) {
        let path_set = !shape.property_path.is_empty();
        let values: Vec<u64> = if path_set {
            self.values_at(focus, &shape.property_path)
        } else {
            vec![focus]
        };
        let path = if path_set {
            Some(shape.property_path.clone())
        } else {
            None
        };
        for c in &shape.constraints {
            self.check(focus, &path, &values, c, shape.severity, resolve, out);
        }
    }

    /// Objects of `(focus, <path>)`.
    fn values_at(&self, focus: u64, path: &str) -> Vec<u64> {
        let p = q_hash(path);
        self.quins
            .iter()
            .filter(|q| q.subject == focus && q.predicate == p)
            .map(|q| q.object)
            .collect()
    }

    /// Whether `node` is an `rdf:type` instance of `class_hash`.
    fn is_a(&self, node: u64, class_hash: u64) -> bool {
        let type_keys = RDF_TYPE_KEYS.map(q_hash);
        self.quins.iter().any(|q| {
            q.subject == node && type_keys.contains(&q.predicate) && q.object == class_hash
        })
    }

    /// Look up a referenced shape by its `shape_class` name.
    fn shape_named(&self, name: &str) -> Option<&CompiledShape> {
        let h = q_hash(name);
        self.shapes.iter().find(|s| q_hash(&s.shape_class) == h)
    }

    /// `true` if `focus` conforms to the named shape (no Violations).
    fn focus_conforms(&self, focus: u64, name: &str, resolve: Resolver) -> bool {
        match self.shape_named(name) {
            Some(s) => {
                let mut tmp = Vec::new();
                self.validate_focus(focus, s, resolve, &mut tmp);
                !tmp.iter().any(|r| r.severity == ShaclSeverity::Violation)
            }
            // An unresolved shape reference cannot be shown to hold → fail closed.
            None => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check(
        &self,
        focus: u64,
        path: &Option<String>,
        values: &[u64],
        c: &ShaclConstraint,
        severity: ShaclSeverity,
        resolve: Resolver,
        out: &mut Vec<ValidationResult>,
    ) {
        let mut violate = |component: &str, value: Option<u64>, msg: String| {
            out.push(ValidationResult {
                severity,
                focus_node: label(focus, resolve),
                result_path: path.clone(),
                message: Some(msg),
                source_constraint: None,
                source_constraint_component: Some(component.to_string()),
                value: value.map(|v| label(v, resolve)),
            });
        };

        match c {
            // ── value range (numeric) ──────────────────────────────────────────
            ShaclConstraint::MinInclusive(m) => {
                for &v in values {
                    match object_as_f64(v) {
                        Some(x) if x >= *m => {}
                        _ => violate(
                            "sh:MinInclusiveConstraintComponent",
                            Some(v),
                            format!("value < minInclusive {m} (or not numeric)"),
                        ),
                    }
                }
            }
            ShaclConstraint::MaxInclusive(m) => {
                for &v in values {
                    match object_as_f64(v) {
                        Some(x) if x <= *m => {}
                        _ => violate(
                            "sh:MaxInclusiveConstraintComponent",
                            Some(v),
                            format!("value > maxInclusive {m} (or not numeric)"),
                        ),
                    }
                }
            }
            ShaclConstraint::MinExclusive(m) => {
                for &v in values {
                    match object_as_f64(v) {
                        Some(x) if x > *m => {}
                        _ => violate(
                            "sh:MinExclusiveConstraintComponent",
                            Some(v),
                            format!("value <= minExclusive {m} (or not numeric)"),
                        ),
                    }
                }
            }
            ShaclConstraint::MaxExclusive(m) => {
                for &v in values {
                    match object_as_f64(v) {
                        Some(x) if x < *m => {}
                        _ => violate(
                            "sh:MaxExclusiveConstraintComponent",
                            Some(v),
                            format!("value >= maxExclusive {m} (or not numeric)"),
                        ),
                    }
                }
            }
            ShaclConstraint::DatatypeRange {
                min_inclusive,
                max_inclusive,
                min_exclusive,
                max_exclusive,
            } => {
                for &v in values {
                    let x = object_as_f64(v);
                    let ok = match x {
                        Some(x) => {
                            min_inclusive.map_or(true, |b| x >= b)
                                && max_inclusive.map_or(true, |b| x <= b)
                                && min_exclusive.map_or(true, |b| x > b)
                                && max_exclusive.map_or(true, |b| x < b)
                        }
                        None => false,
                    };
                    if !ok {
                        violate(
                            "sh:DatatypeRange",
                            Some(v),
                            "value outside datatype range".into(),
                        );
                    }
                }
            }

            // ── cardinality ────────────────────────────────────────────────────
            ShaclConstraint::MinCount(n) => {
                if (values.len() as u32) < *n {
                    violate(
                        "sh:MinCountConstraintComponent",
                        None,
                        format!("{} value(s) < minCount {n}", values.len()),
                    );
                }
            }
            ShaclConstraint::MaxCount(n) => {
                if (values.len() as u32) > *n {
                    violate(
                        "sh:MaxCountConstraintComponent",
                        None,
                        format!("{} value(s) > maxCount {n}", values.len()),
                    );
                }
            }

            // ── value type ─────────────────────────────────────────────────────
            ShaclConstraint::Class(class) => {
                let ch = q_hash(class);
                for &v in values {
                    if !self.is_a(v, ch) {
                        violate(
                            "sh:ClassConstraintComponent",
                            Some(v),
                            format!("value is not an instance of {class}"),
                        );
                    }
                }
            }
            ShaclConstraint::DataType(dt) => {
                if let Some(tag) = datatype_tag(dt) {
                    for &v in values {
                        let ok = if v & MSB_FLAG != 0 {
                            false
                        } else if tag == 0 {
                            object_tag(v) == 0 // xsd:string == untyped hash
                        } else {
                            object_tag(v) == tag
                        };
                        if !ok {
                            violate(
                                "sh:DatatypeConstraintComponent",
                                Some(v),
                                format!("value is not a {dt}"),
                            );
                        }
                    }
                }
            }
            ShaclConstraint::NodeKind(kind_str) => {
                let want = parse_node_kind(kind_str);
                if let Some(want) = want {
                    for &v in values {
                        if !node_kind_matches(v, want) {
                            violate(
                                "sh:NodeKindConstraintComponent",
                                Some(v),
                                format!("value is not nodeKind {kind_str}"),
                            );
                        }
                    }
                }
            }
            ShaclConstraint::NodeKindStrict(kind) => {
                for &v in values {
                    if !node_kind_matches(v, *kind) {
                        violate(
                            "sh:NodeKindConstraintComponent",
                            Some(v),
                            format!("value is not nodeKind {kind:?}"),
                        );
                    }
                }
            }

            // ── value (membership / exact) ───────────────────────────────────────
            ShaclConstraint::In(allowed) => {
                let set: Vec<u64> = allowed.iter().map(|s| q_hash(s)).collect();
                for &v in values {
                    if !set.contains(&v) {
                        violate(
                            "sh:InConstraintComponent",
                            Some(v),
                            "value not in the allowed set".into(),
                        );
                    }
                }
            }
            ShaclConstraint::HasValue(expected) => {
                let e = q_hash(expected);
                if !values.contains(&e) {
                    violate(
                        "sh:HasValueConstraintComponent",
                        None,
                        format!("required value {expected} absent"),
                    );
                }
            }

            // ── string-shaped (need the resolver) ────────────────────────────────
            ShaclConstraint::MinLength(n) => {
                for &v in values {
                    match resolve(v) {
                        Some(s) if s.chars().count() as u32 >= *n => {}
                        _ => violate(
                            "sh:MinLengthConstraintComponent",
                            Some(v),
                            format!("length < minLength {n} (or unresolvable)"),
                        ),
                    }
                }
            }
            ShaclConstraint::MaxLength(n) => {
                for &v in values {
                    match resolve(v) {
                        Some(s) if s.chars().count() as u32 <= *n => {}
                        _ => violate(
                            "sh:MaxLengthConstraintComponent",
                            Some(v),
                            format!("length > maxLength {n} (or unresolvable)"),
                        ),
                    }
                }
            }
            ShaclConstraint::Pattern(pat) => match regex::Regex::new(pat) {
                Ok(re) => {
                    for &v in values {
                        match resolve(v) {
                            Some(s) if re.is_match(&s) => {}
                            _ => violate(
                                "sh:PatternConstraintComponent",
                                Some(v),
                                format!("value does not match /{pat}/ (or unresolvable)"),
                            ),
                        }
                    }
                }
                Err(_) => violate(
                    "sh:PatternConstraintComponent",
                    None,
                    format!("invalid regex pattern /{pat}/"),
                ),
            },
            ShaclConstraint::LanguageIn(langs) => {
                for &v in values {
                    let ok = resolve(v)
                        .map(|s| langs.iter().any(|l| lang_tag_matches(&s, l)))
                        .unwrap_or(false);
                    if !ok {
                        violate(
                            "sh:LanguageInConstraintComponent",
                            Some(v),
                            "value language tag not in languageIn (or unresolvable)".into(),
                        );
                    }
                }
            }
            ShaclConstraint::UniqueLang => {
                let mut seen: Vec<String> = Vec::new();
                for &v in values {
                    if let Some(tag) = resolve(v).and_then(|s| lang_tag_of(&s)) {
                        if seen.contains(&tag) {
                            violate(
                                "sh:UniqueLangConstraintComponent",
                                Some(v),
                                format!("duplicate language tag @{tag}"),
                            );
                        } else {
                            seen.push(tag);
                        }
                    }
                }
            }

            // ── property-pair comparison ─────────────────────────────────────────
            ShaclConstraint::Equals(other) => {
                let theirs = self.values_at(focus, other);
                if !same_set(values, &theirs) {
                    violate(
                        "sh:EqualsConstraintComponent",
                        None,
                        format!("value set != values of {other}"),
                    );
                }
            }
            ShaclConstraint::LessThan(other) => {
                self.compare_pair(
                    focus,
                    values,
                    other,
                    severity,
                    path,
                    resolve,
                    out,
                    "sh:LessThanConstraintComponent",
                    |a, b| a < b,
                );
            }
            ShaclConstraint::LessThanOrEquals(other) => {
                self.compare_pair(
                    focus,
                    values,
                    other,
                    severity,
                    path,
                    resolve,
                    out,
                    "sh:LessThanOrEqualsConstraintComponent",
                    |a, b| a <= b,
                );
            }
            ShaclConstraint::GreaterThan(other) => {
                self.compare_pair(
                    focus,
                    values,
                    other,
                    severity,
                    path,
                    resolve,
                    out,
                    "sh:GreaterThanConstraintComponent",
                    |a, b| a > b,
                );
            }
            ShaclConstraint::GreaterThanOrEquals(other) => {
                self.compare_pair(
                    focus,
                    values,
                    other,
                    severity,
                    path,
                    resolve,
                    out,
                    "sh:GreaterThanOrEqualsConstraintComponent",
                    |a, b| a >= b,
                );
            }

            // ── shape-based / logical ────────────────────────────────────────────
            ShaclConstraint::Node(shape) => {
                for &v in values {
                    if !self.focus_conforms(v, shape, resolve) {
                        violate(
                            "sh:NodeConstraintComponent",
                            Some(v),
                            format!("value does not conform to shape {shape}"),
                        );
                    }
                }
            }
            ShaclConstraint::And(shapes) => {
                for &v in values {
                    if !shapes.iter().all(|s| self.focus_conforms(v, s, resolve)) {
                        violate(
                            "sh:AndConstraintComponent",
                            Some(v),
                            "value fails one or more sh:and shapes".into(),
                        );
                    }
                }
            }
            ShaclConstraint::Or(shapes) => {
                for &v in values {
                    if !shapes.iter().any(|s| self.focus_conforms(v, s, resolve)) {
                        violate(
                            "sh:OrConstraintComponent",
                            Some(v),
                            "value conforms to none of the sh:or shapes".into(),
                        );
                    }
                }
            }
            ShaclConstraint::Not(shape) => {
                for &v in values {
                    if self.focus_conforms(v, shape, resolve) {
                        violate(
                            "sh:NotConstraintComponent",
                            Some(v),
                            format!("value conforms to negated shape {shape}"),
                        );
                    }
                }
            }
            ShaclConstraint::Xone(shapes) => {
                for &v in values {
                    let n = shapes
                        .iter()
                        .filter(|s| self.focus_conforms(v, s, resolve))
                        .count();
                    if n != 1 {
                        violate(
                            "sh:XoneConstraintComponent",
                            Some(v),
                            format!("value conforms to {n} sh:xone shapes (need exactly 1)"),
                        );
                    }
                }
            }
            ShaclConstraint::Closed { ignored_properties } => {
                let mut allowed: Vec<u64> = ignored_properties.iter().map(|s| q_hash(s)).collect();
                if let Some(p) = path {
                    allowed.push(q_hash(p));
                }
                let type_keys = RDF_TYPE_KEYS.map(q_hash);
                for q in self.quins.iter().filter(|q| q.subject == focus) {
                    if !allowed.contains(&q.predicate) && !type_keys.contains(&q.predicate) {
                        violate(
                            "sh:ClosedConstraintComponent",
                            Some(q.object),
                            format!("closed shape: unexpected predicate {:016x}", q.predicate),
                        );
                    }
                }
            }

            // ── property-path wrapper / qualifier ────────────────────────────────
            ShaclConstraint::PropertyPath {
                path: pp,
                constraint,
            } => {
                let pvals = self.values_for_path(focus, pp);
                let pstr = property_path_label(pp);
                self.check(
                    focus,
                    &Some(pstr),
                    &pvals,
                    constraint,
                    severity,
                    resolve,
                    out,
                );
            }
            ShaclConstraint::QualifierValue { path: pp, value } => {
                let pvals = self.values_for_path(focus, pp);
                if !pvals.contains(&q_hash(value)) {
                    violate(
                        "sh:QualifiedValueShapeConstraintComponent",
                        None,
                        format!("qualified value {value} absent at path"),
                    );
                }
            }

            // ── Qualia-native modality constraints ───────────────────────────────
            // These two have a direct data semantics over the value quins'
            // confidence/truth degree (frame_layout::truth_degree in metadata).
            ShaclConstraint::EpistemicConstraint {
                certainty_threshold,
            } => {
                self.check_truth_degree(
                    focus,
                    path,
                    *certainty_threshold,
                    severity,
                    resolve,
                    out,
                    "q42:EpistemicConstraintComponent",
                );
            }
            ShaclConstraint::ProbabilisticConstraint {
                confidence_threshold,
            } => {
                self.check_truth_degree(
                    focus,
                    path,
                    *confidence_threshold,
                    severity,
                    resolve,
                    out,
                    "q42:ProbabilisticConstraintComponent",
                );
            }
            // The remaining native constraints are modality *computations* (deontic
            // evaluation, LTL traces, ASP stable models, calculus, graph, diffusion,
            // linear-resource, control feedback, interval/Allen, paraconsistent
            // isolation, argumentation, dialectical synthesis). They are enforced by
            // their modality engine through the compiled opcode path
            // (`shacl_compiler` → `webizen` VM), not by per-value data validation, so
            // there is nothing for the data validator to check here.
            ShaclConstraint::DeonticPolicy { .. }
            | ShaclConstraint::LtlConstraint { .. }
            | ShaclConstraint::ParaconsistentConstraint { .. }
            | ShaclConstraint::CalculusConstraint { .. }
            | ShaclConstraint::GraphConstraint { .. }
            | ShaclConstraint::ArgumentationConstraint { .. }
            | ShaclConstraint::DialecticalConstraint { .. }
            | ShaclConstraint::AspConstraint { .. }
            | ShaclConstraint::DiffusionConstraint { .. }
            | ShaclConstraint::LinearLogicConstraint { .. }
            | ShaclConstraint::ControlFeedbackConstraint { .. }
            | ShaclConstraint::IntervalArithmeticConstraint { .. } => {}
        }
    }

    /// Values reachable from `focus` along a `PropertyPath` expression.
    fn values_for_path(&self, focus: u64, path: &PropertyPath) -> Vec<u64> {
        match path {
            PropertyPath::Predicate(p) => self.values_at(focus, p),
            PropertyPath::Inverse(inner) => {
                if let PropertyPath::Predicate(p) = inner.as_ref() {
                    let ph = q_hash(p);
                    self.quins
                        .iter()
                        .filter(|q| q.object == focus && q.predicate == ph)
                        .map(|q| q.subject)
                        .collect()
                } else {
                    Vec::new()
                }
            }
            PropertyPath::Sequence(steps) => {
                let mut frontier = vec![focus];
                for step in steps {
                    let mut next = Vec::new();
                    for f in frontier {
                        next.extend(self.values_for_path(f, step));
                    }
                    frontier = next;
                }
                frontier
            }
            PropertyPath::Alternative(alts) => {
                let mut out = Vec::new();
                for a in alts {
                    out.extend(self.values_for_path(focus, a));
                }
                out
            }
            PropertyPath::ZeroOrMore(inner) => {
                let mut seen = vec![focus];
                let mut frontier = vec![focus];
                while let Some(f) = frontier.pop() {
                    for v in self.values_for_path(f, inner) {
                        if !seen.contains(&v) {
                            seen.push(v);
                            frontier.push(v);
                        }
                    }
                }
                seen
            }
            PropertyPath::OneOrMore(inner) => {
                let mut seen = Vec::new();
                let mut frontier = self.values_for_path(focus, inner);
                while let Some(f) = frontier.pop() {
                    if !seen.contains(&f) {
                        seen.push(f);
                        frontier.extend(self.values_for_path(f, inner));
                    }
                }
                seen
            }
            PropertyPath::ZeroOrOne(inner) => {
                let mut out = vec![focus];
                out.extend(self.values_for_path(focus, inner));
                out
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compare_pair(
        &self,
        focus: u64,
        values: &[u64],
        other: &str,
        severity: ShaclSeverity,
        path: &Option<String>,
        resolve: Resolver,
        out: &mut Vec<ValidationResult>,
        component: &str,
        cmp: fn(f64, f64) -> bool,
    ) {
        let theirs = self.values_at(focus, other);
        for &a in values {
            for &b in &theirs {
                let ok = match (object_as_f64(a), object_as_f64(b)) {
                    (Some(x), Some(y)) => cmp(x, y),
                    _ => false,
                };
                if !ok {
                    out.push(ValidationResult {
                        severity,
                        focus_node: label(focus, resolve),
                        result_path: path.clone(),
                        message: Some(format!("comparison vs {other} failed")),
                        source_constraint: None,
                        source_constraint_component: Some(component.to_string()),
                        value: Some(label(a, resolve)),
                    });
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_truth_degree(
        &self,
        focus: u64,
        path: &Option<String>,
        threshold: f32,
        severity: ShaclSeverity,
        resolve: Resolver,
        out: &mut Vec<ValidationResult>,
        component: &str,
    ) {
        let p = path.as_ref().map(|s| q_hash(s));
        for q in self.quins.iter().filter(|q| q.subject == focus) {
            if let Some(ph) = p {
                if q.predicate != ph {
                    continue;
                }
            }
            if crate::frame_layout::truth_degree(q.metadata) < threshold {
                out.push(ValidationResult {
                    severity,
                    focus_node: label(focus, resolve),
                    result_path: path.clone(),
                    message: Some(format!("truth/confidence below {threshold}")),
                    source_constraint: None,
                    source_constraint_component: Some(component.to_string()),
                    value: Some(label(q.object, resolve)),
                });
            }
        }
    }

    /// Target nodes for a shape (class-based: `shape_class` is treated as the
    /// `sh:targetClass`; falls back to "every subject" when no instances exist so
    /// node shapes still validate explicitly-supplied focus nodes).
    fn target_nodes(&self, shape: &CompiledShape) -> Vec<u64> {
        let class = q_hash(&shape.shape_class);
        let type_keys = RDF_TYPE_KEYS.map(q_hash);
        let mut nodes: Vec<u64> = self
            .quins
            .iter()
            .filter(|q| type_keys.contains(&q.predicate) && q.object == class)
            .map(|q| q.subject)
            .collect();
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }
}

fn parse_node_kind(s: &str) -> Option<NodeKindType> {
    match s.trim_start_matches("sh:") {
        "BlankNode" => Some(NodeKindType::BlankNode),
        "IRI" => Some(NodeKindType::Iri),
        "Literal" => Some(NodeKindType::Literal),
        "BlankNodeOrIRI" => Some(NodeKindType::BlankNodeOrIri),
        "BlankNodeOrLiteral" => Some(NodeKindType::BlankNodeOrLiteral),
        "IRIOrLiteral" => Some(NodeKindType::IriOrLiteral),
        _ => None,
    }
}

fn property_path_label(p: &PropertyPath) -> String {
    match p {
        PropertyPath::Predicate(s) => s.clone(),
        PropertyPath::Inverse(i) => format!("^{}", property_path_label(i)),
        PropertyPath::Sequence(s) => s
            .iter()
            .map(property_path_label)
            .collect::<Vec<_>>()
            .join("/"),
        PropertyPath::Alternative(s) => s
            .iter()
            .map(property_path_label)
            .collect::<Vec<_>>()
            .join("|"),
        PropertyPath::ZeroOrMore(i) => format!("{}*", property_path_label(i)),
        PropertyPath::OneOrMore(i) => format!("{}+", property_path_label(i)),
        PropertyPath::ZeroOrOne(i) => format!("{}?", property_path_label(i)),
    }
}

fn same_set(a: &[u64], b: &[u64]) -> bool {
    a.iter().all(|x| b.contains(x)) && b.iter().all(|x| a.contains(x))
}

/// Extract the `@lang` tag of a lexical value (e.g. `"hello"@en` → `en`).
fn lang_tag_of(s: &str) -> Option<String> {
    s.rsplit_once('@')
        .map(|(_, tag)| tag.trim_matches('"').to_ascii_lowercase())
}

/// Whether a value's language tag matches `want` (BCP-47 prefix match, e.g.
/// `en` matches `en-US`).
fn lang_tag_matches(value: &str, want: &str) -> bool {
    match lang_tag_of(value) {
        Some(tag) => {
            let want = want.to_ascii_lowercase();
            tag == want || tag.starts_with(&format!("{want}-"))
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_layout::{pack_float_object, INLINE_TAG_INTEGER};

    fn iri(s: &str) -> u64 {
        q_hash(s)
    }
    fn int_obj(n: i64) -> u64 {
        INLINE_TAG_INTEGER | ((n as u64) & INLINE_VALUE_MASK)
    }
    fn quin(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }
    fn type_quin(node: u64, class: &str) -> NQuin {
        quin(node, q_hash("rdf:type"), q_hash(class))
    }
    fn no_resolve() -> impl Fn(u64) -> Option<String> {
        |_| None
    }

    fn shape(class: &str, path: &str, cs: Vec<ShaclConstraint>) -> CompiledShape {
        let mut s = CompiledShape::new(class.to_string(), cs, ShaclSeverity::Violation);
        s.property_path = path.to_string();
        s
    }

    #[test]
    fn min_inclusive_passes_and_fails_on_real_numbers() {
        let alice = iri("ex:Alice");
        let bob = iri("ex:Bob");
        let quins = vec![
            type_quin(alice, "ex:Adult"),
            quin(alice, q_hash("ex:age"), int_obj(30)),
            type_quin(bob, "ex:Adult"),
            quin(bob, q_hash("ex:age"), int_obj(12)),
        ];
        let shapes = vec![shape(
            "ex:Adult",
            "ex:age",
            vec![ShaclConstraint::MinInclusive(18.0)],
        )];
        let eng = ShaclEngine::new(&quins, &shapes);
        let rep = eng.validate(&no_resolve());
        assert!(!rep.conforms, "bob (age 12) must violate minInclusive 18");
        assert_eq!(rep.results.len(), 1);
        assert_eq!(
            rep.results[0].source_constraint_component.as_deref(),
            Some("sh:MinInclusiveConstraintComponent")
        );
    }

    #[test]
    fn min_max_count_use_property_value_count() {
        let n = iri("ex:N");
        let quins = vec![
            type_quin(n, "ex:Thing"),
            quin(n, q_hash("ex:p"), iri("ex:v1")),
            quin(n, q_hash("ex:p"), iri("ex:v2")),
        ];
        let too_few = vec![shape(
            "ex:Thing",
            "ex:p",
            vec![ShaclConstraint::MinCount(3)],
        )];
        assert!(
            !ShaclEngine::new(&quins, &too_few)
                .validate(&no_resolve())
                .conforms
        );
        let too_many = vec![shape(
            "ex:Thing",
            "ex:p",
            vec![ShaclConstraint::MaxCount(1)],
        )];
        assert!(
            !ShaclEngine::new(&quins, &too_many)
                .validate(&no_resolve())
                .conforms
        );
        let ok = vec![shape(
            "ex:Thing",
            "ex:p",
            vec![ShaclConstraint::MinCount(2), ShaclConstraint::MaxCount(2)],
        )];
        assert!(
            ShaclEngine::new(&quins, &ok)
                .validate(&no_resolve())
                .conforms
        );
    }

    #[test]
    fn class_constraint_checks_rdf_type() {
        let n = iri("ex:N");
        let dog = iri("ex:Rex");
        let quins = vec![
            type_quin(n, "ex:Owner"),
            quin(n, q_hash("ex:pet"), dog),
            type_quin(dog, "ex:Cat"),
        ];
        let shapes = vec![shape(
            "ex:Owner",
            "ex:pet",
            vec![ShaclConstraint::Class("ex:Dog".into())],
        )];
        assert!(
            !ShaclEngine::new(&quins, &shapes)
                .validate(&no_resolve())
                .conforms
        );
    }

    #[test]
    fn in_and_has_value() {
        let n = iri("ex:N");
        let quins = vec![
            type_quin(n, "ex:T"),
            quin(n, q_hash("ex:status"), iri("ex:active")),
        ];
        let in_ok = vec![shape(
            "ex:T",
            "ex:status",
            vec![ShaclConstraint::In(vec![
                "ex:active".into(),
                "ex:inactive".into(),
            ])],
        )];
        assert!(
            ShaclEngine::new(&quins, &in_ok)
                .validate(&no_resolve())
                .conforms
        );
        let in_bad = vec![shape(
            "ex:T",
            "ex:status",
            vec![ShaclConstraint::In(vec!["ex:archived".into()])],
        )];
        assert!(
            !ShaclEngine::new(&quins, &in_bad)
                .validate(&no_resolve())
                .conforms
        );
        let hv = vec![shape(
            "ex:T",
            "ex:status",
            vec![ShaclConstraint::HasValue("ex:active".into())],
        )];
        assert!(
            ShaclEngine::new(&quins, &hv)
                .validate(&no_resolve())
                .conforms
        );
    }

    #[test]
    fn pattern_uses_real_regex_with_resolver() {
        let n = iri("ex:N");
        let email = iri("alice@example.org"); // value hash; resolver supplies the string
        let quins = vec![type_quin(n, "ex:User"), quin(n, q_hash("ex:email"), email)];
        let shapes = vec![shape(
            "ex:User",
            "ex:email",
            vec![ShaclConstraint::Pattern(r"^[^@]+@[^@]+\.[a-z]+$".into())],
        )];
        let resolve = |h: u64| {
            if h == email {
                Some("alice@example.org".to_string())
            } else {
                None
            }
        };
        assert!(
            ShaclEngine::new(&quins, &shapes)
                .validate(&resolve)
                .conforms
        );
        // A non-matching string fails.
        let resolve_bad = |h: u64| {
            if h == email {
                Some("not-an-email".to_string())
            } else {
                None
            }
        };
        assert!(
            !ShaclEngine::new(&quins, &shapes)
                .validate(&resolve_bad)
                .conforms
        );
        // Unresolvable → fail closed.
        assert!(
            !ShaclEngine::new(&quins, &shapes)
                .validate(&no_resolve())
                .conforms
        );
    }

    #[test]
    fn min_length_with_resolver() {
        let n = iri("ex:N");
        let v = iri("ex:val");
        let quins = vec![type_quin(n, "ex:T"), quin(n, q_hash("ex:name"), v)];
        let shapes = vec![shape(
            "ex:T",
            "ex:name",
            vec![ShaclConstraint::MinLength(5)],
        )];
        let ok = |h: u64| {
            if h == v {
                Some("Timothy".to_string())
            } else {
                None
            }
        };
        assert!(ShaclEngine::new(&quins, &shapes).validate(&ok).conforms);
        let bad = |h: u64| {
            if h == v {
                Some("Tim".to_string())
            } else {
                None
            }
        };
        assert!(!ShaclEngine::new(&quins, &shapes).validate(&bad).conforms);
    }

    #[test]
    fn logical_or_and_not_xone() {
        let n = iri("ex:N");
        let quins = vec![
            type_quin(n, "ex:Doc"),
            quin(n, q_hash("ex:title"), iri("t")),
        ];
        // Sub-shape: requires ex:title present (minCount 1).
        let has_title = shape(
            "ex:HasTitle",
            "ex:title",
            vec![ShaclConstraint::MinCount(1)],
        );
        let has_author = shape(
            "ex:HasAuthor",
            "ex:author",
            vec![ShaclConstraint::MinCount(1)],
        );
        // Node shape on the focus: Or(HasTitle, HasAuthor) → passes (has title).
        let mut or_shape = CompiledShape::new(
            "ex:Doc".into(),
            vec![ShaclConstraint::Or(vec![
                "ex:HasTitle".into(),
                "ex:HasAuthor".into(),
            ])],
            ShaclSeverity::Violation,
        );
        or_shape.property_path = String::new(); // node shape (focus itself)
        let shapes = vec![or_shape, has_title.clone(), has_author.clone()];
        assert!(
            ShaclEngine::new(&quins, &shapes)
                .validate(&no_resolve())
                .conforms
        );

        // Not(HasAuthor) → passes (no author). Not(HasTitle) → fails.
        let mut not_author = CompiledShape::new(
            "ex:Doc".into(),
            vec![ShaclConstraint::Not("ex:HasAuthor".into())],
            ShaclSeverity::Violation,
        );
        not_author.property_path = String::new();
        let shapes2 = vec![not_author, has_title.clone(), has_author.clone()];
        assert!(
            ShaclEngine::new(&quins, &shapes2)
                .validate(&no_resolve())
                .conforms
        );

        let mut not_title = CompiledShape::new(
            "ex:Doc".into(),
            vec![ShaclConstraint::Not("ex:HasTitle".into())],
            ShaclSeverity::Violation,
        );
        not_title.property_path = String::new();
        let shapes3 = vec![not_title, has_title, has_author];
        assert!(
            !ShaclEngine::new(&quins, &shapes3)
                .validate(&no_resolve())
                .conforms
        );
    }

    #[test]
    fn closed_shape_rejects_extra_predicates() {
        let n = iri("ex:N");
        let quins = vec![
            type_quin(n, "ex:Strict"),
            quin(n, q_hash("ex:allowed"), iri("v1")),
            quin(n, q_hash("ex:sneaky"), iri("v2")),
        ];
        let mut s = CompiledShape::new(
            "ex:Strict".into(),
            vec![ShaclConstraint::Closed {
                ignored_properties: vec!["ex:allowed".into()],
            }],
            ShaclSeverity::Violation,
        );
        s.property_path = String::new();
        let shapes = vec![s];
        let rep = ShaclEngine::new(&quins, &shapes).validate(&no_resolve());
        assert!(!rep.conforms, "ex:sneaky is not in the allowed/ignored set");
    }

    #[test]
    fn property_pair_less_than() {
        let n = iri("ex:N");
        let quins = vec![
            type_quin(n, "ex:Event"),
            quin(n, q_hash("ex:start"), int_obj(5)),
            quin(n, q_hash("ex:end"), int_obj(10)),
        ];
        let ok = vec![shape(
            "ex:Event",
            "ex:start",
            vec![ShaclConstraint::LessThan("ex:end".into())],
        )];
        assert!(
            ShaclEngine::new(&quins, &ok)
                .validate(&no_resolve())
                .conforms
        );
        // Reverse: end < start should fail.
        let bad = vec![shape(
            "ex:Event",
            "ex:end",
            vec![ShaclConstraint::LessThan("ex:start".into())],
        )];
        assert!(
            !ShaclEngine::new(&quins, &bad)
                .validate(&no_resolve())
                .conforms
        );
    }

    #[test]
    fn object_as_f64_decodes_inline_types() {
        assert_eq!(object_as_f64(int_obj(42)), Some(42.0));
        assert_eq!(object_as_f64(int_obj(-7)), Some(-7.0));
        assert_eq!(object_as_f64(pack_float_object(3.5)), Some(3.5));
        assert_eq!(object_as_f64(q_hash("ex:iri")), None); // IRI hash is not numeric
    }
}
