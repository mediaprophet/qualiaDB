//! VibeScript interactive REPL (T62).

use poet_vibe::{
    eval_cell, load_program, Budget, Diagnostic, Engine, Env, Host, MockHost, Value,
};
use std::io::{self, BufRead, Write};

/// Evaluate a single REPL line/cell within a persistent host and environment.
pub fn eval_line(
    line: &str,
    host: &mut impl Host,
    env: &mut Env,
) -> Result<Value, Diagnostic> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(Value::Null);
    }

    // Try cell evaluation first
    let cell_src = if trimmed.starts_with('=') {
        trimmed.to_string()
    } else {
        format!("= {trimmed}")
    };

    if let Ok(val) = eval_cell(&cell_src, host, env) {
        return Ok(val);
    }

    // Otherwise try parsing as a program (consts, enums, statements, functions)
    if let Ok(prog) = load_program(trimmed) {
        let mut engine = Engine::new(host, Budget::default());
        return engine.eval_program(&prog, env);
    }

    // Fall back to cell evaluation to produce standard diagnostics
    eval_cell(&cell_src, host, env)
}

/// Run an interactive REPL reading from stdin and writing to stdout.
pub fn run_repl() -> io::Result<()> {
    println!("Poet VibeScript REPL v{}", poet_vibe::LANGUAGE_VERSION);
    println!("Type an expression or cell statement to evaluate. Type 'exit' or Ctrl+D to quit.\n");

    let mut host = MockHost::default();
    let mut env = Env::default();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("vibe> ");
        stdout.flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed == "exit" || trimmed == "quit" {
            break;
        }
        if trimmed.is_empty() {
            continue;
        }

        match eval_line(trimmed, &mut host, &mut env) {
            Ok(val) => println!("{val:?}"),
            Err(diag) => println!("Error: {}", diag.message),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repl_eval_simple_math() {
        let mut host = MockHost::default();
        let mut env = Env::default();
        let val = eval_line("1 + 2 * 3", &mut host, &mut env).unwrap();
        assert_eq!(val, Value::I64(7));
    }

    #[test]
    fn repl_maintains_env_bindings() {
        let mut host = MockHost::default();
        let mut env = Env::default();
        env.vars.insert("a".into(), Value::I64(10));
        let val = eval_line("a * 2", &mut host, &mut env).unwrap();
        assert_eq!(val, Value::I64(20));
    }
}
