//! vibe-0.1 AST. Spans are UTF-8 byte offsets.

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub span: Span,
    pub module: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub prefixes: Vec<PrefixDecl>,
    pub requires: Vec<CapSpec>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub span: Span,
    pub name: Name,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub span: Span,
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefixDecl {
    pub span: Span,
    pub prefix: String,
    pub iri: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapSpec {
    pub span: Span,
    pub id: String,
    pub args: Vec<NamedArg>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDecl),
    Hook(HookDecl),
    Const(ConstDecl),
    Enum(EnumDecl),
    Field(FieldDecl),
    Material(MaterialDecl),
    Law(LawDecl),
    Statement(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub span: Span,
    pub effect: Option<EffectClass>,
    pub is_async: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub budget: Vec<NamedArg>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookDecl {
    pub span: Span,
    pub path: Vec<String>,
    pub params: Vec<Param>,
    pub budget: Vec<NamedArg>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub span: Span,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}

/// A user-defined enum declaration (T9).
///
/// ```vibe
/// enum Shape {
///   Circle(f64),
///   Square(f64),
///   Rect(f64, f64),
///   Point,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub span: Span,
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

/// A variant of a user-defined enum (T9).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub span: Span,
    pub name: String,
    /// Payload types (empty = unit variant like `Point`).
    pub payload: Vec<TypeExpr>,
}

/// A field declaration (T28).
///
/// ```vibe
/// field pressure_ambient: Pressure
///   unit: <qudt:KiloPascal>
///   support: region
///   representation: grid;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub span: Span,
    pub name: String,
    pub ty: TypeExpr,
    /// Unit IRI (e.g. `qudt:KiloPascal`). Optional — dimensionless fields
    /// have no unit.
    pub unit: Option<String>,
    /// Support: how the field is distributed in space.
    /// `region` (grid), `point` (sampled), `continuant` (attached to an
    /// enduring thing), `stream` (time-series).
    pub support: FieldSupport,
    /// Representation: how the field is stored/computed.
    /// `grid`, `mesh`, `particles`, `analytic`, `sampled`.
    pub representation: FieldRepresentation,
}

/// Field support describes spatial distribution (T28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSupport {
    /// Field lives on a region (grid of cells).
    Region,
    /// Field is sampled at discrete points.
    Point,
    /// Field is attached to a continuant (enduring thing).
    Continuant,
    /// Field is a time-series stream.
    Stream,
}

impl Default for FieldSupport {
    fn default() -> Self {
        FieldSupport::Region
    }
}

/// Field representation describes storage/computation (T28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRepresentation {
    /// Regular grid of cells.
    Grid,
    /// Unstructured mesh.
    Mesh,
    /// Particle-based (Lagrangian).
    Particles,
    /// Analytic (closed-form expression).
    Analytic,
    /// Sampled (lookup table / measured data).
    Sampled,
}

impl Default for FieldRepresentation {
    fn default() -> Self {
        FieldRepresentation::Grid
    }
}

/// A material declaration (T29).
///
/// ```vibe
/// material sucrose_cube: Material
///   yield: 50.0 <qudt:KiloPascal>
///   density: 1580.0 <qudt:KiloGramPerCubicMetre>;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialDecl {
    pub span: Span,
    pub name: String,
    /// Material properties as named arguments. Each property has a value
    /// and an optional unit IRI (encoded in the NamedArg's value as a
    /// Record or Quantity).
    pub properties: Vec<NamedArg>,
}

/// A law declaration (T30).
///
/// ```vibe
/// law crush
///   when sample(pressure_ambient, pose(self)) > self.material.yield
///   => transform.yield(self);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LawDecl {
    pub span: Span,
    pub name: String,
    /// The condition expression (after `when`).
    pub condition: Expr,
    /// The consequence expression (after `=>`).
    pub consequence: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectClass {
    Pure,
    Hot,
    Cold,
    Async,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub span: Span,
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedArg {
    pub span: Span,
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        span: Span,
        mutable: bool,
        name: String,
        ty: Option<TypeExpr>,
        value: Option<Expr>,
    },
    Assign {
        span: Span,
        target: Expr,
        value: Expr,
    },
    If {
        span: Span,
        cond: Expr,
        then_block: Block,
        else_block: Option<Box<Stmt>>,
    },
    For {
        span: Span,
        name: String,
        iter: Expr,
        body: Block,
    },
    While {
        span: Span,
        cond: Expr,
        body: Block,
    },
    Match {
        span: Span,
        scrutinee: Expr,
        arms: Vec<MatchArm>,
    },
    Return {
        span: Span,
        value: Option<Expr>,
    },
    Yield {
        span: Span,
        value: Option<Expr>,
    },
    Transaction {
        span: Span,
        args: Vec<NamedArg>,
        body: Block,
    },
    Effect {
        span: Span,
        expr: Expr,
    },
    Expr {
        span: Span,
        expr: Expr,
    },
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub span: Span,
    pub pattern: Pattern,
    pub body: ArmBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArmBody {
    Block(Block),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Ident(String),
    Literal(Literal),
    Ok(Box<Pattern>),
    Err(Box<Pattern>),
    Some(Box<Pattern>),
    None,
    /// A user-defined enum variant pattern (T9).
    /// `enum_name.variant_name` with optional inner patterns.
    /// Unit variants have empty `args`.
    Variant {
        enum_name: String,
        variant_name: String,
        args: Vec<Pattern>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Ident(String),
    QueryVar(String),
    Iri(String),
    Prefixed(String, String),
    Blank(String),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Await(Box<Expr>),
    Member {
        recv: Box<Expr>,
        name: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Index {
        recv: Box<Expr>,
        index: Box<Expr>,
    },
    Try(Box<Expr>),
    List(Vec<Expr>),
    Record(Vec<NamedArg>),
    Triple {
        subject: Box<Expr>,
        predicate: Box<Expr>,
        object: Box<Expr>,
    },
    Reified {
        subject: Box<Expr>,
        predicate: Box<Expr>,
        object: Box<Expr>,
        reifier: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Pos(Expr),
    Named(NamedArg),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
    Plus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(u64), // f64 bits
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub span: Span,
    pub name: String,
    pub args: Vec<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Name {
    Ident(String),
    Iri(String),
}

impl Expr {
    pub fn ident_name(&self) -> Option<&str> {
        match &self.kind {
            ExprKind::Ident(s) => Some(s),
            _ => None,
        }
    }
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::If { span, .. }
            | Stmt::For { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Match { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Yield { span, .. }
            | Stmt::Transaction { span, .. }
            | Stmt::Effect { span, .. }
            | Stmt::Expr { span, .. } => *span,
            Stmt::Block(b) => b.span,
        }
    }
}
