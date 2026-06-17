/**
 * ICP relay transport — daemon /chat/publish + /chat/pull with Lamport ordering.
 */

export const ICP_ROLE = {
    COMMAND: 'icp_command',
    CONTEXT: 'icp_context',
    GRAPH: 'icp_graph',
    HELLO: 'icp_hello',
    PUSH: 'icp_push',
};

const PATH_CANDIDATES = [
    { publish: '/chat/publish', pull: '/chat/pull' },
    { publish: '/publish', pull: '/pull' },
];

function trimBase(base) {
    return (base || 'http://127.0.0.1:4242').replace(/\/+$/, '');
}

function nowUnix() {
    return Math.floor(Date.now() / 1000);
}

/**
 * @param {object} opts
 * @param {string} opts.base
 * @param {string} opts.sessionId
 * @param {string} [opts.authorDid]
 * @param {string} [opts.authorName]
 * @param {number} [opts.initialLamport]
 */
export function createIcpRelay(opts) {
    const base = trimBase(opts.base);
    const sessionId = opts.sessionId;
    const authorDid = opts.authorDid || 'did:icp:anonymous';
    const authorName = opts.authorName || 'icp-client';
    let lamport = opts.initialLamport || 0;
    let sinceLamport = opts.sinceLamport || 0;
    let paths = null;
    let polling = false;
    let pollTimer = null;
    const listeners = new Set();

    async function resolvePaths() {
        if (paths) return paths;
        for (const candidate of PATH_CANDIDATES) {
            try {
                const url = `${base}${candidate.pull}?session_id=__probe__&since_lamport=0`;
                const res = await fetch(url, { method: 'GET' });
                if (res.ok || res.status === 400) {
                    paths = candidate;
                    return paths;
                }
            } catch {
                // try next candidate
            }
        }
        paths = PATH_CANDIDATES[0];
        return paths;
    }

    function nextLamport() {
        lamport += 1;
        return lamport;
    }

    function buildEnvelope(role, content) {
        return {
            session_id: sessionId,
            lamport: nextLamport(),
            role,
            content: typeof content === 'string' ? content : JSON.stringify(content),
            author_did: authorDid,
            author_name: authorName,
            reply_to_fragment: null,
            timestamp: nowUnix(),
            signature_hex: '',
        };
    }

    async function publish(role, content) {
        const route = await resolvePaths();
        const envelope = buildEnvelope(role, content);
        const res = await fetch(`${base}${route.publish}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(envelope),
        });
        if (!res.ok) {
            const err = await res.text().catch(() => res.statusText);
            throw new Error(`relay publish ${res.status}: ${err}`);
        }
        return envelope;
    }

    async function pull(since = sinceLamport) {
        const route = await resolvePaths();
        const url = `${base}${route.pull}?session_id=${encodeURIComponent(sessionId)}&since_lamport=${since}`;
        const res = await fetch(url, { method: 'GET' });
        if (!res.ok) {
            const err = await res.text().catch(() => res.statusText);
            throw new Error(`relay pull ${res.status}: ${err}`);
        }
        const data = await res.json();
        const messages = Array.isArray(data.messages) ? data.messages : [];
        if (typeof data.latest_lamport === 'number' && data.latest_lamport > sinceLamport) {
            sinceLamport = data.latest_lamport;
        }
        for (const msg of messages) {
            if (msg.lamport > sinceLamport) sinceLamport = msg.lamport;
        }
        return messages;
    }

    function onMessage(fn) {
        listeners.add(fn);
        return () => listeners.delete(fn);
    }

    function emit(messages) {
        for (const msg of messages) {
            for (const fn of listeners) {
                try {
                    fn(msg);
                } catch (e) {
                    console.warn('[icp-relay] listener', e);
                }
            }
        }
    }

    async function pollOnce() {
        const messages = await pull();
        if (messages.length) emit(messages);
        return messages;
    }

    function startPolling(intervalMs = 400) {
        if (polling) return;
        polling = true;
        const tick = async () => {
            if (!polling) return;
            try {
                await pollOnce();
            } catch (e) {
                console.warn('[icp-relay] poll', e);
            }
            pollTimer = setTimeout(tick, intervalMs);
        };
        tick();
    }

    function stopPolling() {
        polling = false;
        if (pollTimer) {
            clearTimeout(pollTimer);
            pollTimer = null;
        }
    }

    return {
        base,
        sessionId,
        authorDid,
        publish,
        pull,
        pollOnce,
        startPolling,
        stopPolling,
        onMessage,
        nextLamport,
        get sinceLamport() {
            return sinceLamport;
        },
        set sinceLamport(v) {
            sinceLamport = v;
        },
    };
}

/**
 * Parse relay envelope content JSON safely.
 * @param {object} envelope
 */
export function parseEnvelopeContent(envelope) {
    if (!envelope?.content) return null;
    try {
        return JSON.parse(envelope.content);
    } catch {
        return envelope.content;
    }
}