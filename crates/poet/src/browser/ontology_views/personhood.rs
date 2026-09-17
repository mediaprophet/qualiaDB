//! Natural persons are not `owl:Thing`.
//!
//! OWL's universal superclass is `owl:Thing`. Personhood is modelled with
//! `rdfs:Class` plus SHACL/ShEx. `owl:` is a guard target (`sh:not owl:Thing`)
//! or an artifact/class inference vocabulary, never the metamodel for humans.

const PERSON_LOCAL_NAMES: &[&str] = &[
    "person",
    "principal",
    "naturalperson",
    "naturalagent",
    "human",
    "humanbeing",
    "contributor",
];

pub fn looks_like_natural_person(term: &str) -> bool {
    local_name(term).is_some_and(|local| PERSON_LOCAL_NAMES.iter().any(|marker| local == *marker))
}

pub fn is_owl_construct(term: &str) -> bool {
    let lower = term.trim().to_ascii_lowercase();
    lower.starts_with("owl:") || lower.contains("/owl#") || lower == "owl"
}

pub fn is_shacl_negation(predicate: &str) -> bool {
    let lower = predicate.to_ascii_lowercase();
    lower.contains("sh:not") || lower.ends_with("#not") || lower == "not"
}

pub fn owl_forbidden_for_person(subject: &str, predicate: &str, object: &str) -> bool {
    if is_shacl_negation(predicate) {
        return false;
    }
    let person = looks_like_natural_person(subject) || looks_like_natural_person(object);
    if !person {
        return false;
    }
    is_owl_construct(predicate) || (is_type_predicate(predicate) && is_owl_construct(object))
}

pub fn owl_person_source_violation(source: &str) -> Option<&'static str> {
    for raw in source.lines() {
        let line = strip_comment(raw);
        if line.is_empty() || is_shacl_negation(&line) {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        let person = PERSON_LOCAL_NAMES
            .iter()
            .any(|marker| contains_local_name(&lower, marker));
        if !person {
            continue;
        }
        if lower.contains("owl:class")
            || lower.contains("owl:thing")
            || lower.contains("owl:namedindividual")
            || lower.contains("owl:equivalentclass")
            || lower.contains("owl:sameas")
        {
            return Some(
                "Natural persons are modelled with RDFS and SHACL/ShEx. owl:Thing / owl:Class is forbidden for persons.",
            );
        }
    }
    None
}

fn is_type_predicate(predicate: &str) -> bool {
    let lower = predicate.trim().to_ascii_lowercase();
    lower == "a" || lower == "rdf:type" || lower.ends_with("#type")
}

fn local_name(term: &str) -> Option<String> {
    let trimmed = term
        .trim()
        .trim_matches(|c| c == '<' || c == '>' || c == '"');
    let local = trimmed
        .rsplit(['#', '/', ':'])
        .next()
        .unwrap_or(trimmed)
        .trim();
    if local.is_empty() {
        None
    } else {
        Some(local.to_ascii_lowercase())
    }
}

fn contains_local_name(line: &str, marker: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| token == marker)
}

fn strip_comment(line: &str) -> String {
    if let Some(index) = line.find(" #") {
        line[..index].trim().to_string()
    } else if line.trim_start().starts_with('#') {
        String::new()
    } else {
        line.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rdfs_and_shacl_are_the_person_path() {
        assert!(!owl_forbidden_for_person(
            "q42:Principal",
            "rdf:type",
            "rdfs:Class"
        ));
        assert!(!owl_forbidden_for_person(
            "q42:PrincipalShape",
            "sh:not",
            "owl:Thing"
        ));
    }

    #[test]
    fn owl_class_on_a_person_is_rejected() {
        assert!(owl_forbidden_for_person("soc:Person", "a", "owl:Class"));
        assert!(owl_person_source_violation(
            "coop:Contributor a owl:Class ; rdfs:subClassOf soc:Person ."
        )
        .is_some());
    }

    #[test]
    fn owl_class_on_an_artefact_is_allowed() {
        assert!(!owl_forbidden_for_person("coop:Project", "a", "owl:Class"));
        assert!(owl_person_source_violation("coop:Project a owl:Class .").is_none());
    }

    #[test]
    fn person_safe_n3_sample_is_accepted() {
        assert!(owl_person_source_violation(
            crate::browser::ontology_views::persist::PERSON_SAFE_N3
        )
        .is_none());
    }
}
