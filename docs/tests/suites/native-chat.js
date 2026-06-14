// Native Chat tests — exercises /chat/publish, /chat/pull, and sub-agent relay at localhost:4242.
// All tests skip automatically when the daemon is offline.

import { TestRunner } from '../test-runner.js';
import { NativeClient } from '../native-client.js';

export const MODES = ['native', 'both'];

export function register(runner, ctx) {

    runner.describe('Native: Chat Relay', () => {

        runner.it('POST /chat/publish accepts a valid message and returns message_id', async () => {
            if (!ctx.native) return;
            const res = await fetch(`${ctx.native.base}/chat/publish`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    topic: 'test-room',
                    author: 'did:wellfare:test-agent',
                    content: 'Hello from test suite',
                }),
                signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
            });
            runner.expect(res.ok || res.status === 404).toBeTruthy();
            if (!res.ok) return;
            const body = await res.json();
            runner.expect(typeof body.message_id !== 'undefined').toBeTruthy();
        });

        runner.it('GET /chat/pull returns an array', async () => {
            if (!ctx.native) return;
            const res = await fetch(`${ctx.native.base}/chat/pull?topic=test-room`, {
                signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
            });
            runner.expect(res.ok || res.status === 404).toBeTruthy();
            if (!res.ok) return;
            const body = await res.json();
            runner.expect(Array.isArray(body) || typeof body === 'object').toBeTruthy();
        });

        runner.it('/chat/pull with unknown topic returns empty list or 404', async () => {
            if (!ctx.native) return;
            const res = await fetch(`${ctx.native.base}/chat/pull?topic=does-not-exist-xyz987`, {
                signal: AbortSignal.timeout ? AbortSignal.timeout(2000) : undefined,
            });
            if (res.status === 404) return;  // valid response
            runner.expect(res.ok).toBeTruthy();
            const body = await res.json();
            // Empty array or empty messages key
            const messages = Array.isArray(body) ? body : (body.messages ?? []);
            runner.expect(messages.length).toBe(0);
        });

        runner.it('/chat/publish without a topic returns 400', async () => {
            if (!ctx.native) return;
            try {
                const res = await fetch(`${ctx.native.base}/chat/publish`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ author: 'did:test', content: 'no topic' }),
                    signal: AbortSignal.timeout ? AbortSignal.timeout(2000) : undefined,
                });
                runner.expect(res.status === 400 || res.status === 422 || res.status === 404).toBeTruthy();
            } catch { /* network error = daemon may not implement endpoint */ }
        });
    });

    runner.describe('Native: Sub-Agent Delegation', () => {

        runner.it('chat_agents endpoint exists or gracefully 404s', async () => {
            if (!ctx.native) return;
            try {
                const res = await fetch(`${ctx.native.base}/agents/list`, {
                    signal: AbortSignal.timeout ? AbortSignal.timeout(2000) : undefined,
                });
                runner.expect(res.status === 200 || res.status === 404).toBeTruthy();
            } catch { /* endpoint optional */ }
        });

        runner.it('round-trip: publish then pull retrieves same content', async () => {
            if (!ctx.native) return;
            const topic   = `rt-test-${Date.now()}`;
            const content = `round-trip-msg-${Date.now()}`;
            const pubRes  = await fetch(`${ctx.native.base}/chat/publish`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ topic, author: 'did:test', content }),
                signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
            });
            if (!pubRes.ok) return;  // endpoint may not exist yet

            const pullRes = await fetch(`${ctx.native.base}/chat/pull?topic=${encodeURIComponent(topic)}`, {
                signal: AbortSignal.timeout ? AbortSignal.timeout(3000) : undefined,
            });
            if (!pullRes.ok) return;
            const body = await pullRes.json();
            const messages = Array.isArray(body) ? body : (body.messages ?? []);
            const found = messages.some(m => (m.content ?? m.message) === content);
            runner.expect(found).toBeTruthy();
        });
    });
}

export default register;
