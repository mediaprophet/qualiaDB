//! `vibe` — VibeScript CLI toolchain (T62).
//!
//! Subcommands:
//! - `vibe check <file>`    — parse + type-check, print diagnostics
//! - `vibe fmt <file>`      — format source using the pretty printer
//! - `vibe eval <file> [fn] [args]` — parse + evaluate, print result
//! - `vibe translate <file> <locale>` — translate keywords to a locale
//! - `vibe repl`            — simple read-eval-print loop
//!
//! This is a zero-dependency CLI — it uses only vibe's own
//! libraries. No external crates, no networking, no filesystem writes.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.13 T62.

use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

use vibe::{
    check_program, eval_cell, eval_function, load_program, parse_program, Diagnostic, Env,
    LocalHost, Value,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("vibe — VibeScript CLI toolchain");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  vibe check <file>           Parse + type-check, print diagnostics");
        eprintln!("  vibe fmt <file>             Format source (pretty printer)");
        eprintln!("  vibe eval <file> [fn] [args] Parse + evaluate, print result");
        eprintln!("  vibe translate <file> <locale>  Translate keywords to a locale");
        eprintln!("  vibe repl                   Read-eval-print loop");
        eprintln!();
        eprintln!("Locales: en (English/canonical), zh (Chinese)");
        return ExitCode::from(1);
    }

    let cmd = &args[1];
    match cmd.as_str() {
        "check" => cmd_check(&args[2..]),
        "fmt" => cmd_fmt(&args[2..]),
        "eval" => cmd_eval(&args[2..]),
        "translate" => cmd_translate(&args[2..]),
        "repl" => cmd_repl(),
        "--help" | "-h" | "help" => {
            eprintln!("vibe — VibeScript CLI toolchain");
            eprintln!();
            eprintln!("Commands: check, fmt, eval, translate, repl");
            ExitCode::from(0)
        }
        _ => {
            eprintln!("unknown command: {cmd}");
            eprintln!("run 'vibe --help' for usage");
            ExitCode::from(1)
        }
    }
}

fn read_file(path: &str) -> Result<String, ExitCode> {
    match fs::read_to_string(Path::new(path)) {
        Ok(src) => Ok(src),
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            Err(ExitCode::from(2))
        }
    }
}

fn cmd_check(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("usage: poet check <file>");
        return ExitCode::from(1);
    }
    let src = match read_file(&args[0]) {
        Ok(s) => s,
        Err(code) => return code,
    };

    match parse_program(&src) {
        Ok(program) => match check_program(&program) {
            Ok(_) => {
                println!("ok: {} ({} items)", args[0], program.items.len());
                ExitCode::from(0)
            }
            Err(diag) => {
                print_diagnostic(&diag, &src);
                ExitCode::from(3)
            }
        },
        Err(diag) => {
            print_diagnostic(&diag, &src);
            ExitCode::from(3)
        }
    }
}

fn cmd_fmt(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("usage: vibe fmt <file>");
        return ExitCode::from(1);
    }
    let src = match read_file(&args[0]) {
        Ok(s) => s,
        Err(code) => return code,
    };

    // Parse to verify validity, then re-emit.
    match parse_program(&src) {
        Ok(program) => {
            let formatted = vibe::project_program(&program, &vibe::ProjectOptions::default());
            print!("{formatted}");
            ExitCode::from(0)
        }
        Err(diag) => {
            print_diagnostic(&diag, &src);
            ExitCode::from(3)
        }
    }
}

fn cmd_eval(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("usage: vibe eval <file> [fn] [args...]");
        return ExitCode::from(1);
    }
    let src = match read_file(&args[0]) {
        Ok(s) => s,
        Err(code) => return code,
    };

    let program = match load_program(&src) {
        Ok(p) => p,
        Err(diag) => {
            print_diagnostic(&diag, &src);
            return ExitCode::from(3);
        }
    };

    let fn_name = args.get(1).map(|s| s.as_str()).unwrap_or("main");
    let fn_args: Vec<Value> = args[2..].iter().map(|s| Value::String(s.clone())).collect();

    let mut host = LocalHost::default();
    let mut env = Env::default();
    match eval_function(&program, fn_name, fn_args, &mut host, &mut env) {
        Ok(val) => {
            print_value(&val, &mut io::stdout());
            println!();
            ExitCode::from(0)
        }
        Err(diag) => {
            print_diagnostic(&diag, &src);
            ExitCode::from(4)
        }
    }
}

fn cmd_translate(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("usage: vibe translate <file> <locale>");
        eprintln!("locales: en, zh, es, ja, ar, hi, fr, de");
        return ExitCode::from(1);
    }
    let src = match read_file(&args[0]) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let locale = &args[1];

    let registry = vibe::locale::LocaleRegistry::with_all_locales();
    let target = match locale.as_str() {
        "en" => vibe::locale::Locale::EN,
        "zh" => vibe::locale::Locale::ZH,
        "es" => vibe::locale::Locale::ES,
        "ja" => vibe::locale::Locale::JA,
        "ar" => vibe::locale::Locale::AR,
        "hi" => vibe::locale::Locale::HI,
        "fr" => vibe::locale::Locale::FR,
        "de" => vibe::locale::Locale::DE,
        _ => {
            eprintln!("unknown locale: {locale} (supported: en, zh, es, ja, ar, hi, fr, de)");
            return ExitCode::from(1);
        }
    };

    match vibe::translate::translate_source(&registry, &src, target) {
        Some(translated) => {
            print!("{translated}");
            ExitCode::from(0)
        }
        None => {
            eprintln!("no translation table for locale {locale}");
            ExitCode::from(5)
        }
    }
}

fn cmd_repl() -> ExitCode {
    let stdin = io::stdin();
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let mut line_num = 1u32;

    println!("vibe repl — VibeScript 0.1 (type :help for commands, :quit to exit)");

    loop {
        print!("vibe:{line_num}> ");
        io::stdout().flush().ok();
        line_num += 1;

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == ":quit" || trimmed == ":q" {
            break;
        }
        if trimmed == ":help" || trimmed == ":h" {
            println!("Commands:");
            println!("  :help    Show this help");
            println!("  :quit    Exit the REPL");
            println!("  <expr>   Evaluate a cell expression (= ...)");
            continue;
        }

        // Try to evaluate as a cell expression.
        let src = if trimmed.starts_with('=') {
            trimmed.to_string()
        } else {
            format!("={trimmed}")
        };

        match eval_cell(&src, &mut host, &mut env) {
            Ok(val) => {
                print_value(&val, &mut io::stdout());
                println!();
            }
            Err(diag) => print_diagnostic(&diag, &src),
        }
    }

    ExitCode::from(0)
}

fn print_value(val: &Value, out: &mut impl Write) {
    match val {
        Value::Null => write!(out, "null").ok(),
        Value::Bool(b) => write!(out, "{b}").ok(),
        Value::I64(n) => write!(out, "{n}").ok(),
        Value::U64(n) => write!(out, "{n}").ok(),
        Value::F64(n) => write!(out, "{n}").ok(),
        Value::String(s) => write!(out, "\"{s}\"").ok(),
        Value::Iri(s) => write!(out, "<{s}>").ok(),
        _ => write!(out, "{val:?}").ok(),
    };
}

fn print_diagnostic(diag: &Diagnostic, src: &str) {
    // Span uses byte offsets. Find the line containing the start.
    let start = diag.span.start as usize;
    let line_start = src[..start.min(src.len())]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let line_end = src[start.min(src.len())..]
        .find('\n')
        .map(|i| start + i)
        .unwrap_or(src.len());
    let line = &src[line_start..line_end.min(src.len())];
    let col = start - line_start;
    eprintln!("error: {} (byte {}, col {})", diag.message, start, col + 1);
    eprintln!("  | {line}");
    eprintln!("  | {}^", " ".repeat(col));
}
