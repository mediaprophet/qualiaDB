//! Universal Spacetime & Reality Identifier (USRI) — parsing and generation.
//!
//! Grammar (OCS §13.1):
//! ```text
//! urn:omni:<version>:<realm_class>:<universe_or_observer>:<branch_or_state>:<hierarchy_path>[/nested:<spec>]*[/collapsed:<spec>]*#<anchor>
//! ```
//!
//! Reference: OCS Specification v2.2.0 §13.

use crate::value::Value;
use std::collections::BTreeMap;

/// Realm class — the top-level ontology of a USRI (OCS §13.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealmClass {
    Physical,
    Theorized,
    QuantumBranch,
    Hypothetical,
    Fiction,
    Simulation,
    Phenomenology,
    Noosphere,
    Narrative,
    Microverse,
}

impl RealmClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Physical => "physical",
            Self::Theorized => "theorized",
            Self::QuantumBranch => "quantum-branch",
            Self::Hypothetical => "hypothetical",
            Self::Fiction => "fiction",
            Self::Simulation => "simulation",
            Self::Phenomenology => "phenomenology",
            Self::Noosphere => "noosphere",
            Self::Narrative => "narrative",
            Self::Microverse => "microverse",
        }
    }

    /// 8-bit realm class index for CB-USRI (OCS §13.2).
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::Physical => 0,
            Self::Theorized => 1,
            Self::QuantumBranch => 2,
            Self::Fiction => 3,
            Self::Phenomenology => 4,
            Self::Noosphere => 5,
            Self::Microverse => 6,
            Self::Hypothetical => 7,
            Self::Simulation => 8,
            Self::Narrative => 9,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "physical" => Some(Self::Physical),
            "theorized" => Some(Self::Theorized),
            "quantum-branch" => Some(Self::QuantumBranch),
            "hypothetical" => Some(Self::Hypothetical),
            "fiction" => Some(Self::Fiction),
            "simulation" => Some(Self::Simulation),
            "phenomenology" => Some(Self::Phenomenology),
            "noosphere" => Some(Self::Noosphere),
            "narrative" => Some(Self::Narrative),
            "microverse" => Some(Self::Microverse),
            _ => None,
        }
    }
}

/// A parsed USRI (OCS §13.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Usri {
    pub version: String,
    pub realm_class: RealmClass,
    pub universe_or_observer: String,
    pub branch_or_state: String,
    pub hierarchy_path: String,
    pub nested: Vec<String>,
    pub collapsed: Vec<String>,
    pub anchor: String,
}

impl Usri {
    /// Parse a USRI string into its components.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s
            .strip_prefix("urn:omni:")
            .ok_or("USRI must start with 'urn:omni:'")?;

        // Split on '#' to separate anchor
        let (main, anchor) = match s.split_once('#') {
            Some((m, a)) => (m, a.to_string()),
            None => (s, String::new()),
        };

        // Split path on '/' to separate nested/collapsed from main hierarchy
        let parts: Vec<&str> = main.splitn(2, '/').collect();
        let main_path = parts[0];
        let extra = parts.get(1).unwrap_or(&"");

        // Parse main path: version:realm:universe:branch:hierarchy
        let segments: Vec<&str> = main_path.split(':').collect();
        if segments.len() < 5 {
            return Err(format!(
                "USRI needs at least 5 colon-separated segments, got {}",
                segments.len()
            ));
        }

        let version = segments[0].to_string();
        let realm_class = RealmClass::from_str(segments[1])
            .ok_or_else(|| format!("unknown realm class: {}", segments[1]))?;
        let universe_or_observer = segments[2].to_string();
        let branch_or_state = segments[3].to_string();
        // hierarchy_path is the rest joined by ':'
        let hierarchy_path = segments[4..].join(":");

        // Parse nested: and collapsed: segments
        let mut nested = Vec::new();
        let mut collapsed = Vec::new();
        if !extra.is_empty() {
            for seg in extra.split('/') {
                if let Some(spec) = seg.strip_prefix("nested:") {
                    nested.push(spec.to_string());
                } else if let Some(spec) = seg.strip_prefix("collapsed:") {
                    collapsed.push(spec.to_string());
                }
            }
        }

        Ok(Self {
            version,
            realm_class,
            universe_or_observer,
            branch_or_state,
            hierarchy_path,
            nested,
            collapsed,
            anchor,
        })
    }

    /// Render a USRI back to its canonical string form.
    pub fn to_string(&self) -> String {
        let mut out = format!(
            "urn:omni:{}:{}:{}:{}:{}",
            self.version,
            self.realm_class.as_str(),
            self.universe_or_observer,
            self.branch_or_state,
            self.hierarchy_path,
        );
        for n in &self.nested {
            out.push_str(&format!("/nested:{}", n));
        }
        for c in &self.collapsed {
            out.push_str(&format!("/collapsed:{}", c));
        }
        if !self.anchor.is_empty() {
            out.push_str(&format!("#{}", self.anchor));
        }
        out
    }

    /// Convert to a Value::Record for inspection and graph storage.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("version".into(), Value::String(self.version.clone()));
        rec.insert(
            "realm_class".into(),
            Value::String(self.realm_class.as_str().into()),
        );
        rec.insert(
            "universe_or_observer".into(),
            Value::String(self.universe_or_observer.clone()),
        );
        rec.insert(
            "branch_or_state".into(),
            Value::String(self.branch_or_state.clone()),
        );
        rec.insert(
            "hierarchy_path".into(),
            Value::String(self.hierarchy_path.clone()),
        );
        if !self.nested.is_empty() {
            rec.insert(
                "nested".into(),
                Value::List(
                    self.nested
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.collapsed.is_empty() {
            rec.insert(
                "collapsed".into(),
                Value::List(
                    self.collapsed
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.anchor.is_empty() {
            rec.insert("anchor".into(), Value::String(self.anchor.clone()));
        }
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_physical_earth_wgs84() {
        let u = Usri::parse(
            "urn:omni:v1:physical:observable:standard:laniakea:virgo:milkyway:sol:earth:wgs84",
        )
        .unwrap();
        assert_eq!(u.version, "v1");
        assert_eq!(u.realm_class, RealmClass::Physical);
        assert_eq!(u.universe_or_observer, "observable");
        assert_eq!(u.branch_or_state, "standard");
        assert!(u.hierarchy_path.contains("earth"));
        assert!(u.hierarchy_path.contains("wgs84"));
    }

    #[test]
    fn parse_with_geo_anchor() {
        let u = Usri::parse(
            "urn:omni:v1:physical:observable:standard:earth:wgs84#geo(lat=37.8080,lon=-122.4177,alt=10.0)",
        )
        .unwrap();
        assert_eq!(u.anchor, "geo(lat=37.8080,lon=-122.4177,alt=10.0)");
    }

    #[test]
    fn parse_fiction_star_trek() {
        let u = Usri::parse(
            "urn:omni:v1:fiction:star-trek:prime:alpha-quadrant:sector-001:sol:earth:san-francisco:starfleet-academy",
        )
        .unwrap();
        assert_eq!(u.realm_class, RealmClass::Fiction);
        assert_eq!(u.universe_or_observer, "star-trek");
        assert_eq!(u.branch_or_state, "prime");
    }

    #[test]
    fn parse_with_nested_and_collapsed() {
        let u = Usri::parse(
            "urn:omni:v1:narrative:homer:iliad:troy:scaian-gate/collapsed:physical:observable:standard:earth:wgs84:turkey:hisarlik:stratum-viia#geo(lat=39.9575,lon=26.2389,alt=35)",
        )
        .unwrap();
        assert_eq!(u.realm_class, RealmClass::Narrative);
        assert_eq!(u.collapsed.len(), 1);
        assert!(u.collapsed[0].contains("hisarlik"));
        assert!(u.anchor.contains("39.9575"));
    }

    #[test]
    fn round_trip_physical() {
        let original = "urn:omni:v1:physical:observable:standard:earth:wgs84";
        let u = Usri::parse(original).unwrap();
        assert_eq!(u.to_string(), original);
    }

    #[test]
    fn round_trip_with_anchor() {
        let original =
            "urn:omni:v1:physical:observable:standard:earth:wgs84#geo(lat=37.8,lon=-122.4)";
        let u = Usri::parse(original).unwrap();
        assert_eq!(u.to_string(), original);
    }

    #[test]
    fn round_trip_with_collapsed() {
        let original = "urn:omni:v1:narrative:homer:iliad:troy/collapsed:physical:earth:hisarlik";
        let u = Usri::parse(original).unwrap();
        assert_eq!(u.to_string(), original);
    }

    #[test]
    fn realm_class_round_trip() {
        for rc in [
            RealmClass::Physical,
            RealmClass::Fiction,
            RealmClass::Narrative,
            RealmClass::Phenomenology,
            RealmClass::Microverse,
        ] {
            let s = rc.as_str();
            assert_eq!(RealmClass::from_str(s), Some(rc));
        }
    }

    #[test]
    fn invalid_usri_fails() {
        assert!(Usri::parse("not-a-usri").is_err());
        assert!(Usri::parse("urn:omni:v1:badrealm:x:y:z").is_err());
    }

    #[test]
    fn to_value_has_realm_class() {
        let u = Usri::parse("urn:omni:v1:physical:observable:standard:earth").unwrap();
        let v = u.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(
                    r.get("realm_class"),
                    Some(&Value::String("physical".into()))
                );
                assert_eq!(r.get("version"), Some(&Value::String("v1".into())));
            }
            _ => panic!("expected Record"),
        }
    }
}
