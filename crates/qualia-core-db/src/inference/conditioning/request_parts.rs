//! Typed request parts and prioritized budget-bounded selection.
//!
//! Replaces blind byte-prefix truncation with prioritized selection across typed
//! request components (required instructions, user prompt, tool schemas, evidence,
//! and conversation history). Enforces that required instructions and the complete
//! user prompt survive admission, or rejects with `ContextBudgetExceeded`.

use super::spec::ConditioningError;

/// Semantic category of a request part in the conditioning pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPartKind {
    /// Mandatory system instructions (never dropped, never truncated).
    RequiredInstruction = 0,
    /// User's core prompt / objective (never dropped, never truncated).
    UserPrompt = 1,
    /// Authorized tool definitions and schemas.
    ToolSchema = 2,
    /// Grounded evidence and citation context.
    Evidence = 3,
    /// Multi-turn conversation history.
    HistoryMessage = 4,
}

/// A borrowed typed part of an inference request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestPart<'a> {
    pub kind: RequestPartKind,
    pub content: &'a str,
    pub source_id: Option<&'a str>,
    pub qualifier: Option<&'a str>,
    pub is_mandatory: bool,
}

impl<'a> RequestPart<'a> {
    pub const fn required_instruction(content: &'a str) -> Self {
        Self {
            kind: RequestPartKind::RequiredInstruction,
            content,
            source_id: None,
            qualifier: None,
            is_mandatory: true,
        }
    }

    pub const fn user_prompt(content: &'a str) -> Self {
        Self {
            kind: RequestPartKind::UserPrompt,
            content,
            source_id: None,
            qualifier: None,
            is_mandatory: true,
        }
    }

    pub const fn tool_schema(name: &'a str, schema: &'a str) -> Self {
        Self {
            kind: RequestPartKind::ToolSchema,
            content: schema,
            source_id: Some(name),
            qualifier: None,
            is_mandatory: false,
        }
    }

    pub const fn evidence(source_id: &'a str, content: &'a str, qualifier: Option<&'a str>) -> Self {
        Self {
            kind: RequestPartKind::Evidence,
            content,
            source_id: Some(source_id),
            qualifier,
            is_mandatory: false,
        }
    }

    pub const fn history(content: &'a str) -> Self {
        Self {
            kind: RequestPartKind::HistoryMessage,
            content,
            source_id: None,
            qualifier: None,
            is_mandatory: false,
        }
    }

    pub fn byte_len(&self) -> usize {
        self.content.len()
    }
}

/// Select and pack request parts within a strict byte ceiling.
///
/// Guarantees:
/// 1. All mandatory parts (RequiredInstruction, UserPrompt) MUST fit entirely;
///    if the byte ceiling cannot accommodate them, returns `Err(ConditioningError::ContextBudgetExceeded)`.
/// 2. Optional parts are admitted in order of priority (ToolSchema > Evidence > History).
/// 3. Quoted evidence/tool outputs are NEVER promoted to instructions.
pub fn select_prioritized_parts<'a>(
    parts: &[RequestPart<'a>],
    max_bytes: usize,
    out: &mut [RequestPart<'a>],
) -> Result<usize, ConditioningError> {
    // Phase 1: Calculate mandatory byte total
    let mandatory_bytes: usize = parts
        .iter()
        .filter(|p| p.is_mandatory)
        .map(|p| p.byte_len())
        .sum();

    if mandatory_bytes > max_bytes {
        return Err(ConditioningError::ContextBudgetExceeded);
    }

    let mut remaining_bytes = max_bytes - mandatory_bytes;
    let mut selected_count = 0;

    // Phase 2: Add mandatory parts first
    for part in parts.iter().filter(|p| p.is_mandatory) {
        if selected_count >= out.len() {
            return Err(ConditioningError::OutputBufferFull);
        }
        out[selected_count] = *part;
        selected_count += 1;
    }

    // Phase 3: Add optional parts by kind priority
    let optional_kinds = [
        RequestPartKind::ToolSchema,
        RequestPartKind::Evidence,
        RequestPartKind::HistoryMessage,
    ];

    for kind in optional_kinds {
        for part in parts.iter().filter(|p| !p.is_mandatory && p.kind == kind) {
            let len = part.byte_len();
            if len <= remaining_bytes {
                if selected_count >= out.len() {
                    return Err(ConditioningError::OutputBufferFull);
                }
                out[selected_count] = *part;
                selected_count += 1;
                remaining_bytes -= len;
            }
        }
    }

    Ok(selected_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandatory_instructions_and_prompt_survive() {
        let req = RequestPart::required_instruction("SYSTEM: obey ethical laws.");
        let user = RequestPart::user_prompt("USER: explain quantum mechanics.");
        let ev = RequestPart::evidence("wiki", "Evidence text about atoms.", None);

        let parts = [req, user, ev];
        let mut out = [req; 4];

        // Capacity large enough for all
        let count = select_prioritized_parts(&parts, 200, &mut out).unwrap();
        assert_eq!(count, 3);
        assert_eq!(out[0].kind, RequestPartKind::RequiredInstruction);
        assert_eq!(out[1].kind, RequestPartKind::UserPrompt);
        assert_eq!(out[2].kind, RequestPartKind::Evidence);
    }

    #[test]
    fn test_optional_evidence_dropped_when_budget_tight() {
        let req = RequestPart::required_instruction("SYSTEM: adhere to safety.");
        let user = RequestPart::user_prompt("USER: solve 2+2.");
        let ev = RequestPart::evidence("doc1", "Extremely long evidence text...", None);

        let parts = [req, user, ev];
        let mut out = [req; 4];

        let min_bytes = req.byte_len() + user.byte_len();
        // Allow enough for mandatory, but not for evidence
        let count = select_prioritized_parts(&parts, min_bytes + 5, &mut out).unwrap();
        assert_eq!(count, 2);
        assert_eq!(out[0].kind, RequestPartKind::RequiredInstruction);
        assert_eq!(out[1].kind, RequestPartKind::UserPrompt);
    }

    #[test]
    fn test_budget_exceeded_when_mandatory_cannot_fit() {
        let req = RequestPart::required_instruction("SYSTEM: lengthy mandatory rule.");
        let user = RequestPart::user_prompt("USER: lengthy prompt.");

        let parts = [req, user];
        let mut out = [req; 2];

        // Less than mandatory byte total
        let err = select_prioritized_parts(&parts, 10, &mut out);
        assert_eq!(err, Err(ConditioningError::ContextBudgetExceeded));
    }
}
