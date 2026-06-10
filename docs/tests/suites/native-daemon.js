// Native Daemon tests — exercises the Qualia daemon at localhost:4242.
// All tests skip automatically when the daemon is offline.
// Assertions verified against qualia-cli v0.0.10-dev running --dev on port 4242.

import { TestRunner } from '../test-runner.js';
import { NativeClient } from '../native-client.js';

export const MODES = ['native', 'both'];

export function register(runner, ctx) {
    runner.describe('Native: Daemon Health', () => {

        runner.it('GET /health returns HTTP 200', async () => {
            if (!ctx.native) return;
            const { ok } = await ctx.native.health();
            runner.expect(ok).toBeTruthy();
        });

        runner.it('/health body has status = "active"', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.health();
            runner.expect(body.status).toBe('active');
        });

        runner.it('/health body has engine = "qualia-core-db"', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.health();
            runner.expect(body.engine).toBe('qualia-core-db');
        });

        runner.it('/health body has semver version field', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.health();
            runner.expect(typeof body.version).toBe('string');
            runner.expect(/^\d+\.\d+\.\d+/.test(body.version)).toBeTruthy();
        });

        runner.it('/health Content-Type is application/json', async () => {
            if (!ctx.native) return;
            // Fetch raw to inspect headers
            const r = await fetch(`${ctx.native.base}/health`, { signal: AbortSignal.timeout ? AbortSignal.timeout(2000) : undefined });
            runner.expect(r.headers.get('content-type')).toContain('application/json');
        });

        runner.it('/health is reachable cross-origin (CORS allows localhost)', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/health`);
            // If CORS blocked us the fetch would throw; reaching here means CORS passed
            runner.expect(r.ok).toBeTruthy();
        });

        runner.it('/health responds within 1 s', async () => {
            if (!ctx.native) return;
            const t0 = performance.now();
            await ctx.native.health(1000);
            runner.expect(performance.now() - t0).toBeLessThan(1000);
        });
    });

    runner.describe('Native: Dev-mode Authentication', () => {

        runner.it('POST /query without token succeeds in dev mode (200)', async () => {
            if (!ctx.native) return;
            const noAuth = new NativeClient(ctx.native.base, '');
            const { status } = await noAuth.query('?s ?p ?o');
            // In dev mode daemon skips token check → 200
            runner.expect(status).toBe(200);
        });

        runner.it('POST /query with any token also succeeds in dev mode', async () => {
            if (!ctx.native) return;
            const anyToken = new NativeClient(ctx.native.base, 'arbitrary_dev_token');
            const { status } = await anyToken.query('?s ?p ?o');
            runner.expect(status).toBe(200);
        });

        runner.it('POST /query is reachable cross-origin (CORS allows localhost)', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'json-ld' }),
            });
            runner.expect(r.ok).toBeTruthy();
        });
    });
}

export default register;
