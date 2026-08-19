//! Poet CLI toolchain: REPL, formatter, linter, evaluator (T62).

mod format;
mod lint;
mod repl;

use std::env;
use std::fs;
use std::process;

fn print_usage() {
    println!("Poet CLI toolchain for VibeScript (v{})", poet_vibe::LANGUAGE_VERSION);
    println!("\nUsage:");
    println!("  poet repl           Start an interactive VibeScript REPL");
    println!("  poet format <file>  Format a .vibe file");
    println!("  poet lint <file>    Run static analysis checks on a .vibe file");
    println!("  poet eval <file>    Evaluate a .vibe file and print result");
    println!("  poet help           Show this help message");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "repl" => {
            if let Err(e) = repl::run_repl() {
                eprintln!("REPL error: {e}");
                process::exit(1);
            }
        }
        "format" => {
            if args.len() < 3 {
                eprintln!("Error: missing file argument for format");
                process::exit(1);
            }
            let path = &args[2];
            match fs::read_to_string(path) {
                Ok(content) => {
                    let formatted = format::format_source(&content);
                    if let Err(e) = fs::write(path, &formatted) {
                        eprintln!("Error writing formatted file: {e}");
                        process::exit(1);
                    }
                    println!("Formatted {}", path);
                }
                Err(e) => {
                    eprintln!("Error reading file '{path}': {e}");
                    process::exit(1);
                }
            }
        }
        "lint" => {
            if args.len() < 3 {
                eprintln!("Error: missing file argument for lint");
                process::exit(1);
            }
            let path = &args[2];
            match fs::read_to_string(path) {
                Ok(content) => {
                    let issues = lint::lint_source(&content);
                    if issues.is_empty() {
                        println!("No issues found in {}", path);
                    } else {
                        for issue in &issues {
                            println!("{}:{}: {:?}: {}", path, issue.line, issue.severity, issue.message);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading file '{path}': {e}");
                    process::exit(1);
                }
            }
        }
        "eval" => {
            if args.len() < 3 {
                eprintln!("Error: missing file argument for eval");
                process::exit(1);
            }
            let path = &args[2];
            match fs::read_to_string(path) {
                Ok(content) => {
                    let mut host = poet_vibe::MockHost::default();
                    let mut env = poet_vibe::Env::default();
                    match poet_vibe::eval_cell(&content, &mut host, &mut env) {
                        Ok(val) => println!("{val:?}"),
                        Err(diag) => {
                            eprintln!("Eval error: {}", diag.message);
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading file '{path}': {e}");
                    process::exit(1);
                }
            }
        }
        "help" | "-h" | "--help" => {
            print_usage();
        }
        other => {
            eprintln!("Unknown command: '{other}'");
            print_usage();
            process::exit(1);
        }
    }
}
