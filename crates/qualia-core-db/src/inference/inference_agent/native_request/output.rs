//! Caller-owned native output record. Immutable tokenizer ownership remains
//! bound by the request snapshot; this exact accepted ID drives reconstruction.
pub(in crate::inference::inference_agent) struct NativeAcceptedToken(u32);
impl NativeAcceptedToken {
    pub(super) fn new(token: u32) -> Self {
        Self(token)
    }
    pub(in crate::inference::inference_agent) fn token_id(self) -> u32 {
        self.0
    }
}
