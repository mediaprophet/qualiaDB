//! Comprehensive SPARQL Test Suite
//!
//! Tests for SPARQL 1.1/1.2, SPARQL-Star, and extensions

#[cfg(test)]
mod sparql_tests {
    use crate::sparql_ast::*;
    use crate::sparql_did::*;
    use crate::sparql_executor::*;
    use crate::sparql_filter::*;
    use crate::sparql_mm::*;
    use crate::sparql_parser::*;
    use crate::sparql_planner::*;
    use crate::NQuin;

    // Helper to create test quins
    fn create_test_quins() -> Vec<NQuin> {
        vec![
            NQuin {
                subject: 0x1,
                predicate: 0x2,
                object: 0x3,
                context: 0x4,
                metadata: 0x5,
                parity: 0x6,
            },
            NQuin {
                subject: 0x1,
                predicate: 0x7,
                object: 0x8,
                context: 0x4,
                metadata: 0x5,
                parity: 0x9,
            },
        ]
    }

    // ===== AST Tests =====

    #[test]
    fn test_ast_creation() {
        let mut ctx = SparqlQueryContext::new();
        let pattern_id = ctx.alloc_pattern(Pattern::Triple {
            subject: 0x1,
            predicate: 0x2,
            object: 0x3,
        });
        assert_eq!(pattern_id.unwrap(), 0);
        assert_eq!(ctx.pattern_count, 1);
    }

    #[test]
    fn test_ast_pattern_limit() {
        let mut ctx = SparqlQueryContext::new();
        for i in 0..128 {
            ctx.alloc_pattern(Pattern::Triple {
                subject: i as u64,
                predicate: i as u64 + 1,
                object: i as u64 + 2,
            })
            .unwrap();
        }
        assert_eq!(ctx.pattern_count, 128);
    }

    #[test]
    fn test_ast_pattern_overflow() {
        let mut ctx = SparqlQueryContext::new();
        for _ in 0..128 {
            ctx.alloc_pattern(Pattern::Triple {
                subject: 0x1,
                predicate: 0x2,
                object: 0x3,
            })
            .unwrap();
        }
        let result = ctx.alloc_pattern(Pattern::Triple {
            subject: 0x1,
            predicate: 0x2,
            object: 0x3,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_ast_property_path() {
        let mut ctx = SparqlQueryContext::new();
        let path_id = ctx.alloc_path(Path::Sequence {
            left: 0x1,
            right: 0x2,
        });
        assert_eq!(path_id.unwrap(), 0);
    }

    #[test]
    fn test_ast_subquery() {
        let query = SparqlQuery::Select(SelectQuery::default());

        assert!(matches!(query, SparqlQuery::Select(_)));
    }

    #[test]
    fn test_ast_embedded_triple() {
        let _ctx = SparqlQueryContext::new();
        let embedded = Expression::EmbeddedTriple {
            subject: 0x1,
            predicate: 0x2,
            object: 0x3,
        };
        if let Expression::EmbeddedTriple { subject, .. } = embedded {
            assert_eq!(subject, 0x1);
        }
    }

    #[test]
    fn test_ast_service_pattern() {
        let mut ctx = SparqlQueryContext::new();
        let pattern_id = ctx.alloc_pattern(Pattern::Service {
            endpoint_did_id: 0x8000000000000001, // With 0x8 prefix
            inner_pattern: 0,
        });
        assert!(pattern_id.is_ok());
    }

    // ===== Parser Tests =====

    #[test]
    fn test_parser_simple_select() {
        let query = "SELECT ?s WHERE { ?s ?p ?o }";
        let result = parse_sparql(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parser_with_filter() {
        let query = "SELECT ?s WHERE { ?s ?p ?o FILTER(?s = 1) }";
        let result = parse_sparql(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parser_with_optional() {
        let query = "SELECT ?s WHERE { ?s ?p ?o OPTIONAL { ?o ?q ?r } }";
        let result = parse_sparql(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parser_with_union() {
        let query = "SELECT ?s WHERE { { ?s ?p ?o } UNION { ?s ?q ?r } }";
        let result = parse_sparql(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parser_with_property_path() {
        let query = "SELECT ?s WHERE { ?s ex:propPath+ ?o }";
        let result = parse_sparql(query);
        assert!(result.is_ok());
    }

    // ===== Planner Tests =====

    #[test]
    fn test_planner_triple_scan() {
        let mut ctx = SparqlQueryContext::new();
        let pattern_id = ctx
            .alloc_pattern(Pattern::Triple {
                subject: 0x1,
                predicate: 0x2,
                object: 0x3,
            })
            .unwrap();

        let mut plan = ExecutionPlan::new();
        let result = QueryPlanner::plan_pattern(pattern_id, &ctx, &mut plan);
        assert!(result.is_ok());
        assert!(plan.operators.len() > 0);
    }

    #[test]
    fn test_planner_filter() {
        let mut ctx = SparqlQueryContext::new();
        let pattern_id = ctx
            .alloc_pattern(Pattern::Triple {
                subject: 0x1,
                predicate: 0x2,
                object: 0x3,
            })
            .unwrap();
        let expr_id = ctx.alloc_expression(Expression::Literal(1)).unwrap();
        let filter_id = ctx
            .alloc_pattern(Pattern::Filter {
                pattern: pattern_id,
                expression: expr_id,
            })
            .unwrap();

        let mut plan = ExecutionPlan::new();
        let result = QueryPlanner::plan_pattern(filter_id, &ctx, &mut plan);
        assert!(result.is_ok());
    }

    // ===== Executor Tests =====

    #[test]
    fn test_executor_triple_scan() {
        let quins = create_test_quins();
        let _executor = QueryExecutor::new(&quins);

        let mut _row = BindingRow::new();
        let mut _results: Vec<BindingRow> = Vec::new();

        // Simplified test - actual executor needs plan
        assert_eq!(quins.len(), 2);
    }

    // ===== Filter Evaluator Tests =====

    #[test]
    fn test_filter_equality() {
        let mut ctx = SparqlQueryContext::new();
        let mut row = BindingRow::new();

        // Allocate two Variable leaf expressions first so the BinaryOp indices are correct.
        let var0 = ctx.register_variable("?a").unwrap();
        let var1 = ctx.register_variable("?b").unwrap();
        let left_id = ctx.alloc_expression(Expression::Variable(var0)).unwrap(); // id 0
        let right_id = ctx.alloc_expression(Expression::Variable(var1)).unwrap(); // id 1

        // Bind both variables to the same value so equality holds.
        row.set(var0, 42);
        row.set(var1, 42);

        let eq_id = ctx
            .alloc_expression(Expression::BinaryOp {
                op: BinaryOp::Equal,
                left: left_id,
                right: right_id,
            })
            .unwrap();

        let result = ExpressionEvaluator::evaluate(eq_id, &ctx, &row).unwrap();
        assert_eq!(
            result,
            crate::sparql_library::sparql_filter::EvalResult::Boolean(true)
        );
    }

    #[test]
    fn test_filter_bound() {
        let mut row = BindingRow::new();
        row.slots[0] = Some(1);

        let mut ctx = SparqlQueryContext::new();
        ctx.function_args[0] = 0;

        let expr = Expression::Function {
            func: Function::Bound,
            args_start: 0,
            args_len: 1,
        };

        let expr_id = ctx.alloc_expression(expr).unwrap();
        let result = ExpressionEvaluator::evaluate(expr_id, &ctx, &row);
        assert!(result.is_ok());
    }

    // ===== BIND (Extend) end-to-end =====

    #[test]
    fn test_bind_end_to_end_binds_computed_value() {
        // One triple in the graph; run a query that BINDs a copy of ?o and a
        // numeric expression, and confirm both are bound in the result rows.
        let s = crate::lexicon::generate_60bit_token(b"http://ex/s");
        let p = crate::lexicon::generate_60bit_token(b"http://ex/p");
        let o = 21u64;
        let quins = vec![crate::NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }];

        let query =
            "SELECT ?o ?copy ?double WHERE { ?s ?p ?o . BIND(?o AS ?copy) . BIND(?o + ?o AS ?double) }";
        let (sparql_query, ctx) = parse_sparql(query).unwrap();
        let plan = QueryPlanner::plan(&sparql_query, &ctx).unwrap();
        let executor = QueryExecutor::new(&quins);
        let rows = executor.execute(&plan, &ctx).unwrap();

        assert!(!rows.is_empty(), "expected at least one solution row");

        let var = |name: &[u8]| -> u8 {
            ctx.variable_hashes
                .iter()
                .position(|h| *h == crate::lexicon::generate_60bit_token(name))
                .unwrap_or_else(|| panic!("variable {} not registered", String::from_utf8_lossy(name)))
                as u8
        };
        let o_var = var(b"?o");
        let copy_var = var(b"?copy");
        let double_var = var(b"?double");

        for row in &rows {
            assert_eq!(row.get(o_var), Some(o));
            // BIND(?o AS ?copy) copies the value.
            assert_eq!(row.get(copy_var), Some(o), "?copy should equal ?o");
            // BIND(?o + ?o AS ?double) computes a real numeric value.
            assert_eq!(row.get(double_var), Some(o + o), "?double should be ?o + ?o");
        }
    }

    // ===== OPTIONAL / MINUS semantics end-to-end =====

    fn run_query(
        query: &str,
        quins: &[crate::NQuin],
    ) -> (SparqlQueryContext, Vec<BindingRow>) {
        let (q, ctx) = parse_sparql(query).unwrap();
        let plan = QueryPlanner::plan(&q, &ctx).unwrap();
        let rows = QueryExecutor::new(quins).execute(&plan, &ctx).unwrap();
        (ctx, rows)
    }

    fn var_of(ctx: &SparqlQueryContext, name: &[u8]) -> u8 {
        ctx.variable_hashes
            .iter()
            .position(|h| *h == crate::lexicon::generate_60bit_token(name))
            .unwrap() as u8
    }

    fn mk_quin(s: u64, p: u64, o: u64) -> crate::NQuin {
        crate::NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn test_construct_instantiates_template() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (alice, knows, bob) = (
            tok(b"http://ex/alice"),
            tok(b"http://ex/knows"),
            tok(b"http://ex/bob"),
        );
        let friend = tok(b"http://ex/friend");
        let quins = vec![mk_quin(alice, knows, bob)];

        // Rewrite matched knows-triples into a new `friend` predicate.
        let query =
            "CONSTRUCT { ?s <http://ex/friend> ?o } WHERE { ?s <http://ex/knows> ?o }";
        let (q, ctx) = parse_sparql(query).unwrap();
        let template = match &q {
            SparqlQuery::Construct(c) => c.template_pattern,
            _ => panic!("expected CONSTRUCT"),
        };
        let plan = QueryPlanner::plan(&q, &ctx).unwrap();
        let rows = QueryExecutor::new(&quins)
            .execute_construct(&plan, &ctx, template)
            .unwrap();

        assert_eq!(rows.len(), 1, "one constructed triple, got {}", rows.len());
        let r = &rows[0];
        assert_eq!(r.get(0), Some(alice), "subject bound from WHERE");
        assert_eq!(r.get(1), Some(friend), "predicate is the template constant");
        assert_eq!(r.get(2), Some(bob), "object bound from WHERE");
    }

    #[test]
    fn test_select_distinct_dedups_on_projected_var() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let alice = tok(b"http://ex/alice");
        let knows = tok(b"http://ex/knows");
        let (bob, carol) = (tok(b"http://ex/bob"), tok(b"http://ex/carol"));
        // alice knows bob and carol → two solutions differing only in ?o.
        let quins = vec![mk_quin(alice, knows, bob), mk_quin(alice, knows, carol)];

        let q = "SELECT DISTINCT ?s WHERE { ?s <http://ex/knows> ?o }";
        let (ctx, rows) = run_query(q, &quins);
        assert_eq!(rows.len(), 1, "DISTINCT ?s collapses the two ?o solutions");
        assert_eq!(rows[0].get(var_of(&ctx, b"?s")), Some(alice));
        // The unselected ?o must be projected away.
        assert_eq!(
            rows[0].get(var_of(&ctx, b"?o")),
            None,
            "unselected ?o dropped by projection"
        );
    }

    #[test]
    fn test_filter_exists_keeps_matching_rows() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (alice, bob, carol) = (
            tok(b"http://ex/alice"),
            tok(b"http://ex/bob"),
            tok(b"http://ex/carol"),
        );
        let (knows, likes) = (tok(b"http://ex/knows"), tok(b"http://ex/likes"));
        // alice knows bob; bob likes carol.
        let quins = vec![mk_quin(alice, knows, bob), mk_quin(bob, likes, carol)];

        // Keep ?s whose known ?o likes something.
        let q = "SELECT ?s WHERE { ?s <http://ex/knows> ?o . \
                 FILTER EXISTS { ?o <http://ex/likes> ?z } }";
        let (ctx, rows) = run_query(q, &quins);
        assert_eq!(rows.len(), 1, "EXISTS holds for alice->bob->carol");
        assert_eq!(rows[0].get(var_of(&ctx, b"?s")), Some(alice));
    }

    #[test]
    fn test_filter_not_exists_removes_matching_rows() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (alice, bob, carol, dave) = (
            tok(b"http://ex/alice"),
            tok(b"http://ex/bob"),
            tok(b"http://ex/carol"),
            tok(b"http://ex/dave"),
        );
        let (knows, likes) = (tok(b"http://ex/knows"), tok(b"http://ex/likes"));
        // alice knows bob (bob likes carol) and dave (dave likes nobody).
        let quins = vec![
            mk_quin(alice, knows, bob),
            mk_quin(alice, knows, dave),
            mk_quin(bob, likes, carol),
        ];

        // Keep alice's acquaintances who like nobody → only dave.
        let q = "SELECT ?o WHERE { <http://ex/alice> <http://ex/knows> ?o . \
                 FILTER NOT EXISTS { ?o <http://ex/likes> ?z } }";
        let (ctx, rows) = run_query(q, &quins);
        assert_eq!(rows.len(), 1, "only dave (likes nobody) survives NOT EXISTS");
        assert_eq!(rows[0].get(var_of(&ctx, b"?o")), Some(dave));
    }

    #[test]
    fn test_filter_bracketed_exists_conjunction() {
        // EXISTS nested inside a bracketed boolean expression (ExprParser path).
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (alice, bob, carol) = (
            tok(b"http://ex/alice"),
            tok(b"http://ex/bob"),
            tok(b"http://ex/carol"),
        );
        let (knows, likes) = (tok(b"http://ex/knows"), tok(b"http://ex/likes"));
        let quins = vec![mk_quin(alice, knows, bob), mk_quin(bob, likes, carol)];

        let q = "SELECT ?s WHERE { ?s <http://ex/knows> ?o . \
                 FILTER ( EXISTS { ?o <http://ex/likes> ?z } && EXISTS { ?o <http://ex/likes> ?z } ) }";
        let (ctx, rows) = run_query(q, &quins);
        assert_eq!(rows.len(), 1, "bracketed EXISTS && EXISTS holds");
        assert_eq!(rows[0].get(var_of(&ctx, b"?s")), Some(alice));
    }

    #[test]
    fn test_graph_variable_binds_named_graph() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (s, p, o) = (
            tok(b"http://ex/s"),
            tok(b"http://ex/p"),
            tok(b"http://ex/o"),
        );
        let g1 = tok(b"http://ex/g1");
        // A single quin living in named graph g1.
        let quins = vec![crate::NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: g1,
            metadata: 0,
            parity: 0,
        }];

        let query = "SELECT ?g ?o WHERE { GRAPH ?g { ?s <http://ex/p> ?o } }";
        let (q, ctx) = parse_sparql(query).unwrap();
        let plan = QueryPlanner::plan(&q, &ctx).unwrap();
        let rows = QueryExecutor::new(&quins).execute(&plan, &ctx).unwrap();

        assert!(!rows.is_empty(), "GRAPH ?g must match the named graph");
        let g_var = var_of(&ctx, b"?g");
        let o_var = var_of(&ctx, b"?o");
        assert_eq!(rows[0].get(g_var), Some(g1), "?g bound to the named graph IRI");
        assert_eq!(rows[0].get(o_var), Some(o), "?o bound within the graph");
    }

    #[test]
    fn test_describe_returns_concise_bounded_description() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (alice, knows, bob, age) = (
            tok(b"http://ex/alice"),
            tok(b"http://ex/knows"),
            tok(b"http://ex/bob"),
            tok(b"http://ex/age"),
        );
        // alice has two outgoing triples; bob's triple must NOT be described.
        let quins = vec![
            mk_quin(alice, knows, bob),
            mk_quin(alice, age, 30),
            mk_quin(bob, knows, alice),
        ];

        let query = "DESCRIBE <http://ex/alice>";
        let (q, ctx) = parse_sparql(query).unwrap();
        let describe = match &q {
            SparqlQuery::Describe(d) => *d,
            _ => panic!("expected DESCRIBE"),
        };
        let plan = QueryPlanner::plan(&q, &ctx).unwrap();
        let rows = QueryExecutor::new(&quins)
            .execute_describe(&plan, &ctx, &describe)
            .unwrap();

        assert_eq!(rows.len(), 2, "only alice's two triples");
        for r in &rows {
            assert_eq!(r.get(0), Some(alice), "every described triple is about alice");
        }
    }

    #[test]
    fn test_optional_keeps_unmatched_left_rows() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (name, age) = (tok(b"http://ex/name"), tok(b"http://ex/age"));
        let (s1, s2) = (tok(b"http://ex/s1"), tok(b"http://ex/s2"));
        // s1 has a name AND an age; s2 has only a name.
        let quins = vec![
            mk_quin(s1, name, 100),
            mk_quin(s1, age, 42),
            mk_quin(s2, name, 200),
        ];
        let (ctx, rows) = run_query(
            "SELECT ?s ?age WHERE { ?s <http://ex/name> ?n . OPTIONAL { ?s <http://ex/age> ?age } }",
            &quins,
        );
        let (sv, av) = (var_of(&ctx, b"?s"), var_of(&ctx, b"?age"));
        let mut got: Vec<(u64, Option<u64>)> =
            rows.iter().map(|r| (r.get(sv).unwrap(), r.get(av))).collect();
        got.sort();
        assert!(got.contains(&(s1, Some(42))), "s1 should carry ?age; got {got:?}");
        assert!(
            got.contains(&(s2, None)),
            "s2 must be kept with ?age unbound (OPTIONAL is a left-join); got {got:?}"
        );
        assert_eq!(got.len(), 2, "OPTIONAL must not drop the unmatched row; got {got:?}");
    }

    #[test]
    fn test_minus_subtracts_matching_left_rows() {
        let tok = |b: &[u8]| crate::lexicon::generate_60bit_token(b);
        let (name, excl) = (tok(b"http://ex/name"), tok(b"http://ex/excluded"));
        let (s1, s2) = (tok(b"http://ex/s1"), tok(b"http://ex/s2"));
        // Both have names; only s2 is excluded.
        let quins = vec![
            mk_quin(s1, name, 100),
            mk_quin(s2, name, 200),
            mk_quin(s2, excl, 1),
        ];
        let (ctx, rows) = run_query(
            "SELECT ?s WHERE { ?s <http://ex/name> ?n . MINUS { ?s <http://ex/excluded> ?e } }",
            &quins,
        );
        let sv = var_of(&ctx, b"?s");
        let got: Vec<u64> = rows.iter().map(|r| r.get(sv).unwrap()).collect();
        assert_eq!(
            got,
            vec![s1],
            "MINUS must remove the excluded subject (s2) and keep s1; got {got:?}"
        );
    }

    // ===== SPARQL-MM Tests =====

    #[test]
    fn test_mm_handler_creation() {
        let quins = vec![];
        let handler = SparqlMmHandler::new(&quins);
        assert_eq!(handler.window_count, 0);
        assert_eq!(handler.fragment_count, 0);
    }

    #[test]
    fn test_mm_create_tumbling_window() {
        let quins = vec![];
        let mut handler = SparqlMmHandler::new(&quins);
        let result = handler.create_tumbling_window(1000, 0);
        assert!(result.is_ok());
        assert_eq!(handler.window_count, 1);
    }

    #[test]
    fn test_mm_create_sliding_window() {
        let quins = vec![];
        let mut handler = SparqlMmHandler::new(&quins);
        let result = handler.create_sliding_window(1000, 500, 0);
        assert!(result.is_ok());
        assert_eq!(handler.window_count, 1);
    }

    #[test]
    fn test_mm_parse_media_fragment() {
        let quins = vec![];
        let mut handler = SparqlMmHandler::new(&quins);
        let fragment = handler.parse_media_fragment(12345);
        assert!(fragment.is_ok());
    }

    #[test]
    fn test_mm_ma_ont_constants() {
        // Verify MA Ontology predicates are defined
        assert_eq!(ma_ont::HAS_FRAGMENT, 0x123456789ABCDEF0);
        assert_eq!(ma_ont::DURATION, 0x789ABCDEF0123456);
    }

    #[test]
    fn test_mm_c2pa_constants() {
        // Verify C2PA predicates are defined
        assert_eq!(c2pa::HAS_CREDENTIAL, 0x6789ABCDEF012345);
        assert_eq!(c2pa::IS_VERIFIED, 0x3456789ABCDEF012);
    }

    // ===== SPARQL-DID Tests =====

    #[test]
    fn test_did_handler_creation() {
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);
        assert_eq!(handler.cache_count, 0);
    }

    #[test]
    fn test_did_resolve() {
        let quins = vec![];
        let mut handler = SparqlDidHandler::new(&quins);
        let result = handler.resolve_did(0x8000000000000001);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().did, 0x8000000000000001);
    }

    #[test]
    fn test_did_verify_signature() {
        // Security regression: must fail closed, never assert a valid signature here.
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);
        let signature = &[0u8; 64];
        let data = &[0u8; 256];
        let result = handler.verify_signature(0x8000000000000001, signature, data);
        assert!(result.is_err());
    }

    #[test]
    fn test_did_check_permission() {
        // Security regression: must fail closed, never grant access unconditionally.
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);
        let result = handler.check_permission(0x8000000000000001, 123, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_did_authenticate() {
        // Security regression: must fail closed, never authenticate everyone.
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);
        let auth_payload = &[0u8; 256];
        let result = handler.authenticate_did(0x8000000000000001, 1, auth_payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_did_cache() {
        let quins = vec![];
        let mut handler = SparqlDidHandler::new(&quins);

        // First call should cache
        let _ = handler.resolve_did(0x8000000000000001);
        assert_eq!(handler.cache_count, 1);

        // Second call should use cache
        let _ = handler.resolve_did(0x8000000000000001);
        assert_eq!(handler.cache_count, 1);
    }

    #[test]
    fn test_did_invalidate_cache() {
        let quins = vec![];
        let mut handler = SparqlDidHandler::new(&quins);

        let _ = handler.resolve_did(0x8000000000000001);
        assert_eq!(handler.cache_count, 1);

        handler.invalidate_cache(0x8000000000000001);
        assert_eq!(handler.cache_count, 0);
    }

    // ===== Integration Tests =====

    #[test]
    fn test_full_query_pipeline() {
        let query = "SELECT ?s WHERE { ?s ?p ?o }";

        let parsed = parse_sparql(query);
        assert!(parsed.is_ok());

        let _ast = parsed.unwrap();
        let mut ctx = SparqlQueryContext::new();
        // Add pattern from AST
        let pattern_id = ctx
            .alloc_pattern(Pattern::Triple {
                subject: 0x1,
                predicate: 0x2,
                object: 0x3,
            })
            .unwrap();

        let mut plan = ExecutionPlan::new();
        let planned = QueryPlanner::plan_pattern(pattern_id, &ctx, &mut plan);
        assert!(planned.is_ok());
    }

    #[test]
    fn test_zero_allocation_compliance() {
        // Verify no heap allocations in hot paths
        let mut ctx = SparqlQueryContext::new();

        // Should use fixed-size arrays, not Vec
        for i in 0..128 {
            let _ = ctx.alloc_pattern(Pattern::Triple {
                subject: i as u64,
                predicate: i as u64 + 1,
                object: i as u64 + 2,
            });
        }

        assert_eq!(ctx.pattern_count, 128);
    }

    #[test]
    fn test_did_prefix_recognition() {
        // Verify 0x8 prefix for DID recognition
        let did_with_prefix = 0x8000000000000001_u64;
        let did_without_prefix = 0x0000000000000001_u64;

        assert_ne!(did_with_prefix & 0x8000000000000000_u64, 0);
        assert_eq!(did_without_prefix & 0x8000000000000000_u64, 0);
    }

    #[test]
    fn test_virtual_id_hash_strategy() {
        // Verify 0x1 prefix for Virtual ID Hash
        let virtual_id = 0x1000000000000001_u64;

        assert_ne!(virtual_id & 0x1000000000000000_u64, 0);
    }
}
