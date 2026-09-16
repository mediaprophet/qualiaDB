//! Shared text and message rendering for backend lowering.

use super::compile::CompiledPlanSummary;
use super::select::EvidencePart;
use super::spec::{ConditioningError, ConditioningSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    NativeChatml,
    NativePlain,
    McpEnvelope,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderedRequestSummary {
    pub bytes_written: usize,
    pub target: RenderTarget,
}

/// Helper to write bytes into caller slice.
#[inline]
fn write_str(buf: &mut [u8], offset: &mut usize, s: &str) -> Result<(), ConditioningError> {
    let bytes = s.as_bytes();
    if *offset + bytes.len() > buf.len() {
        return Err(ConditioningError::OutputBufferFull);
    }
    buf[*offset..*offset + bytes.len()].copy_from_slice(bytes);
    *offset += bytes.len();
    Ok(())
}

/// Render a compiled plan and its selected evidence into a caller-supplied byte buffer.
/// Completely zero-heap: returns bytes written in caller's buffer.
pub fn render_into<'a>(
    plan: &CompiledPlanSummary<'a>,
    spec: &ConditioningSpec<'a>,
    selected_evidence: &[EvidencePart<'a>],
    target: RenderTarget,
    out: &mut [u8],
) -> Result<RenderedRequestSummary, ConditioningError> {
    let mut offset = 0;

    match target {
        RenderTarget::NativeChatml => {
            // System block: instructions + requirements
            write_str(out, &mut offset, "<|im_start|>system\n# Profile: ")?;
            write_str(out, &mut offset, plan.profile_id)?;
            write_str(out, &mut offset, "\n# Objective: ")?;
            write_str(out, &mut offset, plan.objective)?;
            write_str(out, &mut offset, "\n\n# Requirements:\n")?;

            for req in spec.requirements {
                write_str(out, &mut offset, "- [")?;
                write_str(out, &mut offset, req.id)?;
                write_str(out, &mut offset, "] ")?;
                write_str(out, &mut offset, req.rule)?;
                write_str(out, &mut offset, "\n")?;
            }

            if !selected_evidence.is_empty() {
                write_str(out, &mut offset, "\n# Evidence:\n")?;
                for ev in selected_evidence {
                    write_str(out, &mut offset, "Source ")?;
                    write_str(out, &mut offset, ev.source_id)?;
                    write_str(out, &mut offset, ": ")?;
                    write_str(out, &mut offset, ev.content)?;
                    write_str(out, &mut offset, "\n")?;
                }
            }
            write_str(out, &mut offset, "<|im_end|>\n<|im_start|>user\n")?;
            write_str(out, &mut offset, plan.objective)?;
            write_str(out, &mut offset, "<|im_end|>\n<|im_start|>assistant\n")?;
        }
        RenderTarget::NativePlain | RenderTarget::McpEnvelope => {
            // Plain text envelope
            write_str(out, &mut offset, "Objective: ")?;
            write_str(out, &mut offset, plan.objective)?;
            write_str(out, &mut offset, "\n\nRequirements:\n")?;

            for req in spec.requirements {
                write_str(out, &mut offset, "- ")?;
                write_str(out, &mut offset, req.rule)?;
                write_str(out, &mut offset, "\n")?;
            }

            if !selected_evidence.is_empty() {
                write_str(out, &mut offset, "\nEvidence:\n")?;
                for ev in selected_evidence {
                    write_str(out, &mut offset, "- ")?;
                    write_str(out, &mut offset, ev.content)?;
                    write_str(out, &mut offset, "\n")?;
                }
            }
        }
    }

    Ok(RenderedRequestSummary {
        bytes_written: offset,
        target,
    })
}
