//! Error types for portable application contracts.

use thiserror::Error;

/// Error type for application contract validation and serialization.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ContractError {
    #[error("command ID cannot be empty")]
    EmptyCommandId,

    #[error("invalid command ID format '{0}': must follow '<app-id>::<upstream-id>' with valid ASCII identifiers")]
    InvalidCommandIdFormat(String),

    #[error("app ID cannot be empty")]
    EmptyAppId,

    #[error("JS-unsafe numeric conversion: integer {0} exceeds JavaScript MAX_SAFE_INTEGER (9007199254740991); must use decimal string representation")]
    JsUnsafeNumericConversion(u64),

    #[error("invalid decimal u64 string '{0}'")]
    InvalidDecimalU64(String),

    #[error("non-finite coordinate detected: {0}")]
    NonFiniteCoordinate(String),

    #[error("malformed schema or parameters: {0}")]
    MalformedParams(String),

    #[error("duplicate command alias '{0}'")]
    DuplicateAlias(String),

    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("interactive gestures unsupported: {0}")]
    GesturesUnsupported(String),

    #[error("unsupported target kind '{0}' for command '{1}'")]
    UnsupportedTarget(String, String),

    #[error("oversized payload: size {size} bytes exceeds limit {limit} bytes")]
    OversizedPayload { size: usize, limit: usize },

    #[error("unknown schema or contract version: {0}")]
    UnknownVersion(u32),

    #[error("unknown effect name: '{0}'")]
    UnknownEffect(String),

    #[error("duplicate command ID registered: '{0}'")]
    DuplicateCommandId(String),

    #[error("serialization or deserialization error: {0}")]
    Serialization(String),
}
