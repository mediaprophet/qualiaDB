use std::collections::HashMap;

use crate::modalities::spatio_temporal::{self, Rcc8Relation};
use crate::q_hash;
use crate::NQuin;

/// A registry mapping spatial regions (e.g., geohash or bounding box hash) to
/// governed N3 trigger rules. When a user or entity crosses into a region,
/// the corresponding rules are injected into the Webizen VM for evaluation.
pub struct LocationTriggerRegistry {
    pub region_rules: HashMap<u64, Vec<NQuin>>,
}

impl LocationTriggerRegistry {
    pub fn new() -> Self {
        Self {
            region_rules: HashMap::new(),
        }
    }

    pub fn register_trigger(&mut self, region_hash: u64, rule_quin: NQuin) {
        self.region_rules
            .entry(region_hash)
            .or_default()
            .push(rule_quin);
    }

    pub fn evaluate_triggers_at(&self, location_hash: u64) -> Vec<NQuin> {
        let mut triggers = Vec::new();
        if let Some(rules) = self.region_rules.get(&location_hash) {
            triggers.extend(rules.iter().copied());
        }
        triggers
    }
}

impl Default for LocationTriggerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Engine that fires location triggers, optionally using RCC-8 containment when
/// boundary quins are available in the arena.
pub struct LocationTriggerEngine {
    registry: LocationTriggerRegistry,
}

impl LocationTriggerEngine {
    pub fn new() -> Self {
        Self {
            registry: LocationTriggerRegistry::new(),
        }
    }

    pub fn register_trigger(&mut self, region_hash: u64, rule_quin: NQuin) {
        self.registry.register_trigger(region_hash, rule_quin);
    }

    /// Collect triggers that should fire when entering `location_hash`.
    /// Evaluates RCC-8 containment when boundary quins and a location point are present.
    pub fn on_enter(&self, location_hash: u64, arena_quins: &[NQuin]) -> Vec<NQuin> {
        self.fire_triggers_at(location_hash, arena_quins)
    }

    /// Returns rule quins to inject for the given location.
    pub fn fire_triggers_at(&self, location_hash: u64, arena_quins: &[NQuin]) -> Vec<NQuin> {
        let mut triggers = self.registry.evaluate_triggers_at(location_hash);

        let boundary = q_hash("spatial:boundary");
        let location_point = q_hash("q42:locationPoint");

        let loc_point = arena_quins.iter().find_map(|quin| {
            if quin.subject == location_hash && quin.predicate == location_point {
                Some(spatio_temporal::unpack_point(quin.object))
            } else {
                None
            }
        });

        if let Some(point) = loc_point {
            let point_slice = [point];
            for (region_hash, rules) in &self.registry.region_rules {
                if *region_hash == location_hash {
                    continue;
                }
                let region_poly = collect_boundary_points(*region_hash, arena_quins, boundary);
                if region_poly.len() < 3 {
                    continue;
                }
                let relation = spatio_temporal::evaluate_rcc8_points(
                    location_hash,
                    &point_slice,
                    *region_hash,
                    &region_poly,
                );
                if is_containment(relation) {
                    for rule in rules {
                        if !triggers.iter().any(|t| t == rule) {
                            triggers.push(*rule);
                        }
                    }
                }
            }
        }

        triggers
    }
}

impl Default for LocationTriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn is_containment(relation: Rcc8Relation) -> bool {
    matches!(
        relation,
        Rcc8Relation::NonTangentialProperPart
            | Rcc8Relation::TangentiallyProperPart
            | Rcc8Relation::Equal
    )
}

fn collect_boundary_points(
    region_id: u64,
    arena_quins: &[NQuin],
    boundary_pred: u64,
) -> Vec<(f64, f64)> {
    let mut indexed: Vec<(u32, (f64, f64))> = Vec::new();
    for quin in arena_quins {
        if quin.subject == region_id && quin.predicate == boundary_pred {
            let idx = quin.metadata as u32;
            indexed.push((idx, spatio_temporal::unpack_point(quin.object)));
        }
    }
    indexed.sort_by_key(|(idx, _)| *idx);
    indexed.into_iter().map(|(_, pt)| pt).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_evaluate_trigger() {
        let mut registry = LocationTriggerRegistry::new();
        let region = 0x123456789;
        let rule = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        registry.register_trigger(region, rule);

        let triggers = registry.evaluate_triggers_at(region);
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].subject, 1);

        let no_triggers = registry.evaluate_triggers_at(0x999);
        assert_eq!(no_triggers.len(), 0);
    }

    #[test]
    fn test_location_trigger_engine_rcc8_containment() {
        let mut engine = LocationTriggerEngine::new();
        let region = 0xDEAD_BEEF;
        let location = 0xCAFE_BABE;

        let rule = NQuin {
            subject: 10,
            predicate: 20,
            object: 30,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        engine.register_trigger(region, rule);

        let boundary = q_hash("spatial:boundary");
        let location_point = q_hash("q42:locationPoint");

        // Square region containing the point (0.5, 0.5)
        let region_quins = [
            boundary_quin(region, boundary, 0, (0.0, 0.0)),
            boundary_quin(region, boundary, 1, (2.0, 0.0)),
            boundary_quin(region, boundary, 2, (2.0, 2.0)),
            boundary_quin(region, boundary, 3, (0.0, 2.0)),
        ];

        let mut loc_quin = NQuin {
            subject: location,
            predicate: location_point,
            object: spatio_temporal::pack_point(0.5, 0.5),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        loc_quin.parity =
            loc_quin.subject ^ loc_quin.predicate ^ loc_quin.object ^ loc_quin.context;

        let mut arena: Vec<NQuin> = region_quins.to_vec();
        arena.push(loc_quin);

        let triggers = engine.fire_triggers_at(location, &arena);
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].subject, 10);
    }

    fn boundary_quin(region: u64, pred: u64, seq: u32, pt: (f64, f64)) -> NQuin {
        let mut q = NQuin {
            subject: region,
            predicate: pred,
            object: spatio_temporal::pack_point(pt.0, pt.1),
            context: 0,
            metadata: seq as u64,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }
}
