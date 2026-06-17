#!/usr/bin/env node
/**
 * ICP scaffold verification — JS modules + optional WASM API exports.
 */
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
let failed = 0;

function ok(msg) {
    console.log(`  ✓ ${msg}`);
}

function fail(msg) {
    console.error(`  ✗ ${msg}`);
    failed += 1;
}

const required = [
    'js/qualia-icp-profile.js',
    'js/qualia-icp-rules.js',
    'js/qualia-icp-local.js',
    'js/qualia-icp-relay.js',
    'js/qualia-icp-session.js',
    'js/qualia-icp-context.js',
    'js/qualia-icp-host.js',
    'js/qualia-icp-phone.js',
    'js/qualia-icp-vault.js',
    'css/qualia-icp-layout.css',
    'phone-console.html',
    'phone-console.webmanifest',
];

console.log('phone-console-verify: scaffold files');
for (const rel of required) {
    const p = join(root, rel);
    if (existsSync(p)) ok(rel);
    else fail(`missing ${rel}`);
}

const dtsPath = join(root, 'pkg/qualia/qualia.d.ts');
if (existsSync(dtsPath)) {
    const dts = readFileSync(dtsPath, 'utf8');
    for (const sym of ['push_control_command', 'control_pending', 'drain_control_commands']) {
        if (dts.includes(sym)) ok(`qualia.d.ts export ${sym}`);
        else fail(`qualia.d.ts missing ${sym} — rebuild portal WASM`);
    }
} else {
    console.log('  ⚠ qualia.d.ts not found — skip WASM API check (run wasm-pack)');
}

console.log('\nphone-console-verify: codec round-trip');
try {
    const { packControlCommand, packCameraDelta, ICP_OP } = await import(
        new URL('../js/qualia-icp-local.js', import.meta.url)
    );
    const raw = packCameraDelta(0.1, -0.05, 0.02);
    if (typeof raw === 'bigint' && raw !== 0n) ok('packCameraDelta returns bigint u64');
    else fail('packCameraDelta invalid');

    const nav = packControlCommand(ICP_OP.NAVIGATE_INDEX, 0, 42, 0, 0);
    if (Number(nav & 0x7fn) === ICP_OP.NAVIGATE_INDEX) ok('packControlCommand opcode preserved');
    else fail('packControlCommand opcode mismatch');
} catch (e) {
    fail(`qualia-icp-local import: ${e.message}`);
}

try {
    const { parsePairingPayload, createConsoleSession } = await import(
        new URL('../js/qualia-icp-session.js', import.meta.url)
    );
    const session = createConsoleSession({ relayBase: 'http://127.0.0.1:4242' });
    const round = parsePairingPayload(JSON.stringify(session));
    if (round?.session_id === session.session_id) ok('pairing payload round-trip');
    else fail('pairing payload round-trip failed');
} catch (e) {
    fail(`qualia-icp-session import: ${e.message}`);
}

try {
    const { buildContextFrame, buildGraphLensFromTensor } = await import(
        new URL('../js/qualia-icp-context.js', import.meta.url)
    );
    const frame = buildContextFrame({ focusLabel: 'test' });
    if (JSON.stringify(frame).length <= 4096) ok('ContextFrame within 4 KiB');
    else fail('ContextFrame exceeds 4 KiB');
    const lens = buildGraphLensFromTensor(null);
    if (lens.nodes.length === 0) ok('GraphLensFrame empty tensor');
    else fail('GraphLensFrame should be empty for null buffer');
} catch (e) {
    fail(`qualia-icp-context import: ${e.message}`);
}

try {
    const { ICP_ROLE } = await import(
        new URL('../js/qualia-icp-relay.js', import.meta.url)
    );
    if (ICP_ROLE.COMMAND === 'icp_command') ok('ICP_ROLE constants');
    else fail('ICP_ROLE constants missing');
} catch (e) {
    fail(`qualia-icp-relay import: ${e.message}`);
}

try {
    const { defaultRelayBase } = await import(
        new URL('../js/qualia-icp-session.js', import.meta.url)
    );
    const base = defaultRelayBase(4242);
    if (base.includes(':4242')) ok('defaultRelayBase includes port');
    else fail('defaultRelayBase malformed');
} catch (e) {
    fail(`defaultRelayBase: ${e.message}`);
}

try {
    const {
        deviceDidFromHash,
        STANDPOINT_IDENTIFIER,
        STANDPOINT_VAULT,
    } = await import(new URL('../js/qualia-icp-vault.js', import.meta.url));
    const did = deviceDidFromHash('abc123');
    if (did.startsWith('did:icp:device:')) ok('device DID format');
    else fail('device DID format');
    if (STANDPOINT_IDENTIFIER === 2 && STANDPOINT_VAULT === 3) ok('standpoint class constants');
    else fail('standpoint class constants');
} catch (e) {
    fail(`qualia-icp-vault import: ${e.message}`);
}

console.log('\nphone-console-verify: optional daemon round-trip');
const daemonBase = process.env.QUALIA_DAEMON_BASE || 'http://127.0.0.1:4242';
try {
    const health = await fetch(`${daemonBase}/health`, { signal: AbortSignal.timeout(1500) });
    if (health.ok) {
        const { createIcpRelay, ICP_ROLE } = await import(
            new URL('../js/qualia-icp-relay.js', import.meta.url)
        );
        const { randomSessionId } = await import(
            new URL('../js/qualia-icp-session.js', import.meta.url)
        );
        const sid = `verify-${randomSessionId()}`;
        const relay = createIcpRelay({
            base: daemonBase,
            sessionId: sid,
            authorDid: 'did:icp:verify',
            authorName: 'verify',
        });
        await relay.publish(ICP_ROLE.HELLO, { probe: true });
        const msgs = await relay.pull(0);
        if (msgs.some((m) => m.role === ICP_ROLE.HELLO)) ok('daemon relay round-trip');
        else fail('daemon relay round-trip: message missing');
    } else {
        console.log('  ⚠ daemon not healthy — skip relay round-trip');
    }
} catch {
    console.log('  ⚠ daemon offline — skip relay round-trip');
}

if (failed > 0) {
    console.error(`\nphone-console-verify: ${failed} failure(s)`);
    process.exit(1);
}
console.log('\nphone-console-verify: OK');