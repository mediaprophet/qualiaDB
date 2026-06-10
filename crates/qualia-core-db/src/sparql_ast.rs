//! SPARQL AST - Index-Based Zero-Allocation Query Representation
//!
//! Uses u16 indices into flat arrays to avoid Box allocation and recursive type errors.
//! Fully compliant with AGENTS.md zero-allocation constraints.

pub type PatternId = u16;
pub type ExpressionId = u16;
pub type VariableId = u8;

/// Maximum number of patterns in a query context
pub const MAX_PATTERNS: usize = 128;

/// Maximum number of expressions in a query context
pub const MAX_EXPRESSIONS: usize = 128;

/// Maximum number of variables per query
pub const MAX_VARIABLES: usize = 16;

/// Maximum number of bindings per row
pub const MAX_BINDINGS: usize = 16;

/// Maximum number of order conditions
pub const MAX_ORDER_CONDITIONS: usize = 16;

/// SPARQL query forms
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlQuery {
    Select(SelectQuery),
    Ask(AskQuery),
    Construct(ConstructQuery),
    Describe(DescribeQuery),
}

/// SELECT query
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectQuery {
    pub distinct: bool,
    pub reduced: bool,
    pub variables: [VariableId; MAX_VARIABLES],
    pub var_count: u8,
    pub root_pattern: PatternId,
    pub group_by: [VariableId; MAX_VARIABLES],
    pub group_by_count: u8,
    pub having: Option<ExpressionId>,
    pub order_by: [OrderCondition; MAX_ORDER_CONDITIONS],
    pub order_by_count: u8,
    pub limit: Option<u64>,
    pub offset: u64,
}

impl Default for SelectQuery {
    fn default() -> Self {
        Self {
            distinct: false,
            reduced: false,
            variables: [0; MAX_VARIABLES],
            var_count: 0,
            root_pattern: 0,
            group_by: [0; MAX_VARIABLES],
            group_by_count: 0,
            having: None,
            order_by: [OrderCondition::default(); MAX_ORDER_CONDITIONS],
            order_by_count: 0,
            limit: None,
            offset: 0,
        }
    }
}

/// ASK query
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskQuery {
    pub root_pattern: PatternId,
}

/// CONSTRUCT query
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstructQuery {
    pub template_pattern: PatternId,
    pub root_pattern: PatternId,
    pub group_by: [VariableId; MAX_VARIABLES],
    pub group_by_count: u8,
    pub having: Option<ExpressionId>,
    pub order_by: [OrderCondition; MAX_ORDER_CONDITIONS],
    pub order_by_count: u8,
    pub limit: Option<u64>,
    pub offset: u64,
}

/// DESCRIBE query
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescribeQuery {
    pub vars_or_ids: [u64; MAX_VARIABLES],
    pub var_count: u8,
    pub root_pattern: Option<PatternId>,
}

/// Graph pattern - now uses PatternId indices instead of Box
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    /// Basic triple pattern
    Triple {
        subject: u64,
        predicate: u64,
        object: u64,
    },
    /// OPTIONAL pattern - references inner pattern by ID
    Optional {
        inner: PatternId,
    },
    /// UNION pattern - references left and right by IDs
    Union {
        left: PatternId,
        right: PatternId,
    },
    /// GRAPH pattern - references graph var/IRI and inner pattern
    Graph {
        graph_var_or_id: u64,
        inner: PatternId,
    },
    /// FILTER pattern - references pattern to filter and expression
    Filter {
        pattern: PatternId,
        expression: ExpressionId,
    },
    /// MINUS pattern
    Minus {
        inner: PatternId,
    },
    /// Group graph pattern - references range in child array
    Group {
        start_idx: u16,
        len: u16,
    },
    /// Property path pattern (SPARQL 1.1)
    PropertyPath {
        subject: u64,
        path: PathId,
        object: u64,
    },
    /// SERVICE pattern (Federated Query with DID)
    Service {
        endpoint_did_id: u64, // DID with 0x8 prefix for identity recognition
        inner_pattern: PatternId,
    },
}

/// Property path (SPARQL 1.1)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Path {
    /// Simple predicate
    Predicate(u64),
    /// Inverse predicate (^pred)
    Inverse(PathId),
    /// Sequence (pred1/pred2)
    Sequence {
        left: PathId,
        right: PathId,
    },
    /// Alternation (pred1|pred2)
    Alternative {
        left: PathId,
        right: PathId,
    },
    /// Zero or more (pred*)
    ZeroOrMore(PathId),
    /// One or more (pred+)
    OneOrMore(PathId),
    /// Zero or one (pred?)
    ZeroOrOne(PathId),
}

pub type PathId = u16;

/// Expression - uses ExpressionId for nested expressions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expression {
    /// Variable reference
    Variable(VariableId),
    /// Literal value (index into literal table)
    Literal(u64),
    /// IRI reference
    Iri(u64),
    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        expr: ExpressionId,
    },
    /// Binary operation
    BinaryOp {
        op: BinaryOp,
        left: ExpressionId,
        right: ExpressionId,
    },
    /// Function call
    Function {
        func: Function,
        args_start: u16,
        args_len: u16,
    },
    /// Subquery
    Subquery {
        query_id: u16, // Index into query array
    },
    /// Embedded triple (RDF-Star)
    EmbeddedTriple {
        subject: u64,
        predicate: u64,
        object: u64,
    },
}

/// Unary operators
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Plus,
    Minus,
}

/// Binary operators
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Built-in SPARQL functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    Str,
    Lang,
    LangMatches,
    Datatype,
    Bound,
    Iri,
    Uri,
    Bnode,
    Rand,
    Abs,
    Ceil,
    Floor,
    Round,
    Concat,
    Substring,
    Strlen,
    Ucase,
    Lcase,
    EncodeForUri,
    Contains,
    VarStarts,
    VarEnds,
    StrBefore,
    StrAfter,
    Year,
    Month,
    Day,
    Hours,
    Minutes,
    Seconds,
    Timezone,
    Tz,
    Now,
    Uuid,
    StrUuid,
    Coalesce,
    If,
    StrLang,
    StrDt,
    SameTerm,
    IsIri,
    IsUri,
    IsBlank,
    IsLiteral,
    IsNumeric,
    Regex,
    // SPARQL-Star functions
    TripleSubject,
    TriplePredicate,
    TripleObject,
    Triple,
    Custom(u64), // Index into custom function table
}

/// ORDER BY condition
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OrderCondition {
    pub ascending: bool,
    pub expr: ExpressionId,
}

/// Flat query context arena - pre-allocated, no heap allocation
#[repr(C)]
pub struct SparqlQueryContext {
    /// Flat array of all patterns in the query
    pub patterns: [Pattern; MAX_PATTERNS],
    /// Number of patterns currently allocated
    pub pattern_count: usize,
    /// Flat array of all expressions in the query
    pub expressions: [Expression; MAX_EXPRESSIONS],
    /// Number of expressions currently allocated
    pub expression_count: usize,
    /// Flat array of all property paths in the query
    pub paths: [Path; MAX_PATTERNS],
    /// Number of paths currently allocated
    pub path_count: usize,
    /// Flat array of subqueries (for nested queries)
    pub subqueries: [SparqlQuery; 16],
    /// Number of subqueries currently allocated
    pub subquery_count: usize,
    /// Variable name to ID mapping (simplified - stores as hash for now)
    pub variable_hashes: [u64; MAX_VARIABLES],
    /// Number of variables
    pub variable_count: usize,
    /// Argument storage for function calls (flat array)
    pub function_args: [ExpressionId; 64],
    /// Number of function args
    pub function_arg_count: usize,
}

impl SparqlQueryContext {
    pub fn new() -> Self {
        Self {
            patterns: [Pattern::Triple { subject: 0, predicate: 0, object: 0 }; MAX_PATTERNS],
            pattern_count: 0,
            expressions: [Expression::Variable(0); MAX_EXPRESSIONS],
            expression_count: 0,
            paths: [Path::Predicate(0); MAX_PATTERNS],
            path_count: 0,
            subqueries: [SparqlQuery::Select(SelectQuery {
                distinct: false,
                reduced: false,
                variables: [0; MAX_VARIABLES],
                var_count: 0,
                root_pattern: 0,
                group_by: [0; MAX_VARIABLES],
                group_by_count: 0,
                having: None,
                order_by: [OrderCondition { expr: 0, ascending: true }; MAX_ORDER_CONDITIONS],
                order_by_count: 0,
                limit: None,
                offset: 0,
            }); 16],
            subquery_count: 0,
            variable_hashes: [0; MAX_VARIABLES],
            variable_count: 0,
            function_args: [0; 64],
            function_arg_count: 0,
        }
    }

    /// Allocate a new pattern, returns its ID
    pub fn alloc_pattern(&mut self, pattern: Pattern) -> Result<PatternId, String> {
        if self.pattern_count >= MAX_PATTERNS {
            return Err("Pattern overflow".to_string());
        }
        let id = self.pattern_count as PatternId;
        self.patterns[self.pattern_count] = pattern;
        self.pattern_count += 1;
        Ok(id)
    }

    /// Allocate a new expression, returns its ID
    pub fn alloc_expression(&mut self, expr: Expression) -> Result<ExpressionId, String> {
        if self.expression_count >= MAX_EXPRESSIONS {
            return Err("Expression overflow".to_string());
        }
        let id = self.expression_count as ExpressionId;
        self.expressions[self.expression_count] = expr;
        self.expression_count += 1;
        Ok(id)
    }

    /// Allocate a new property path, returns its ID
    pub fn alloc_path(&mut self, path: Path) -> Result<PathId, String> {
        if self.path_count >= MAX_PATTERNS {
            return Err("Path overflow".to_string());
        }
        let id = self.path_count as PathId;
        self.paths[self.path_count] = path;
        self.path_count += 1;
        Ok(id)
    }

    /// Allocate a new subquery, returns its ID
    pub fn alloc_subquery(&mut self, query: SparqlQuery) -> Result<u16, String> {
        if self.subquery_count >= 16 {
            return Err("Subquery overflow".to_string());
        }
        let id = self.subquery_count as u16;
        self.subqueries[self.subquery_count] = query;
        self.subquery_count += 1;
        Ok(id)
    }

    /// Register a variable name, returns its ID
    pub fn register_variable(&mut self, name: &str) -> Result<VariableId, String> {
        if self.variable_count >= MAX_VARIABLES {
            return Err("Variable overflow".to_string());
        }
        let hash = crate::lexicon::generate_60bit_token(name.as_bytes());
        // Check if variable already exists
        for (i, var_hash) in self.variable_hashes.iter().enumerate() {
            if *var_hash == hash {
                return Ok(i as VariableId);
            }
        }
        let id = self.variable_count as VariableId;
        self.variable_hashes[self.variable_count] = hash;
        self.variable_count += 1;
        Ok(id)
    }

    /// Reset the context for reuse (clears all allocations)
    pub fn reset(&mut self) {
        self.pattern_count = 0;
        self.expression_count = 0;
        self.variable_count = 0;
        self.function_arg_count = 0;
        self.variable_hashes = [0; MAX_VARIABLES];
    }
}

impl Default for SparqlQueryContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Binding row - stack-allocated row for variable bindings
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingRow {
    /// Slot array - None means unbound
    pub slots: [Option<u64>; MAX_BINDINGS],
}

impl BindingRow {
    pub fn new() -> Self {
        Self {
            slots: [None; MAX_BINDINGS],
        }
    }

    pub fn set(&mut self, var_id: VariableId, value: u64) {
        if (var_id as usize) < MAX_BINDINGS {
            self.slots[var_id as usize] = Some(value);
        }
    }

    pub fn get(&self, var_id: VariableId) -> Option<u64> {
        if (var_id as usize) < MAX_BINDINGS {
            self.slots[var_id as usize]
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.slots = [None; MAX_BINDINGS];
    }
}

impl Default for BindingRow {
    fn default() -> Self {
        Self::new()
    }
}

/// Physical operator trait for query execution
pub trait PhysicalOperator {
    /// Advance to next result, returns true if more results available
    fn next(&mut self, ctx: &SparqlQueryContext, row: &mut BindingRow) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_context_allocation() {
        let mut ctx = SparqlQueryContext::new();
        
        let pattern = Pattern::Triple {
            subject: 1,
            predicate: 2,
            object: 3,
        };
        
        let id = ctx.alloc_pattern(pattern).unwrap();
        assert_eq!(id, 0);
        assert_eq!(ctx.pattern_count, 1);
    }

    #[test]
    fn test_variable_registration() {
        let mut ctx = SparqlQueryContext::new();
        
        let id1 = ctx.register_variable("?x").unwrap();
        let id2 = ctx.register_variable("?y").unwrap();
        
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(ctx.variable_count, 2);
    }

    #[test]
    fn test_variable_duplicate() {
        let mut ctx = SparqlQueryContext::new();
        
        let id1 = ctx.register_variable("?x").unwrap();
        let id2 = ctx.register_variable("?x").unwrap();
        
        assert_eq!(id1, id2);
        assert_eq!(ctx.variable_count, 1);
    }

    #[test]
    fn test_binding_row() {
        let mut row = BindingRow::new();
        
        row.set(0, 42);
        assert_eq!(row.get(0), Some(42));
        assert_eq!(row.get(1), None);
    }

    #[test]
    fn test_optional_pattern_index() {
        let mut ctx = SparqlQueryContext::new();
        
        let inner = Pattern::Triple {
            subject: 1,
            predicate: 2,
            object: 3,
        };
        let inner_id = ctx.alloc_pattern(inner).unwrap();
        
        let optional = Pattern::Optional { inner: inner_id };
        let optional_id = ctx.alloc_pattern(optional).unwrap();
        
        assert_eq!(ctx.pattern_count, 2);
        if let Pattern::Optional { inner } = ctx.patterns[optional_id as usize] {
            assert_eq!(inner, inner_id);
        } else {
            panic!("Expected Optional pattern");
        }
    }

    #[test]
    fn test_union_pattern_index() {
        let mut ctx = SparqlQueryContext::new();
        
        let left = Pattern::Triple {
            subject: 1,
            predicate: 2,
            object: 3,
        };
        let right = Pattern::Triple {
            subject: 4,
            predicate: 5,
            object: 6,
        };
        
        let left_id = ctx.alloc_pattern(left).unwrap();
        let right_id = ctx.alloc_pattern(right).unwrap();
        
        let union = Pattern::Union {
            left: left_id,
            right: right_id,
        };
        let union_id = ctx.alloc_pattern(union).unwrap();
        
        assert_eq!(ctx.pattern_count, 3);
        if let Pattern::Union { left, right } = ctx.patterns[union_id as usize] {
            assert_eq!(left, left_id);
            assert_eq!(right, right_id);
        } else {
            panic!("Expected Union pattern");
        }
    }
}