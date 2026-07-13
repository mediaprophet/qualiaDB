//! Recursive-descent SPARQL grammar — the parser front-end the engine was
//! missing.
//!
//! The AST (`sparql_ast`), planner (`sparql_planner`), and executor
//! (`sparql_executor` + `sparql_filter`) already implement the full SPARQL
//! algebra: `Filter`, `Optional`, `Union`, `Minus`, `Bind`/`Project`,
//! `GroupBy`, `Having`, `PropertyPath`, `Graph`, `Service`, `StarTripleScan`,
//! and a complete `Expression` evaluator. What was missing was a parser that
//! produces that AST from a query string — the legacy `sparql_parser.rs` only
//! recognised a flat basic graph pattern. This module supplies the real thing,
//! built in verified slices (see `docs/plans/sparql-full-implementation.md`).

pub mod expr;
pub mod pattern;
pub mod tokenizer;
pub mod update;

pub use expr::parse_expression;
pub use pattern::parse_where_group;
pub use tokenizer::{tokenize, Token};
pub use update::{is_update, parse_update};
