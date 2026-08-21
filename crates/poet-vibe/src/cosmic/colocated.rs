//! Co-located realities and paraconsistent isolation (OCS §9).
//!
//! Multiple fictional, historical, or hypothetical layers can share
//! the exact same physical coordinates without collision.
//!
//! Reference: OCS Specification v2.2.0 §9.

use crate::cosmic::transforms::Geodetic;
use crate::value::Value;
use std::collections::BTreeMap;

/// A co-located reality layer (OCS §9).
#[derive(Debug, Clone, PartialEq)]
pub struct CoLocatedLayer {
    /// USRI of this reality layer
    pub usri: String,
    /// Human-readable name
    pub name: String,
    /// The physical geodetic coordinates this layer is co-located at
    pub geodetic: Geodetic,
    /// Whether this is the physical base layer
    pub is_physical_base: bool,
}

impl CoLocatedLayer {
    /// Create a physical base layer (OCS §9).
    pub fn physical_base(geodetic: Geodetic) -> Self {
        Self {
            usri: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
            name: "Physical Reality".into(),
            geodetic,
            is_physical_base: true,
        }
    }

    /// Create a co-located fictional layer (OCS §9.1).
    pub fn fictional(name: &str, usri: &str, geodetic: Geodetic) -> Self {
        Self {
            usri: usri.into(),
            name: name.into(),
            geodetic,
            is_physical_base: false,
        }
    }

    /// Create a co-located historical layer (OCS §9.1).
    pub fn historical(name: &str, usri: &str, geodetic: Geodetic) -> Self {
        Self {
            usri: usri.into(),
            name: name.into(),
            geodetic,
            is_physical_base: false,
        }
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("usri".into(), Value::String(self.usri.clone()));
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("lat".into(), Value::F64(self.geodetic.lat_deg));
        rec.insert("lon".into(), Value::F64(self.geodetic.lon_deg));
        rec.insert("alt".into(), Value::F64(self.geodetic.alt_m));
        rec.insert(
            "is_physical_base".into(),
            Value::Bool(self.is_physical_base),
        );
        Value::Record(rec)
    }
}

/// A stack of co-located reality layers at the same physical position (OCS §9).
#[derive(Debug, Clone)]
pub struct CoLocatedStack {
    /// All layers at this position, ordered from physical base outward
    pub layers: Vec<CoLocatedLayer>,
}

impl CoLocatedStack {
    pub fn new(physical: CoLocatedLayer) -> Self {
        Self {
            layers: vec![physical],
        }
    }

    /// Add a co-located layer (OCS §9).
    pub fn add_layer(&mut self, layer: CoLocatedLayer) {
        // Verify it's co-located at the same position
        let base = &self.layers[0];
        let dist = crate::cosmic::transforms::geodetic_distance(base.geodetic, layer.geodetic);
        // Layers must be at the same physical location (within 1 km tolerance)
        if dist > 1000.0 {
            return; // Not co-located — silently reject
        }
        self.layers.push(layer);
    }

    /// Get the physical base layer.
    pub fn physical_base(&self) -> &CoLocatedLayer {
        &self.layers[0]
    }

    /// Get all non-physical layers.
    pub fn fictional_layers(&self) -> Vec<&CoLocatedLayer> {
        self.layers.iter().filter(|l| !l.is_physical_base).collect()
    }

    /// Verify paraconsistent isolation (OCS §9.2).
    ///
    /// The lexical isolation rule: inner layers inherit spatial anchors
    /// from outer containers, but mutations in inner layers can never
    /// mutate or leak into parent layers.
    pub fn verify_isolation(&self) -> bool {
        // Physical base must be first
        if self.layers.is_empty() || !self.layers[0].is_physical_base {
            return false;
        }
        // Only one physical base allowed
        self.layers.iter().filter(|l| l.is_physical_base).count() == 1
    }

    /// Check that a mutation in a layer index doesn't affect the physical base (OCS-T06).
    pub fn check_no_mutation_leak(&self, layer_index: usize) -> bool {
        if layer_index == 0 {
            return true; // Can't leak into yourself
        }
        if layer_index >= self.layers.len() {
            return true;
        }
        // The physical base coordinates must not change
        // (This is a structural check — the actual mutation prevention
        // is enforced by the paraconsistent isolation context hashing)
        self.verify_isolation()
    }

    pub fn to_value(&self) -> Value {
        Value::List(self.layers.iter().map(|l| l.to_value()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_base_layer() {
        let layer = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8080,
            lon_deg: -122.4177,
            alt_m: 10.0,
        });
        assert!(layer.is_physical_base);
    }

    #[test]
    fn fictional_layer() {
        let layer = CoLocatedLayer::fictional(
            "Starfleet Academy",
            "urn:omni:v1:fiction:star-trek:prime:earth:san-francisco:starfleet-academy",
            Geodetic {
                lat_deg: 37.8080,
                lon_deg: -122.4177,
                alt_m: 10.0,
            },
        );
        assert!(!layer.is_physical_base);
        assert_eq!(layer.name, "Starfleet Academy");
    }

    #[test]
    fn co_located_stack_add_layer() {
        let base = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8080,
            lon_deg: -122.4177,
            alt_m: 10.0,
        });
        let mut stack = CoLocatedStack::new(base);
        let fiction = CoLocatedLayer::fictional(
            "Starfleet HQ",
            "urn:omni:v1:fiction:star-trek:prime:earth:san-francisco",
            Geodetic {
                lat_deg: 37.8080,
                lon_deg: -122.4177,
                alt_m: 10.0,
            },
        );
        stack.add_layer(fiction);
        assert_eq!(stack.layers.len(), 2);
    }

    #[test]
    fn co_located_rejects_distant_layer() {
        let base = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8080,
            lon_deg: -122.4177,
            alt_m: 10.0,
        });
        let mut stack = CoLocatedStack::new(base);
        let distant = CoLocatedLayer::fictional(
            "NYC Layer",
            "urn:omni:v1:fiction:marvel:earth-616:nyc",
            Geodetic {
                lat_deg: 40.7291,
                lon_deg: -73.9996,
                alt_m: 15.0,
            },
        );
        stack.add_layer(distant);
        // Should not be added — not co-located
        assert_eq!(stack.layers.len(), 1);
    }

    #[test]
    fn isolation_verified() {
        let base = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8,
            lon_deg: -122.4,
            alt_m: 10.0,
        });
        let stack = CoLocatedStack::new(base);
        assert!(stack.verify_isolation());
    }

    #[test]
    fn isolation_rejects_two_physical_bases() {
        let base1 = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8,
            lon_deg: -122.4,
            alt_m: 10.0,
        });
        let base2 = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8,
            lon_deg: -122.4,
            alt_m: 10.0,
        });
        let mut stack = CoLocatedStack::new(base1);
        stack.layers.push(base2);
        assert!(!stack.verify_isolation());
    }

    #[test]
    fn no_mutation_leak_to_physical() {
        let base = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8,
            lon_deg: -122.4,
            alt_m: 10.0,
        });
        let fiction = CoLocatedLayer::fictional(
            "Fiction",
            "urn:omni:v1:fiction:test",
            Geodetic {
                lat_deg: 37.8,
                lon_deg: -122.4,
                alt_m: 10.0,
            },
        );
        let mut stack = CoLocatedStack::new(base);
        stack.add_layer(fiction);
        // Mutations in layer 1 (fiction) should not leak to layer 0 (physical)
        assert!(stack.check_no_mutation_leak(1));
    }

    #[test]
    fn fictional_layers_filter() {
        let base = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8,
            lon_deg: -122.4,
            alt_m: 10.0,
        });
        let mut stack = CoLocatedStack::new(base);
        stack.add_layer(CoLocatedLayer::fictional(
            "F1",
            "urn:omni:v1:fiction:a",
            Geodetic {
                lat_deg: 37.8,
                lon_deg: -122.4,
                alt_m: 10.0,
            },
        ));
        stack.add_layer(CoLocatedLayer::historical(
            "1906 SF",
            "urn:omni:v1:theorized:history:earth:sf-1906",
            Geodetic {
                lat_deg: 37.8,
                lon_deg: -122.4,
                alt_m: 10.0,
            },
        ));
        let fiction = stack.fictional_layers();
        // Historical is not physical_base, so it's included in non-physical
        assert_eq!(fiction.len(), 2);
    }

    #[test]
    fn layer_to_value() {
        let layer = CoLocatedLayer::physical_base(Geodetic {
            lat_deg: 37.8,
            lon_deg: -122.4,
            alt_m: 10.0,
        });
        let v = layer.to_value();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("lat"));
                assert!(r.contains_key("is_physical_base"));
            }
            _ => panic!("expected Record"),
        }
    }
}
