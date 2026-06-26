//! WASM-bindgen API — chemistry domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
use super::*;


// ─── Organic chemistry ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct SmilesParams {
    pub smiles: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_molecular_descriptors_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: SmilesParams = serde_wasm_bindgen::from_value(val)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.smiles);
    if !mol.is_valid {
        return Err(JsValue::from_str(
            &mol.error.unwrap_or_else(|| "Invalid SMILES".into()),
        ));
    }
    let d = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
    #[derive(Serialize)]
    struct Desc {
        molecular_weight: f64,
        formula: String,
        heavy_atom_count: usize,
        hb_donors: u32,
        hb_acceptors: u32,
        rotatable_bonds: u32,
        aromatic_ring_count: u32,
        ring_count: u32,
        logp_crippen: f64,
        tpsa_ertl: f64,
        chiral_centers: u32,
        fraction_csp3: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Desc {
        molecular_weight: d.molecular_weight,
        formula: d.formula,
        heavy_atom_count: d.heavy_atom_count,
        hb_donors: d.hb_donors,
        hb_acceptors: d.hb_acceptors,
        rotatable_bonds: d.rotatable_bonds,
        aromatic_ring_count: d.aromatic_ring_count,
        ring_count: d.ring_count,
        logp_crippen: d.logp_crippen,
        tpsa_ertl: d.tpsa_ertl,
        chiral_centers: d.chiral_centers,
        fraction_csp3: d.fraction_csp3,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_lipinski_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: SmilesParams = serde_wasm_bindgen::from_value(val)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.smiles);
    let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
    let lip = crate::domains::chemical::organic_chemistry::evaluate_lipinski(&desc);
    let veb = crate::domains::chemical::organic_chemistry::evaluate_veber(&desc);
    let gho = crate::domains::chemical::organic_chemistry::evaluate_ghose(&desc);
    let ega = crate::domains::chemical::organic_chemistry::evaluate_egan(&desc);
    #[derive(Serialize)]
    struct Filters {
        lipinski_passes: bool,
        lipinski_violations: u8,
        veber_passes: bool,
        ghose_passes: bool,
        egan_passes: bool,
        mw: f64,
        logp: f64,
        tpsa: f64,
        hbd: u32,
        hba: u32,
        rot_bonds: u32,
    }
    Ok(serde_wasm_bindgen::to_value(&Filters {
        lipinski_passes: lip.passes,
        lipinski_violations: lip.violations,
        veber_passes: veb.passes,
        ghose_passes: gho.passes,
        egan_passes: ega.passes,
        mw: desc.molecular_weight,
        logp: desc.logp_crippen,
        tpsa: desc.tpsa_ertl,
        hbd: desc.hb_donors,
        hba: desc.hb_acceptors,
        rot_bonds: desc.rotatable_bonds,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn detect_functional_groups_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: SmilesParams = serde_wasm_bindgen::from_value(val)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.smiles);
    let groups: Vec<String> = crate::domains::chemical::organic_chemistry::detect_functional_groups(&mol)
        .iter()
        .map(|g| format!("{:?}", g))
        .collect();
    let pkas: Vec<(String, f64, bool)> = crate::domains::chemical::organic_chemistry::estimate_pka(&mol)
        .iter()
        .map(|p| (format!("{:?}", p.group), p.pka, p.is_acid))
        .collect();
    #[derive(Serialize)]
    struct GroupResult {
        functional_groups: Vec<String>,
        pka_estimates: Vec<(String, f64, bool)>,
    }
    Ok(serde_wasm_bindgen::to_value(&GroupResult {
        functional_groups: groups,
        pka_estimates: pkas,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ReactionMetricsParams {
    /// Reactant SMILES strings (used to compute MW)
    pub reactant_smiles: Vec<String>,
    /// Desired product SMILES
    pub product_smiles: String,
    /// Reaction yield (0.0–1.0)
    pub yield_fraction: f64,
    /// kg of solvent + auxiliary used per batch
    pub solvent_kg: f64,
    /// kg of product collected
    pub product_kg: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_reaction_metrics_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ReactionMetricsParams = serde_wasm_bindgen::from_value(val)?;
    let reactant_mws: Vec<f64> = p
        .reactant_smiles
        .iter()
        .map(|s| {
            let mol = crate::domains::chemical::organic_chemistry::parse_smiles(s);
            crate::domains::chemical::organic_chemistry::exact_molecular_weight(&mol)
        })
        .collect();
    let product_mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.product_smiles);
    let product_mw = crate::domains::chemical::organic_chemistry::exact_molecular_weight(&product_mol);
    let ae = crate::domains::chemical::organic_chemistry::atom_economy(&reactant_mws, product_mw);
    let ef = crate::domains::chemical::organic_chemistry::e_factor(
        reactant_mws.iter().sum::<f64>() + p.solvent_kg - p.product_kg,
        p.product_kg,
    );
    let gm = crate::domains::chemical::organic_chemistry::green_metrics(
        &reactant_mws,
        product_mw,
        &[],
        p.yield_fraction,
        p.solvent_kg,
        p.product_kg,
        0,
        0,
    );
    #[derive(Serialize)]
    struct RxnResult {
        atom_economy_pct: f64,
        e_factor: f64,
        process_mass_intensity: f64,
        reaction_mass_efficiency_pct: f64,
        yield_corrected_ae_pct: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&RxnResult {
        atom_economy_pct: ae,
        e_factor: ef,
        process_mass_intensity: gm.process_mass_intensity,
        reaction_mass_efficiency_pct: gm.reaction_mass_efficiency_pct,
        yield_corrected_ae_pct: gm.yield_corrected_ae_pct,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_thermochemistry_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ThermochemParams = serde_wasm_bindgen::from_value(val)?;
    let dg =
        crate::domains::chemical::organic_chemistry::gibbs_free_energy(p.delta_h_j_mol, p.delta_s_j_mol_k, p.temp_k);
    let k_eq = crate::domains::chemical::organic_chemistry::equilibrium_constant(dg, p.temp_k);
    let ph = p.pka.map(|pka| {
        crate::domains::chemical::organic_chemistry::henderson_hasselbalch(
            pka,
            p.conc_base.unwrap_or(1.0),
            p.conc_acid.unwrap_or(1.0),
        )
    });
    let k_rate = p.activation_energy_j_mol.map(|ea| {
        crate::domains::chemical::organic_chemistry::arrhenius_rate(p.pre_exponential_a.unwrap_or(1e13), ea, p.temp_k)
    });
    #[derive(Serialize)]
    struct ThermResult {
        gibbs_energy_j_mol: f64,
        equilibrium_constant: f64,
        ph: Option<f64>,
        rate_constant: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&ThermResult {
        gibbs_energy_j_mol: dg,
        equilibrium_constant: k_eq,
        ph,
        rate_constant: k_rate,
    })?)
}
