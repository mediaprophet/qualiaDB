//! Client-core conditioning integration (Prompt Precision P3).

pub mod auto_route;
pub mod mcp;
pub mod result;

pub use auto_route::{apply_active_model_precision, apply_contract_to_prompt, AppliedPrecision};
pub use mcp::{lower_mcp_tool_arguments, McpLoweringMode, McpLoweringReceipt};
pub use result::{MeasurementStatus, NormalizedInferenceResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_lowering_structured_when_supported() {
        let (args, receipt) = lower_mcp_tool_arguments(
            Some("You are a helpful assistant"),
            "Summarise this graph",
            true,
        );

        assert_eq!(receipt.mode, McpLoweringMode::Structured);
        assert!(!receipt.role_degraded);
        assert_eq!(receipt.degradation_reason, None);
        assert_eq!(args["system"], "You are a helpful assistant");
        assert_eq!(args["prompt"], "Summarise this graph");
    }

    #[test]
    fn test_mcp_lowering_flattens_and_records_degradation_when_unsupported() {
        let (args, receipt) = lower_mcp_tool_arguments(
            Some("You are a helpful assistant"),
            "Summarise this graph",
            false,
        );

        assert_eq!(receipt.mode, McpLoweringMode::Flattened);
        assert!(receipt.role_degraded);
        assert!(receipt.degradation_reason.is_some());
        assert!(args.get("system").is_none());
        let prompt_str = args["prompt"].as_str().unwrap();
        assert!(prompt_str.starts_with("You are a helpful assistant\n\nSummarise this graph"));
    }

    #[test]
    fn test_normalized_inference_result_coverage_status() {
        let unmeasured =
            NormalizedInferenceResult::new_unmeasured("Raw response text".into(), true);
        assert_eq!(unmeasured.token_measurement, MeasurementStatus::Unknown);
        assert_eq!(unmeasured.latency_measurement, MeasurementStatus::Unknown);
        assert_eq!(unmeasured.tokens_generated, None);

        let measured = NormalizedInferenceResult::new_measured(
            "Measured text".into(),
            true,
            42,
            150,
            vec!["urn:qualia:doc:1".into()],
        );
        assert_eq!(measured.token_measurement, MeasurementStatus::Measured);
        assert_eq!(measured.tokens_generated, Some(42));
        assert_eq!(measured.latency_ms, Some(150));
        assert_eq!(measured.citations.len(), 1);
    }
}
