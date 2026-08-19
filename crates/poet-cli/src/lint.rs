//! VibeScript static linter (T62).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LintSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintIssue {
    pub line: usize,
    pub severity: LintSeverity,
    pub message: String,
}

const BUILTIN_NAMES: &[&str] = &[
    "null", "true", "false", "time", "pulse", "capability", "graph", "math",
];

/// Run static lint checks on VibeScript source code.
pub fn lint_source(source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let mut let_bindings = Vec::new(); // (name, line_num, used)
    let mut returned_in_block = false;

    for (idx, raw_line) in source.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = raw_line.trim();

        if trimmed.starts_with("//") || trimmed.is_empty() {
            continue;
        }

        // Check for unreachable code after return
        if returned_in_block {
            if trimmed.starts_with('}') {
                returned_in_block = false;
            } else {
                issues.push(LintIssue {
                    line: line_num,
                    severity: LintSeverity::Warning,
                    message: format!("unreachable code after return: '{trimmed}'"),
                });
            }
        }

        if trimmed.starts_with("return ") || trimmed == "return;" || trimmed == "return" {
            returned_in_block = true;
        }

        // Check for let bindings
        if trimmed.starts_with("let ") || trimmed.starts_with("let mut ") {
            let rest = if trimmed.starts_with("let mut ") {
                &trimmed["let mut ".len()..]
            } else {
                &trimmed["let ".len()..]
            };

            if let Some(eq_pos) = rest.find('=') {
                let name = rest[..eq_pos].trim().to_string();
                if BUILTIN_NAMES.contains(&name.as_str()) {
                    issues.push(LintIssue {
                        line: line_num,
                        severity: LintSeverity::Warning,
                        message: format!("binding '{name}' shadows built-in name"),
                    });
                }
                let_bindings.push((name, line_num, false));
            }
        } else {
            // Check if any previously declared let binding is used on this line
            for (name, _, used) in &mut let_bindings {
                if trimmed.contains(name.as_str()) {
                    *used = true;
                }
            }
        }
    }

    // Check for unused bindings
    for (name, line_num, used) in let_bindings {
        if !used && !name.starts_with('_') {
            issues.push(LintIssue {
                line: line_num,
                severity: LintSeverity::Warning,
                message: format!("unused binding '{name}' (prefix with '_' to silence)"),
            });
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_detects_unused_binding() {
        let src = "let unused_var = 42;\nlet used_var = 1;\nreturn used_var;\n";
        let issues = lint_source(src);
        assert!(issues.iter().any(|i| i.message.contains("unused binding 'unused_var'")));
        assert!(!issues.iter().any(|i| i.message.contains("unused binding 'used_var'")));
    }

    #[test]
    fn lint_detects_unreachable_code() {
        let src = "return 1;\nlet x = 2;\n";
        let issues = lint_source(src);
        assert!(issues.iter().any(|i| i.message.contains("unreachable code after return")));
    }

    #[test]
    fn lint_detects_builtin_shadowing() {
        let src = "let time = 100;\nreturn time;\n";
        let issues = lint_source(src);
        assert!(issues.iter().any(|i| i.message.contains("shadows built-in name")));
    }
}
