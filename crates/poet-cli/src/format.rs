//! VibeScript source formatter (T62).

/// Format a VibeScript source string with consistent indentation,
/// trimmed trailing whitespace, and normalized blank lines.
pub fn format_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent_level: usize = 0;
    let mut prev_was_blank = false;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            if !prev_was_blank && !out.is_empty() {
                out.push('\n');
                prev_was_blank = true;
            }
            continue;
        }
        prev_was_blank = false;

        // Dedent if the line starts with a closing brace
        let starts_with_close = trimmed.starts_with('}') || trimmed.starts_with(']');
        let current_indent = if starts_with_close && indent_level > 0 {
            indent_level - 1
        } else {
            indent_level
        };

        for _ in 0..current_indent {
            out.push_str("    ");
        }
        out.push_str(trimmed);
        out.push('\n');

        // Adjust indent level for braces in line
        let open_count = trimmed.chars().filter(|&c| c == '{' || c == '[').count();
        let close_count = trimmed.chars().filter(|&c| c == '}' || c == ']').count();

        if open_count > close_count {
            indent_level += open_count - close_count;
        } else if close_count > open_count {
            indent_level = indent_level.saturating_sub(close_count - open_count);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_indentation() {
        let input = "effect fn go() {\nlet x = 1;\nif x > 0 {\nreturn x;\n}\n}\n";
        let expected =
            "effect fn go() {\n    let x = 1;\n    if x > 0 {\n        return x;\n    }\n}\n";
        assert_eq!(format_source(input), expected);
    }

    #[test]
    fn format_trims_and_dedup_blank_lines() {
        let input = "let a = 1;   \n\n\n\nlet b = 2;\n";
        let expected = "let a = 1;\n\nlet b = 2;\n";
        assert_eq!(format_source(input), expected);
    }
}
