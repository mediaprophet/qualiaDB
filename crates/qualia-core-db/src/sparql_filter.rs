//! SPARQL FILTER Expression Evaluator
//!
//! Evaluates SPARQL FILTER expressions against binding rows using zero-allocation patterns.

use crate::sparql_ast::*;

/// Expression evaluator
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate an expression against a binding row
    pub fn evaluate(
        expr_id: ExpressionId,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<EvalResult, String> {
        let expr = ctx.expressions.get(expr_id as usize)
            .ok_or("Expression ID out of bounds")?;
        
        Self::evaluate_expression(expr, ctx, row)
    }

    fn evaluate_expression(
        expr: &Expression,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<EvalResult, String> {
        match expr {
            Expression::Variable(var_id) => {
                let value = row.get(*var_id);
                Ok(EvalResult::Numeric(value.unwrap_or(0)))
            }
            Expression::Literal(value) => {
                Ok(EvalResult::Numeric(*value))
            }
            Expression::Iri(value) => {
                Ok(EvalResult::Iri(*value))
            }
            Expression::UnaryOp { op, expr: inner_id } => {
                let inner = Self::evaluate_expression(
                    &ctx.expressions[*inner_id as usize],
                    ctx,
                    row,
                )?;
                Self::evaluate_unary_op(*op, inner)
            }
            Expression::BinaryOp { op, left, right } => {
                let left_val = Self::evaluate_expression(
                    &ctx.expressions[*left as usize],
                    ctx,
                    row,
                )?;
                let right_val = Self::evaluate_expression(
                    &ctx.expressions[*right as usize],
                    ctx,
                    row,
                )?;
                Self::evaluate_binary_op(*op, left_val, right_val)
            }
            Expression::Function { func, args_start, args_len } => {
                Self::evaluate_function(*func, *args_start, *args_len, ctx, row)
            }
            Expression::Subquery { query_id } => {
                Self::evaluate_subquery(*query_id, ctx, row)
            }
            Expression::EmbeddedTriple { subject, predicate, object } => {
                // Evaluate embedded triple - return hash representation
                let triple_hash = *subject ^ *predicate ^ *object;
                Ok(EvalResult::Numeric(triple_hash))
            }
        }
    }

    fn evaluate_unary_op(op: UnaryOp, inner: EvalResult) -> Result<EvalResult, String> {
        match op {
            UnaryOp::Not => {
                match inner {
                    EvalResult::Boolean(b) => Ok(EvalResult::Boolean(!b)),
                    _ => Err("NOT operator requires boolean operand".to_string()),
                }
            }
            UnaryOp::Plus => Ok(inner),
            UnaryOp::Minus => {
                match inner {
                    EvalResult::Numeric(n) => Ok(EvalResult::Numeric((n as i64 * -1) as u64)),
                    _ => Err("MINUS operator requires numeric operand".to_string()),
                }
            }
        }
    }

    fn evaluate_binary_op(op: BinaryOp, left: EvalResult, right: EvalResult) -> Result<EvalResult, String> {
        match op {
            BinaryOp::Or => {
                match (left, right) {
                    (EvalResult::Boolean(l), EvalResult::Boolean(r)) => {
                        Ok(EvalResult::Boolean(l || r))
                    }
                    _ => Err("OR operator requires boolean operands".to_string()),
                }
            }
            BinaryOp::And => {
                match (left, right) {
                    (EvalResult::Boolean(l), EvalResult::Boolean(r)) => {
                        Ok(EvalResult::Boolean(l && r))
                    }
                    _ => Err("AND operator requires boolean operands".to_string()),
                }
            }
            BinaryOp::Equal => {
                Ok(EvalResult::Boolean(left == right))
            }
            BinaryOp::NotEqual => {
                Ok(EvalResult::Boolean(left != right))
            }
            BinaryOp::LessThan => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Boolean(l < r))
                    }
                    _ => Err("LESS THAN operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::LessThanOrEqual => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Boolean(l <= r))
                    }
                    _ => Err("LESS THAN OR EQUAL operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::GreaterThan => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Boolean(l > r))
                    }
                    _ => Err("GREATER THAN operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::GreaterThanOrEqual => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Boolean(l >= r))
                    }
                    _ => Err("GREATER THAN OR EQUAL operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::Add => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Numeric(l.wrapping_add(r)))
                    }
                    _ => Err("ADD operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::Subtract => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Numeric(l.wrapping_sub(r)))
                    }
                    _ => Err("SUBTRACT operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::Multiply => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        Ok(EvalResult::Numeric(l.wrapping_mul(r)))
                    }
                    _ => Err("MULTIPLY operator requires numeric operands".to_string()),
                }
            }
            BinaryOp::Divide => {
                match (left, right) {
                    (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                        if r == 0 {
                            return Err("Division by zero".to_string());
                        }
                        Ok(EvalResult::Numeric(l / r))
                    }
                    _ => Err("DIVIDE operator requires numeric operands".to_string()),
                }
            }
        }
    }

    fn evaluate_function(
        func: Function,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<EvalResult, String> {
        match func {
            Function::Bound => {
                // BOUND(?var) - check if variable is bound
                if args_len >= 1 {
                    let var_id = ctx.function_args[args_start as usize] as VariableId;
                    Ok(EvalResult::Boolean(row.get(var_id).is_some()))
                } else {
                    Err("BOUND requires at least one argument".to_string())
                }
            }
            Function::Str => {
                // STR(expr) - convert to string (simplified)
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(result)
                } else {
                    Err("STR requires at least one argument".to_string())
                }
            }
            Function::Lang => {
                // LANG(expr) - get language tag (simplified)
                if args_len >= 1 {
                    Ok(EvalResult::String(0)) // Placeholder
                } else {
                    Err("LANG requires at least one argument".to_string())
                }
            }
            Function::Datatype => {
                // DATATYPE(expr) - get datatype IRI (simplified)
                if args_len >= 1 {
                    Ok(EvalResult::Iri(0)) // Placeholder
                } else {
                    Err("DATATYPE requires at least one argument".to_string())
                }
            }
            Function::IsIri | Function::IsUri => {
                // isIRI(expr) - check if IRI
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(EvalResult::Boolean(matches!(result, EvalResult::Iri(_))))
                } else {
                    Err("isIRI requires at least one argument".to_string())
                }
            }
            Function::IsBlank => {
                // isBlank(expr) - check if blank node
                if args_len >= 1 {
                    Ok(EvalResult::Boolean(false)) // Simplified
                } else {
                    Err("isBlank requires at least one argument".to_string())
                }
            }
            Function::IsLiteral => {
                // isLiteral(expr) - check if literal
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(EvalResult::Boolean(matches!(result, EvalResult::Numeric(_))))
                } else {
                    Err("isLiteral requires at least one argument".to_string())
                }
            }
            Function::IsNumeric => {
                // isNumeric(expr) - check if numeric
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(EvalResult::Boolean(matches!(result, EvalResult::Numeric(_))))
                } else {
                    Err("isNumeric requires at least one argument".to_string())
                }
            }
            Function::Abs => {
                // ABS(expr) - absolute value
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => {
                            Ok(EvalResult::Numeric((n as i64).abs() as u64))
                        }
                        _ => Err("ABS requires numeric argument".to_string()),
                    }
                } else {
                    Err("ABS requires at least one argument".to_string())
                }
            }
            Function::Ceil => {
                // CEIL(expr) - ceiling
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => {
                            Ok(EvalResult::Numeric((n as f64).ceil() as u64))
                        }
                        _ => Err("CEIL requires numeric argument".to_string()),
                    }
                } else {
                    Err("CEIL requires at least one argument".to_string())
                }
            }
            Function::Floor => {
                // FLOOR(expr) - floor
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => {
                            Ok(EvalResult::Numeric((n as f64).floor() as u64))
                        }
                        _ => Err("FLOOR requires numeric argument".to_string()),
                    }
                } else {
                    Err("FLOOR requires at least one argument".to_string())
                }
            }
            Function::Round => {
                // ROUND(expr) - round
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => {
                            Ok(EvalResult::Numeric((n as f64).round() as u64))
                        }
                        _ => Err("ROUND requires numeric argument".to_string()),
                    }
                } else {
                    Err("ROUND requires at least one argument".to_string())
                }
            }
            _ => {
                // Placeholder for other functions including SPARQL-Star
                Ok(EvalResult::Boolean(true))
            }
            Function::TripleSubject => {
                // TRIPLESUBJECT(<<s p o>>) - return subject
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let expr = &ctx.expressions[expr_id as usize];
                    if let Expression::EmbeddedTriple { subject, .. } = expr {
                        Ok(EvalResult::Numeric(*subject))
                    } else {
                        Err("TRIPLESUBJECT requires embedded triple".to_string())
                    }
                } else {
                    Err("TRIPLESUBJECT requires at least one argument".to_string())
                }
            }
            Function::TriplePredicate => {
                // TRIPLEPREDICATE(<<s p o>>) - return predicate
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let expr = &ctx.expressions[expr_id as usize];
                    if let Expression::EmbeddedTriple { predicate, .. } = expr {
                        Ok(EvalResult::Numeric(*predicate))
                    } else {
                        Err("TRIPLEPREDICATE requires embedded triple".to_string())
                    }
                } else {
                    Err("TRIPLEPREDICATE requires at least one argument".to_string())
                }
            }
            Function::TripleObject => {
                // TRIPLEOBJECT(<<s p o>>) - return object
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let expr = &ctx.expressions[expr_id as usize];
                    if let Expression::EmbeddedTriple { object, .. } = expr {
                        Ok(EvalResult::Numeric(*object))
                    } else {
                        Err("TRIPLEOBJECT requires embedded triple".to_string())
                    }
                } else {
                    Err("TRIPLEOBJECT requires at least one argument".to_string())
                }
            }
            Function::Triple => {
                // TRIPLE(s, p, o) - create embedded triple
                if args_len >= 3 {
                    let s_id = ctx.function_args[args_start as usize];
                    let p_id = ctx.function_args[args_start as usize + 1];
                    let o_id = ctx.function_args[args_start as usize + 2];
                    
                    let s_result = Self::evaluate(s_id, ctx, row)?;
                    let p_result = Self::evaluate(p_id, ctx, row)?;
                    let o_result = Self::evaluate(o_id, ctx, row)?;
                    
                    match (s_result, p_result, o_result) {
                        (EvalResult::Numeric(s), EvalResult::Numeric(p), EvalResult::Numeric(o)) => {
                            // Return a hash representing the embedded triple
                            let triple_hash = s ^ p ^ o; // Simplified hash
                            Ok(EvalResult::Numeric(triple_hash))
                        }
                        _ => Err("TRIPLE requires numeric arguments".to_string()),
                    }
                } else {
                    Err("TRIPLE requires at least three arguments".to_string())
                }
            }
        }
    }

    fn evaluate_subquery(
        query_id: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<EvalResult, String> {
        let subquery = ctx.subqueries.get(query_id as usize)
            .ok_or("Subquery ID out of bounds")?;
        
        // Simplified: return true if subquery would return results
        // Full implementation would:
        // 1. Plan the subquery
        // 2. Execute it with current bindings
        // 3. Return the result (e.g., EXISTS, count, etc.)
        Ok(EvalResult::Boolean(true))
    }
}

/// Evaluation result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalResult {
    Numeric(u64),
    Boolean(bool),
    Iri(u64),
    String(u64), // Hash of string
}

impl EvalResult {
    pub fn as_bool(&self) -> bool {
        match self {
            EvalResult::Boolean(b) => *b,
            EvalResult::Numeric(n) => *n != 0,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_variable() {
        let mut ctx = SparqlQueryContext::new();
        let var_id = ctx.register_variable("?x").unwrap();
        
        let expr = Expression::Variable(var_id);
        ctx.alloc_expression(expr).unwrap();
        
        let mut row = BindingRow::new();
        row.set(var_id, 42);
        
        let result = ExpressionEvaluator::evaluate(0, &ctx, &row).unwrap();
        assert_eq!(result, EvalResult::Numeric(42));
    }

    #[test]
    fn test_evaluate_binary_op() {
        let left = EvalResult::Numeric(5);
        let right = EvalResult::Numeric(3);
        
        let result = ExpressionEvaluator::evaluate_binary_op(BinaryOp::Add, left, right).unwrap();
        assert_eq!(result, EvalResult::Numeric(8));
    }

    #[test]
    fn test_evaluate_unary_not() {
        let inner = EvalResult::Boolean(true);
        let result = ExpressionEvaluator::evaluate_unary_op(UnaryOp::Not, inner).unwrap();
        assert_eq!(result, EvalResult::Boolean(false));
    }

    #[test]
    fn test_bound_function() {
        let mut ctx = SparqlQueryContext::new();
        let var_id = ctx.register_variable("?x").unwrap();
        
        ctx.function_args[0] = var_id as ExpressionId;
        ctx.function_arg_count = 1;
        
        let mut row = BindingRow::new();
        row.set(var_id, 42);
        
        let result = ExpressionEvaluator::evaluate_function(
            Function::Bound,
            0,
            1,
            &ctx,
            &row,
        ).unwrap();
        
        assert_eq!(result, EvalResult::Boolean(true));
    }
}