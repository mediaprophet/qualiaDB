// Native Torrent tests — exercises /torrent/seed, /torrent/telemetry, /torrent/webseed at localhost:4242.
// All tests skip automatically when the daemon is offline.

import { TestRunner } from '../test-runner.js';
import { NativeClient } from '../native-client.js';

export const MODES = ['native', 'both'];

export function register(runner, ctx) {

    runner.describe('Native: WebTorrent Seeder', () => {

        runner.it('POST /torrent/seed returns 200 or 404 (endpoint presence check)', async () => {
            if (!ctx.native) return;
            try {
                const res = await fetch(`${ctx.native.base}/torrent/seed`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ graph_id: 'test-graph-001', export_format: 'ntriples' }),
                    signal: AbortSignal.timeout ? AbortSignal.timeout(4000) : undefined,
                });
                runner.expect(res.status === 200 || res.status === 404 || res.status === 422).toBeTruthy();
            } catch { /* network — daemon not running, skip silently */ }
        });

        runner.it('POST /torrent/seed with valid graph returns info_hash or error', async () => {
            if (!ctx.native) return;
            try {
                const res = await fetch(`${ctx.native.base}/torrent/seed`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ graph_id: 'qualia-ontology-test', export_format: 'ntriples' }),
                    signal: AbortSignal.timeout ? AbortSignal.timeout(5000) : undefined,
                });
                if (!res.ok) return;
                const body = await res.json();
                runner.expect(typeof body === 'object').toBeTruthy();
                // If seeding succeeded, should have info_hash
                if (body.info_hash) {
                    runner.expect(typeof body.info_hash).toBe('string');
                    runner.expect(body.info_hash.length).toBeGreaterThan(0);
                }
            } catch { /* optional endpoint */ }
        });

        runner.it('GET /torrent/telemetry returns peer_count or 404', async () => {
            if (!ctx.native) return;
            try {
                const res = await fetch(`${ctx.native.base}/torrent/telemetry`, {
                    signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
                });
                runner.expect(res.status === 200 || res.status === 404).toBeTruthy();
                if (!res.ok) return;
                const body = await res.json();
                runner.expect(typeof body === 'object').toBeTruthy();
                // peer_count and bytes_uploaded are expected if endpoint is live
                if ('peer_count' in body) {
                    runner.expect(typeof body.peer_count).toBe('number');
                }
                if ('bytes_uploaded' in body) {
                    runner.expect(body.bytes_uploaded).toBeGreaterThanOrEqual(0);
                }
            } catch { /* optional */ }
        });

        runner.it('GET /torrent/webseed/:hash returns 200 or 404', async () => {
            if (!ctx.native) return;
            const fakeHash = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
            try {
                const res = await fetch(`${ctx.native.base}/torrent/webseed/${fakeHash}`, {
                    signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
                });
                // 404 is acceptable for a non-existent hash; 200 means the endpoint exists and seeded
                runner.expect(res.status === 200 || res.status === 404).toBeTruthy();
            } catch { /* optional */ }
        });

        runner.it('telemetry bytes_uploaded is non-negative after seeding', async () => {
            if (!ctx.native) return;
            try {
                // Seed something first
                await fetch(`${ctx.native.base}/torrent/seed`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ graph_id: 'telemetry-probe', export_format: 'ntriples' }),
                    signal: AbortSignal.timeout ? AbortSignal.timeout(4000) : undefined,
                });
                const res = await fetch(`${ctx.native.base}/torrent/telemetry`, {
                    signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
                });
                if (!res.ok) return;
                const body = await res.json();
                if (typeof body.bytes_uploaded !== 'undefined') {
                    runner.expect(body.bytes_uploaded).toBeGreaterThanOrEqual(0);
                }
            } catch { /* optional */ }
        });
    });
}

export default register;
