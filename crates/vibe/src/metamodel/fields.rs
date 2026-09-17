//! Field / property descriptors for the Vibe metamodel.

use super::namespace::term;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub local_name: &'static str,
    pub rdfs_property_local: &'static str,
    pub domain_class: &'static str,
    pub range_hint: &'static str,
}

pub const FIELD_DESCRIPTORS: &[FieldDescriptor] = &[
    FieldDescriptor {
        local_name: "languageVersion",
        rdfs_property_local: "languageVersion",
        domain_class: "Program",
        range_hint: "xsd:string",
    },
    FieldDescriptor {
        local_name: "nodeKind",
        rdfs_property_local: "nodeKind",
        domain_class: "AstNode",
        range_hint: "xsd:string",
    },
    FieldDescriptor {
        local_name: "contains",
        rdfs_property_local: "contains",
        domain_class: "Program",
        range_hint: "AstNode",
    },
    FieldDescriptor {
        local_name: "declares",
        rdfs_property_local: "declares",
        domain_class: "Module",
        range_hint: "Declaration",
    },
    FieldDescriptor {
        local_name: "hasExpression",
        rdfs_property_local: "hasExpression",
        domain_class: "Statement",
        range_hint: "Expression",
    },
    FieldDescriptor {
        local_name: "hasStatement",
        rdfs_property_local: "hasStatement",
        domain_class: "Block",
        range_hint: "Statement",
    },
    FieldDescriptor {
        local_name: "hasType",
        rdfs_property_local: "hasType",
        domain_class: "Declaration",
        range_hint: "Type",
    },
    FieldDescriptor {
        local_name: "hasEffect",
        rdfs_property_local: "hasEffect",
        domain_class: "FunctionDeclaration",
        range_hint: "EffectClass",
    },
    FieldDescriptor {
        local_name: "requiresCapability",
        rdfs_property_local: "requiresCapability",
        domain_class: "Program",
        range_hint: "Capability",
    },
    FieldDescriptor {
        local_name: "invokeId",
        rdfs_property_local: "invokeId",
        domain_class: "Capability",
        range_hint: "InvokeId",
    },
    FieldDescriptor {
        local_name: "hasArgument",
        rdfs_property_local: "hasArgument",
        domain_class: "CapabilityArgument",
        range_hint: "xsd:string",
    },
    FieldDescriptor {
        local_name: "hasBudget",
        rdfs_property_local: "hasBudget",
        domain_class: "FunctionDeclaration",
        range_hint: "Budget",
    },
    FieldDescriptor {
        local_name: "sourceStart",
        rdfs_property_local: "sourceStart",
        domain_class: "SourceSpan",
        range_hint: "xsd:integer",
    },
    FieldDescriptor {
        local_name: "sourceEnd",
        rdfs_property_local: "sourceEnd",
        domain_class: "SourceSpan",
        range_hint: "xsd:integer",
    },
    FieldDescriptor {
        local_name: "diagnosticCode",
        rdfs_property_local: "diagnosticCode",
        domain_class: "Diagnostic",
        range_hint: "xsd:string",
    },
];

pub fn property_iri(f: &FieldDescriptor) -> String {
    term(f.rdfs_property_local)
}
