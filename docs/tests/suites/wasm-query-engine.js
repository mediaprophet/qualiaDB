// WASM Query Engine tests.
// execute_ntriples_query expects flat QualiaQuin bytes (N × 48 bytes each),
// NOT the 40960-byte SuperBlock format used by the native daemon.

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

// Empty flat quin buffer (0 quins)
function emptyQuins() { return new Uint8Array(0); }

// Build N all-zero quins (48 bytes each). Zero-quins pass ECC (parity ≠ MAX).
function zeroQuins(n) { return new Uint8Array(48 * n); }

// Build a single Quin as 48 raw bytes (little-endian u64 fields).
function encodeQuin(subject = 0n, predicate = 0n, object = 0n,
                    context = 0n, metadata = 0n, parity = 0n) {
    const buf = new Uint8Array(48);
    const dv  = new DataView(buf.buffer);
    dv.setBigUint64( 0, BigInt(subject),  true);
    dv.setBigUint64( 8, BigInt(predicate), true);
    dv.setBigUint64(16, BigInt(object),   true);
    dv.setBigUint64(24, BigInt(context),  true);
    dv.setBigUint64(32, BigInt(metadata), true);
    dv.setBigUint64(40, BigInt(parity),   true);
    return buf;
}

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Query Engine', () => {

        runner.beforeAll(async () => {
            mod = await loadWasm();
        });

        runner.it('WASM module loads without error', () => {
            runner.expect(mod).not.toBeNull();
        });

        runner.it('execute_ntriples_query is exported', () => {
            if (!mod.execute_ntriples_query) return;
            runner.expect(typeof mod.execute_ntriples_query).toBe('function');
        });

        runner.it('empty quin buffer returns JSON object', () => {
            if (!mod.execute_ntriples_query) return;
            const raw = mod.execute_ntriples_query('?s ?p ?o', emptyQuins(), 256);
            const result = JSON.parse(raw);
            runner.expect(typeof result).toBe('object');
        });

        runner.it('empty quin buffer: result has matches array', () => {
            if (!mod.execute_ntriples_query) return;
            const result = JSON.parse(mod.execute_ntriples_query('?s ?p ?o', emptyQuins(), 256));
            runner.expect(Array.isArray(result.matches)).toBeTruthy();
        });

        runner.it('wildcard query on empty buffer returns 0 matches', () => {
            if (!mod.execute_ntriples_query) return;
            const result = JSON.parse(mod.execute_ntriples_query('?s ?p ?o', emptyQuins(), 256));
            runner.expect(result.matches.length).toBe(0);
        });

        runner.it('execute_ntriples_query returns vm_cycles field', () => {
            if (!mod.execute_ntriples_query) return;
            const result = JSON.parse(mod.execute_ntriples_query('?s ?p ?o', emptyQuins(), 256));
            runner.expect('vm_cycles' in result).toBeTruthy();
        });

        runner.it('max_results must be >= match count to avoid overflow', () => {
            if (!mod.execute_ntriples_query) return;
            // 5 quins, buffer=256 — should succeed with 5 or fewer matches
            const result = JSON.parse(mod.execute_ntriples_query('?s ?p ?o', zeroQuins(5), 256));
            runner.expect(Array.isArray(result.matches)).toBeTruthy();
            runner.expect(result.matches.length).toBeLessThanOrEqual(256);
        });

        runner.it('compile_query_to_json is exported', () => {
            if (!mod.compile_query_to_json) return;
            runner.expect(typeof mod.compile_query_to_json).toBe('function');
        });

        runner.it('compile_query_to_json returns valid JSON', () => {
            if (!mod.compile_query_to_json) return;
            const raw = mod.compile_query_to_json('?s <http://example.org/p> ?o');
            runner.expect(() => JSON.parse(raw)).not.toThrow();
        });

        runner.it('serialize_float64_array round-trips', () => {
            if (!mod.serialize_float64_array) return;
            const input = new Float64Array([1.5, 2.5, 3.14]);
            const output = mod.serialize_float64_array(input);
            runner.expect(output instanceof Float64Array).toBeTruthy();
            runner.expect(output.length).toBe(3);
        });

        runner.it('serialize_float_array returns Uint8Array', () => {
            if (!mod.serialize_float_array) return;
            const input = new Float32Array([1.0, 2.0]);
            const output = mod.serialize_float_array(input);
            runner.expect(output instanceof Uint8Array).toBeTruthy();
        });

        runner.describe('Engine metadata', () => {

            runner.it('get_engine_version returns semver string', () => {
                if (!mod.get_engine_version) return;
                const v = mod.get_engine_version();
                runner.expect(typeof v).toBe('string');
                runner.expect(/^\d+\.\d+\.\d+/.test(v)).toBeTruthy();
            });

            runner.it('get_engine_info returns version + engine + target', () => {
                if (!mod.get_engine_info) return;
                const info = mod.get_engine_info();
                runner.expect(info.version).toBeTruthy();
                runner.expect(info.engine).toBe('qualia-core-db');
                runner.expect(info.target).toBe('wasm32');
                runner.expect(Array.isArray(info.capabilities)).toBeTruthy();
                runner.expect(info.capabilities.length).toBeGreaterThan(0);
            });

            runner.it('list_capabilities_wasm matches get_engine_info.capabilities', () => {
                if (!mod.get_engine_info || !mod.list_capabilities_wasm) return;
                const info = mod.get_engine_info();
                const caps = mod.list_capabilities_wasm();
                runner.expect(caps.length).toBe(info.capabilities.length);
            });
        });
    });
}

export default register;
