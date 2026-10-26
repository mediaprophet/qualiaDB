use serde::Serialize;
use serde_json::Value;

pub const AUTHORING_PROTOCOL_VERSION: &str = "2025-11-25";

/// Backends advertise only implemented operations. Authorization belongs in the
/// shared native gateway on EVERY call, never in generic protocol arguments.
/// Backend methods must not print to stdout or expose private error details.
pub trait AuthoringBackend {
    fn tools(&self) -> Vec<AuthoringTool>;
    /// Return a domain result object; the protocol supplies MCP result wrapping.
    fn call(&mut self, name: &str, args: Value) -> Result<Value, ToolError>;
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthoringTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Public, sanitized domain failure. Code/message must not contain source,
/// credentials, private paths, raw engine diagnostics or debug object dumps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
}

impl ToolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProtocolLimits {
    pub max_message_bytes: usize,
    pub max_tools: usize,
    pub max_id_bytes: usize,
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: 1024 * 1024,
            max_tools: 64,
            max_id_bytes: 256,
        }
    }
}

impl ProtocolLimits {
    pub(super) fn validate(self) -> Result<Self, ProtocolError> {
        if self.max_message_bytes < 512
            || self.max_id_bytes == 0
            || self.max_id_bytes > self.max_message_bytes / 16
        {
            return Err(ProtocolError::InvalidLimits);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Uninitialized,
    AwaitingInitialized,
    Ready,
}

#[derive(Debug)]
pub enum ProtocolError {
    InvalidLimits,
    InvalidCatalog,
    FrameTooLarge,
    InvalidUtf8,
    UnterminatedFrame,
    ResourceLimit,
    Io(std::io::ErrorKind),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // I/O kind only: never disclose the reader/backend's private error text.
        write!(f, "MCP authoring transport failed: {self:?}")
    }
}
impl std::error::Error for ProtocolError {}
impl From<std::io::Error> for ProtocolError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.kind())
    }
}

/// Count serialized bytes without allocating a second unbounded output buffer.
pub(super) struct ByteBudget {
    pub remaining: usize,
}
impl std::io::Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(super) fn within_budget(value: &impl Serialize, bytes: usize) -> bool {
    serde_json::to_writer(ByteBudget { remaining: bytes }, value).is_ok()
}
