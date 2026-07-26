//! Ontology and NQuin bridge for computational economics results.
//!
//! Provides helpers to encode model configs, calibration records, and
//! outputs as NQuin facts for the graph layer, with valid parity.
//! This satisfies the §5.12 and P9 requirements.

use crate::q_hash;
use crate::NQuin; // for FIBO constants example

/// Encode a simple scalar result (e.g. VaR, price) as a NQuin.
/// subject = model_hash, predicate = metric_hash | opcode, object = packed float, context = provenance.
pub fn encode_scalar_result(
    model_hash: u64,
    metric_hash: u64,
    value: f64,
    provenance_hash: u64,
    out: &mut NQuin,
) {
    out.subject = model_hash;
    out.predicate = metric_hash; // caller puts opcode in low byte if needed
                                 // pack f64 as u64 bits in object low, with tag if needed; here simple for demo
    out.object = value.to_bits() & 0x0FFF_FFFF_FFFF_FFFF; // mask to 60 bit as per lexicon
    out.context = provenance_hash;
    out.metadata = 0; // lamport etc caller
    out.parity = out.subject ^ out.predicate ^ out.object ^ out.context;
}

/// Encode a vector result (e.g. nquin vector components) into multiple Quins.
/// Caller supplies the base hashes.
pub fn encode_vector_result(
    base_subject: u64,
    component_hashes: &[u64],
    values: &[f64],
    context: u64,
    out: &mut [NQuin],
) -> Result<usize, ()> {
    if values.len() > out.len() || component_hashes.len() < values.len() {
        return Err(());
    }
    for (i, &v) in values.iter().enumerate() {
        out[i].subject = base_subject;
        out[i].predicate = component_hashes[i];
        out[i].object = v.to_bits() & 0x0FFF_FFFF_FFFF_FFFF;
        out[i].context = context;
        out[i].metadata = 0;
        out[i].parity = out[i].subject ^ out[i].predicate ^ out[i].object ^ out[i].context;
    }
    Ok(values.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_encodes_with_parity() {
        let mut q = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        encode_scalar_result(0x111, 0x222, 42.0, 0x333, &mut q);
        let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        assert_eq!(q.parity, expected_parity);
    }
}

/// Basic SHACL-like constraint check for a scalar econ result (e.g. VaR must be positive).
pub fn validate_scalar_econ_constraint(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && value >= min && value <= max
}

/// Example FIBO-style note encoder (placeholder for full bridge).
pub const FIBO_INSTRUMENT_PRICE: u64 = q_hash("fibo:price");

pub fn encode_fibo_price(model: u64, price: f64, prov: u64, out: &mut NQuin) {
    encode_scalar_result(model, FIBO_INSTRUMENT_PRICE, price, prov, out);
}
