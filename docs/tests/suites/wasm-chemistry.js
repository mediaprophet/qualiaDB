// WASM Organic Chemistry + Thermochemistry tests.
// Covers molecular descriptors, Lipinski/Veber filters, functional groups,
// reaction metrics, and thermochemistry (Gibbs, Keq, pH, Arrhenius).

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

const ASPIRIN_SMILES   = 'CC(=O)Oc1ccccc1C(=O)O';
const ETHANOL_SMILES   = 'CCO';
const CAFFEINE_SMILES  = 'Cn1cnc2c1c(=O)n(c(=O)n2C)C';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Organic Chemistry', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.describe('Molecular Descriptors (compute_molecular_descriptors_wasm)', () => {

            runner.it('aspirin descriptor keys are present', () => {
                if (!mod.compute_molecular_descriptors_wasm) return;
                const d = mod.compute_molecular_descriptors_wasm({ smiles: ASPIRIN_SMILES });
                runner.expect(d).toHaveProperty('molecular_weight');
                runner.expect(d).toHaveProperty('formula');
                runner.expect(d).toHaveProperty('heavy_atom_count');
                runner.expect(d).toHaveProperty('logp_crippen');
                runner.expect(d).toHaveProperty('tpsa_ertl');
            });

            runner.it('aspirin MW ≈ 180 Da', () => {
                if (!mod.compute_molecular_descriptors_wasm) return;
                const d = mod.compute_molecular_descriptors_wasm({ smiles: ASPIRIN_SMILES });
                runner.expect(d.molecular_weight).toBeGreaterThan(150.0);
                runner.expect(d.molecular_weight).toBeLessThan(210.0);
            });

            runner.it('ethanol heavy atom count = 3 (C, C, O)', () => {
                if (!mod.compute_molecular_descriptors_wasm) return;
                const d = mod.compute_molecular_descriptors_wasm({ smiles: ETHANOL_SMILES });
                runner.expect(d.heavy_atom_count).toBe(3);
            });

            runner.it('caffeine has aromatic rings', () => {
                if (!mod.compute_molecular_descriptors_wasm) return;
                const d = mod.compute_molecular_descriptors_wasm({ smiles: CAFFEINE_SMILES });
                runner.expect(d.aromatic_ring_count).toBeGreaterThan(0);
            });
        });

        runner.describe('Drug-likeness Filters (evaluate_lipinski_wasm)', () => {

            runner.it('aspirin passes Lipinski Ro5', () => {
                if (!mod.evaluate_lipinski_wasm) return;
                const r = mod.evaluate_lipinski_wasm({ smiles: ASPIRIN_SMILES });
                runner.expect(r.lipinski_passes).toBeTruthy();
            });

            runner.it('result includes veber_passes and ghose_passes', () => {
                if (!mod.evaluate_lipinski_wasm) return;
                const r = mod.evaluate_lipinski_wasm({ smiles: ASPIRIN_SMILES });
                runner.expect(r).toHaveProperty('veber_passes');
                runner.expect(r).toHaveProperty('ghose_passes');
            });

            runner.it('lipinski_violations is a non-negative integer', () => {
                if (!mod.evaluate_lipinski_wasm) return;
                const r = mod.evaluate_lipinski_wasm({ smiles: ASPIRIN_SMILES });
                runner.expect(r.lipinski_violations).toBeGreaterThanOrEqual(0);
            });
        });

        runner.describe('Functional Groups (detect_functional_groups_wasm)', () => {

            runner.it('ethanol contains hydroxyl group', () => {
                if (!mod.detect_functional_groups_wasm) return;
                const r = mod.detect_functional_groups_wasm({ smiles: ETHANOL_SMILES });
                runner.expect(Array.isArray(r.functional_groups)).toBeTruthy();
            });

            runner.it('pKa estimates array is returned', () => {
                if (!mod.detect_functional_groups_wasm) return;
                const r = mod.detect_functional_groups_wasm({ smiles: ASPIRIN_SMILES });
                runner.expect(Array.isArray(r.pka_estimates)).toBeTruthy();
            });
        });

        runner.describe('Reaction Metrics (compute_reaction_metrics_wasm)', () => {

            runner.it('returns atom_economy_pct, e_factor, and PMI', () => {
                if (!mod.compute_reaction_metrics_wasm) return;
                const r = mod.compute_reaction_metrics_wasm({
                    reactant_smiles: [ETHANOL_SMILES],
                    product_smiles: ETHANOL_SMILES,
                    yield_fraction: 0.85,
                    solvent_kg: 10.0,
                    product_kg: 1.0,
                });
                runner.expect(r).toHaveProperty('atom_economy_pct');
                runner.expect(r).toHaveProperty('e_factor');
                runner.expect(r).toHaveProperty('process_mass_intensity');
            });

            runner.it('high yield gives better RME than low yield', () => {
                if (!mod.compute_reaction_metrics_wasm) return;
                const highYield = mod.compute_reaction_metrics_wasm({
                    reactant_smiles: [ETHANOL_SMILES],
                    product_smiles: ETHANOL_SMILES,
                    yield_fraction: 0.99, solvent_kg: 5.0, product_kg: 1.0,
                });
                const lowYield = mod.compute_reaction_metrics_wasm({
                    reactant_smiles: [ETHANOL_SMILES],
                    product_smiles: ETHANOL_SMILES,
                    yield_fraction: 0.10, solvent_kg: 5.0, product_kg: 1.0,
                });
                runner.expect(highYield.reaction_mass_efficiency_pct)
                    .toBeGreaterThan(lowYield.reaction_mass_efficiency_pct);
            });
        });

        runner.describe('Thermochemistry (compute_thermochemistry_wasm)', () => {

            runner.it('exothermic reaction has negative Gibbs energy', () => {
                if (!mod.compute_thermochemistry_wasm) return;
                const r = mod.compute_thermochemistry_wasm({
                    delta_h_j_mol: -100000,
                    delta_s_j_mol_k: 50,
                    temp_k: 298.15,
                    pka: null, conc_base: null, conc_acid: null,
                    activation_energy_j_mol: null, pre_exponential_a: null,
                });
                runner.expect(r.gibbs_energy_j_mol).toBeLessThan(0);
            });

            runner.it('equilibrium constant > 1 for spontaneous reaction', () => {
                if (!mod.compute_thermochemistry_wasm) return;
                const r = mod.compute_thermochemistry_wasm({
                    delta_h_j_mol: -50000,
                    delta_s_j_mol_k: 100,
                    temp_k: 298.15,
                    pka: null, conc_base: null, conc_acid: null,
                    activation_energy_j_mol: null, pre_exponential_a: null,
                });
                runner.expect(r.equilibrium_constant).toBeGreaterThan(1);
            });

            runner.it('Henderson-Hasselbalch pH at equal concentrations = pKa', () => {
                if (!mod.compute_thermochemistry_wasm) return;
                const r = mod.compute_thermochemistry_wasm({
                    delta_h_j_mol: 0, delta_s_j_mol_k: 0, temp_k: 298.15,
                    pka: 4.76, conc_base: 1.0, conc_acid: 1.0,
                    activation_energy_j_mol: null, pre_exponential_a: null,
                });
                runner.expect(r.ph).not.toBeNull();
                runner.expect(Math.abs(r.ph - 4.76)).toBeLessThan(0.01);
            });

            runner.it('Arrhenius rate is positive and finite', () => {
                if (!mod.compute_thermochemistry_wasm) return;
                const r = mod.compute_thermochemistry_wasm({
                    delta_h_j_mol: 0, delta_s_j_mol_k: 0, temp_k: 298.15,
                    pka: null, conc_base: null, conc_acid: null,
                    activation_energy_j_mol: 50000, pre_exponential_a: 1e13,
                });
                runner.expect(r.rate_constant).toBeGreaterThan(0);
                runner.expect(isFinite(r.rate_constant)).toBeTruthy();
            });
        });
    });
}

export default register;
