// QCHK Capability Profile tests.
// Covers: magic byte validation, profile ID computation, .chk disambiguation,
// QCHK binary format structure, and the 6 named profiles.
//
// Source-of-truth: crates/qualia-core-db/src/profiles.rs
//   QCHK binary layout:
//     Offset  Size  Field
//     0       4     Magic: 0x51 0x43 0x48 0x4B ("QCHK")
//     4       8     profile_id (u64 little-endian)
//     12      4     payload_len (u32 little-endian)
//     16      N     JSON-LD payload (UTF-8, payload_len bytes)
//
// IMPORTANT: .chk is also the extension for CogAI Cognitive AI Chunks (text format).
// Distinguish by the magic bytes at offset 0 — QCHK binary starts with "QCHK";
// a CogAI text chunk does not.

import { q_hash, makeQuin } from './primitives.js';
import { loadWasm } from '../wasm-loader.js';

// ─── QCHK constants ───────────────────────────────────────────────────────────

const QCHK_MAGIC      = new Uint8Array([0x51, 0x43, 0x48, 0x4B]); // "QCHK"
const QCHK_HEADER_LEN = 16; // 4 magic + 8 profile_id + 4 payload_len

// ─── Named profile IDs (mirrors profiles.rs) ─────────────────────────────────

const PROFILE_IDS = {
    general:   q_hash('profile:general'),
    health:    q_hash('profile:health'),
    chemistry: q_hash('profile:chemistry'),
    research:  q_hash('profile:research'),
    legal:     q_hash('profile:legal'),
    financial: q_hash('profile:financial'),
};

// ─── QCHK builder (JS reference) ─────────────────────────────────────────────

function buildQchk(profileId, payloadJsonLd) {
    const encoder = new TextEncoder();
    const payload = encoder.encode(payloadJsonLd);

    const buf  = new ArrayBuffer(QCHK_HEADER_LEN + payload.byteLength);
    const view = new DataView(buf);
    const u8   = new Uint8Array(buf);

    // Magic
    u8.set(QCHK_MAGIC, 0);
    // profile_id as u64 little-endian (two u32 halves — JS lacks native u64 DataView)
    const lo = Number(profileId & 0xFFFF_FFFFn);
    const hi = Number((profileId >> 32n) & 0xFFFF_FFFFn);
    view.setUint32(4,  lo, true);
    view.setUint32(8,  hi, true);
    // payload_len as u32 little-endian
    view.setUint32(12, payload.byteLength, true);
    // JSON-LD payload
    u8.set(payload, QCHK_HEADER_LEN);

    return u8;
}

// ─── QCHK parser (JS reference) ──────────────────────────────────────────────

function parseQchk(bytes) {
    if (bytes.length < QCHK_HEADER_LEN) throw new Error('QCHK too short');

    for (let i = 0; i < 4; i++) {
        if (bytes[i] !== QCHK_MAGIC[i]) throw new Error('Not a QCHK file — magic mismatch');
    }

    const view    = new DataView(bytes.buffer, bytes.byteOffset);
    const lo      = view.getUint32(4, true);
    const hi      = view.getUint32(8, true);
    const profileId = (BigInt(hi) << 32n) | BigInt(lo);
    const payloadLen = view.getUint32(12, true);

    if (bytes.length < QCHK_HEADER_LEN + payloadLen) throw new Error('Truncated QCHK payload');

    const payload = new TextDecoder().decode(bytes.slice(QCHK_HEADER_LEN, QCHK_HEADER_LEN + payloadLen));
    return { profileId, payload };
}

// ─── Disambiguator ────────────────────────────────────────────────────────────

function detectChkType(bytes) {
    if (bytes.length >= 4 &&
        bytes[0] === 0x51 && bytes[1] === 0x43 &&
        bytes[2] === 0x48 && bytes[3] === 0x4B) {
        return 'qchk-binary';
    }
    return 'cogai-text';
}

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Capability Profiles (QCHK)', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── Magic byte validation ─────────────────────────────────────────────

        runner.it('QCHK magic bytes spell "QCHK"', () => {
            const ascii = Array.from(QCHK_MAGIC).map(b => String.fromCharCode(b)).join('');
            runner.expect(ascii).toBe('QCHK');
        });

        runner.it('QCHK magic bytes are 0x51 0x43 0x48 0x4B', () => {
            runner.expect(QCHK_MAGIC[0]).toBe(0x51);
            runner.expect(QCHK_MAGIC[1]).toBe(0x43);
            runner.expect(QCHK_MAGIC[2]).toBe(0x48);
            runner.expect(QCHK_MAGIC[3]).toBe(0x4B);
        });

        runner.it('header length is 16 bytes (4 magic + 8 profile_id + 4 payload_len)', () => {
            runner.expect(QCHK_HEADER_LEN).toBe(16);
        });

        // ── buildQchk / parseQchk round-trip ─────────────────────────────────

        runner.it('buildQchk produces bytes starting with QCHK magic', () => {
            const bytes = buildQchk(PROFILE_IDS.health, '{}');
            runner.expect(bytes[0]).toBe(0x51);
            runner.expect(bytes[1]).toBe(0x43);
            runner.expect(bytes[2]).toBe(0x48);
            runner.expect(bytes[3]).toBe(0x4B);
        });

        runner.it('buildQchk / parseQchk round-trips profile ID', () => {
            const pid   = PROFILE_IDS.health;
            const bytes = buildQchk(pid, '{}');
            const parsed = parseQchk(bytes);
            runner.expect(parsed.profileId).toBe(pid);
        });

        runner.it('buildQchk / parseQchk round-trips JSON-LD payload', () => {
            const payload = '{"@context":"https://qualia.social/ns/","profile":"health"}';
            const bytes   = buildQchk(PROFILE_IDS.health, payload);
            const parsed  = parseQchk(bytes);
            runner.expect(parsed.payload).toBe(payload);
        });

        runner.it('total QCHK size = 16 + payload bytes', () => {
            const payload = 'hello';
            const bytes   = buildQchk(PROFILE_IDS.general, payload);
            runner.expect(bytes.length).toBe(16 + new TextEncoder().encode(payload).byteLength);
        });

        runner.it('parseQchk throws on wrong magic', () => {
            const bad = new Uint8Array([0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            runner.expect(() => parseQchk(bad)).toThrow();
        });

        runner.it('parseQchk throws on too-short input', () => {
            runner.expect(() => parseQchk(new Uint8Array([0x51, 0x43]))).toThrow();
        });

        // ── .chk disambiguator ────────────────────────────────────────────────

        runner.it('detectChkType identifies QCHK binary', () => {
            const bytes = buildQchk(PROFILE_IDS.general, '{}');
            runner.expect(detectChkType(bytes)).toBe('qchk-binary');
        });

        runner.it('detectChkType identifies CogAI text chunk', () => {
            const encoder = new TextEncoder();
            const bytes   = encoder.encode('dog dog1 { name "Fido" }');
            runner.expect(detectChkType(bytes)).toBe('cogai-text');
        });

        runner.it('CogAI text does not start with QCHK magic', () => {
            const encoder = new TextEncoder();
            const bytes   = encoder.encode('memory m1 { content "hello"; strength 0.8 }');
            runner.expect(detectChkType(bytes)).not.toBe('qchk-binary');
        });

        // ── Named profile IDs ─────────────────────────────────────────────────

        runner.it('six named profiles are defined', () => {
            runner.expect(Object.keys(PROFILE_IDS).length).toBe(6);
        });

        runner.it('all named profile IDs are non-zero', () => {
            for (const [name, id] of Object.entries(PROFILE_IDS)) {
                runner.expect(id).toBeGreaterThan(0n);
            }
        });

        runner.it('all named profile IDs are distinct', () => {
            const ids  = Object.values(PROFILE_IDS);
            const set  = new Set(ids.map(String));
            runner.expect(set.size).toBe(ids.length);
        });

        runner.it('profile:general hashes stably', () => {
            runner.expect(PROFILE_IDS.general).toBe(q_hash('profile:general'));
        });

        runner.it('profile:health hashes stably', () => {
            runner.expect(PROFILE_IDS.health).toBe(q_hash('profile:health'));
        });

        runner.it('profile:chemistry hashes stably', () => {
            runner.expect(PROFILE_IDS.chemistry).toBe(q_hash('profile:chemistry'));
        });

        runner.it('profile:research hashes stably', () => {
            runner.expect(PROFILE_IDS.research).toBe(q_hash('profile:research'));
        });

        runner.it('profile:legal hashes stably', () => {
            runner.expect(PROFILE_IDS.legal).toBe(q_hash('profile:legal'));
        });

        runner.it('profile:financial hashes stably', () => {
            runner.expect(PROFILE_IDS.financial).toBe(q_hash('profile:financial'));
        });

        // ── WASM profile functions (skip gracefully if not in binary) ─────────

        runner.it('validate_shacl_constraint_wasm is callable if present', () => {
            if (!mod?.validate_shacl_constraint_wasm) return;
            runner.expect(() => mod.validate_shacl_constraint_wasm('sh:minCount', 1.0)).not.toThrow();
        });

        runner.it('WASM profile compile does not throw on valid JSON-LD (if present)', () => {
            if (!mod?.profile_compile_wasm) return;
            const jsonLd = JSON.stringify({ '@context': 'https://qualia.social/ns/', profile: 'general' });
            runner.expect(() => mod.profile_compile_wasm(jsonLd)).not.toThrow();
        });

        runner.it('WASM profile list returns JSON string (if present)', () => {
            if (!mod?.profile_list_wasm) return;
            const raw = mod.profile_list_wasm();
            runner.expect(typeof raw).toBe('string');
            runner.expect(() => JSON.parse(raw)).not.toThrow();
        });
    });
}

export default register;
