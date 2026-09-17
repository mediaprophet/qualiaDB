//! Graceful degradation copy (P16.7). Fail closed; never pretend GPU/LLM ran.

/// GPU missing → stay on bytecode / scalar CPU.
pub fn gpu_missing(detail: impl AsRef<str>) -> String {
    format!(
        "{}; falling back to bytecode / scalar CPU (no GPU this pass)",
        detail.as_ref()
    )
}

/// Local model missing → diagnostic. Consented remote is not implied.
pub fn llm_missing(detail: impl AsRef<str>) -> String {
    format!(
        "{}; local model unavailable; consented remote is the only fallback and is not taken automatically",
        detail.as_ref()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_message_names_bytecode() {
        let m = gpu_missing("gpu_init: no adapter");
        assert!(m.contains("bytecode"));
        assert!(m.contains("gpu_init"));
    }

    #[test]
    fn llm_message_does_not_auto_remote() {
        let m = llm_missing("Inference.load_model: not found");
        assert!(m.contains("not taken automatically"));
        assert!(m.contains("consented remote"));
    }
}
