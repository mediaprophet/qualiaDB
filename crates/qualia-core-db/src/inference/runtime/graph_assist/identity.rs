use crate::NQuin;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PrefixIdentity {
    pub words: [u64; 4],
}

#[inline]
fn mix(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Bind reusable KV pages to the exact model, tokenizer, graph scope/revision and selected facts.
pub fn derive_prefix_identity(
    model_instance: u64,
    tokenizer_revision: u64,
    graph_context: u64,
    graph_revision: u64,
    facts: &[NQuin],
) -> PrefixIdentity {
    let mut words = [
        mix(model_instance ^ 0x243f_6a88_85a3_08d3),
        mix(tokenizer_revision ^ 0x1319_8a2e_0370_7344),
        mix(graph_context ^ 0xa409_3822_299f_31d0),
        mix(graph_revision ^ 0x082e_fa98_ec4e_6c89),
    ];
    for (index, fact) in facts.iter().enumerate() {
        let lanes = [
            fact.subject,
            fact.predicate,
            fact.object,
            fact.context,
            fact.metadata,
            fact.parity,
        ];
        for (lane, value) in lanes.into_iter().enumerate() {
            let target = (index + lane) & 3;
            words[target] = mix(words[target] ^ value ^ ((index as u64) << 32) ^ lane as u64);
        }
    }
    PrefixIdentity { words }
}
