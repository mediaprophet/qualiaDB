//! Explicit developer adapters for the canonical Etalon Qwen engine.
//! Product generation uses NativeProductSession::decode_qwen_into with retained sources.
pub use etalon_inference::qwen::decode::{Qwen4ExpDecodeError, Qwen4ExpDecodeReceipt, Qwen4ExpDecodeScratch, Qwen4ExpSession, Qwen4ExpTraceRecord, Qwen4ExpTraceSink, Qwen4ExpTraceSummary, Qwen4ExpTraceBuffer, QWEN4EXP_TRACE_META_LAYER};
use crate::inference::inference_agent::NativeDecodeRequest;
use super::{Qwen4ExpNativeRuntime, StreamedArgmax};

pub fn decode_step(request: NativeDecodeRequest, runtime: &mut Qwen4ExpNativeRuntime, session: &mut Qwen4ExpSession, scratch: &mut Qwen4ExpDecodeScratch, token: u32, trace: &mut Vec<Qwen4ExpTraceRecord>, capture: bool) -> Result<StreamedArgmax, Qwen4ExpDecodeError> {
    if !matches!(request, NativeDecodeRequest::Developer) { return Err(Qwen4ExpDecodeError::PublicationDenied); }
    etalon_inference::qwen::decode_step(&mut runtime.engine(), session, scratch, token, trace, capture)
}
pub fn decode_tokens(request: NativeDecodeRequest, runtime: &mut Qwen4ExpNativeRuntime, session: &mut Qwen4ExpSession, scratch: &mut Qwen4ExpDecodeScratch, prompt: &[u32], max_tokens: usize, is_stop: &dyn Fn(u32) -> bool, receipt: &mut Qwen4ExpDecodeReceipt, trace: &mut Vec<Qwen4ExpTraceRecord>) -> Result<(), Qwen4ExpDecodeError> {
    if !matches!(request, NativeDecodeRequest::Developer) { return Err(Qwen4ExpDecodeError::PublicationDenied); }
    etalon_inference::qwen::decode_tokens(&mut runtime.engine(), session, scratch, prompt, max_tokens, is_stop, receipt, trace)
}
