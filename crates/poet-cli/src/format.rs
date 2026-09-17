//! VibeScript source formatter (T62).

/// Format a VibeScript source string.
///
/// Valid programs are re-emitted from the AST (`project_program`).
/// If the source does not parse, it is returned unchanged so we never
/// destroy author work with a brace counter.
pub fn format_source(source: &str) -> String {
    match vibe::parse_program(source) {
        Ok(prog) => vibe::project_program(&prog, &vibe::ProjectOptions::default()),
        Err(_) => source.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_valid_program_round_trips() {
        let input = "effect fn go() {\nlet x = 1;\nif x > 0 {\nreturn x;\n}\n}\n";
        let out = format_source(input);
        assert!(out.contains("fn go"));
        vibe::parse_program(&out).expect("formatted source still parses");
    }

    #[test]
    fn format_invalid_source_is_left_alone() {
        let input = "this is not vibe {{{";
        assert_eq!(format_source(input), input);
    }
}
