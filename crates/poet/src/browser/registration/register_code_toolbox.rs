//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_code_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:place_vibe".into(),
                label: "+ VibeScript Cell".into(),
                icon: "vibe".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "vibe".into(),
                description: "Place a reactive VibeScript cell container.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:quin_statement".into(),
                label: "quin.statement".into(),
                icon: "quin".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "vibe".into(),
                description: "Construct a quin.statement.".into(),
            },
            ActionType::Mutate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "code".into(),
            label: "Code IDE & Vibe REPL".into(),
            icon: "code".into(),
            ontology_prefix: "vibe".into(),
            description: "VibeScript 0.1, WebGPU WGSL, SPARQL, and reactive AST cells.".into(),
            enabled_by_default: true,
            family: "code".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:repl".into(),
                    label: "Runtime & Language Dialects".into(),
                    icon: "code".into(),
                    description:
                        "Select language dialect (Vibe/WGSL/Turtle) and configure gas budget."
                            .into(),
                },
                vec![Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "code:vibe_diagnose".into(),
                        label: "Diagnose Vibe".into(),
                        icon: "vibe".into(),
                        kind: ToolKind::Query,
                        capability_scope: None,
                        ontology_prefix: "vibe".into(),
                        description: "Frozen four-op diagnose on the selected VibeScript source."
                            .into(),
                    },
                    ActionType::Query,
                ))],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:tools".into(),
                    label: "IDE Cells & Statements".into(),
                    icon: "tools".into(),
                    description: "Place VibeScript cells and construct quin statements.".into(),
                },
                tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:logic".into(),
                    label: "Symbolic & Temporal".into(),
                    icon: "logic".into(),
                    description: "Evaluate formulas and check properties over time.".into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:ltl_evaluate".into(),
                            label: "Check over time".into(),
                            icon: "ltl".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "TemporalAndDescriptionLogic.ltl.evaluate".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description: "Evaluate an LTL Globally formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_eval".into(),
                            label: "Work out the formula".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.eval".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Evaluate a formula from data-formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_differentiate".into(),
                            label: "Differentiate".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.differentiate".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Differentiate the formula on the selected surface with respect to data-var."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_simplify".into(),
                            label: "Simplify".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.simplify".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Simplify the formula on the selected surface.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_expand".into(),
                            label: "Expand".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.expand".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Expand products and powers on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_factor".into(),
                            label: "Factor quadratic".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.factor".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Factor a real quadratic from data-a / data-b / data-c on the surface."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_integrate".into(),
                            label: "Integrate".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.integrate".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Indefinite integral of the formula on the selected surface."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_simplify_trig".into(),
                            label: "Simplify trig".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.simplify_trig".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Apply trig identity rewrites to the formula on the surface."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_partial".into(),
                            label: "Partial derivative".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.partial".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Partial derivative of the formula with respect to data-var."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_limit".into(),
                            label: "Limit".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.limit".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Limit of the formula as the variable approaches data-limit-at."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_add".into(),
                            label: "Add".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.add".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Build an addition node from data-a / data-b (or formula + 1)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_sub".into(),
                            label: "Subtract".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.sub".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Build a subtraction node from data-a / data-b (or formula − 1)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_mul".into(),
                            label: "Multiply".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.mul".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Build a multiplication node from data-a / data-b (or formula × 1)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_div".into(),
                            label: "Divide".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.div".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Build a division node from data-a / data-b (or formula / 1)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_pow".into(),
                            label: "Power".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.pow".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Raise the formula to an integer power from data-exp (default 2)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_neg".into(),
                            label: "Negate".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.neg".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Unary minus of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_sqrt".into(),
                            label: "Square root".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.sqrt".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Square-root node of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_exp".into(),
                            label: "Exponential".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.exp".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Exponential node of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_ln".into(),
                            label: "Natural log".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.ln".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Natural-log node of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_sin".into(),
                            label: "Sine".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.sin".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Sine node of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_cos".into(),
                            label: "Cosine".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.cos".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Cosine node of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_tan".into(),
                            label: "Tangent".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.tan".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Tangent node of the formula on the selected surface."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_parse".into(),
                            label: "Parse formula".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.parse".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Parse the formula on the selected surface into canonical form."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_c".into(),
                            label: "Constant".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.c".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Build a constant leaf from data-value (default 1) on the surface."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_var".into(),
                            label: "Variable".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.var".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Build a variable leaf from data-name or data-var on the surface."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_hessian".into(),
                            label: "Hessian".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.hessian".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Symbolic Hessian of the formula over data-vars (or data-var)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_integrate_definite".into(),
                            label: "Definite integral".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.integrate_definite".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Definite integral of the formula from data-from/data-to (or data-a/data-b)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_limit_at_infinity".into(),
                            label: "Limit at infinity".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.limit_at_infinity".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Limit of the formula as the variable tends to infinity."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_real_roots".into(),
                            label: "Real roots".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.real_roots".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Real roots of a descending-coeff polynomial from data-coeffs."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_roots".into(),
                            label: "Complex roots".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.roots".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Complex roots of a descending-coeff polynomial from data-coeffs."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_taylor_coefficients".into(),
                            label: "Taylor coefficients".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.taylor_coefficients".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Taylor coefficients of the formula about data-center / data-order."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_taylor_eval".into(),
                            label: "Taylor evaluate".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.taylor_eval".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Evaluate a truncated Taylor series from data-coeffs at data-x."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_jacobian".into(),
                            label: "Jacobian".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.jacobian".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Symbolic Jacobian of data-exprs (pipe-separated) over data-vars."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_gradient_at".into(),
                            label: "Gradient at point".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.gradient_at".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Numeric gradient of the formula at data-point over data-vars."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_hessian_at".into(),
                            label: "Hessian at point".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.hessian_at".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Numeric Hessian of the formula at data-point over data-vars."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_solve_quadratic".into(),
                            label: "Solve quadratic".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.solve_quadratic".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Symbolic+numeric roots of a·x²+b·x+c from data-a/data-b/data-c."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_solve_quadratic_symbolic".into(),
                            label: "Solve quadratic (exact)".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "SymbolicAlgebra.solve_quadratic_symbolic".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Exact symbolic roots of a·x²+b·x+c from data-a/data-b/data-c."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_factor_quadratic".into(),
                            label: "Factor quadratic".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.factor_quadratic".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Real factorisation a(x−r₁)(x−r₂) from data-a/data-b/data-c."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_solve_polynomial_expr".into(),
                            label: "Solve polynomial expr".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "SymbolicAlgebra.solve_polynomial_expr".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Real roots of a polynomial formula using data-degree / data-tol."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_simplify_with_assumptions".into(),
                            label: "Simplify with assumptions".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "SymbolicAlgebra.simplify_with_assumptions".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Sign-gated simplify using data-assumptions (var:sign list)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_expr_citation_hash".into(),
                            label: "Expression citation hash".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.expr_citation_hash".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "FNV-1a citation hash of the formula on this surface.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_to_quins".into(),
                            label: "Encode to quins".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.to_quins".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Serialise the formula into post-order quin records.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:symbolic_from_quins".into(),
                            label: "Decode from quins".into(),
                            icon: "formula".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SymbolicAlgebra.from_quins".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Reconstruct an expression from data-quins JSON records.".into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:constr".into(),
                    label: "Constructibility".into(),
                    icon: "logic".into(),
                    description:
                        "Compass-and-straightedge feasibility: Gauss–Wantzel, Wantzel degree, classical impossibilities."
                            .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_regular_polygon".into(),
                            label: "Regular polygon".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.is_regular_polygon_constructible".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Gauss–Wantzel test for a regular n-gon from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_fermat_prime".into(),
                            label: "Fermat prime".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("Constructibility.is_fermat_prime".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Test whether data-n is a Fermat prime.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_min_poly_degree".into(),
                            label: "Wantzel degree".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.constructible_from_min_poly_degree".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Wantzel gate: constructible iff data-degree is a power of two."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_power_of_two".into(),
                            label: "Power of two".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("Constructibility.is_power_of_two".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Test whether data-n is a power of two.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_central_angle".into(),
                            label: "Central angle".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.is_central_angle_constructible".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Test whether the central angle 2π/n is constructible.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_doubling_cube".into(),
                            label: "Doubling the cube".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.doubling_the_cube_constructible".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Classical impossibility: doubling the cube with compass and straightedge."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_trisect_angle".into(),
                            label: "Trisect angle".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.trisecting_general_angle_constructible".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Classical impossibility: trisecting a general angle.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_square_circle".into(),
                            label: "Square the circle".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.squaring_the_circle_constructible".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Classical impossibility: squaring the circle.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:constr_number".into(),
                            label: "Constructible number".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some(
                                "Constructibility.is_constructible_number".into(),
                            ),
                            ontology_prefix: "vibe".into(),
                            description:
                                "CAS constructibility verdict for the formula on this surface."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:nt".into(),
                    label: "Number Theory".into(),
                    icon: "logic".into(),
                    description:
                        "Pure integer kernels: gcd, primality, totient, modular arithmetic, partitions, Stirling, CRT."
                            .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_gcd".into(),
                            label: "GCD".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.gcd".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Greatest common divisor from data-a and data-b.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_lcm".into(),
                            label: "LCM".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.lcm".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Least common multiple from data-a and data-b.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_is_prime".into(),
                            label: "Is prime".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.is_prime".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Primality test for data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_factorial".into(),
                            label: "Factorial".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.factorial".into()),
                            ontology_prefix: "vibe".into(),
                            description: "n! from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_binomial".into(),
                            label: "Binomial".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.binomial".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Binomial C(n, k) from data-n and data-k.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_euler_totient".into(),
                            label: "Euler totient".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.euler_totient".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Euler's φ(n) from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_mod_pow".into(),
                            label: "Mod pow".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.mod_pow".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Modular exponentiation from data-base, data-exp, data-modulus."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_next_prime".into(),
                            label: "Next prime".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.next_prime".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Smallest prime strictly greater than data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_mod_inverse".into(),
                            label: "Mod inverse".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.mod_inverse".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Modular multiplicative inverse from data-a and data-modulus."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_divisor_count".into(),
                            label: "Divisor count".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.divisor_count".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Number of positive divisors d(n) from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_prime_factors".into(),
                            label: "Prime factors".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.prime_factors".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Prime factorization of data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_divisors".into(),
                            label: "Divisors".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.divisors".into()),
                            ontology_prefix: "vibe".into(),
                            description: "List positive divisors of data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_mobius".into(),
                            label: "Möbius".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.mobius".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Möbius function μ(n) from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_divisor_sum".into(),
                            label: "Divisor sum".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.divisor_sum".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Sum of positive divisors σ(n) from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_partitions".into(),
                            label: "Partitions".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.partitions".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Partition function p(n) from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_catalan".into(),
                            label: "Catalan".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.catalan".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Catalan number C_n from data-n.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_stirling_second".into(),
                            label: "Stirling 2nd".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.stirling_second".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Stirling number of the second kind S(n,k).".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_stirling_first".into(),
                            label: "Stirling 1st".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.stirling_first".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Unsigned Stirling number of the first kind c(n,k)."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_extended_gcd".into(),
                            label: "Extended GCD".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.extended_gcd".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Bézout coefficients for data-a and data-b.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:nt_crt".into(),
                            label: "CRT".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("NumberTheory.crt".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Chinese Remainder for coprime moduli (data-r1/m1/r2/m2)."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:fuzzy".into(),
                    label: "Fuzzy Query".into(),
                    icon: "logic".into(),
                    description:
                        "Membership degrees, α-cuts, top-k ranking, and t-norm / t-conorm algebra."
                            .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_triangular".into(),
                            label: "Triangular".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.triangular".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Triangular membership from data-x / data-a / data-m / data-b."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_trapezoidal".into(),
                            label: "Trapezoidal".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.trapezoidal".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Trapezoidal membership from data-x and data-a..data-d.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_approximately".into(),
                            label: "Approximately".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.approximately".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Approximately-target membership from data-x / data-target / data-tol."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_ramp_up".into(),
                            label: "Ramp up".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.ramp_up".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Rising ramp membership from data-x / data-a / data-b."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_ramp_down".into(),
                            label: "Ramp down".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.ramp_down".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Falling ramp membership from data-x / data-a / data-b."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_much_greater_than".into(),
                            label: "Much greater".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.much_greater_than".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Much-greater-than membership from data-x / data-reference / data-spread."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_much_less_than".into(),
                            label: "Much less".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.much_less_than".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Much-less-than membership from data-x / data-reference / data-spread."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_threshold".into(),
                            label: "Threshold".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.threshold".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "α-cut over data-degrees with data-alpha.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_top_k".into(),
                            label: "Top k".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.top_k".into()),
                            ontology_prefix: "vibe".into(),
                            description: "Keep the k highest degrees from data-degrees / data-k."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_negate".into(),
                            label: "Negate".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.negate".into()),
                            ontology_prefix: "vibe".into(),
                            description:
                                "Complement degrees under optional data-norm.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_and".into(),
                            label: "And".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.and".into()),
                            ontology_prefix: "vibe".into(),
                            description: "t-norm of data-a and data-b (optional data-norm)."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "code:fuzzy_or".into(),
                            label: "Or".into(),
                            icon: "logic".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("FuzzyQuery.or".into()),
                            ontology_prefix: "vibe".into(),
                            description: "t-conorm of data-a and data-b (optional data-norm)."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
        ],
    ));
}
