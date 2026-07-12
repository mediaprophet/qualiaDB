//! Rule-based compound screening — descriptor filtering + Tanimoto similarity ranking.
//!
//! HONESTY (CLAUDE.md §15): this is real cheminformatics — molecular descriptors are
//! computed from the provided structure via the tested `organic_chemistry` SMILES
//! engine, Lipinski's Rule-of-Five and Veber's rules are applied as a genuine pass/fail
//! filter, and candidates are ranked by Tanimoto similarity of a simple structural
//! fingerprint. It is **not** a binding-affinity, potency, or efficacy prediction, and
//! never presented as one — every result carries [`SCREENING_EPISTEMIC_STATUS`].

use super::{Compound, DrugTarget, MedicalError};
use crate::domains::chemical::organic_chemistry as oc;

/// Honest epistemic label stamped on every [`ScreeningProposal`].
pub const SCREENING_EPISTEMIC_STATUS: &str = "Rule-based filtering (Lipinski \
Rule-of-Five, Veber) plus structural-fingerprint Tanimoto similarity ranking. NOT a \
binding-affinity, potency, or efficacy prediction.";

const METHOD: &str = "descriptors via SMILES parse \u{2192} Lipinski/Veber pass-fail \u{2192} Tanimoto rank of a folded element/bonded-pair fingerprint";

/// Fingerprint length in bits.
pub const FINGERPRINT_BITS: usize = 256;

/// Tanimoto (Jaccard) similarity of two equal-length boolean fingerprints:
/// `|A ∩ B| / |A ∪ B|`. Returns 0.0 when the union is empty (both all-zero).
/// If lengths differ, only the overlapping prefix is compared.
pub fn tanimoto_bits(a: &[bool], b: &[bool]) -> f64 {
    let n = a.len().min(b.len());
    let mut inter = 0usize;
    let mut uni = 0usize;
    for i in 0..n {
        let x = a[i];
        let y = b[i];
        if x && y {
            inter += 1;
        }
        if x || y {
            uni += 1;
        }
    }
    if uni == 0 {
        0.0
    } else {
        inter as f64 / uni as f64
    }
}

/// FNV-1a hash of a feature string, folded to a fingerprint bit index.
fn feature_bit(s: &str) -> usize {
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    (h as usize) % FINGERPRINT_BITS
}

/// Build a deterministic structural fingerprint from a SMILES string: a folded bitset
/// over per-atom element features (radius 0) and canonical bonded-pair features
/// (radius 1). An unparseable/empty structure yields an all-zero fingerprint.
pub fn structural_fingerprint(smiles: &str) -> Vec<bool> {
    let mol = oc::parse_smiles(smiles);
    let mut fp = vec![false; FINGERPRINT_BITS];
    for a in &mol.atoms {
        fp[feature_bit(&format!("A:{}", a.element))] = true;
    }
    for b in &mol.bonds {
        let ea = &mol.atoms[b.atom_a].element;
        let eb = &mol.atoms[b.atom_b].element;
        let (x, y) = if ea <= eb { (ea, eb) } else { (eb, ea) };
        fp[feature_bit(&format!("B:{}-{:?}-{}", x, b.order, y))] = true;
    }
    fp
}

/// One screened compound's descriptor row + filter verdicts + similarity to the query.
#[derive(Debug, Clone)]
pub struct CompoundScreenRow {
    pub compound_id: String,
    pub molecular_weight: f64,
    /// Atom/structure-based logP estimate (Crippen-style, from `organic_chemistry`).
    pub logp_estimate: f64,
    pub hb_donors: u32,
    pub hb_acceptors: u32,
    pub rotatable_bonds: u32,
    pub tpsa: f64,
    /// Number of Lipinski Rule-of-Five violations (0 or 1 is orally acceptable).
    pub lipinski_violations: u32,
    /// Lipinski verdict (≤ 1 violation).
    pub passes_lipinski: bool,
    /// Veber verdict (rotatable bonds ≤ 10 and TPSA ≤ 140 Å²).
    pub passes_veber: bool,
    /// Tanimoto similarity of this compound's fingerprint to the query (0.0 if no query).
    pub tanimoto_to_query: f64,
    /// `true` if descriptors were computed from a parsed structure; `false` if the
    /// structure string did not parse and the caller-declared `Compound.properties`
    /// were used instead (stated honestly rather than silently substituted).
    pub descriptors_from_structure: bool,
}

/// Ranked, honestly-labeled screening proposal.
#[derive(Debug, Clone)]
pub struct ScreeningProposal {
    /// Honest label — rule-based filter + similarity ranking, never an affinity/efficacy claim.
    pub epistemic_status: &'static str,
    pub method: &'static str,
    pub target_id: String,
    pub query_smiles: Option<String>,
    /// Rows sorted descending by Tanimoto to the query; with no query, sorted ascending
    /// by Lipinski violations then by molecular weight. Ties broken by `compound_id`.
    pub ranked: Vec<CompoundScreenRow>,
}

/// Screen `compounds` against `target`, ranking by structural similarity to an optional
/// `query_smiles`. Descriptors come from the compound's `chemical_structure` (SMILES);
/// if a structure does not parse, the caller-declared `Compound.properties` are used and
/// the row is flagged `descriptors_from_structure = false`.
///
/// Fails closed with [`MedicalError::ValidationError`] when `compounds` is empty.
pub fn screen_compounds_rulebased(
    compounds: &[Compound],
    target: &DrugTarget,
    query_smiles: Option<&str>,
) -> Result<ScreeningProposal, MedicalError> {
    if compounds.is_empty() {
        return Err(MedicalError::ValidationError(
            "screen_compounds: at least one compound must be provided".to_string(),
        ));
    }
    let query_fp = query_smiles.map(structural_fingerprint);

    let mut ranked: Vec<CompoundScreenRow> = Vec::with_capacity(compounds.len());
    for c in compounds {
        let mol = oc::parse_smiles(&c.chemical_structure);
        let (desc, from_structure) = if mol.atoms.is_empty() {
            // Structure unparseable — fall back to caller-declared properties, flagged honestly.
            (
                oc::MolecularDescriptors {
                    molecular_weight: c.properties.molecular_weight,
                    formula: c.chemical_structure.clone(),
                    heavy_atom_count: 0,
                    hb_donors: 0,
                    hb_acceptors: 0,
                    rotatable_bonds: 0,
                    aromatic_ring_count: 0,
                    ring_count: 0,
                    logp_crippen: c.properties.logp,
                    tpsa_ertl: 0.0,
                    chiral_centers: 0,
                    fraction_csp3: 0.0,
                },
                false,
            )
        } else {
            (oc::compute_descriptors(&mol), true)
        };

        let lip = oc::evaluate_lipinski(&desc);
        let veb = oc::evaluate_veber(&desc);
        let fp = structural_fingerprint(&c.chemical_structure);
        let tanimoto_to_query = query_fp
            .as_ref()
            .map(|q| tanimoto_bits(q, &fp))
            .unwrap_or(0.0);

        ranked.push(CompoundScreenRow {
            compound_id: c.compound_id.clone(),
            molecular_weight: desc.molecular_weight,
            logp_estimate: desc.logp_crippen,
            hb_donors: desc.hb_donors,
            hb_acceptors: desc.hb_acceptors,
            rotatable_bonds: desc.rotatable_bonds,
            tpsa: desc.tpsa_ertl,
            lipinski_violations: lip.violations as u32,
            passes_lipinski: lip.passes,
            passes_veber: veb.passes,
            tanimoto_to_query,
            descriptors_from_structure: from_structure,
        });
    }

    if query_fp.is_some() {
        ranked.sort_by(|a, b| {
            b.tanimoto_to_query
                .partial_cmp(&a.tanimoto_to_query)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.compound_id.cmp(&b.compound_id))
        });
    } else {
        ranked.sort_by(|a, b| {
            a.lipinski_violations
                .cmp(&b.lipinski_violations)
                .then_with(|| {
                    a.molecular_weight
                        .partial_cmp(&b.molecular_weight)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.compound_id.cmp(&b.compound_id))
        });
    }

    Ok(ScreeningProposal {
        epistemic_status: SCREENING_EPISTEMIC_STATUS,
        method: METHOD,
        target_id: target.target_id.clone(),
        query_smiles: query_smiles.map(|s| s.to_string()),
        ranked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tanimoto_hand_value() {
        // A = {0,2,3}, B = {0,1,3}; ∩ = {0,3} (2), ∪ = {0,1,2,3} (4); 2/4 = 0.5
        let a = [true, false, true, true];
        let b = [true, true, false, true];
        assert!((tanimoto_bits(&a, &b) - 0.5).abs() < 1e-12);
        // Identical fingerprints → 1.0; disjoint → 0.0; both empty → 0.0
        assert!((tanimoto_bits(&a, &a) - 1.0).abs() < 1e-12);
        assert_eq!(tanimoto_bits(&[true, false], &[false, true]), 0.0);
        assert_eq!(tanimoto_bits(&[false, false], &[false, false]), 0.0);
    }
}
