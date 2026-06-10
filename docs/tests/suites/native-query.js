// Native Query Engine tests — precise assertions against running qualia-cli daemon.
// Verified against v0.0.10-dev --dev mode on port 4242.

import { TestRunner } from '../test-runner.js';

export const MODES = ['native', 'both'];

export function register(runner, ctx) {
    runner.describe('Native: Query — JSON-LD format', () => {

        runner.it('wildcard query returns HTTP 200', async () => {
            if (!ctx.native) return;
            const { status } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(status).toBe(200);
        });

        runner.it('Content-Type is application/ld+json', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'json-ld' }),
            });
            runner.expect(r.headers.get('content-type')).toContain('application/ld+json');
        });

        runner.it('response has @context with @vocab key', async () => {
            if (!ctx.native) return;
            const { ok, body } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(ok).toBeTruthy();
            runner.expect(body).toHaveProperty('@context');
            runner.expect(body['@context']).toHaveProperty('@vocab');
        });

        runner.it('@vocab is https://qualia-db.org/vocab#', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(body['@context']['@vocab']).toBe('https://qualia-db.org/vocab#');
        });

        runner.it('response has @graph array', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(Array.isArray(body['@graph'])).toBeTruthy();
        });

        runner.it('response has match_count field', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(typeof body.match_count).toBe('number');
        });

        runner.it('match_count equals @graph.length', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(body.match_count).toBe(body['@graph'].length);
        });

        runner.it('X-Qualia-Compute-Cost header is present', async () => {
            if (!ctx.native) return;
            const { computeCost } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(typeof computeCost).toBe('string');
            runner.expect(computeCost.length).toBeGreaterThan(0);
        });

        runner.it('X-Qualia-Compute-Cost format is {matchCount}+{vmCycles}', async () => {
            if (!ctx.native) return;
            const { computeCost } = await ctx.native.query('?s ?p ?o', 'json-ld');
            runner.expect(/^\d+\+\d+$/.test(computeCost)).toBeTruthy();
        });

        runner.it('bound-subject query compiles and returns 200', async () => {
            if (!ctx.native) return;
            const { status } = await ctx.native.query('<http://example.org/Alice> ?p ?o', 'json-ld');
            runner.expect(status).toBe(200);
        });

        runner.it('bound-predicate query compiles and returns 200', async () => {
            if (!ctx.native) return;
            const { status } = await ctx.native.query('?s <http://xmlns.com/foaf/0.1/name> ?o', 'json-ld');
            runner.expect(status).toBe(200);
        });

        runner.it('fully-bound triple compiles and returns 200', async () => {
            if (!ctx.native) return;
            const { status } = await ctx.native.query(
                '<http://example.org/Alice> <http://xmlns.com/foaf/0.1/name> "Alice" .', 'json-ld');
            runner.expect(status).toBe(200);
        });
    });

    runner.describe('Native: Query — N-Triples format', () => {

        runner.it('n-triples query returns HTTP 200', async () => {
            if (!ctx.native) return;
            const { ok } = await ctx.native.queryText('?s ?p ?o');
            runner.expect(ok).toBeTruthy();
        });

        runner.it('Content-Type is application/n-triples', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'Accept': 'application/n-triples' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'n-triples' }),
            });
            runner.expect(r.headers.get('content-type')).toContain('application/n-triples');
        });

        runner.it('X-Qualia-Compute-Cost present for n-triples response', async () => {
            if (!ctx.native) return;
            const { computeCost } = await ctx.native.queryText('?s ?p ?o');
            runner.expect(typeof computeCost).toBe('string');
        });

        runner.it('n-triples body is a string (not JSON)', async () => {
            if (!ctx.native) return;
            const { text } = await ctx.native.queryText('?s ?p ?o');
            runner.expect(typeof text).toBe('string');
        });
    });

    runner.describe('Native: Query — Error handling', () => {

        runner.it('unsupported format returns 406 Not Acceptable', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'turtle' }),
            });
            runner.expect(r.status).toBe(406);
        });

        runner.it('406 body has code = "not_acceptable"', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'turtle' }),
            });
            const body = await r.json();
            runner.expect(body.code).toBe('not_acceptable');
            runner.expect(body.status).toBe('error');
            runner.expect(typeof body.message).toBe('string');
        });

        runner.it('406 message lists supported formats', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'xml' }),
            });
            const body = await r.json();
            runner.expect(body.message).toContain('application/ld+json');
            runner.expect(body.message).toContain('application/n-triples');
        });
    });

    runner.describe('Native: Query — Latency', () => {

        runner.it('wildcard query responds within 2 s', async () => {
            if (!ctx.native) return;
            const t0 = performance.now();
            await ctx.native.query('?s ?p ?o');
            runner.expect(performance.now() - t0).toBeLessThan(2000);
        });

        runner.it('five sequential queries stay under 500 ms each', async () => {
            if (!ctx.native) return;
            for (let i = 0; i < 5; i++) {
                const t0 = performance.now();
                await ctx.native.query('?s ?p ?o');
                runner.expect(performance.now() - t0).toBeLessThan(500);
            }
        });
    });
}

export default register;
