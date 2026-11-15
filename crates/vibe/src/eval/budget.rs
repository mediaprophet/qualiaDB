use super::*;

pub(super) fn budget_steps<H: Host>(
    args: &[NamedArg],
    env: &mut Env,
    eng: &mut Engine<H>,
) -> Result<Option<u64>, Diagnostic> {
    for a in args {
        if a.name == "steps" {
            let previous = eng.evaluating_budget;
            eng.evaluating_budget = true;
            let result = eng.eval_expr(&a.value, env);
            eng.evaluating_budget = previous;
            let value = result?;
            return value
                .as_i64()
                .filter(|n| *n >= 0)
                .map(|n| Some(n as u64))
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E400,
                        a.span,
                        "steps budget must evaluate to a nonnegative integer",
                    )
                });
        }
    }
    Ok(None)
}
