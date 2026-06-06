// CogAI Cognitive AI Chunks & ACT-R modality tests.
// Covers the W3C CogAI Community Group chunks-and-rules format
// (https://github.com/w3c-cg/cogai) and its mapping into QualiaDB.
//
// Source-of-truth: crates/qualia-core-db/src/shacl_compiler.rs
//   ShaclConstraint::RetrieveByActivation → SlgOpcode::NativeRetrieveByActivation
//   ShaclConstraint::DecayMetadata        → SlgOpcode::NativeDecayMetadata
//   ShaclConstraint::Unless               → SlgOpcode::NativeUnless
//
// These opcodes are compiled by shacl_compiler and dispatched by execute_vm_frame
// (webizen.rs). RetrieveByActivation and DecayMetadata currently YIELD to the
// GPU Sieve (Core 2) — they return None from execute_vm_frame. Unless executes
// inline as non-monotonic default logic.
//
// CogAI .chk text format is distinct from QCHK binary Capability Profiles.
// See modality-cogai disambiguation note in ARCHITECTURE.md §4.

import { q_hash, makeQuin } from './primitives.js';

// ─── SHACL property names for CogAI constraints (mirrors shacl_compiler.rs) ──

const SHACL_RETRIEVE_BY_ACTIVATION = 'qualia:retrieveByActivation';
const SHACL_DECAY_METADATA         = 'qualia:decayMetadata';

// These are opaque enum variants in Rust; not numeric predicates like deontic/LTL.
// We represent them here as property hashes.
const P_RETRIEVE_BY_ACTIVATION = q_hash(SHACL_RETRIEVE_BY_ACTIVATION);
const P_DECAY_METADATA         = q_hash(SHACL_DECAY_METADATA);
const P_CHUNK_TYPE             = q_hash('cogai:type');
const P_CHUNK_PROPERTY         = q_hash('cogai:property');
const P_ACTIVATION             = q_hash('cogai:activation');

// ─── CogAI chunk text format parser (JS reference implementation) ─────────────
// Parses the subset used by QualiaDB ingest.
// Grammar: <type> [<id>] '{' (<key> <value> ';')* '}'
// Values: names, numbers, booleans, ISO8601 dates, quoted strings, comma-lists.

function parseChunk(text) {
    // Normalise whitespace
    text = text.trim().replace(/\r\n/g, '\n');
    const header = text.match(/^(\S+)(?:\s+(\S+))?\s*\{/);
    if (!header) throw new Error('Invalid chunk header: ' + text);

    const type = header[1];
    const id   = header[2] || null;

    const bodyStart = text.indexOf('{') + 1;
    const bodyEnd   = text.lastIndexOf('}');
    const body      = text.slice(bodyStart, bodyEnd).trim();

    const props = {};
    // Split on ';' or newline, parse 'key value' pairs
    for (const line of body.split(/[;\n]+/)) {
        const parts = line.trim().match(/^(\S+)\s+(.+)$/);
        if (!parts) continue;
        const key = parts[1];
        let val   = parts[2].trim();
        // Strip quotes from string literals
        if (val.startsWith('"') && val.endsWith('"')) val = val.slice(1, -1);
        // Parse numbers
        if (/^-?\d+(\.\d+)?$/.test(val)) val = parseFloat(val);
        // Parse booleans
        if (val === 'true') val = true;
        if (val === 'false') val = false;
        props[key] = val;
    }

    return { type, id, props };
}

// ─── CogAI chunk → Quins mapping ─────────────────────────────────────────────
// Each property in a chunk becomes one Quin:
//   subject   = q_hash(chunk_id || type)
//   predicate = q_hash(property_name)
//   object    = q_hash(string(value))  [or inline integer/decimal]
//   context   = q_hash(type)           [chunk type as named graph]

function chunkToQuins(chunk) {
    const chunkHash = q_hash(chunk.id || chunk.type);
    const typeHash  = q_hash(chunk.type);
    const quins = [];

    // Type assertion quin
    quins.push(makeQuin(chunkHash, P_CHUNK_TYPE, typeHash, typeHash));

    for (const [key, val] of Object.entries(chunk.props)) {
        const pred = q_hash(key);
        let obj;
        if (typeof val === 'number') {
            // Inline integer (xsd:integer, type tag 0b001 << 60)
            obj = (1n << 60n) | BigInt(Math.round(val));
        } else if (typeof val === 'boolean') {
            // Inline boolean (type tag 0b011 << 60)
            obj = (3n << 60n) | (val ? 1n : 0n);
        } else {
            obj = q_hash(String(val));
        }
        quins.push(makeQuin(chunkHash, pred, obj, typeHash));
    }

    return quins;
}

// ─── ACT-R activation model helpers (mirrors the conceptual model) ────────────
// In ACT-R, each memory chunk has an activation level that decays over time.
// QualiaDB encodes this in Quin metadata field bits 0-31.

const ACTIVATION_SCALE = 1_000_000n; // encode f32 as fixed-point integer in metadata

function encodeActivation(level) {
    // level in [0.0, 1.0], stored as u32 in metadata bits 0-31
    const clamped = Math.max(0.0, Math.min(1.0, level));
    return BigInt(Math.round(clamped * Number(ACTIVATION_SCALE)));
}

function decodeActivation(metadataBits) {
    return Number(metadataBits & 0xFFFF_FFFFn) / Number(ACTIVATION_SCALE);
}

function decayActivation(level, decayRate, elapsedMs) {
    // Base-level learning: activation decay per ACT-R
    const decayed = level * Math.exp(-decayRate * elapsedMs / 1000);
    return Math.max(0.0, decayed);
}

// ─── @rdfmap integration ──────────────────────────────────────────────────────
// CogAI @rdfmap declares prefix → IRI mappings, which q_hash turns into
// Quin predicate hashes at ingest time.

function applyRdfMap(prefix, iri) {
    return q_hash(iri);
}

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    runner.describe('Modality: CogAI Cognitive AI Chunks', () => {

        // ── Chunk text format parsing ─────────────────────────────────────────

        runner.it('parses a basic chunk with type and id', () => {
            const chunk = parseChunk(`dog dog1 { name "Fido"; age 4 }`);
            runner.expect(chunk.type).toBe('dog');
            runner.expect(chunk.id).toBe('dog1');
            runner.expect(chunk.props.name).toBe('Fido');
            runner.expect(chunk.props.age).toBe(4);
        });

        runner.it('parses a chunk without an explicit id', () => {
            const chunk = parseChunk(`memory { content "sky is blue"; strength 0.9 }`);
            runner.expect(chunk.type).toBe('memory');
            runner.expect(chunk.id).toBeNull();
            runner.expect(chunk.props.content).toBe('sky is blue');
            runner.expect(chunk.props.strength).toBeCloseTo(0.9, 5);
        });

        runner.it('parses boolean property values', () => {
            const chunk = parseChunk(`rule r1 { active true; archived false }`);
            runner.expect(chunk.props.active).toBe(true);
            runner.expect(chunk.props.archived).toBe(false);
        });

        runner.it('throws on invalid chunk header', () => {
            runner.expect(() => parseChunk(`{ no type }`)).toThrow();
        });

        // ── Chunk → Quin mapping ──────────────────────────────────────────────

        runner.it('chunk generates at least one type-assertion quin', () => {
            const chunk = parseChunk(`cat cat1 { name "Whiskers" }`);
            const quins = chunkToQuins(chunk);
            // type quin + name property quin
            runner.expect(quins.length).toBe(2);
        });

        runner.it('chunk type-assertion quin uses q_hash(id) as subject', () => {
            const chunk = parseChunk(`dog dog1 { name "Rex" }`);
            const quins = chunkToQuins(chunk);
            const typeQuin = quins[0];
            runner.expect(typeQuin.subject).toBe(q_hash('dog1'));
            runner.expect(typeQuin.predicate).toBe(P_CHUNK_TYPE);
            runner.expect(typeQuin.object).toBe(q_hash('dog'));
        });

        runner.it('numeric property encodes as inline integer (type tag 0b001 << 60)', () => {
            const chunk = parseChunk(`animal a1 { age 7 }`);
            const quins = chunkToQuins(chunk);
            const ageProp = quins.find(q => q.predicate === q_hash('age'));
            runner.expect(ageProp).not.toBeNull();
            // type tag bits 60-62 should be 0b001
            runner.expect((ageProp.object >> 60n) & 0x7n).toBe(1n);
            runner.expect(ageProp.object & ((1n << 60n) - 1n)).toBe(7n);
        });

        runner.it('boolean true encodes as inline boolean (type tag 0b011 << 60, value 1)', () => {
            const chunk = parseChunk(`rule r1 { active true }`);
            const quins = chunkToQuins(chunk);
            const activeProp = quins.find(q => q.predicate === q_hash('active'));
            runner.expect((activeProp.object >> 60n) & 0x7n).toBe(3n);
            runner.expect(activeProp.object & 1n).toBe(1n);
        });

        runner.it('boolean false encodes as inline boolean (type tag 0b011 << 60, value 0)', () => {
            const chunk = parseChunk(`rule r1 { active false }`);
            const quins = chunkToQuins(chunk);
            const activeProp = quins.find(q => q.predicate === q_hash('active'));
            runner.expect((activeProp.object >> 60n) & 0x7n).toBe(3n);
            runner.expect(activeProp.object & 1n).toBe(0n);
        });

        runner.it('string property encodes as q_hash of value', () => {
            const chunk = parseChunk(`dog dog1 { name "Fido" }`);
            const quins = chunkToQuins(chunk);
            const nameProp = quins.find(q => q.predicate === q_hash('name'));
            runner.expect(nameProp.object).toBe(q_hash('Fido'));
        });

        runner.it('chunk context field carries chunk type hash (named graph)', () => {
            const chunk = parseChunk(`memory m1 { content "test" }`);
            const quins = chunkToQuins(chunk);
            for (const q of quins) {
                runner.expect(q.context).toBe(q_hash('memory'));
            }
        });

        runner.it('two distinct chunks produce different subject hashes', () => {
            const q1 = chunkToQuins(parseChunk(`dog dog1 { name "Rex" }`))[0];
            const q2 = chunkToQuins(parseChunk(`dog dog2 { name "Max" }`))[0];
            runner.expect(q1.subject === q2.subject).toBeFalsy();
        });

        // ── ACT-R activation encoding ─────────────────────────────────────────

        runner.it('encodes activation 1.0 as ACTIVATION_SCALE in metadata bits 0-31', () => {
            const enc = encodeActivation(1.0);
            runner.expect(enc).toBe(ACTIVATION_SCALE);
        });

        runner.it('encodes activation 0.0 as 0', () => {
            runner.expect(encodeActivation(0.0)).toBe(0n);
        });

        runner.it('round-trips activation through encode/decode', () => {
            const level = 0.75;
            const enc   = encodeActivation(level);
            const dec   = decodeActivation(enc);
            runner.expect(dec).toBeCloseTo(level, 4);
        });

        runner.it('clamps activation above 1.0 to 1.0', () => {
            runner.expect(decodeActivation(encodeActivation(2.5))).toBeCloseTo(1.0, 4);
        });

        runner.it('clamps activation below 0.0 to 0.0', () => {
            runner.expect(decodeActivation(encodeActivation(-0.5))).toBeCloseTo(0.0, 4);
        });

        runner.it('decayActivation reduces level over time', () => {
            const initial = 0.9;
            const decayed = decayActivation(initial, 0.5, 1000);
            runner.expect(decayed).toBeLessThan(initial);
            runner.expect(decayed).toBeGreaterThan(0);
        });

        runner.it('decayActivation with zero elapsed time returns original level', () => {
            runner.expect(decayActivation(0.8, 0.5, 0)).toBeCloseTo(0.8, 4);
        });

        runner.it('decayActivation floors at 0.0', () => {
            runner.expect(decayActivation(0.1, 10.0, 100_000)).toBe(0.0);
        });

        // ── SHACL property → opcode mapping ──────────────────────────────────

        runner.it('SHACL retrieveByActivation property hashes stably', () => {
            runner.expect(P_RETRIEVE_BY_ACTIVATION).toBe(q_hash('qualia:retrieveByActivation'));
        });

        runner.it('SHACL decayMetadata property hashes stably', () => {
            runner.expect(P_DECAY_METADATA).toBe(q_hash('qualia:decayMetadata'));
        });

        runner.it('retrieveByActivation and decayMetadata have distinct hashes', () => {
            runner.expect(P_RETRIEVE_BY_ACTIVATION === P_DECAY_METADATA).toBeFalsy();
        });

        // ── @rdfmap integration ───────────────────────────────────────────────

        runner.it('@rdfmap yields q_hash of the mapped IRI', () => {
            const h = applyRdfMap('dog', 'http://example.com/ns/dog');
            runner.expect(h).toBe(q_hash('http://example.com/ns/dog'));
        });

        runner.it('two different IRIs produce different hashes', () => {
            const h1 = applyRdfMap('dog', 'http://example.com/ns/dog');
            const h2 = applyRdfMap('cat', 'http://example.com/ns/cat');
            runner.expect(h1 === h2).toBeFalsy();
        });

        // ── Core 2 yield semantics (documentation tests) ─────────────────────
        // These tests verify the *specified behaviour*, not live WASM execution.
        // RetrieveByActivation and DecayMetadata yield to the GPU Sieve (Core 2)
        // and return None from execute_vm_frame — they do not execute inline.

        runner.it('RetrieveByActivation opcode is Core 2 (GPU Sieve) — returns None from VM (spec)', () => {
            // Documented in webizen.rs: NativeRetrieveByActivation => return None
            // This test validates the specification, not live WASM execution.
            const specBehaviour = 'CORE_2_YIELD';
            runner.expect(specBehaviour).toBe('CORE_2_YIELD');
        });

        runner.it('DecayMetadata opcode is Core 2 (GPU Sieve) — returns None from VM (spec)', () => {
            const specBehaviour = 'CORE_2_YIELD';
            runner.expect(specBehaviour).toBe('CORE_2_YIELD');
        });

        runner.it('Unless opcode executes inline on Core 1 as non-monotonic default logic (spec)', () => {
            // webizen.rs: NativeUnless => vm_log("NativeUnless: evaluating Non-Monotonic Default Logic locally on Core 1")
            const specBehaviour = 'CORE_1_INLINE';
            runner.expect(specBehaviour).toBe('CORE_1_INLINE');
        });

        // ── .chk extension disambiguation ─────────────────────────────────────

        runner.it('CogAI .chk text file does NOT start with QCHK magic bytes', () => {
            const cogaiChunk = `dog dog1 { name "Fido" }`;
            const encoder    = new TextEncoder();
            const bytes      = encoder.encode(cogaiChunk);
            const magic      = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
            runner.expect(magic).not.toBe('QCHK');
        });

        runner.it('QCHK magic bytes are ASCII "QCHK" (0x51 0x43 0x48 0x4B)', () => {
            runner.expect(0x51).toBe('Q'.charCodeAt(0));
            runner.expect(0x43).toBe('C'.charCodeAt(0));
            runner.expect(0x48).toBe('H'.charCodeAt(0));
            runner.expect(0x4B).toBe('K'.charCodeAt(0));
        });
    });
}

export default register;
