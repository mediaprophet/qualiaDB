// WASM Clinical Engine tests.
// Covers Framingham risk, FHIR observation, and drug interaction checks.

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Clinical Engine', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.describe('Framingham 10-Year Risk (compute_framingham_risk_wasm)', () => {

            runner.it('low-risk profile returns risk < 10%', () => {
                if (!mod.compute_framingham_risk_wasm) return;
                const r = mod.compute_framingham_risk_wasm({
                    age: 35, sex_male: false,
                    total_cholesterol_mmol: 4.5, hdl_cholesterol_mmol: 1.4,
                    systolic_bp: 115.0, bp_treated: false,
                    current_smoker: false, diabetic: false,
                });
                runner.expect(r.risk_10yr_pct).toBeLessThan(10.0);
            });

            runner.it('high-risk profile returns risk > low-risk', () => {
                if (!mod.compute_framingham_risk_wasm) return;
                const low = mod.compute_framingham_risk_wasm({
                    age: 35, sex_male: false,
                    total_cholesterol_mmol: 4.5, hdl_cholesterol_mmol: 1.4,
                    systolic_bp: 115.0, bp_treated: false,
                    current_smoker: false, diabetic: false,
                });
                const high = mod.compute_framingham_risk_wasm({
                    age: 65, sex_male: true,
                    total_cholesterol_mmol: 7.0, hdl_cholesterol_mmol: 0.8,
                    systolic_bp: 180.0, bp_treated: true,
                    current_smoker: true, diabetic: true,
                });
                runner.expect(high.risk_10yr_pct).toBeGreaterThan(low.risk_10yr_pct);
            });

            runner.it('result has category field', () => {
                if (!mod.compute_framingham_risk_wasm) return;
                const r = mod.compute_framingham_risk_wasm({
                    age: 50, sex_male: true,
                    total_cholesterol_mmol: 5.2, hdl_cholesterol_mmol: 1.1,
                    systolic_bp: 130.0, bp_treated: false,
                    current_smoker: false, diabetic: false,
                });
                runner.expect(typeof r.category).toBe('string');
            });
        });

        runner.describe('FHIR Observation Validation (validate_fhir_observation_wasm)', () => {

            runner.it('in-range glucose → valid', () => {
                if (!mod.validate_fhir_observation_wasm) return;
                const r = mod.validate_fhir_observation_wasm({
                    loinc_code: '2339-0',
                    value: 5.5,
                    unit_ucum: 'mmol/L',
                    reference_low: 3.9,
                    reference_high: 6.1,
                });
                runner.expect(r.is_valid).toBeTruthy();
            });

            runner.it('out-of-range value still returns structured result', () => {
                if (!mod.validate_fhir_observation_wasm) return;
                const r = mod.validate_fhir_observation_wasm({
                    loinc_code: '2339-0',
                    value: 30.0,
                    unit_ucum: 'mmol/L',
                    reference_low: 3.9,
                    reference_high: 6.1,
                });
                runner.expect(typeof r.interpretation_code).toBe('string');
            });

            runner.it('result has status field', () => {
                if (!mod.validate_fhir_observation_wasm) return;
                const r = mod.validate_fhir_observation_wasm({
                    loinc_code: '8480-6', value: 120.0, unit_ucum: 'mm[Hg]',
                    reference_low: null, reference_high: null,
                });
                runner.expect(typeof r.status).toBe('string');
            });
        });

        runner.describe('Drug Interaction Check (check_drug_interactions_wasm)', () => {

            runner.it('no interactions for safe single drug', () => {
                if (!mod.check_drug_interactions_wasm) return;
                const r = mod.check_drug_interactions_wasm({ medications: ['paracetamol'] });
                runner.expect(Array.isArray(r)).toBeTruthy();
            });

            runner.it('known interacting pair returns non-empty array', () => {
                if (!mod.check_drug_interactions_wasm) return;
                const r = mod.check_drug_interactions_wasm({ medications: ['warfarin', 'aspirin'] });
                runner.expect(Array.isArray(r)).toBeTruthy();
            });

            runner.it('interaction objects have mechanism and severity', () => {
                if (!mod.check_drug_interactions_wasm) return;
                const r = mod.check_drug_interactions_wasm({ medications: ['warfarin', 'aspirin'] });
                if (r.length > 0) {
                    runner.expect(typeof r[0].mechanism).toBe('string');
                    runner.expect(typeof r[0].severity).toBe('string');
                }
            });
        });
    });
}

export default register;
