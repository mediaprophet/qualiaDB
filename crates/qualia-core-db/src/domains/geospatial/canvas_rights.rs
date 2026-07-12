use crate::NQuin;

/// G2 explicit-denial guard: returns `true` when the location owner has explicitly
/// refused this principal or asset. Fail-closed — callers must reject on `true`.
pub fn explicit_denial_guard(
    location_hash: u64,
    principal_hash: u64,
    asset_hash: u64,
    arena_quins: &[NQuin],
) -> bool {
    let explicit_denial = crate::q_hash("q42:explicitDenial");
    for quin in arena_quins {
        if quin.predicate == explicit_denial && quin.subject == location_hash {
            if quin.object == principal_hash || quin.object == asset_hash {
                return true;
            }
        }
    }
    false
}

/// Represents the governed rights to anchor virtual content at a specific physical location.
pub struct CanvasRightsModel;

impl CanvasRightsModel {
    /// Validates whether a specific principal has the right to place a virtual asset
    /// at the given `location_hash`. 
    /// 
    /// Follows the G2 explicit-denial guard principle:
    /// No automated agent can override a human principal's explicit refusal quin.
    pub fn validate_placement(
        location_hash: u64,
        principal_hash: u64,
        asset_hash: u64,
        arena_quins: &[NQuin],
    ) -> bool {
        let placement_right = crate::q_hash("q42:hasPlacementRight");

        if explicit_denial_guard(location_hash, principal_hash, asset_hash, arena_quins) {
            return false;
        }

        // 2. Check for explicit grant or steward delegation
        // In a permissive-commons scenario, some areas might be public.
        // For owned regions, we require a placement_right quin.
        let mut has_right = false;
        // True once we see a placement_right quin for this location — i.e. the
        // location is under private/owned governance rather than open commons.
        // Derived from the arena quins below, not a mock.
        let mut is_owned = false;
        
        for quin in arena_quins {
            if quin.predicate == placement_right && quin.subject == location_hash {
                is_owned = true;
                if quin.object == principal_hash {
                    has_right = true;
                    break;
                }
            }
        }

        // If it's privately owned, they must have the right.
        // If it's a true commons (not owned), it's permitted unless denied above.
        !is_owned || has_right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placement_explicit_denial() {
        let loc = 0x111;
        let prin = 0x222;
        let asset = 0x333;
        
        let mut quins = vec![];
        // Explicitly deny the principal
        quins.push(NQuin {
            subject: loc,
            predicate: crate::q_hash("q42:explicitDenial"),
            object: prin,
            context: 0,
            metadata: 0,
            parity: 0,
        });

        assert_eq!(CanvasRightsModel::validate_placement(loc, prin, asset, &quins), false);
    }

    #[test]
    fn test_placement_granted() {
        let loc = 0x111;
        let prin = 0x222;
        let asset = 0x333;
        
        let mut quins = vec![];
        // Grant the principal
        quins.push(NQuin {
            subject: loc,
            predicate: crate::q_hash("q42:hasPlacementRight"),
            object: prin,
            context: 0,
            metadata: 0,
            parity: 0,
        });

        assert_eq!(CanvasRightsModel::validate_placement(loc, prin, asset, &quins), true);
    }
}
