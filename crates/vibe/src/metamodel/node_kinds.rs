//! AST / effect / type / modality node-kind registry for projection and export.

use super::namespace::term;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeKindDescriptor {
    pub local_name: &'static str,
    pub rust_path: &'static str,
    pub rdfs_class_local: &'static str,
}

macro_rules! kinds {
    ($($local:literal, $rust:literal, $class:literal);+ $(;)?) => {
        &[
            $(NodeKindDescriptor {
                local_name: $local,
                rust_path: $rust,
                rdfs_class_local: $class,
            }),+
        ]
    };
}

pub const ITEM_KINDS: &[NodeKindDescriptor] = kinds!(
    "Item.Function", "ast::Item::Function", "FunctionDeclaration";
    "Item.Hook", "ast::Item::Hook", "HookDeclaration";
    "Item.Const", "ast::Item::Const", "Declaration";
    "Item.Enum", "ast::Item::Enum", "Declaration";
    "Item.Field", "ast::Item::Field", "FieldDeclaration";
    "Item.Material", "ast::Item::Material", "MaterialDeclaration";
    "Item.Law", "ast::Item::Law", "LawDeclaration";
    "Item.Cell", "ast::Item::Cell", "CellDeclaration";
    "Item.Present", "ast::Item::Present", "Declaration";
    "Item.Bind", "ast::Item::Bind", "Declaration";
    "Item.Statement", "ast::Item::Statement", "Statement";
);

pub const STMT_KINDS: &[NodeKindDescriptor] = kinds!(
    "Stmt.Let", "ast::Stmt::Let", "Statement";
    "Stmt.LetPat", "ast::Stmt::LetPat", "Statement";
    "Stmt.Assign", "ast::Stmt::Assign", "Statement";
    "Stmt.If", "ast::Stmt::If", "Statement";
    "Stmt.For", "ast::Stmt::For", "Statement";
    "Stmt.While", "ast::Stmt::While", "Statement";
    "Stmt.Match", "ast::Stmt::Match", "Statement";
    "Stmt.Return", "ast::Stmt::Return", "Statement";
    "Stmt.Yield", "ast::Stmt::Yield", "Statement";
    "Stmt.Transaction", "ast::Stmt::Transaction", "Statement";
    "Stmt.Effect", "ast::Stmt::Effect", "Statement";
    "Stmt.Expr", "ast::Stmt::Expr", "Statement";
    "Stmt.Block", "ast::Stmt::Block", "Statement";
);

pub const PATTERN_KINDS: &[NodeKindDescriptor] = kinds!(
    "Pattern.Wildcard", "ast::Pattern::Wildcard", "Pattern";
    "Pattern.Ident", "ast::Pattern::Ident", "Pattern";
    "Pattern.Literal", "ast::Pattern::Literal", "Pattern";
    "Pattern.Ok", "ast::Pattern::Ok", "Pattern";
    "Pattern.Err", "ast::Pattern::Err", "Pattern";
    "Pattern.Some", "ast::Pattern::Some", "Pattern";
    "Pattern.None", "ast::Pattern::None", "Pattern";
    "Pattern.Record", "ast::Pattern::Record", "Pattern";
    "Pattern.List", "ast::Pattern::List", "Pattern";
    "Pattern.Constructor", "ast::Pattern::Constructor", "Pattern";
    "Pattern.Variant", "ast::Pattern::Variant", "Pattern";
);

pub const EXPR_KINDS: &[NodeKindDescriptor] = kinds!(
    "ExprKind.Literal", "ast::ExprKind::Literal", "Expression";
    "ExprKind.Ident", "ast::ExprKind::Ident", "Expression";
    "ExprKind.QueryVar", "ast::ExprKind::QueryVar", "Expression";
    "ExprKind.Iri", "ast::ExprKind::Iri", "Expression";
    "ExprKind.Prefixed", "ast::ExprKind::Prefixed", "Expression";
    "ExprKind.Blank", "ast::ExprKind::Blank", "Expression";
    "ExprKind.Binary", "ast::ExprKind::Binary", "Expression";
    "ExprKind.Unary", "ast::ExprKind::Unary", "Expression";
    "ExprKind.Await", "ast::ExprKind::Await", "Expression";
    "ExprKind.Member", "ast::ExprKind::Member", "Expression";
    "ExprKind.Call", "ast::ExprKind::Call", "Expression";
    "ExprKind.Index", "ast::ExprKind::Index", "Expression";
    "ExprKind.Try", "ast::ExprKind::Try", "Expression";
    "ExprKind.List", "ast::ExprKind::List", "Expression";
    "ExprKind.Record", "ast::ExprKind::Record", "Expression";
    "ExprKind.Triple", "ast::ExprKind::Triple", "Expression";
    "ExprKind.Reified", "ast::ExprKind::Reified", "Expression";
    "ExprKind.Pipe", "ast::ExprKind::Pipe", "Expression";
    "ExprKind.GraphQuery", "ast::ExprKind::GraphQuery", "Expression";
    "ExprKind.ModalLogic", "ast::ExprKind::ModalLogic", "Expression";
    "ExprKind.Interpolate", "ast::ExprKind::Interpolate", "Expression";
    "ExprKind.Lambda", "ast::ExprKind::Lambda", "Expression";
    "ExprKind.Tween", "ast::ExprKind::Tween", "Expression";
);

pub const MODAL_KINDS: &[NodeKindDescriptor] = kinds!(
    "ModalKind.DeonticObligate", "ast::ModalKind::DeonticObligate", "Expression";
    "ModalKind.DeonticPermit", "ast::ModalKind::DeonticPermit", "Expression";
    "ModalKind.DeonticForbid", "ast::ModalKind::DeonticForbid", "Expression";
    "ModalKind.EpistemicKnows", "ast::ModalKind::EpistemicKnows", "Expression";
    "ModalKind.EpistemicBelieves", "ast::ModalKind::EpistemicBelieves", "Expression";
    "ModalKind.Paraconsistent", "ast::ModalKind::Paraconsistent", "Expression";
    "ModalKind.LtlGlobally", "ast::ModalKind::LtlGlobally", "Expression";
    "ModalKind.LtlFinally", "ast::ModalKind::LtlFinally", "Expression";
    "ModalKind.LtlUntil", "ast::ModalKind::LtlUntil", "Expression";
    "ModalKind.DlSubsumes", "ast::ModalKind::DlSubsumes", "Expression";
    "ModalKind.N3Defeasible", "ast::ModalKind::N3Defeasible", "Expression";
);

pub const EFFECT_KINDS: &[NodeKindDescriptor] = kinds!(
    "EffectClass.Pure", "ast::EffectClass::Pure", "EffectClass";
    "EffectClass.Hot", "ast::EffectClass::Hot", "EffectClass";
    "EffectClass.Cold", "ast::EffectClass::Cold", "EffectClass";
    "EffectClass.Async", "ast::EffectClass::Async", "EffectClass";
    "EffectClass.External", "ast::EffectClass::External", "EffectClass";
);

pub const TYPE_KINDS: &[NodeKindDescriptor] = kinds!(
    "Type.I64", "types::Type::I64", "Type";
    "Type.U64", "types::Type::U64", "Type";
    "Type.F64", "types::Type::F64", "Type";
    "Type.Bool", "types::Type::Bool", "Type";
    "Type.String", "types::Type::String", "Type";
    "Type.Bytes", "types::Type::Bytes", "Type";
    "Type.Iri", "types::Type::Iri", "Type";
    "Type.BlankNode", "types::Type::BlankNode", "Type";
    "Type.Did", "types::Type::Did", "Type";
    "Type.Continuant", "types::Type::Continuant", "Type";
    "Type.Hash", "types::Type::Hash", "Type";
    "Type.Var", "types::Type::Var", "Type";
    "Type.Literal", "types::Type::Literal", "Type";
    "Type.TripleTerm", "types::Type::TripleTerm", "Type";
    "Type.Reifier", "types::Type::Reifier", "Type";
    "Type.Quin", "types::Type::Quin", "Type";
    "Type.QuinRef", "types::Type::QuinRef", "Type";
    "Type.AssetRef", "types::Type::AssetRef", "Type";
    "Type.TensorRef", "types::Type::TensorRef", "Type";
    "Type.GeometryRef", "types::Type::GeometryRef", "Type";
    "Type.Instant", "types::Type::Instant", "Type";
    "Type.Duration", "types::Type::Duration", "Type";
    "Type.Quantity", "types::Type::Quantity", "Type";
    "Type.Frame", "types::Type::Frame", "Type";
    "Type.Pose", "types::Type::Pose", "Type";
    "Type.Transform", "types::Type::Transform", "Type";
    "Type.FieldRef", "types::Type::FieldRef", "Type";
    "Type.MaterialRef", "types::Type::MaterialRef", "Type";
    "Type.WorldLine", "types::Type::WorldLine", "Type";
    "Type.Enum", "types::Type::Enum", "Type";
    "Type.SpeciesRef", "types::Type::SpeciesRef", "Type";
    "Type.Mixture", "types::Type::Mixture", "Type";
    "Type.Option", "types::Type::Option", "Type";
    "Type.Result", "types::Type::Result", "Type";
    "Type.List", "types::Type::List", "Type";
    "Type.Record", "types::Type::Record", "Type";
    "Type.Receipt", "types::Type::Receipt", "Type";
    "Type.Stream", "types::Type::Stream", "Type";
    "Type.Future", "types::Type::Future", "Type";
    "Type.Named", "types::Type::Named", "Type";
    "Type.Unknown", "types::Type::Unknown", "Type";
);

/// All kind descriptors flattened for export.
pub fn all_node_kinds() -> Vec<&'static NodeKindDescriptor> {
    let mut out = Vec::new();
    for slice in [
        ITEM_KINDS, STMT_KINDS, PATTERN_KINDS, EXPR_KINDS, MODAL_KINDS, EFFECT_KINDS, TYPE_KINDS,
    ] {
        out.extend(slice.iter());
    }
    out
}

pub fn kind_iri(d: &NodeKindDescriptor) -> String {
    term(d.rdfs_class_local)
}
