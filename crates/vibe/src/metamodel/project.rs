//! Typed AST → neutral semantic statements (no Quin / core-db).

use crate::ast::{Item, Program};

use super::namespace::{term, LANGUAGE_VERSION_IRI, VIBE_NS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementKind {
    Type,
    ClassAssertion,
    Property,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticStatement {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub kind: StatementKind,
}

/// Project a program into a bounded list of neutral RDF-like statements.
pub fn project_program_summary(program: &Program, module_iri: &str) -> Vec<SemanticStatement> {
    let mut out = Vec::new();
    out.push(SemanticStatement {
        subject: module_iri.to_string(),
        predicate: term("languageVersion"),
        object: LANGUAGE_VERSION_IRI.to_string(),
        kind: StatementKind::Property,
    });
    out.push(SemanticStatement {
        subject: module_iri.to_string(),
        predicate: format!("{VIBE_NS}type"),
        object: term("Program"),
        kind: StatementKind::ClassAssertion,
    });
    for (i, item) in program.items.iter().enumerate() {
        let node = format!("{module_iri}#item-{i}");
        let kind = item_kind_local(item);
        out.push(SemanticStatement {
            subject: module_iri.to_string(),
            predicate: term("contains"),
            object: node.clone(),
            kind: StatementKind::Property,
        });
        out.push(SemanticStatement {
            subject: node.clone(),
            predicate: term("nodeKind"),
            object: kind.to_string(),
            kind: StatementKind::Property,
        });
        out.push(SemanticStatement {
            subject: node,
            predicate: format!("{VIBE_NS}type"),
            object: term("AstNode"),
            kind: StatementKind::ClassAssertion,
        });
    }
    out
}

fn item_kind_local(item: &Item) -> &'static str {
    match item {
        Item::Function(_) => "Item.Function",
        Item::Hook(_) => "Item.Hook",
        Item::Const(_) => "Item.Const",
        Item::Enum(_) => "Item.Enum",
        Item::Field(_) => "Item.Field",
        Item::Material(_) => "Item.Material",
        Item::Law(_) => "Item.Law",
        Item::Cell(_) => "Item.Cell",
        Item::Present(_) => "Item.Present",
        Item::Bind(_) => "Item.Bind",
        Item::Statement(_) => "Item.Statement",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_program;
    use crate::span::Span;

    #[test]
    fn projects_empty_program() {
        let p = Program {
            span: Span::point(0),
            module: None,
            imports: vec![],
            prefixes: vec![],
            locales: vec![],
            requires: vec![],
            items: vec![],
        };
        let stmts = project_program_summary(&p, "urn:qualia:vibe:fixture");
        assert!(stmts.iter().any(|s| s.object.contains("LanguageVersion")));
    }

    #[test]
    fn projects_function_item_kind() {
        let src = "fn hello() {}";
        let p = parse_program(src).expect("parse");
        let stmts = project_program_summary(&p, "urn:qualia:vibe:hello");
        assert!(stmts.iter().any(|s| s.object == "Item.Function"));
    }
}
