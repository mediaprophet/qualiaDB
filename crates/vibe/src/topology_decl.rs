//! Declarative VibeScript `geometry` and `topology` complex parser and capability lowerer.
//!
//! Implements §6 of the POET Geometry-Topology Complex architecture ("Geometries Harbour Many Graphs"):
//! - `geometry <Name> : <ComplexKind> { ... graph <G> : <Morphology> { ... } }`
//! - Standalone `topology <Name> : <Morphology> { ... }` syntactic sugar (desugars into a host
//!   `GeometryDecl` of kind `MetricPointCloud10D` harbouring a single primary `HarbouredGraphDecl` at `slot = 0`).
//! - Deterministic lowering to `Topology.define_geometry`, `Topology.harbour_graph`, and `Poet.topology_bind`.

use std::collections::BTreeMap;

use crate::ast::{Arg, Block, Expr, ExprKind, Literal, NamedArg, Stmt};
use crate::error::{DiagCode, Diagnostic};
use crate::lex::TokenKind;
use crate::parse::Parser;
use crate::span::Span;
use crate::value::Value;

/// Boundary source/sink degree predicates on a harboured graph (`boundary { sources: ...; sinks: ...; }`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundaryDecl {
    pub sources: Option<String>,
    pub sinks: Option<String>,
}

/// A single `graph <Name> : <Morphology> { ... }` harboured inside a host `GeometryDecl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarbouredGraphDecl {
    pub span: Span,
    pub name: String,
    pub slot: u16,
    pub morphology: String,
    pub role: String,
    pub anchors: Vec<String>,
    pub weights: Option<String>,
    pub edge_binding: Vec<String>,
    pub invariants: Vec<String>,
    pub boundary: Option<BoundaryDecl>,
}

/// A declarative `geometry <Name> : <ComplexKind> { ... }` harbouring $M \ge 1$ graphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryDecl {
    pub span: Span,
    pub name: String,
    pub complex_kind: String,
    pub satisfies: Option<String>,
    pub manifold: Option<String>,
    pub v_register: Option<u64>,
    pub w_domain: Option<u64>,
    pub metric: Option<String>,
    pub metric_axes: Vec<String>,
    pub cells_source: Option<String>,
    pub operators: Vec<String>,
    pub harboured_graphs: Vec<HarbouredGraphDecl>,
    pub is_standalone_topology_sugar: bool,
}

/// Deterministic `capability.invoke` call descriptor lowered from a `GeometryDecl`.
#[derive(Debug, Clone, PartialEq)]
pub struct TopologyInvokeDescriptor {
    pub capability_id: &'static str,
    pub args: Value,
}

impl GeometryDecl {
    /// Lowers this `GeometryDecl` and its harboured graphs into deterministic
    /// `Topology.define_geometry`, `Topology.harbour_graph`, and `Poet.topology_bind` descriptors.
    pub fn lower_to_invokes(&self) -> Vec<TopologyInvokeDescriptor> {
        let mut out = Vec::with_capacity(self.harboured_graphs.len() + 2);
        let mut def_map = BTreeMap::new();
        def_map.insert("name".into(), Value::String(self.name.clone()));
        def_map.insert("complex_kind".into(), Value::String(self.complex_kind.clone()));
        def_map.insert("harboured_graph_count".into(), Value::U64(self.harboured_graphs.len() as u64));
        if let Some(s) = &self.satisfies { def_map.insert("satisfies".into(), Value::String(s.clone())); }
        if let Some(m) = &self.manifold { def_map.insert("manifold".into(), Value::String(m.clone())); }
        if let Some(v) = self.v_register { def_map.insert("v_register".into(), Value::U64(v)); }
        if let Some(w) = self.w_domain { def_map.insert("w_domain".into(), Value::U64(w)); }
        if let Some(metric) = &self.metric { def_map.insert("metric".into(), Value::String(metric.clone())); }
        if !self.metric_axes.is_empty() {
            def_map.insert("metric_axes".into(), Value::List(self.metric_axes.iter().cloned().map(Value::String).collect()));
        }
        if let Some(src) = &self.cells_source { def_map.insert("cells_source".into(), Value::String(src.clone())); }
        if !self.operators.is_empty() {
            def_map.insert("operators".into(), Value::List(self.operators.iter().cloned().map(Value::String).collect()));
        }
        out.push(TopologyInvokeDescriptor { capability_id: "Topology.define_geometry", args: Value::Record(def_map) });

        for g in &self.harboured_graphs {
            let mut g_map = BTreeMap::new();
            g_map.insert("geometry".into(), Value::String(self.name.clone()));
            g_map.insert("name".into(), Value::String(g.name.clone()));
            g_map.insert("slot".into(), Value::U64(g.slot as u64));
            g_map.insert("morphology".into(), Value::String(g.morphology.clone()));
            g_map.insert("role".into(), Value::String(g.role.clone()));
            g_map.insert("anchors".into(), Value::List(g.anchors.iter().cloned().map(Value::String).collect()));
            g_map.insert("invariants".into(), Value::List(g.invariants.iter().cloned().map(Value::String).collect()));
            if let Some(w) = &g.weights { g_map.insert("weights".into(), Value::String(w.clone())); }
            if !g.edge_binding.is_empty() {
                g_map.insert("edge_binding".into(), Value::List(g.edge_binding.iter().cloned().map(Value::String).collect()));
            }
            if let Some(b) = &g.boundary {
                if let Some(src) = &b.sources { g_map.insert("boundary_sources".into(), Value::String(src.clone())); }
                if let Some(snk) = &b.sinks { g_map.insert("boundary_sinks".into(), Value::String(snk.clone())); }
            }
            out.push(TopologyInvokeDescriptor { capability_id: "Topology.harbour_graph", args: Value::Record(g_map) });
        }

        let mut bind_map = BTreeMap::new();
        bind_map.insert("geometry".into(), Value::String(self.name.clone()));
        bind_map.insert("complex_kind".into(), Value::String(self.complex_kind.clone()));
        bind_map.insert("manifold".into(), Value::String(self.manifold.clone().unwrap_or_else(|| "Spatial".into())));
        bind_map.insert("harboured_graph_count".into(), Value::U64(self.harboured_graphs.len() as u64));
        bind_map.insert("active_graph_slot".into(), Value::U64(0));
        out.push(TopologyInvokeDescriptor { capability_id: "Poet.topology_bind", args: Value::Record(bind_map) });
        out
    }

    /// Lowers this declaration to an AST `Block` of capability invoke statements.
    pub fn lower_to_block(&self) -> Block {
        let span = self.span;
        let stmts = self.lower_to_invokes().into_iter().map(|desc| {
            let (fam, method) = desc.capability_id.split_once('.').unwrap_or(("Topology", desc.capability_id));
            let callee = Expr {
                span,
                kind: ExprKind::Member {
                    recv: Box::new(Expr { span, kind: ExprKind::Ident(fam.to_string()) }),
                    name: method.to_string(),
                },
            };
            let args = match desc.args {
                Value::Record(map) => map.into_iter().map(|(k, v)| {
                    Arg::Named(NamedArg { span, name: k, value: value_to_ast_expr(&v, span) })
                }).collect(),
                _ => Vec::new(),
            };
            Stmt::Expr { span, expr: Expr { span, kind: ExprKind::Call { callee: Box::new(callee), args } } }
        }).collect();
        Block { span, stmts }
    }
}

fn value_to_ast_expr(v: &Value, span: Span) -> Expr {
    let kind = match v {
        Value::String(s) => ExprKind::Literal(Literal::String(s.clone())),
        Value::U64(n) => ExprKind::Literal(Literal::UInt(*n)),
        Value::I64(n) => ExprKind::Literal(Literal::Int(*n)),
        Value::Bool(b) => ExprKind::Literal(Literal::Bool(*b)),
        Value::List(items) => ExprKind::List(items.iter().map(|i| value_to_ast_expr(i, span)).collect()),
        _ => ExprKind::Literal(Literal::Null),
    };
    Expr { span, kind }
}

/// Parses a single `geometry <Name> : <ComplexKind> { ... }` or standalone `topology <Name> : <Morphology> { ... }`.
pub fn parse_geometry_or_topology(src: &str) -> Result<GeometryDecl, Diagnostic> {
    let mut parser = Parser::new(src)?;
    parser.parse_geometry_or_topology_decl()
}

/// Parses all `geometry` and `topology` declarations in a VibeScript source snippet, skipping `construct` or other items.
pub fn parse_geometry_blocks(src: &str) -> Result<Vec<GeometryDecl>, Diagnostic> {
    let mut parser = Parser::new(src)?;
    let mut out = Vec::new();
    while parser.cur.kind != TokenKind::Eof {
        if parser.looks_like_geometry_or_topology() {
            out.push(parser.parse_geometry_or_topology_decl()?);
        } else {
            parser.cur = parser.lex.next_token()?;
        }
    }
    Ok(out)
}

impl<'a> Parser<'a> {
    pub(crate) fn looks_like_geometry_or_topology(&self) -> bool {
        let tok = self.cur.span.slice(self.lex.source());
        if tok != "geometry" && tok != "topology" { return false; }
        let rest = self.lex.source()[self.cur.span.end as usize..].trim_start();
        let ident_len = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').map(char::len_utf8).sum::<usize>();
        ident_len > 0 && rest[ident_len..].trim_start().starts_with(':')
    }

    pub(crate) fn parse_geometry_or_topology_decl(&mut self) -> Result<GeometryDecl, Diagnostic> {
        let start = self.cur.span.start;
        let kw = self.cur.span.slice(self.lex.source()).to_string();
        self.cur = self.lex.next_token()?;
        let name = self.take_any_ident()?;
        self.expect_tok(TokenKind::Colon, "expected ':' after geometry/topology name")?;
        let header_kind = self.take_any_ident()?;
        self.expect_tok(TokenKind::LBrace, "expected '{' to open geometry/topology body")?;

        if kw == "topology" {
            let mut graph = self.parse_harboured_graph_body(start, name.clone(), 0, header_kind)?;
            if graph.role.is_empty() { graph.role = "SemanticOverlay".into(); }
            if graph.anchors.is_empty() { graph.anchors.push("SemanticQuin".into()); }
            let end = self.expect_tok(TokenKind::RBrace, "expected '}' after topology body")?.end;
            graph.span = Span::new(start, end);
            return Ok(GeometryDecl {
                span: Span::new(start, end), name, complex_kind: "MetricPointCloud10D".into(),
                satisfies: None, manifold: Some("Spatial".into()), v_register: Some(0), w_domain: None,
                metric: Some("Euclidean".into()), metric_axes: Vec::new(), cells_source: None,
                operators: Vec::new(), harboured_graphs: vec![graph], is_standalone_topology_sugar: true,
            });
        }

        let mut decl = GeometryDecl {
            span: Span::new(start, start), name, complex_kind: header_kind,
            satisfies: None, manifold: None, v_register: None, w_domain: None,
            metric: None, metric_axes: Vec::new(), cells_source: None,
            operators: Vec::new(), harboured_graphs: Vec::new(), is_standalone_topology_sugar: false,
        };

        while self.cur.kind != TokenKind::RBrace && self.cur.kind != TokenKind::Eof {
            let key = self.take_any_ident()?;
            if key == "graph" {
                let g_start = self.cur.span.start;
                let g_name = self.take_any_ident()?;
                self.expect_tok(TokenKind::Colon, "expected ':' after graph name")?;
                let morph = self.take_any_ident()?;
                self.expect_tok(TokenKind::LBrace, "expected '{' after graph morphology")?;
                let slot = decl.harboured_graphs.len() as u16;
                let mut g = self.parse_harboured_graph_body(g_start, g_name, slot, morph)?;
                let g_end = self.expect_tok(TokenKind::RBrace, "expected '}' after graph block")?.end;
                g.span = Span::new(g_start, g_end);
                decl.harboured_graphs.push(g);
                continue;
            }
            self.expect_tok(TokenKind::Colon, "expected ':' after geometry property")?;
            match key.as_str() {
                "satisfies" => decl.satisfies = Some(self.take_scalar_text()?),
                "manifold" => {
                    decl.manifold = Some(self.take_scalar_text()?);
                    if self.cur.kind == TokenKind::LParen {
                        self.cur = self.lex.next_token()?;
                        while self.cur.kind != TokenKind::RParen && self.cur.kind != TokenKind::Eof {
                            let pk = self.take_any_ident()?;
                            self.expect_tok(TokenKind::Colon, "expected ':'")?;
                            let pv = self.take_scalar_text()?.parse::<u64>().unwrap_or(0);
                            if pk == "v_register" { decl.v_register = Some(pv); }
                            else if pk == "w_domain" { decl.w_domain = Some(pv); }
                            if self.cur.kind == TokenKind::Comma { self.cur = self.lex.next_token()?; }
                        }
                        self.expect_tok(TokenKind::RParen, "expected ')'")?;
                    }
                }
                "metric" => {
                    decl.metric = Some(self.take_any_ident()?);
                    if self.cur.kind == TokenKind::LParen { self.skip_balanced_parens(&mut decl.metric_axes)?; }
                }
                "cells" => {
                    let first = self.take_scalar_text()?;
                    if self.cur.kind == TokenKind::LParen {
                        self.cur = self.lex.next_token()?;
                        decl.cells_source = Some(self.take_scalar_text()?);
                        self.expect_tok(TokenKind::RParen, "expected ')' after cells source")?;
                    } else {
                        decl.cells_source = Some(first);
                    }
                }
                "operators" => decl.operators = self.take_string_or_ident_list()?,
                _ => { let _ = self.take_string_or_ident_list()?; }
            }
            if self.cur.kind == TokenKind::Semicolon { self.cur = self.lex.next_token()?; }
        }
        let end = self.expect_tok(TokenKind::RBrace, "expected '}' after geometry body")?.end;
        decl.span = Span::new(start, end);
        Ok(decl)
    }

    fn parse_harboured_graph_body(&mut self, start: u32, name: String, slot: u16, morphology: String) -> Result<HarbouredGraphDecl, Diagnostic> {
        let mut graph = HarbouredGraphDecl {
            span: Span::new(start, start), name, slot, morphology, role: "Primal1Skeleton".into(),
            anchors: Vec::new(), weights: None, edge_binding: Vec::new(), invariants: Vec::new(), boundary: None,
        };
        while self.cur.kind != TokenKind::RBrace && self.cur.kind != TokenKind::Eof {
            let key = self.take_any_ident()?;
            if key == "boundary" {
                self.expect_tok(TokenKind::LBrace, "expected '{' after boundary")?;
                let mut b = BoundaryDecl::default();
                while self.cur.kind != TokenKind::RBrace && self.cur.kind != TokenKind::Eof {
                    let bk = self.take_any_ident()?;
                    self.expect_tok(TokenKind::Colon, "expected ':' after boundary key")?;
                    let bv = self.take_scalar_text()?;
                    if bk == "sources" { b.sources = Some(bv); } else if bk == "sinks" { b.sinks = Some(bv); }
                    if self.cur.kind == TokenKind::Semicolon { self.cur = self.lex.next_token()?; }
                }
                self.expect_tok(TokenKind::RBrace, "expected '}' after boundary")?;
                graph.boundary = Some(b);
                if self.cur.kind == TokenKind::Semicolon { self.cur = self.lex.next_token()?; }
                continue;
            }
            self.expect_tok(TokenKind::Colon, "expected ':' after graph property")?;
            match key.as_str() {
                "role" => graph.role = self.take_scalar_text()?,
                "anchors" => graph.anchors = self.take_string_or_ident_list()?,
                "weights" => graph.weights = Some(self.take_scalar_text()?),
                "edge_binding" => graph.edge_binding = self.take_string_or_ident_list()?,
                "invariants" => graph.invariants = self.take_string_or_ident_list()?,
                _ => { let _ = self.take_string_or_ident_list()?; }
            }
            if self.cur.kind == TokenKind::Semicolon { self.cur = self.lex.next_token()?; }
        }
        Ok(graph)
    }

    fn expect_tok(&mut self, kind: TokenKind, msg: &str) -> Result<Span, Diagnostic> {
        if self.cur.kind == kind {
            let sp = self.cur.span;
            self.cur = self.lex.next_token()?;
            Ok(sp)
        } else {
            Err(Diagnostic::new(DiagCode::E001, self.cur.span, msg.to_string()))
        }
    }

    fn take_any_ident(&mut self) -> Result<String, Diagnostic> {
        if matches!(self.cur.kind, TokenKind::Ident | TokenKind::Keyword) {
            let s = self.cur.span.slice(self.lex.source()).to_string();
            self.cur = self.lex.next_token()?;
            Ok(s)
        } else {
            Err(Diagnostic::new(DiagCode::E001, self.cur.span, "expected identifier"))
        }
    }

    fn take_scalar_text(&mut self) -> Result<String, Diagnostic> {
        let raw = self.cur.span.slice(self.lex.source()).to_string();
        match self.cur.kind {
            TokenKind::String => {
                self.cur = self.lex.next_token()?;
                Ok(raw.trim_matches('"').to_string())
            }
            TokenKind::Ident | TokenKind::Keyword | TokenKind::Int | TokenKind::Float => {
                self.cur = self.lex.next_token()?;
                if self.cur.kind == TokenKind::Colon {
                    self.cur = self.lex.next_token()?;
                    let local = self.cur.span.slice(self.lex.source()).to_string();
                    self.cur = self.lex.next_token()?;
                    return Ok(format!("{raw}:{local}"));
                }
                Ok(raw)
            }
            _ => Err(Diagnostic::new(DiagCode::E001, self.cur.span, "expected value")),
        }
    }

    fn take_string_or_ident_list(&mut self) -> Result<Vec<String>, Diagnostic> {
        if self.cur.kind == TokenKind::LBracket {
            self.cur = self.lex.next_token()?;
            let mut items = Vec::new();
            while self.cur.kind != TokenKind::RBracket && self.cur.kind != TokenKind::Eof {
                items.push(self.take_scalar_text()?);
                if self.cur.kind == TokenKind::Comma { self.cur = self.lex.next_token()?; }
            }
            self.expect_tok(TokenKind::RBracket, "expected ']'")?;
            Ok(items)
        } else {
            Ok(vec![self.take_scalar_text()?])
        }
    }

    fn skip_balanced_parens(&mut self, axes: &mut Vec<String>) -> Result<(), Diagnostic> {
        self.expect_tok(TokenKind::LParen, "expected '('")?;
        let mut depth = 1usize;
        while depth > 0 && self.cur.kind != TokenKind::Eof {
            match self.cur.kind {
                TokenKind::LParen => { depth += 1; self.cur = self.lex.next_token()?; }
                TokenKind::RParen => { depth -= 1; self.cur = self.lex.next_token()?; }
                TokenKind::LBracket => *axes = self.take_string_or_ident_list()?,
                _ => self.cur = self.lex.next_token()?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_geometry_harbouring_multiple_graphs_and_lowers() {
        let src = r#"
        geometry NorthSpringCatchment : MixedCellComplex3D {
            satisfies: CatchmentHydrology;
            manifold: "research" (v_register: 0, w_domain: 3);
            metric: Euclidean10D(axes: [x, y, z, t, alpha, mu, sigma]);
            cells: source("catchment_corridor.10d");
            operators: [BoundaryExact, CotangentLaplacian, HodgeStar];

            graph PrimalMesh : SymmetricGraph {
                role: Primal1Skeleton;
                anchors: Cell0Vertex;
                invariants: [SYMMETRIC, SPATIAL_METRIC];
            }

            graph SubsurfaceFluxDual : DirectedMultigraph {
                role: PoincareDual;
                anchors: Cell3Volume;
                weights: HodgeStarRatio;
                invariants: [MANIFOLD_CLOSED, SPATIAL_METRIC];
            }

            graph WatershedDrainage : DeterministicDAG {
                role: CausalFlux;
                anchors: Cell2Face;
                edge_binding: ["qualia:drains_to"];
                invariants: [ACYCLIC, TRANSITIVE, TEMPORAL_MONOTONIC];
                boundary {
                    sources: "in_degree == 0";
                    sinks: "out_degree == 0";
                }
            }
        }
        "#;
        let decl = parse_geometry_or_topology(src).expect("should parse geometry block");
        assert_eq!(decl.name, "NorthSpringCatchment");
        assert_eq!(decl.complex_kind, "MixedCellComplex3D");
        assert_eq!(decl.satisfies.as_deref(), Some("CatchmentHydrology"));
        assert_eq!(decl.cells_source.as_deref(), Some("catchment_corridor.10d"));
        assert_eq!(decl.harboured_graphs.len(), 3);
        assert_eq!(decl.harboured_graphs[0].name, "PrimalMesh");
        assert_eq!(decl.harboured_graphs[1].name, "SubsurfaceFluxDual");
        assert_eq!(decl.harboured_graphs[2].name, "WatershedDrainage");
        assert_eq!(
            decl.harboured_graphs[2].boundary.as_ref().and_then(|b| b.sources.as_deref()),
            Some("in_degree == 0")
        );

        let invokes = decl.lower_to_invokes();
        assert_eq!(invokes.len(), 5);
        assert_eq!(invokes[0].capability_id, "Topology.define_geometry");
        assert_eq!(invokes[1].capability_id, "Topology.harbour_graph");
        assert_eq!(invokes[2].capability_id, "Topology.harbour_graph");
        assert_eq!(invokes[3].capability_id, "Topology.harbour_graph");
        assert_eq!(invokes[4].capability_id, "Poet.topology_bind");
    }

    #[test]
    fn parses_standalone_topology_sugar_into_metric_point_cloud() {
        let src = r#"
        topology WordNetHierarchy : DeterministicDAG {
            edge_binding: [rdfs:subClassOf, skos:broader];
            invariants: [ACYCLIC, TRANSITIVE];
        }
        "#;
        let decl = parse_geometry_or_topology(src).expect("should parse standalone topology sugar");
        assert!(decl.is_standalone_topology_sugar);
        assert_eq!(decl.name, "WordNetHierarchy");
        assert_eq!(decl.complex_kind, "MetricPointCloud10D");
        assert_eq!(decl.harboured_graphs.len(), 1);
        assert_eq!(decl.harboured_graphs[0].morphology, "DeterministicDAG");
        assert_eq!(decl.harboured_graphs[0].edge_binding, vec!["rdfs:subClassOf", "skos:broader"]);
    }
}
