//! SPARQL Extension Registry
//!
//! Provides static dispatch table for magic predicates and custom functions.
//! Follows zero-allocation architecture with fixed-size function arrays.

use crate::sparql_ast::*;
use crate::NQuin;

/// Extension function signature
/// 
/// Args: array of argument values (hashes or literals)
/// Returns: bool indicating success/failure, modifies binding row in-place
pub type ExtensionFn = fn(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool;

/// Extension registry for magic predicates and custom functions
#[repr(C)]
pub struct ExtensionRegistry {
    /// Maps dictionary IDs to function pointers
    pub functions: [(u64, ExtensionFn); 32],
    pub count: usize,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            functions: [(0, no_op); 32],
            count: 0,
        }
    }

    /// Register a magic predicate or custom function
    pub fn register(&mut self, dict_id: u64, func: ExtensionFn) -> Result<(), String> {
        if self.count >= 32 {
            return Err("Extension registry full".to_string());
        }
        
        // Check if already registered
        for i in 0..self.count {
            if self.functions[i].0 == dict_id {
                return Err("Dictionary ID already registered".to_string());
            }
        }
        
        self.functions[self.count] = (dict_id, func);
        self.count += 1;
        Ok(())
    }

    /// Look up a function by dictionary ID
    pub fn lookup(&self, dict_id: u64) -> Option<ExtensionFn> {
        for i in 0..self.count {
            if self.functions[i].0 == dict_id {
                return Some(self.functions[i].1);
            }
        }
        None
    }

    /// Check if a predicate is a magic predicate
    pub fn is_magic_predicate(&self, predicate_hash: u64) -> bool {
        self.lookup(predicate_hash).is_some()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Default no-op function for unregistered extensions
fn no_op(_args: &[u64], _quins: &[NQuin], _result: &mut BindingRow) -> bool {
    false
}

/// Built-in extension functions

/// BOUND function (already in FILTER, but can be called as extension)
pub fn ext_bound(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let var_id = args[0] as VariableId;
    result.slots[var_id as usize].is_some()
}

/// STR function
pub fn ext_str(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Simplified: just return the value
    result.slots[0] = Some(args[0]);
    true
}

/// LANG function
pub fn ext_lang(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Simplified: return 0 as placeholder
    result.slots[0] = Some(0);
    true
}

/// DATATYPE function
pub fn ext_datatype(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Simplified: return 0 as placeholder
    result.slots[0] = Some(0);
    true
}

/// isIRI function
pub fn ext_is_iri(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Check if value is an IRI (not a did:q42 pointer)
    let value = args[0];
    let is_iri = value < 0x8000_0000_0000_0000;
    result.slots[0] = Some(if is_iri { 1 } else { 0 });
    true
}

/// isBlank function
pub fn ext_is_blank(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Simplified: always false
    result.slots[0] = Some(0);
    true
}

/// isLiteral function
pub fn ext_is_literal(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Simplified: assume all values are literals
    result.slots[0] = Some(1);
    true
}

/// isNumeric function
pub fn ext_is_numeric(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    // Simplified: assume all values are numeric
    result.slots[0] = Some(1);
    true
}

/// ABS function
pub fn ext_abs(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let value = args[0] as i64;
    result.slots[0] = Some(value.abs() as u64);
    true
}

/// CEIL function
pub fn ext_ceil(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let value = args[0] as f64;
    result.slots[0] = Some(value.ceil() as u64);
    true
}

/// FLOOR function
pub fn ext_floor(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let value = args[0] as f64;
    result.slots[0] = Some(value.floor() as u64);
    true
}

/// ROUND function
pub fn ext_round(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let value = args[0] as f64;
    result.slots[0] = Some(value.round() as u64);
    true
}

/// GeoSPARQL functions

/// geof:distance - calculate distance between two points
pub fn ext_geof_distance(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    // Simplified: return Euclidean distance (in production, use proper geospatial library)
    let a = args[0] as f64;
    let b = args[1] as f64;
    let distance = (a - b).abs();
    result.slots[0] = Some(distance as u64);
    true
}

/// geof:sfContains - check if point is within polygon
pub fn ext_geof_sfcontains(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    // Simplified: always false (in production, use proper geospatial library)
    result.slots[0] = Some(0);
    true
}

/// geof:sfWithin - check if point is within distance of another
pub fn ext_geof_sfwithin(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    // Simplified: check if distance < threshold
    let point = args[0] as f64;
    let center = args[1] as f64;
    let within = (point - center).abs() < 100.0; // Arbitrary threshold
    result.slots[0] = Some(if within { 1 } else { 0 });
    true
}

/// geof:sfIntersects - check if geometries intersect
pub fn ext_geof_sfintersects(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    // Simplified: always true (in production, use proper geospatial library)
    result.slots[0] = Some(1);
    true
}

/// geof:sfTouches - check if geometries touch
pub fn ext_geof_sftouches(args: &[u64], _quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    // Simplified: always false (in production, use proper geospatial library)
    result.slots[0] = Some(0);
    true
}

/// SPARQL-MM functions

/// mm:duration - get media duration
pub fn ext_mm_duration(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::mm_duration(args, quins, result)
}

/// mm:dimensions - get media dimensions
pub fn ext_mm_dimensions(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::mm_dimensions(args, quins, result)
}

/// mm:temporalFragment - create temporal fragment
pub fn ext_mm_temporal_fragment(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::mm_temporal_fragment(args, quins, result)
}

/// MA Ontology functions

/// ma:format - get media format
pub fn ext_ma_format(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::ma_format(args, quins, result)
}

/// ma:mimeType - get media MIME type
pub fn ext_ma_mime_type(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::ma_mime_type(args, quins, result)
}

/// ma:codec - get media codec
pub fn ext_ma_codec(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::ma_codec(args, quins, result)
}

/// ma:bitrate - get media bitrate
pub fn ext_ma_bitrate(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::ma_bitrate(args, quins, result)
}

/// ma:framerate - get media framerate
pub fn ext_ma_framerate(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::ma_framerate(args, quins, result)
}

/// C2PA functions

/// c2pa:credential - get content credential
pub fn ext_c2pa_credential(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_credential(args, quins, result)
}

/// c2pa:isVerified - check if media is verified
pub fn ext_c2pa_is_verified(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_is_verified(args, quins, result)
}

/// c2pa:verificationStatus - get verification status
pub fn ext_c2pa_verification_status(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_verification_status(args, quins, result)
}

/// c2pa:createdAt - get creation timestamp
pub fn ext_c2pa_created_at(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_created_at(args, quins, result)
}

/// c2pa:createdBy - get creator
pub fn ext_c2pa_created_by(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_created_by(args, quins, result)
}

/// c2pa:verifySignature - verify content signature
pub fn ext_c2pa_verify_signature(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_verify_signature(args, quins, result)
}

/// c2pa:derivedFrom - get source asset
pub fn ext_c2pa_derived_from(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_mm::c2pa_derived_from(args, quins, result)
}

/// DID functions

/// did:resolve - Resolve DID to Document
pub fn ext_did_resolve(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_did::did_resolve(args, quins, result)
}

/// did:verify - Verify DID signature
pub fn ext_did_verify(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_did::did_verify(args, quins, result)
}

/// did:auth - Authenticate with DID
pub fn ext_did_auth(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_did::did_auth(args, quins, result)
}

/// did:sign - Sign with DID
pub fn ext_did_sign(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_did::did_sign(args, quins, result)
}

/// did:permission - Check DID permission
pub fn ext_did_permission(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    crate::sparql_did::did_permission(args, quins, result)
}

/// Create a registry with built-in SPARQL functions
pub fn create_builtin_registry() -> ExtensionRegistry {
    let mut registry = ExtensionRegistry::new();
    
    // Register built-in SPARQL functions (using placeholder hashes - in production, use actual q_hash)
    let _ = registry.register(0xB0_0000_0000_0000_u64, ext_bound);
    let _ = registry.register(0x5452000000000000, ext_str);
    let _ = registry.register(0x4C414E0000000000, ext_lang);
    let _ = registry.register(0x4454595000000000_u64, ext_datatype);
    let _ = registry.register(0x4953495200000000_u64, ext_is_iri);
    let _ = registry.register(0x4953424C00000000_u64, ext_is_blank);
    let _ = registry.register(0x49534C5400000000_u64, ext_is_literal);
    let _ = registry.register(0x49534E554D000000_u64, ext_is_numeric);
    let _ = registry.register(0x4142530000000000_u64, ext_abs);
    let _ = registry.register(0x4345494C00000000_u64, ext_ceil);
    let _ = registry.register(0x464C5F5200000000_u64, ext_floor);
    let _ = registry.register(0x524E440000000000_u64, ext_round);

    // Register GeoSPARQL functions
    let _ = registry.register(0x47454F445F444553_u64, ext_geof_distance);
    let _ = registry.register(0x47454F5F5346436E_u64, ext_geof_sfcontains);
    let _ = registry.register(0x47454F5F53465749_u64, ext_geof_sfwithin);
    let _ = registry.register(0x47454F5F53464954_u64, ext_geof_sfintersects);
    let _ = registry.register(0x47454F5F5346540F_u64, ext_geof_sftouches);

    // Register SPARQL-MM functions
    let _ = registry.register(0x4D4D5F4455520000_u64, ext_mm_duration);
    let _ = registry.register(0x4D4D5F44494D0000_u64, ext_mm_dimensions);
    let _ = registry.register(0x4D4D5F54456D7000_u64, ext_mm_temporal_fragment);

    // Register MA Ontology functions
    let _ = registry.register(0x4D415F464F524D00_u64, ext_ma_format);
    let _ = registry.register(0x4D415F4D494D4500_u64, ext_ma_mime_type);
    let _ = registry.register(0x4D415F434F444500_u64, ext_ma_codec);
    let _ = registry.register(0x4D415F4249545241_u64, ext_ma_bitrate);
    let _ = registry.register(0x4D415F4652414D45_u64, ext_ma_framerate);

    // Register C2PA functions
    let _ = registry.register(0x433250415F435245_u64, ext_c2pa_credential);
    let _ = registry.register(0x433250415F495356_u64, ext_c2pa_is_verified);
    let _ = registry.register(0x433250415F564552_u64, ext_c2pa_verification_status);
    let _ = registry.register(0x433250415F435245_u64, ext_c2pa_created_at);
    let _ = registry.register(0x433250415F435245_u64, ext_c2pa_created_by);
    let _ = registry.register(0x433250415F564552_u64, ext_c2pa_verify_signature);
    let _ = registry.register(0x433250415F444552_u64, ext_c2pa_derived_from);
    
    registry
}

/// Magic predicate executor
pub struct MagicPredicateExecutor<'a> {
    pub registry: &'a ExtensionRegistry,
    pub quins: &'a [NQuin],
}

impl<'a> MagicPredicateExecutor<'a> {
    pub fn new(registry: &'a ExtensionRegistry, quins: &'a [NQuin]) -> Self {
        Self { registry, quins }
    }

    /// Execute a magic predicate
    pub fn execute(&self, predicate_hash: u64, args: &[u64], result: &mut BindingRow) -> bool {
        if let Some(func) = self.registry.lookup(predicate_hash) {
            func(args, self.quins, result)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_registry() {
        let mut registry = ExtensionRegistry::new();
        assert_eq!(registry.count, 0);
        
        registry.register(0x1234567890ABCDEF, ext_bound).unwrap();
        assert_eq!(registry.count, 1);
        
        assert!(registry.lookup(0x1234567890ABCDEF).is_some());
        assert!(registry.lookup(0x0000000000000000).is_none());
    }

    #[test]
    fn test_registry_full() {
        let mut registry = ExtensionRegistry::new();
        
        for i in 0..33 {
            let result = registry.register(i as u64, ext_bound);
            if i < 32 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_magic_predicate_executor() {
        let registry = create_builtin_registry();
        let quins = vec![];
        let executor = MagicPredicateExecutor::new(&registry, &quins);
        
        let mut result = BindingRow::new();
        let success = executor.execute(0x4142530000000000_u64, &[42], &mut result);
        assert!(success);
    }

    #[test]
    fn test_ext_abs() {
        let mut result = BindingRow::new();
        ext_abs(&[42], &[], &mut result);
        assert_eq!(result.slots[0], Some(42));
        
        ext_abs(&[18446744073709551615], &[], &mut result);
        assert_eq!(result.slots[0], Some(1)); // abs(i64::MAX)
    }

    #[test]
    fn test_ext_is_iri() {
        let mut result = BindingRow::new();
        ext_is_iri(&[0x1234], &[], &mut result);
        assert_eq!(result.slots[0], Some(1));
        
        ext_is_iri(&[0x8000000000000000], &[], &mut result);
        assert_eq!(result.slots[0], Some(0));
    }
}