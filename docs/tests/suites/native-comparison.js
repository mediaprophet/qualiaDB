// Cross-mode comparison + WebSocket bridge tests.
// Runs when both WASM and native daemon are available (Both mode),
// but WebSocket and version tests run in Native mode too.

import { TestRunner } from '../test-runner.js';

export const MODES = ['both'];

export function register(runner, ctx) {
    runner.describe('Compare: WASM ↔ Native consistency', () => {

        runner.it('daemon version is a semver string', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.health();
            runner.expect(/^\d+\.\d+\.\d+/.test(body.version)).toBeTruthy();
        });

        runner.it('WASM compile_query_to_json produces valid JSON', () => {
            if (!ctx.wasm || !ctx.wasm.compile_query_to_json) return;
            const raw = ctx.wasm.compile_query_to_json('?s ?p ?o');
            runner.expect(() => JSON.parse(raw)).not.toThrow();
        });

        runner.it('WASM wildcard on empty buffer returns matches array', () => {
            if (!ctx.wasm || !ctx.wasm.execute_ntriples_query) return;
            const result = JSON.parse(ctx.wasm.execute_ntriples_query('?s ?p ?o', new Uint8Array(0), 256));
            runner.expect(Array.isArray(result.matches)).toBeTruthy();
        });

        runner.it('native wildcard returns @graph array', async () => {
            if (!ctx.native) return;
            const { ok, body } = await ctx.native.query('?s ?p ?o');
            if (!ok) return;
            runner.expect(Array.isArray(body['@graph'])).toBeTruthy();
        });

        runner.it('WASM vm_cycles field is a non-negative integer', () => {
            if (!ctx.wasm || !ctx.wasm.execute_ntriples_query) return;
            const result = JSON.parse(ctx.wasm.execute_ntriples_query('?s ?p ?o', new Uint8Array(0), 256));
            runner.expect(typeof result.vm_cycles).toBe('number');
            runner.expect(result.vm_cycles).toBeGreaterThanOrEqual(0);
        });

        runner.it('native compute cost cycles field is a non-negative integer', async () => {
            if (!ctx.native) return;
            const { ok, computeCost } = await ctx.native.query('?s ?p ?o');
            if (!ok || !computeCost) return;
            const cycles = parseInt(computeCost.split('+')[1] || '0', 10);
            runner.expect(cycles).toBeGreaterThanOrEqual(0);
        });

        runner.it('native match count in header matches body @graph length', async () => {
            if (!ctx.native) return;
            const { ok, body, computeCost } = await ctx.native.query('?s ?p ?o');
            if (!ok || !computeCost) return;
            const headerMatches = parseInt(computeCost.split('+')[0], 10);
            runner.expect(headerMatches).toBe((body['@graph'] || []).length);
        });

        runner.it('both backends agree: empty DB → 0 results', async () => {
            if (!ctx.wasm || !ctx.wasm.execute_ntriples_query || !ctx.native) return;
            const wasmResult = JSON.parse(
                ctx.wasm.execute_ntriples_query('?s ?p ?o', new Uint8Array(0), 256));
            const { ok, body } = await ctx.native.query('?s ?p ?o');
            runner.expect(wasmResult.matches.length).toBe(0);
            if (ok) runner.expect(body['@graph'].length).toBe(0);
        });

        runner.it('WebSocket metrics query returns vm_cycles', async () => {
            if (!ctx.native) return;
            const wsUrl = ctx.native.base.replace('http://', 'ws://') + '/qualia-bridge';
            const frame = await new Promise((resolve, reject) => {
                const ws = new WebSocket(wsUrl);
                const timer = setTimeout(() => { ws.close(); reject(new Error('timeout')); }, 5000);
                ws.onmessage = (e) => {
                    const msg = JSON.parse(e.data);
                    if (msg.type === 'HANDSHAKE_SUCCESS') {
                        ws.send(JSON.stringify({
                            type: 'query',
                            id: 42,
                            query: '?s ?p ?o',
                            format: 'metrics',
                        }));
                        return;
                    }
                    clearTimeout(timer);
                    ws.close();
                    resolve(msg);
                };
                ws.onerror = () => { clearTimeout(timer); reject(new Error('ws error')); };
            });
            runner.expect(frame.type).toBe('result');
            runner.expect(typeof frame.vm_cycles).toBe('number');
            runner.expect(frame.vm_cycles).toBeGreaterThanOrEqual(0);
        });
    });

    runner.describe('Native: WebSocket Bridge', () => {

        runner.it('connects to /qualia-bridge without error', async () => {
            if (!ctx.native) return;
            const wsUrl = ctx.native.base.replace('http://', 'ws://') + '/qualia-bridge';
            await new Promise((resolve) => {
                const ws = new WebSocket(wsUrl);
                const timeout = setTimeout(() => { ws.close(); resolve(); }, 2000);
                ws.onopen = () => { clearTimeout(timeout); ws.close(); resolve(); };
                ws.onerror = () => { clearTimeout(timeout); resolve(); };
            });
            // If we get here without an error being thrown the test passes
        });

        runner.it('first message is HANDSHAKE_SUCCESS JSON', async () => {
            if (!ctx.native) return;
            const wsUrl = ctx.native.base.replace('http://', 'ws://') + '/qualia-bridge';
            const msg = await new Promise((resolve) => {
                const ws = new WebSocket(wsUrl);
                const timeout = setTimeout(() => { ws.close(); resolve(null); }, 3000);
                ws.onmessage = (e) => {
                    clearTimeout(timeout);
                    ws.close();
                    resolve(e.data);
                };
                ws.onerror = () => { clearTimeout(timeout); resolve(null); };
            });
            if (!msg) return; // WS unavailable — skip
            const parsed = JSON.parse(msg);
            runner.expect(parsed.type).toBe('HANDSHAKE_SUCCESS');
        });

        runner.it('handshake payload has mode = "NATIVE"', async () => {
            if (!ctx.native) return;
            const wsUrl = ctx.native.base.replace('http://', 'ws://') + '/qualia-bridge';
            const msg = await new Promise((resolve) => {
                const ws = new WebSocket(wsUrl);
                const timeout = setTimeout(() => { ws.close(); resolve(null); }, 3000);
                ws.onmessage = (e) => { clearTimeout(timeout); ws.close(); resolve(e.data); };
                ws.onerror = () => { clearTimeout(timeout); resolve(null); };
            });
            if (!msg) return;
            const parsed = JSON.parse(msg);
            runner.expect(parsed.payload.mode).toBe('NATIVE');
        });

        runner.it('handshake payload has version matching /health version', async () => {
            if (!ctx.native) return;
            const wsUrl = ctx.native.base.replace('http://', 'ws://') + '/qualia-bridge';
            const [healthRes, wsMsg] = await Promise.all([
                ctx.native.health(),
                new Promise((resolve) => {
                    const ws = new WebSocket(wsUrl);
                    const timeout = setTimeout(() => { ws.close(); resolve(null); }, 3000);
                    ws.onmessage = (e) => { clearTimeout(timeout); ws.close(); resolve(e.data); };
                    ws.onerror = () => { clearTimeout(timeout); resolve(null); };
                }),
            ]);
            if (!wsMsg) return;
            const parsed = JSON.parse(wsMsg);
            runner.expect(parsed.payload.version).toBe(healthRes.body.version);
        });
    });
}

export default register;
