// WASM SHACL constraint validation tests.
// Covers validate_shacl_constraint_wasm from wasm_bridge.rs.

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: SHACL Constraints', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.it('minInclusive: 5 ≥ 5 → passes', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'minInclusive', value: 5.0, target_value: 5.0,
            });
            runner.expect(r.passes).toBeTruthy();
        });

        runner.it('minInclusive: 4 < 5 → fails', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'minInclusive', value: 5.0, target_value: 4.0,
            });
            runner.expect(r.passes).toBeFalsy();
        });

        runner.it('maxInclusive: 5 ≤ 10 → passes', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'maxInclusive', value: 10.0, target_value: 5.0,
            });
            runner.expect(r.passes).toBeTruthy();
        });

        runner.it('maxInclusive: 11 > 10 → fails', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'maxInclusive', value: 10.0, target_value: 11.0,
            });
            runner.expect(r.passes).toBeFalsy();
        });

        runner.it('minExclusive: 5.1 > 5.0 → passes', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'minExclusive', value: 5.0, target_value: 5.1,
            });
            runner.expect(r.passes).toBeTruthy();
        });

        runner.it('minExclusive: 5.0 == 5.0 → fails (exclusive)', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'minExclusive', value: 5.0, target_value: 5.0,
            });
            runner.expect(r.passes).toBeFalsy();
        });

        runner.it('result echoes constraint_type, value, target_value', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'minInclusive', value: 3.0, target_value: 7.0,
            });
            runner.expect(r.constraint_type).toBe('minInclusive');
            runner.expect(r.value).toBe(3.0);
            runner.expect(r.target_value).toBe(7.0);
        });

        runner.it('maxExclusive: 9.9 < 10.0 → passes', () => {
            if (!mod.validate_shacl_constraint_wasm) return;
            const r = mod.validate_shacl_constraint_wasm({
                constraint_type: 'maxExclusive', value: 10.0, target_value: 9.9,
            });
            runner.expect(r.passes).toBeTruthy();
        });
    });
}

export default register;
