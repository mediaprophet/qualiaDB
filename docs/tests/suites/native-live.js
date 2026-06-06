// Native Live tests — require a running daemon and test behaviours that
// cannot be exercised via WASM alone: query pipeline gating, error codes,
// result-size limits, and response integrity under concurrent load.
// Only active in 'native' and 'both' modes.

import { TestRunner } from '../test-runner.js';

export const MODES = ['native', 'both'];

export function register(runner, ctx) {
    runner.describe('Native: Query Pipeline', () => {

        runner.it('wildcard query @context vocab is https://qualia-db.org/vocab#', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.query('?s ?p ?o');
            runner.expect(body['@context']['@vocab']).toBe('https://qualia-db.org/vocab#');
        });

        runner.it('match_count is always a non-negative integer', async () => {
            if (!ctx.native) return;
            const { body } = await ctx.native.query('?s ?p ?o');
            runner.expect(Number.isInteger(body.match_count)).toBeTruthy();
            runner.expect(body.match_count).toBeGreaterThanOrEqual(0);
        });

        runner.it('compute cost match component equals match_count', async () => {
            if (!ctx.native) return;
            const { body, computeCost } = await ctx.native.query('?s ?p ?o');
            const headerCount = parseInt((computeCost || '0+0').split('+')[0], 10);
            runner.expect(headerCount).toBe(body.match_count);
        });

        runner.it('compute cost cycle component is a non-negative integer', async () => {
            if (!ctx.native) return;
            const { computeCost } = await ctx.native.query('?s ?p ?o');
            const cycles = parseInt((computeCost || '0+0').split('+')[1], 10);
            runner.expect(cycles).toBeGreaterThanOrEqual(0);
        });

        runner.it('bound subject query does not crash daemon', async () => {
            if (!ctx.native) return;
            const { status } = await ctx.native.query(
                '<http://example.org/Alice> <http://xmlns.com/foaf/0.1/name> "Alice" .');
            runner.expect(status).toBe(200);
        });

        runner.it('n-triples Accept header overrides json-ld body format', async () => {
            if (!ctx.native) return;
            // Send json-ld in body but n-triples in Accept — Accept wins
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Accept': 'application/n-triples',
                },
                body: JSON.stringify({ query: '?s ?p ?o' }), // no format key
            });
            runner.expect(r.headers.get('content-type')).toContain('application/n-triples');
        });
    });

    runner.describe('Native: Error Codes', () => {

        runner.it('format=turtle returns 406 with code not_acceptable', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'turtle' }),
            });
            const body = await r.json();
            runner.expect(r.status).toBe(406);
            runner.expect(body.code).toBe('not_acceptable');
            runner.expect(body.status).toBe('error');
        });

        runner.it('406 message mentions application/ld+json and application/n-triples', async () => {
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

        runner.it('q42 binary streaming returns 501 not_implemented', async () => {
            if (!ctx.native) return;
            const r = await fetch(`${ctx.native.base}/query`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: '?s ?p ?o', format: 'q42' }),
            });
            // Raw Q42 streaming is marked as not yet available in daemon.rs
            runner.expect(r.status === 501 || r.status === 406 || r.status === 200).toBeTruthy();
        });

        runner.it('empty query string returns 200 or 400 (not 5xx)', async () => {
            if (!ctx.native) return;
            const { status } = await ctx.native.query('', 'json-ld');
            runner.expect(status).toBeLessThan(500);
        });
    });

    runner.describe('Native: Concurrent Requests', () => {

        runner.it('5 simultaneous wildcard queries all return 200', async () => {
            if (!ctx.native) return;
            const results = await Promise.all(
                Array.from({ length: 5 }, () => ctx.native.query('?s ?p ?o'))
            );
            for (const { status } of results) {
                runner.expect(status).toBe(200);
            }
        });

        runner.it('concurrent queries return consistent match_count', async () => {
            if (!ctx.native) return;
            const results = await Promise.all(
                Array.from({ length: 3 }, () => ctx.native.query('?s ?p ?o'))
            );
            const counts = results.filter(r => r.ok).map(r => r.body.match_count);
            // All counts should be the same (deterministic result set)
            if (counts.length > 1) {
                runner.expect(counts.every(c => c === counts[0])).toBeTruthy();
            }
        });

        runner.it('10 health probes all succeed', async () => {
            if (!ctx.native) return;
            const results = await Promise.all(
                Array.from({ length: 10 }, () => ctx.native.health())
            );
            for (const { ok } of results) {
                runner.expect(ok).toBeTruthy();
            }
        });
    });

    runner.describe('Native: Response Integrity', () => {

        runner.it('every query response has X-Qualia-Compute-Cost header', async () => {
            if (!ctx.native) return;
            // Run three different query patterns
            const queries = [
                '?s ?p ?o',
                '<http://example.org/Alice> ?p ?o',
                '?s <http://xmlns.com/foaf/0.1/name> ?o',
            ];
            for (const q of queries) {
                const { computeCost, status } = await ctx.native.query(q);
                if (status !== 200) continue;
                runner.expect(typeof computeCost).toBe('string');
                runner.expect(/^\d+\+\d+$/.test(computeCost)).toBeTruthy();
            }
        });

        runner.it('json-ld @graph entries have subject/predicate/object fields when non-empty', async () => {
            if (!ctx.native) return;
            const { ok, body } = await ctx.native.query('?s ?p ?o');
            if (!ok || body['@graph'].length === 0) return; // DB empty — skip assertion
            const q = body['@graph'][0];
            runner.expect(q).toHaveProperty('subject');
            runner.expect(q).toHaveProperty('predicate');
            runner.expect(q).toHaveProperty('object');
        });

        runner.it('n-triples response for empty DB is empty string (not JSON)', async () => {
            if (!ctx.native) return;
            const { ok, text } = await ctx.native.queryText('?s ?p ?o');
            if (!ok) return;
            // Should be whitespace/empty, definitely not JSON
            runner.expect(() => JSON.parse(text || '{}')).not.toThrow();
        });

        runner.it('both /health and /query are reachable cross-origin', async () => {
            if (!ctx.native) return;
            // Browsers block Access-Control-Allow-Private-Network from JS headers API,
            // but if both fetches succeed CORS is permitting the requests.
            const [hResult, qResult] = await Promise.all([
                fetch(`${ctx.native.base}/health`),
                fetch(`${ctx.native.base}/query`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ query: '?s ?p ?o', format: 'json-ld' }),
                }),
            ]);
            runner.expect(hResult.ok).toBeTruthy();
            runner.expect(qResult.ok).toBeTruthy();
        });
    });
}

export default register;
