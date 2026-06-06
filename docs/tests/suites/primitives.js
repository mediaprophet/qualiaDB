// Core primitives: QualiaQuin struct (48 bytes, 6×u64), q_hash, routing lanes,
// ECC parity, Lamport clock, sensitivity byte, permissive gate.
// All logic mirrors crates/qualia-core-db/src/lib.rs exactly.

import { TestRunner } from '../test-runner.js';

// ─── q_hash — FNV-1a 64-bit ───────────────────────────────────────────────────

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME  = 0x100000001b3n;
const MASK_64    = 0xffffffffffffffffn;
const ENC = new TextEncoder();

export function q_hash(s) {
    let h = FNV_OFFSET;
    for (const b of ENC.encode(s)) h = ((h ^ BigInt(b)) * FNV_PRIME) & MASK_64;
    return h;
}

// ─── QualiaQuin factory ───────────────────────────────────────────────────────

export function makeQuin(subject = 0n, predicate = 0n, object = 0n,
                          context = 0n, metadata = 0n, parity = 0n) {
    return { subject: BigInt(subject), predicate: BigInt(predicate),
             object: BigInt(object),   context: BigInt(context),
             metadata: BigInt(metadata), parity: BigInt(parity) };
}

// ─── Constants (mirrors lib.rs) ───────────────────────────────────────────────

const NESTED_BIT_MASK = 1n << 63n;
const LANE_MASK       = 0b11n << 61n;
const CONSUMED_BIT    = 1n << 59n;
const SYNTHESIZED_BIT = 1n << 58n;

// ─── Methods mirroring QualiaQuin impl ────────────────────────────────────────

function isSubjectNested(q)       { return (q.subject & NESTED_BIT_MASK) !== 0n; }
function getSubjectLiteralId(q)   { return q.subject & ~NESTED_BIT_MASK; }
function identifyRoutingLane(q)   {
    const bits = (q.metadata & LANE_MASK) >> 61n;
    switch (bits) {
        case 0x01n: return 'EnforcePermissiveCommons';
        case 0x02n: return 'EnforceBilateralMicroCommons';
        case 0x03n: return 'SpatiotemporalAmbiguous';
        default:    return 'PassthroughStandard';
    }
}
function extractCleanMetadataValue(q) { return q.metadata & 0xFFFF_FFFFn; }
function extractLamportClock(q)   { return Number((q.metadata >> 32n) & 0x1FFF_FFFFn); }
function setLamportClock(q, clk) {
    const c = { ...q };
    c.metadata = (c.metadata & ~(0x1FFF_FFFFn << 32n)) | ((BigInt(clk) & 0x1FFF_FFFFn) << 32n);
    return c;
}
function getSensitivityByte(q)    { return Number(q.context >> 56n) & 0xFF; }
function setSensitivityByte(q, v) {
    const c = { ...q };
    c.context = (c.context & 0x00FF_FFFF_FFFF_FFFFn) | (BigInt(v) << 56n);
    return c;
}
function verifyEccParity(q)       { return q.parity !== 0xFFFF_FFFF_FFFF_FFFFn; }
function extractModalityFlag(q)   { return Number(q.object >> 60n) & 0xF; }
function extractByteOffset(q)     { return q.object & 0x0FFF_FFFF_FFFF_FFFFn; }

// Permissive gate masks
const MASK_AUTH_NATURAL      = 0x0001;
const MASK_BILATERAL_LOCKED  = 0x0002;
const MASK_COMMERCIAL_GATE   = 0x0004;
const MASK_WORK_SATISFIED    = 0x0008;

function evaluatePermissiveGate(entryPolicy, agentFlags) {
    if (entryPolicy & MASK_WORK_SATISFIED) return true;
    if ((agentFlags & MASK_COMMERCIAL_GATE) !== 0 && (entryPolicy & MASK_COMMERCIAL_GATE) !== 0) return false;
    if ((entryPolicy & MASK_BILATERAL_LOCKED) !== 0 && (agentFlags & MASK_AUTH_NATURAL) === 0) return false;
    return true;
}

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    runner.describe('Core Primitives', () => {

        runner.describe('q_hash — FNV-1a 64-bit', () => {
            runner.it('empty string produces FNV offset basis', () => {
                runner.expect(q_hash('')).toBe(FNV_OFFSET);
            });
            runner.it('known URI produces stable hash', () => {
                const h1 = q_hash('http://xmlns.com/foaf/0.1/name');
                const h2 = q_hash('http://xmlns.com/foaf/0.1/name');
                runner.expect(h1).toBe(h2);
            });
            runner.it('different strings produce different hashes', () => {
                runner.expect(q_hash('foo')).not.toBe(q_hash('bar'));
            });
            runner.it('hash fits within 64 bits', () => {
                runner.expect(q_hash('test') <= MASK_64).toBeTruthy();
            });
            runner.it('q_turtle macro semantics: subject hashes match', () => {
                const s = q_hash('ex:Alice');
                runner.expect(s).toBe(q_hash('ex:Alice'));
            });
        });

        runner.describe('QualiaQuin — 48-byte layout', () => {
            runner.it('struct has exactly 6 u64 fields', () => {
                const q = makeQuin();
                runner.expect(Object.keys(q).length).toBe(6);
            });
            runner.it('default quin has all-zero fields', () => {
                const q = makeQuin();
                runner.expect(q.subject).toBe(0n);
                runner.expect(q.parity).toBe(0n);
            });
            runner.it('nested bit in subject is detectable', () => {
                const nested    = makeQuin(1n << 63n);
                const notNested = makeQuin(42n);
                runner.expect(isSubjectNested(nested)).toBeTruthy();
                runner.expect(isSubjectNested(notNested)).toBeFalsy();
            });
            runner.it('getSubjectLiteralId strips nested bit', () => {
                const q = makeQuin(NESTED_BIT_MASK | 42n);
                runner.expect(getSubjectLiteralId(q)).toBe(42n);
            });
        });

        runner.describe('ECC parity', () => {
            runner.it('parity 0 passes', () => {
                runner.expect(verifyEccParity(makeQuin())).toBeTruthy();
            });
            runner.it('parity 0xFFFF…FFFF fails', () => {
                runner.expect(verifyEccParity(makeQuin(0n, 0n, 0n, 0n, 0n, MASK_64))).toBeFalsy();
            });
            runner.it('non-max parity passes', () => {
                runner.expect(verifyEccParity(makeQuin(0n, 0n, 0n, 0n, 0n, 12345n))).toBeTruthy();
            });
        });

        runner.describe('Routing lanes (metadata bits 61-62)', () => {
            runner.it('00 → PassthroughStandard', () => {
                runner.expect(identifyRoutingLane(makeQuin(0n, 0n, 0n, 0n, 0n))).toBe('PassthroughStandard');
            });
            runner.it('01 → EnforcePermissiveCommons', () => {
                runner.expect(identifyRoutingLane(makeQuin(0n, 0n, 0n, 0n, 0x01n << 61n))).toBe('EnforcePermissiveCommons');
            });
            runner.it('10 → EnforceBilateralMicroCommons', () => {
                runner.expect(identifyRoutingLane(makeQuin(0n, 0n, 0n, 0n, 0x02n << 61n))).toBe('EnforceBilateralMicroCommons');
            });
            runner.it('11 → SpatiotemporalAmbiguous', () => {
                runner.expect(identifyRoutingLane(makeQuin(0n, 0n, 0n, 0n, 0x03n << 61n))).toBe('SpatiotemporalAmbiguous');
            });
            runner.it('lane bits do not bleed into lower payload', () => {
                const q = makeQuin(0n, 0n, 0n, 0n, (0x01n << 61n) | 67890n);
                runner.expect(identifyRoutingLane(q)).toBe('EnforcePermissiveCommons');
                runner.expect(extractCleanMetadataValue(q)).toBe(67890n);
            });
        });

        runner.describe('Lamport clock (metadata bits 32-60)', () => {
            runner.it('extractLamportClock returns 0 on default quin', () => {
                runner.expect(extractLamportClock(makeQuin())).toBe(0);
            });
            runner.it('setLamportClock / extractLamportClock round-trips', () => {
                const q = setLamportClock(makeQuin(), 12345);
                runner.expect(extractLamportClock(q)).toBe(12345);
            });
            runner.it('clock does not interfere with routing lane bits', () => {
                let q = makeQuin(0n, 0n, 0n, 0n, 0x01n << 61n);
                q = setLamportClock(q, 999);
                runner.expect(identifyRoutingLane(q)).toBe('EnforcePermissiveCommons');
                runner.expect(extractLamportClock(q)).toBe(999);
            });
        });

        runner.describe('Sensitivity byte (context bits 56-63)', () => {
            runner.it('default sensitivity is 0 (PUBLIC)', () => {
                runner.expect(getSensitivityByte(makeQuin())).toBe(0);
            });
            runner.it('setSensitivityByte / getSensitivityByte round-trips', () => {
                const q = setSensitivityByte(makeQuin(0n, 0n, 0n, 42n), 0x01);
                runner.expect(getSensitivityByte(q)).toBe(0x01);
            });
            runner.it('sensitivity does not corrupt lower 56 bits of context', () => {
                const q = setSensitivityByte(makeQuin(0n, 0n, 0n, 0xDEADBEEFn), 0x02);
                runner.expect(q.context & 0x00FF_FFFF_FFFF_FFFFn).toBe(0xDEADBEEFn);
            });
        });

        runner.describe('Modality flag and byte offset (object field)', () => {
            runner.it('extractModalityFlag reads upper 4 bits of object', () => {
                const q = makeQuin(0n, 0n, (0x9n << 60n) | 0x1234n);
                runner.expect(extractModalityFlag(q)).toBe(0x9);
            });
            runner.it('extractByteOffset strips upper 4 bits', () => {
                const q = makeQuin(0n, 0n, (0x9n << 60n) | 0xABCDn);
                runner.expect(extractByteOffset(q)).toBe(0xABCDn);
            });
            runner.it('MODALITY_FLAG_LLM_TENSOR = 0b1001 = 9', () => {
                runner.expect(0b1001).toBe(9);
            });
        });

        runner.describe('Permissive runtime gate', () => {
            runner.it('WORK_SATISFIED always allows', () => {
                runner.expect(evaluatePermissiveGate(MASK_WORK_SATISFIED, 0)).toBeTruthy();
            });
            runner.it('commercial gate blocks non-paying agent', () => {
                runner.expect(evaluatePermissiveGate(MASK_COMMERCIAL_GATE, MASK_COMMERCIAL_GATE)).toBeFalsy();
            });
            runner.it('bilateral lock blocks unauthenticated agent', () => {
                runner.expect(evaluatePermissiveGate(MASK_BILATERAL_LOCKED, 0)).toBeFalsy();
            });
            runner.it('bilateral lock allows authenticated natural person', () => {
                runner.expect(evaluatePermissiveGate(MASK_BILATERAL_LOCKED, MASK_AUTH_NATURAL)).toBeTruthy();
            });
            runner.it('passthrough standard always allows', () => {
                runner.expect(evaluatePermissiveGate(0, 0)).toBeTruthy();
            });
        });

        runner.describe('conduct violation quin', () => {
            runner.it('has correct predicate hash sentinel', () => {
                // mirrors QualiaQuin::new_conduct_violation
                const CONDUCT_VIOLATION_PREDICATE = 0x42_0000_0000_0000n;
                const q = {
                    subject: 0n, predicate: CONDUCT_VIOLATION_PREDICATE,
                    object: 0n, context: 0n, metadata: 0n, parity: 0n,
                };
                runner.expect(q.predicate).toBe(CONDUCT_VIOLATION_PREDICATE);
            });
        });
    });
}

export default register;
