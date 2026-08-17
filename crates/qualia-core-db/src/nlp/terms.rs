//! Interned gazetteer lexicon. Hits only emit these IRIs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexemeKind {
    Entity,
    Place,
    Person,
    Relation,
    Quantity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexeme {
    pub surface: &'static str,
    pub iri: &'static str,
    pub kind: LexemeKind,
}

/// North Spring catchment + principal identity. No third-party persons.
pub const DEFAULT_LEXICON: &[Lexeme] = &[
    Lexeme {
        surface: "North Spring",
        iri: "https://qualiadb.org/catchment/NorthSpring",
        kind: LexemeKind::Place,
    },
    Lexeme {
        surface: "reference catchment",
        iri: "https://qualiadb.org/catchment/ReferenceCatchment",
        kind: LexemeKind::Place,
    },
    Lexeme {
        surface: "reference site",
        iri: "https://qualiadb.org/catchment/ReferenceSite",
        kind: LexemeKind::Place,
    },
    Lexeme {
        surface: "catchment",
        iri: "https://qualiadb.org/catchment/Catchment",
        kind: LexemeKind::Entity,
    },
    Lexeme {
        surface: "Timothy Charles Holborn",
        iri: "did:qualia:timothy_charles_holborn",
        kind: LexemeKind::Person,
    },
    Lexeme {
        surface: "rain",
        iri: "https://qualiadb.org/meteo/Rain",
        kind: LexemeKind::Quantity,
    },
    Lexeme {
        surface: "hasCondition",
        iri: "https://qualiadb.org/clinic/hasCondition",
        kind: LexemeKind::Relation,
    },
];

pub fn lookup_iri(iri: &str) -> Option<&'static Lexeme> {
    DEFAULT_LEXICON.iter().find(|l| l.iri == iri)
}

pub fn known_iri(iri: &str) -> bool {
    lookup_iri(iri).is_some()
}
