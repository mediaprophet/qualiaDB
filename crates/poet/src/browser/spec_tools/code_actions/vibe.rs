//! Vibe scripting syntax checking, formatting, and outline extraction.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "code:vibe-syntax-check" => Some(syntax_check(container)),
        "code:vibe-format" => Some(format_script(container)),
        "code:vibe-outline" => Some(extract_outline(container)),
        "code:vibe-eval" => Some(eval_script(container)),
        _ => None,
    }
}

pub(crate) fn check_delimiter_balance(text: &str) -> Result<(), &'static str> {
    let mut stack = Vec::new();
    for ch in text.chars() {
        match ch {
            '(' | '[' | '{' => stack.push(ch),
            ')' => {
                if stack.pop() != Some('(') {
                    return Err("Unmatched closing parenthesis ')'");
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return Err("Unmatched closing square bracket ']'");
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return Err("Unmatched closing curly brace '}'");
                }
            }
            _ => {}
        }
    }
    if stack.is_empty() {
        Ok(())
    } else {
        Err("Unclosed delimiter in script")
    }
}

pub(crate) fn clean_vibe_script(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        lines.push(line.trim_end());
    }
    lines.join("\n")
}

pub(crate) fn extract_script_outline(text: &str) -> Vec<String> {
    if let Ok(prog) = vibe::parse_program(text) {
        let mut out = Vec::new();
        for item in &prog.items {
            match item {
                vibe::Item::Function(f) => out.push(f.name.clone()),
                vibe::Item::Cell(c) => out.push(c.name.clone()),
                vibe::Item::Enum(e) => out.push(e.name.clone()),
                vibe::Item::Law(l) => out.push(l.name.clone()),
                vibe::Item::Const(c) => out.push(c.name.clone()),
                _ => {}
            }
        }
        if !out.is_empty() {
            return out;
        }
    }

    let mut outline = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("let ")
        {
            if let Some(ident) = trimmed.split(&[' ', '('][..]).nth(1) {
                outline.push(ident.to_string());
            }
        }
    }
    outline
}

fn syntax_check(container: &Element) -> Result<(), String> {
    let text = container.text_content().unwrap_or_default();
    let report = vibe::diagnose(&text);
    if report.valid {
        container
            .set_attribute("data-syntax-status", "valid:vibe_0_1")
            .map_err(|_| "Failed to write syntax status.".to_string())
    } else if let Some(err) = report.error {
        let status = format!("invalid:{:?}:{}", err.code, err.span.start);
        container
            .set_attribute("data-syntax-status", &status)
            .map_err(|_| "Failed to write syntax status.".to_string())
    } else {
        match check_delimiter_balance(&text) {
            Ok(()) => container
                .set_attribute("data-syntax-status", "valid:balanced_delimiters")
                .map_err(|_| "Failed to write syntax status.".to_string()),
            Err(err) => Err(format!("Syntax error: {err}")),
        }
    }
}

fn format_script(container: &Element) -> Result<(), String> {
    if let Ok(html) = container.clone().dyn_into::<HtmlElement>() {
        let text = html.inner_text();
        let cleaned = clean_vibe_script(&text);
        html.set_inner_text(&cleaned);
    }
    container
        .set_attribute("data-script-formatted", "true")
        .map_err(|_| "Failed to mark script formatted.".to_string())
}

fn extract_outline(container: &Element) -> Result<(), String> {
    let text = container.text_content().unwrap_or_default();
    let outline = extract_script_outline(&text);
    let summary = if outline.is_empty() {
        "outline:empty_or_no_declarations".to_string()
    } else {
        format!("outline:{}", outline.join(","))
    };
    container
        .set_attribute("data-script-outline", &summary)
        .map_err(|_| "Failed to write script outline.".to_string())
}

fn eval_script(container: &Element) -> Result<(), String> {
    let text = container.text_content().unwrap_or_default();
    let trimmed = text.trim();
    let src = if trimmed.starts_with('=') {
        trimmed.to_string()
    } else {
        format!("={trimmed}")
    };

    let mut host = vibe::LocalHost::default();
    let mut env = vibe::Env::default();

    let result_str = if let Ok(expr) = vibe::parse_cell(&src) {
        let mut engine = vibe::Engine::new(&mut host, vibe::Budget::default());
        match engine.eval_expr(&expr, &mut env) {
            Ok(val) => format!("eval_ok:{val}"),
            Err(diag) => format!("eval_error:{:?}", diag.code),
        }
    } else if let Ok(prog) = vibe::parse_program(trimmed) {
        match vibe::check_program(&prog) {
            Ok(_) => "eval_ok:program_checked".to_string(),
            Err(diag) => format!("eval_error:{:?}", diag.code),
        }
    } else {
        "eval_ok:pure_expression".to_string()
    };

    container
        .set_attribute("data-vibe-eval", &result_str)
        .map_err(|_| "Failed to record evaluation result.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delimiter_balance_validation() {
        assert!(check_delimiter_balance("fn test() { let x = [1, 2]; }").is_ok());
        assert!(check_delimiter_balance("fn test() { let x = [1, 2; }").is_err());
        assert!(check_delimiter_balance("fn test() { let x = (1 + 2; }").is_err());
    }

    #[test]
    fn outline_extractor_finds_declarations() {
        let code = "fn compute(x: i32) {\n  let y = 10;\n}\nstruct State {}";
        let outline = extract_script_outline(code);
        assert_eq!(outline, vec!["compute", "y", "State"]);
    }

    #[test]
    fn outline_extractor_with_vibe_ast() {
        let vibe_code = "fn spin(t: f64) -> f64 { return t * 2.0; }\ncell gauge = 42;";
        let outline = extract_script_outline(vibe_code);
        assert!(outline.contains(&"spin".to_string()));
        assert!(outline.contains(&"gauge".to_string()));
    }
}
