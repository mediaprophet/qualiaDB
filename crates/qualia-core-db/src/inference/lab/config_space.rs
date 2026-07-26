//! Typed configuration space for the optimization lab.
//!
//! Each optimization strategy exposes tunable parameters. The lab defines these
//! as a `ConfigurationSpace` (analogous to SMAC3's `ConfigurationSpace`).
//! Configurations are serialized to CBOR and hashed with FNV-1a for deduplication.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One parameter definition in the configuration space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterDef {
    /// Integer in `[lo, hi]` (inclusive).
    Int { lo: i64, hi: i64 },
    /// Float in `[lo, hi]`.
    Float { lo: f64, hi: f64 },
    /// Boolean on/off.
    Bool,
    /// Categorical choice from a fixed set of strings.
    Categorical { choices: Vec<String> },
}

impl ParameterDef {
    /// Clamp a raw value into this parameter's valid range.
    pub fn clamp(&self, raw: &ParameterValue) -> ParameterValue {
        match (self, raw) {
            (ParameterDef::Int { lo, hi }, ParameterValue::Int(v)) => {
                ParameterValue::Int((*v).clamp(*lo, *hi))
            }
            (ParameterDef::Float { lo, hi }, ParameterValue::Float(v)) => {
                ParameterValue::Float(v.clamp(*lo, *hi))
            }
            (ParameterDef::Bool, ParameterValue::Bool(v)) => ParameterValue::Bool(*v),
            (ParameterDef::Categorical { choices }, ParameterValue::String(v)) => {
                if choices.iter().any(|c| c == v) {
                    ParameterValue::String(v.clone())
                } else {
                    ParameterValue::String(choices.first().cloned().unwrap_or_default())
                }
            }
            _ => raw.clone(),
        }
    }

    /// Check if a value is valid for this parameter.
    pub fn is_valid(&self, val: &ParameterValue) -> bool {
        match (self, val) {
            (ParameterDef::Int { lo, hi }, ParameterValue::Int(v)) => v >= lo && v <= hi,
            (ParameterDef::Float { lo, hi }, ParameterValue::Float(v)) => v >= lo && v <= hi,
            (ParameterDef::Bool, ParameterValue::Bool(_)) => true,
            (ParameterDef::Categorical { choices }, ParameterValue::String(v)) => {
                choices.iter().any(|c| c == v)
            }
            _ => false,
        }
    }
}

/// A concrete parameter value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

impl ParameterValue {
    /// Convert to a normalized `[0, 1]` float for Sobol / search purposes.
    pub fn normalize(&self, def: &ParameterDef) -> f64 {
        match (def, self) {
            (ParameterDef::Int { lo, hi }, ParameterValue::Int(v)) => {
                if hi == lo {
                    0.5
                } else {
                    (*v as f64 - *lo as f64) / (*hi as f64 - *lo as f64)
                }
            }
            (ParameterDef::Float { lo, hi }, ParameterValue::Float(v)) => {
                if hi == lo {
                    0.5
                } else {
                    (v - lo) / (hi - lo)
                }
            }
            (ParameterDef::Bool, ParameterValue::Bool(v)) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            }
            (ParameterDef::Categorical { choices }, ParameterValue::String(v)) => {
                let idx = choices.iter().position(|c| c == v).unwrap_or(0);
                if choices.len() <= 1 {
                    0.5
                } else {
                    idx as f64 / (choices.len() - 1) as f64
                }
            }
            _ => 0.5,
        }
    }

    /// Denormalize a `[0, 1]` float back to a concrete value.
    pub fn denormalize(t: f64, def: &ParameterDef) -> ParameterValue {
        let t = t.clamp(0.0, 1.0);
        match def {
            ParameterDef::Int { lo, hi } => {
                let v = lo + ((t * (*hi - *lo) as f64).round() as i64);
                ParameterValue::Int(v.clamp(*lo, *hi))
            }
            ParameterDef::Float { lo, hi } => ParameterValue::Float(lo + t * (hi - lo)),
            ParameterDef::Bool => ParameterValue::Bool(t >= 0.5),
            ParameterDef::Categorical { choices } => {
                if choices.is_empty() {
                    return ParameterValue::String(String::new());
                }
                let idx = if choices.len() == 1 {
                    0
                } else {
                    ((t * (choices.len() - 1) as f64).round() as usize).min(choices.len() - 1)
                };
                ParameterValue::String(choices[idx].clone())
            }
        }
    }
}

/// A configuration space: named parameters with definitions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigurationSpace {
    pub name: String,
    pub params: BTreeMap<String, ParameterDef>,
}

impl ConfigurationSpace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn with(mut self, name: impl Into<String>, def: ParameterDef) -> Self {
        self.params.insert(name.into(), def);
        self
    }

    /// Number of dimensions in the search space.
    pub fn dims(&self) -> usize {
        self.params.len()
    }

    /// Build a default `Configuration` with all parameters at their lower bound
    /// (Int: lo, Float: lo, Bool: false, Categorical: first choice).
    pub fn default_config(&self) -> Configuration {
        let values: BTreeMap<String, ParameterValue> = self
            .params
            .iter()
            .map(|(name, def)| {
                let val = match def {
                    ParameterDef::Int { lo, .. } => ParameterValue::Int(*lo),
                    ParameterDef::Float { lo, .. } => ParameterValue::Float(*lo),
                    ParameterDef::Bool => ParameterValue::Bool(false),
                    ParameterDef::Categorical { choices } => {
                        ParameterValue::String(choices.first().cloned().unwrap_or_default())
                    }
                };
                (name.clone(), val)
            })
            .collect();
        Configuration {
            space_name: self.name.clone(),
            values,
        }
    }

    /// Build a `Configuration` from a map of values, clamping each to its def.
    pub fn build_config(&self, values: BTreeMap<String, ParameterValue>) -> Configuration {
        let clamped = values
            .iter()
            .map(|(k, v)| {
                let cv = self
                    .params
                    .get(k)
                    .map(|d| d.clamp(v))
                    .unwrap_or_else(|| v.clone());
                (k.clone(), cv)
            })
            .collect();
        Configuration {
            space_name: self.name.clone(),
            values: clamped,
        }
    }

    /// Build a `Configuration` from a normalized `[0,1]^d` vector (Sobol output).
    pub fn build_from_normalized(&self, t: &[f64]) -> Configuration {
        let mut values = BTreeMap::new();
        for (i, (name, def)) in self.params.iter().enumerate() {
            let t_i = t.get(i).copied().unwrap_or(0.5);
            values.insert(name.clone(), ParameterValue::denormalize(t_i, def));
        }
        Configuration {
            space_name: self.name.clone(),
            values,
        }
    }

    /// Normalize a configuration back to `[0,1]^d`.
    pub fn normalize_config(&self, cfg: &Configuration) -> Vec<f64> {
        self.params
            .iter()
            .map(|(name, def)| {
                cfg.values
                    .get(name)
                    .map(|v| v.normalize(def))
                    .unwrap_or(0.5)
            })
            .collect()
    }
}

/// A concrete configuration: one point in the space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub space_name: String,
    pub values: BTreeMap<String, ParameterValue>,
}

impl Configuration {
    /// Serialize to CBOR bytes.
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(self, &mut buf);
        buf
    }

    /// Deserialize from CBOR bytes.
    pub fn from_cbor(data: &[u8]) -> Result<Self, String> {
        ciborium::from_reader(data).map_err(|e| e.to_string())
    }

    /// FNV-1a hash of the CBOR serialization (dedup key).
    pub fn hash(&self) -> u64 {
        let data = self.to_cbor();
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &data {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Get a parameter value.
    pub fn get(&self, name: &str) -> Option<&ParameterValue> {
        self.values.get(name)
    }

    /// Get an integer parameter.
    pub fn get_int(&self, name: &str) -> Option<i64> {
        match self.values.get(name) {
            Some(ParameterValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a float parameter.
    pub fn get_float(&self, name: &str) -> Option<f64> {
        match self.values.get(name) {
            Some(ParameterValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a bool parameter.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.values.get(name) {
            Some(ParameterValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a string parameter.
    pub fn get_string(&self, name: &str) -> Option<&str> {
        match self.values.get(name) {
            Some(ParameterValue::String(v)) => Some(v),
            _ => None,
        }
    }

    /// JSON representation for logging.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_space_roundtrip() {
        let space = ConfigurationSpace::new("test")
            .with("ngram", ParameterDef::Int { lo: 1, hi: 3 })
            .with("bias", ParameterDef::Float { lo: 0.5, hi: 5.0 })
            .with("enabled", ParameterDef::Bool)
            .with(
                "mode",
                ParameterDef::Categorical {
                    choices: vec!["fast".into(), "slow".into()],
                },
            );
        assert_eq!(space.dims(), 4);

        // BTreeMap iterates in alphabetical key order: bias, enabled, mode, ngram.
        let cfg = space.build_from_normalized(&[0.5, 1.0, 0.0, 0.0]);
        // bias: t=0.5 → 0.5 + 0.5*(5.0-0.5) = 2.75
        assert!((cfg.get_float("bias").unwrap() - 2.75).abs() < 0.01);
        // enabled: t=1.0 → true
        assert_eq!(cfg.get_bool("enabled"), Some(true));
        // mode: t=0.0 → "fast" (first choice)
        assert_eq!(cfg.get_string("mode"), Some("fast"));
        // ngram: t=0.0 → lo=1
        assert_eq!(cfg.get_int("ngram"), Some(1));
    }

    #[test]
    fn config_hash_dedup() {
        let space = ConfigurationSpace::new("test").with("x", ParameterDef::Int { lo: 0, hi: 10 });
        let a = space.build_from_normalized(&[0.5]);
        let b = space.build_from_normalized(&[0.5]);
        let c = space.build_from_normalized(&[0.0]);
        assert_eq!(a.hash(), b.hash());
        assert_ne!(a.hash(), c.hash());
    }

    #[test]
    fn config_cbor_roundtrip() {
        let space = ConfigurationSpace::new("test")
            .with("x", ParameterDef::Int { lo: 0, hi: 10 })
            .with("y", ParameterDef::Bool);
        let cfg = space.build_from_normalized(&[0.3, 0.6]);
        let cbor = cfg.to_cbor();
        let back = Configuration::from_cbor(&cbor).unwrap();
        assert_eq!(back.get_int("x"), cfg.get_int("x"));
        assert_eq!(back.get_bool("y"), cfg.get_bool("y"));
    }

    #[test]
    fn clamp_works() {
        let def = ParameterDef::Int { lo: 1, hi: 5 };
        assert_eq!(def.clamp(&ParameterValue::Int(10)), ParameterValue::Int(5));
        assert_eq!(def.clamp(&ParameterValue::Int(0)), ParameterValue::Int(1));
    }
}
