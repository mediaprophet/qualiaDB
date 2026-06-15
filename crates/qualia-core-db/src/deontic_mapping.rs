// ── Deontic Field Mapping Helper ───────────────────────────────────────────
/// Safe mapping of arbitrary byte slices to BLS12-381 Scalar field elements
/// Uses 512-bit uniform cryptographic hash reduction to avoid field modulus overflow

#[cfg(feature = "zk-culling")]
use bls12_381::Scalar;
#[cfg(feature = "zk-culling")]
use ff::FromUniformBytes;
#[cfg(feature = "zk-culling")]
use blake2b_simd::Params as Blake2bParams;

/// Maps an arbitrary byte slice to a valid BLS12-381 Scalar field element
/// using a 512-bit uniform cryptographic hash reduction.
/// 
/// This avoids the statistical bias of simple truncation and prevents field
/// modulus overflow panics when mapping 256-bit hashes into the 254-bit field.
#[cfg(feature = "zk-culling")]
pub fn bytes_to_field_element(data: &[u8]) -> Scalar {
    let mut hash_state = Blake2bParams::new().hash_length(64).to_state();
    hash_state.update(data);
    let hash_result = hash_state.finalize();
    
    // safe allocation: from_uniform_bytes handles the internal reduction modulo q
    Scalar::from_uniform_bytes(hash_result.as_array())
}

/// 8-bit action permission enum for Phase 1 (simple binary access states)
#[cfg(feature = "zk-culling")]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPermission {
    Read = 0,
    Write = 1,
    Execute = 2,
    Admin = 3,
}

#[cfg(feature = "zk-culling")]
impl ActionPermission {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ActionPermission::Read),
            1 => Some(ActionPermission::Write),
            2 => Some(ActionPermission::Execute),
            3 => Some(ActionPermission::Admin),
            _ => None,
        }
    }
    
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
#[cfg(feature = "zk-culling")]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_field_element() {
        let data = b"test data for field mapping";
        let scalar = bytes_to_field_element(data);
        // Should not panic and should produce a valid Scalar
        // The actual value is not important for this test
    }
    
    #[test]
    fn test_action_permission_roundtrip() {
        let perm = ActionPermission::Read;
        assert_eq!(ActionPermission::from_u8(perm.as_u8()), Some(perm));
        
        let perm = ActionPermission::Write;
        assert_eq!(ActionPermission::from_u8(perm.as_u8()), Some(perm));
        
        assert_eq!(ActionPermission::from_u8(255), None);
    }
}