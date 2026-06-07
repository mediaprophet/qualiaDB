//! QPU response ingress firewall — collapses provider JSON into Quins / bitstrings.

use crate::QualiaQuin;

pub const MAX_QPU_SAMPLES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QpuIngressError {
    InvalidPayload,
    SampleBufferFull,
}

/// D-Wave sample: binary assignment per variable.
pub fn parse_dwave_samples(json: &str, out_bits: &mut [u8; MAX_QPU_SAMPLES], out_len: &mut usize) -> Result<(), QpuIngressError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| QpuIngressError::InvalidPayload)?;
    let samples = v
        .pointer("/solutions/0/solutions")
        .or_else(|| v.pointer("/samples"))
        .and_then(|s| s.as_array())
        .ok_or(QpuIngressError::InvalidPayload)?;

    *out_len = 0;
    if let Some(first) = samples.first() {
        if let Some(sample_str) = first.get("sample").and_then(|s| s.as_array()) {
            for bit in sample_str {
                if *out_len >= MAX_QPU_SAMPLES {
                    return Err(QpuIngressError::SampleBufferFull);
                }
                out_bits[*out_len] = bit.as_i64().unwrap_or(0) as u8;
                *out_len += 1;
            }
            return Ok(());
        }
        if let Some(sample_obj) = first.as_object() {
            let mut idx = 0usize;
            while sample_obj.contains_key(&idx.to_string()) {
                if idx >= MAX_QPU_SAMPLES {
                    return Err(QpuIngressError::SampleBufferFull);
                }
                out_bits[idx] = sample_obj[&idx.to_string()].as_i64().unwrap_or(0) as u8;
                idx += 1;
            }
            *out_len = idx;
            return Ok(());
        }
    }
    Err(QpuIngressError::InvalidPayload)
}

/// IBM job result: quasi-probability bitstrings or counts.
pub fn parse_ibm_counts(json: &str, out_bits: &mut [u8; MAX_QPU_SAMPLES], out_len: &mut usize) -> Result<(), QpuIngressError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| QpuIngressError::InvalidPayload)?;
    let counts = v
        .pointer("/results/0/data/counts")
        .or_else(|| v.pointer("/counts"))
        .and_then(|c| c.as_object())
        .ok_or(QpuIngressError::InvalidPayload)?;

    let (best_key, _) = counts
        .iter()
        .max_by_key(|(_, v)| v.as_i64().unwrap_or(0))
        .ok_or(QpuIngressError::InvalidPayload)?;

    *out_len = 0;
    for ch in best_key.chars() {
        if *out_len >= MAX_QPU_SAMPLES {
            return Err(QpuIngressError::SampleBufferFull);
        }
        out_bits[*out_len] = if ch == '1' { 1 } else { 0 };
        *out_len += 1;
    }
    Ok(())
}

/// Pack bitstring into provenance Quins for orchestrator post-flight.
pub fn bits_to_provenance_quins(bits: &[u8], num_vars: u8, out: &mut [QualiaQuin]) -> usize {
    let mut n = 0usize;
    for (i, &b) in bits.iter().enumerate().take(num_vars as usize) {
        if n >= out.len() {
            break;
        }
        let predicate = crate::q_hash("q42:qpuBit");
        let context = crate::q_hash("q42:qpuIngress");
        let subject = i as u64;
        let object = b as u64;
        let q = QualiaQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0xC000_0000_0000_0001,
            parity: subject ^ predicate ^ object ^ context,
        };
        out[n] = q;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ibm_counts_bitstring() {
        let json = r#"{"counts":{"101":42,"010":1}}"#;
        let mut bits = [0u8; MAX_QPU_SAMPLES];
        let mut len = 0;
        parse_ibm_counts(json, &mut bits, &mut len).unwrap();
        assert_eq!(len, 3);
        assert_eq!(bits[0], 1);
        assert_eq!(bits[1], 0);
        assert_eq!(bits[2], 1);
    }
}
