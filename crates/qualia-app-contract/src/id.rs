//! Identifiers and safe numeric types for application contracts.

use crate::error::ContractError;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Maximum integer safely representable in JavaScript IEEE-754 numbers (2^53 - 1).
pub const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Application identifier (e.g. `gridcraft`, `wordcraft`, `poet`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId(String);

impl AppId {
    /// Create and validate a new AppId.
    pub fn new(id: impl Into<String>) -> Result<Self, ContractError> {
        let s = id.into();
        if s.trim().is_empty() {
            return Err(ContractError::EmptyAppId);
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return Err(ContractError::InvalidCommandIdFormat(s));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AppId {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// Fully-qualified command identifier following the `<app-id>::<upstream-id>` schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandId(String);

impl CommandId {
    /// Parse and validate a command identifier.
    pub fn parse(raw: impl Into<String>) -> Result<Self, ContractError> {
        let s = raw.into();
        if s.trim().is_empty() {
            return Err(ContractError::EmptyCommandId);
        }

        let parts: Vec<&str> = s.split("::").collect();
        if parts.len() != 2 {
            return Err(ContractError::InvalidCommandIdFormat(s));
        }

        let app_part = parts[0];
        let cmd_part = parts[1];

        if app_part.is_empty()
            || cmd_part.is_empty()
            || !app_part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            || !cmd_part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ContractError::InvalidCommandIdFormat(s));
        }

        Ok(Self(s))
    }

    /// Construct a command identifier from app ID and upstream ID.
    pub fn from_parts(app_id: &AppId, upstream_id: &str) -> Result<Self, ContractError> {
        Self::parse(format!("{}::{}", app_id.as_str(), upstream_id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn app_id(&self) -> &str {
        self.0.split("::").next().unwrap_or("")
    }

    pub fn upstream_id(&self) -> &str {
        self.0.split("::").nth(1).unwrap_or("")
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CommandId {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// A 64-bit unsigned integer that is serialized as a decimal string in JSON
/// to prevent precision loss across JavaScript, browser, and JSON boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DecimalU64(pub u64);

impl DecimalU64 {
    pub fn new(val: u64) -> Self {
        Self(val)
    }

    pub fn get(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for DecimalU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for DecimalU64 {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl From<DecimalU64> for u64 {
    fn from(val: DecimalU64) -> Self {
        val.0
    }
}

impl FromStr for DecimalU64 {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        trimmed
            .parse::<u64>()
            .map(Self)
            .map_err(|_| ContractError::InvalidDecimalU64(s.to_string()))
    }
}

impl Serialize for DecimalU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecimalU64Visitor;

        impl<'de> de::Visitor<'de> for DecimalU64Visitor {
            type Value = DecimalU64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a decimal string representing a u64, or a safe integer <= 2^53 - 1")
            }

            fn visit_str<E>(self, value: &str) -> Result<DecimalU64, E>
            where
                E: de::Error,
            {
                DecimalU64::from_str(value).map_err(de::Error::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<DecimalU64, E>
            where
                E: de::Error,
            {
                if value > JS_MAX_SAFE_INTEGER {
                    return Err(de::Error::custom(ContractError::JsUnsafeNumericConversion(value)));
                }
                Ok(DecimalU64(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<DecimalU64, E>
            where
                E: de::Error,
            {
                if value < 0 {
                    return Err(de::Error::custom("u64 cannot be negative"));
                }
                self.visit_u64(value as u64)
            }
        }

        deserializer.deserialize_any(DecimalU64Visitor)
    }
}

/// Helper function to validate that a floating point value is strictly finite.
pub fn validate_finite_f64(val: f64, field_name: &str) -> Result<f64, ContractError> {
    if !val.is_finite() {
        return Err(ContractError::NonFiniteCoordinate(format!(
            "field '{}' has non-finite value {}",
            field_name, val
        )));
    }
    Ok(val)
}
