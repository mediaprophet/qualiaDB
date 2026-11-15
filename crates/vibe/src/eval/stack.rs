//! Stack admission shared across expression evaluation and nested function calls.
use super::*;

impl<'a, H: Host> Engine<'a, H> {
    /// Evaluate once; the source-session profile supplies a total expression
    /// depth bound so independent call/path limits cannot multiply unchecked.
    pub fn eval_expr(&mut self, expr: &Expr, env: &mut Env) -> Result<Value, Diagnostic> {
        if self.expression_depth >= self.expression_depth_limit {
            return Err(Diagnostic::new(
                DiagCode::E400,
                expr.span,
                "source-session expression evaluation depth exceeded",
            ));
        }
        self.expression_depth += 1;
        let result = self.eval_expr_inner(expr, env);
        self.expression_depth -= 1;
        result
    }
}
