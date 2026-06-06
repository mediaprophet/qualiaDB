// Paraconsistent Logic modality tests.
// Mirrors crates/qualia-core-db/src/modalities/paraconsistent.rs exactly.
// Opcodes: OP_ISOLATE=0x30, OP_CONTRADICTION_SCORE=0x31, OP_PARACONSISTENT_MERGE=0x32

import { TestRunner } from '../test-runner.js';
import { q_hash, makeQuin } from './primitives.js';

// ─── Constants ────────────────────────────────────────────────────────────────

const OP_ISOLATE              = 0x30;
const OP_CONTRADICTION_SCORE  = 0x31;
const OP_PARACONSISTENT_MERGE = 0x32;
const ISOLATED_CONTEXT_PREFIX = q_hash('q42:isolated');

// ─── JS implementation ────────────────────────────────────────────────────────

function routeParaconsistent(quins) {
    const consistent = [];
    const isolated   = [];

    for (const quin of quins) {
        let isContradiction = false;
        for (const c of consistent) {
            if (c.context   === quin.context &&
                c.subject   === quin.subject &&
                c.predicate === quin.predicate &&
                c.object    !== quin.object) {
                isContradiction = true;
                break;
            }
        }
        if (isContradiction) {
            const iso = { ...quin };
            if (quin.context !== ISOLATED_CONTEXT_PREFIX) {
                iso.context = quin.context ^ ISOLATED_CONTEXT_PREFIX;
            }
            isolated.push(iso);
        } else {
            consistent.push(quin);
        }
    }
    return { consistent, isolated };
}

function dummyQuin(subject, predicate, object, context) {
    return makeQuin(BigInt(subject), BigInt(predicate), BigInt(object), BigInt(context));
}

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    runner.describe('Modality: Paraconsistent Logic', () => {

        runner.it('no contradictions — all pass to consistent', () => {
            const q1 = dummyQuin(1, 1, 1, 100);
            const q2 = dummyQuin(2, 2, 2, 100);
            const { consistent, isolated } = routeParaconsistent([q1, q2]);
            runner.expect(consistent.length).toBe(2);
            runner.expect(isolated.length).toBe(0);
        });

        runner.it('same s/p/c but different object → contradiction', () => {
            const q1 = dummyQuin(1, 1, 1, 100);
            const q2 = dummyQuin(1, 1, 2, 100);
            const { consistent, isolated } = routeParaconsistent([q1, q2]);
            runner.expect(consistent.length).toBe(1);
            runner.expect(isolated.length).toBe(1);
            runner.expect(consistent[0].object).toBe(1n);
            runner.expect(isolated[0].object).toBe(2n);
        });

        runner.it('isolated quin has XOR-ed isolation context', () => {
            const q1 = dummyQuin(1, 1, 1, 100);
            const q2 = dummyQuin(1, 1, 2, 100);
            const { isolated } = routeParaconsistent([q1, q2]);
            runner.expect(isolated[0].context).toBe(100n ^ ISOLATED_CONTEXT_PREFIX);
        });

        runner.it('already-isolated context passes through unchanged', () => {
            const q1 = makeQuin(1n, 1n, 1n, ISOLATED_CONTEXT_PREFIX);
            const { consistent, isolated } = routeParaconsistent([q1]);
            runner.expect(consistent.length).toBe(1);
            runner.expect(isolated.length).toBe(0);
            runner.expect(consistent[0].context).toBe(ISOLATED_CONTEXT_PREFIX);
        });

        runner.it('three quins — one contradiction + one normal', () => {
            const q1 = dummyQuin(1, 1, 1, 100);
            const q2 = dummyQuin(1, 1, 2, 100); // contradicts q1
            const q3 = dummyQuin(2, 2, 2, 100); // normal
            const { consistent, isolated } = routeParaconsistent([q1, q2, q3]);
            runner.expect(consistent.length).toBe(2);
            runner.expect(isolated.length).toBe(1);
        });

        runner.it('two independent contradictions are deterministic', () => {
            const q1 = dummyQuin(1, 1, 1, 100);
            const q2 = dummyQuin(1, 1, 2, 100);
            const q3 = dummyQuin(5, 5, 5, 100);
            const q4 = dummyQuin(5, 5, 6, 100);
            const { isolated } = routeParaconsistent([q1, q2, q3, q4]);
            runner.expect(isolated.length).toBe(2);
            runner.expect(isolated[0].context).toBe(isolated[1].context);
        });

        runner.it('different context prevents contradiction detection', () => {
            const q1 = dummyQuin(1, 1, 1, 100);
            const q2 = dummyQuin(1, 1, 2, 200); // same s/p/o≠ but different context
            const { consistent, isolated } = routeParaconsistent([q1, q2]);
            runner.expect(consistent.length).toBe(2);
            runner.expect(isolated.length).toBe(0);
        });

        runner.it('ISOLATED_CONTEXT_PREFIX is the FNV hash of "q42:isolated"', () => {
            runner.expect(typeof ISOLATED_CONTEXT_PREFIX).toBe('bigint');
            runner.expect(ISOLATED_CONTEXT_PREFIX).toBe(q_hash('q42:isolated'));
        });

        runner.it('opcode constants', () => {
            runner.expect(OP_ISOLATE).toBe(0x30);
            runner.expect(OP_CONTRADICTION_SCORE).toBe(0x31);
            runner.expect(OP_PARACONSISTENT_MERGE).toBe(0x32);
        });
    });
}

export default register;
