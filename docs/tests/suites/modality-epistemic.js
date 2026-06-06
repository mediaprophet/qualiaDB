// Epistemic Logic modality tests.
// Mirrors crates/qualia-core-db/src/modalities/epistemic.rs exactly.
// Opcodes: OP_KNOWS=0x20, OP_BELIEVES=0x21, OP_COMMON_KNOWLEDGE=0x22

import { TestRunner } from '../test-runner.js';
import { q_hash, makeQuin } from './primitives.js';

// ─── Constants ────────────────────────────────────────────────────────────────

const OP_KNOWS            = 0x20n;
const OP_BELIEVES         = 0x21n;
const OP_COMMON_KNOWLEDGE = 0x22n;
const CERTAINTY_BIT_SHIFT = 8n;
const THRESHOLD_CERTAIN   = 128n;

// ─── JS implementation ────────────────────────────────────────────────────────

function buildEpistemicQuin(subject, opcode, certainty, claimFingerprint, world) {
    const predicate = opcode | (certainty << CERTAINTY_BIT_SHIFT);
    return makeQuin(subject, predicate, claimFingerprint, world, 0n,
        subject ^ predicate ^ claimFingerprint ^ world);
}

function evaluateEpistemicFrame(quins, agentDidHash, worldHash) {
    // Pass 1: collect common-knowledge fingerprints
    const ck = new Set();
    for (const q of quins) {
        const opcode = q.predicate & 0xFFn;
        if (opcode === OP_COMMON_KNOWLEDGE) ck.add(q.object);
    }

    const verdicts = [];
    for (const q of quins) {
        const opcode = q.predicate & 0xFFn;
        if (opcode !== OP_KNOWS && opcode !== OP_BELIEVES && opcode !== OP_COMMON_KNOWLEDGE) continue;

        if (agentDidHash !== 0n && q.subject !== agentDidHash && opcode !== OP_COMMON_KNOWLEDGE) continue;
        if (worldHash   !== 0n && q.context !== worldHash) continue;

        let certainty = Number((q.predicate >> CERTAINTY_BIT_SHIFT) & 0xFFn);
        if (ck.has(q.object)) certainty = 255;

        const status = (opcode === OP_BELIEVES && certainty < Number(THRESHOLD_CERTAIN))
            ? 'Uncertain' : 'Active';

        verdicts.push({ claim: q, status, certainty });
    }
    return verdicts;
}

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    runner.describe('Modality: Epistemic Logic', () => {

        runner.it('single agent KNOWS → status Active', () => {
            const agent = q_hash('agent1'), claim = q_hash('claim1');
            const q = buildEpistemicQuin(agent, OP_KNOWS, 200n, claim, 0n);
            const v = evaluateEpistemicFrame([q], agent, 0n);
            runner.expect(v.length).toBe(1);
            runner.expect(v[0].status).toBe('Active');
        });

        runner.it('BELIEVES below threshold → status Uncertain', () => {
            const agent = q_hash('agent1'), claim = q_hash('claim1');
            const q = buildEpistemicQuin(agent, OP_BELIEVES, 50n, claim, 0n);
            const v = evaluateEpistemicFrame([q], agent, 0n);
            runner.expect(v[0].status).toBe('Uncertain');
        });

        runner.it('BELIEVES at threshold → status Active', () => {
            const agent = q_hash('agent1'), claim = q_hash('claim1');
            const q = buildEpistemicQuin(agent, OP_BELIEVES, THRESHOLD_CERTAIN, claim, 0n);
            const v = evaluateEpistemicFrame([q], agent, 0n);
            runner.expect(v[0].status).toBe('Active');
        });

        runner.it('COMMON_KNOWLEDGE promotes belief certainty to 255', () => {
            const agent1 = q_hash('agent1'), claim = q_hash('claim1');
            const ck = buildEpistemicQuin(0n, OP_COMMON_KNOWLEDGE, 255n, claim, 0n);
            const b  = buildEpistemicQuin(agent1, OP_BELIEVES, 50n, claim, 0n);
            const v  = evaluateEpistemicFrame([ck, b], 0n, 0n);
            runner.expect(v.length).toBe(2);
            runner.expect(v[1].status).toBe('Active');
            runner.expect(v[1].certainty).toBe(255);
        });

        runner.it('world hash mismatch excludes quin', () => {
            const agent = q_hash('agent1'), world = q_hash('world1'), claim = q_hash('claim1');
            const q = buildEpistemicQuin(agent, OP_KNOWS, 200n, claim, world);
            const v = evaluateEpistemicFrame([q], 0n, q_hash('world2'));
            runner.expect(v.length).toBe(0);
        });

        runner.it('wrong agent hash excludes quin', () => {
            const agent1 = q_hash('agent1'), agent2 = q_hash('agent2');
            const claim  = q_hash('claim1');
            const q = buildEpistemicQuin(agent1, OP_KNOWS, 200n, claim, 0n);
            const v = evaluateEpistemicFrame([q], agent2, 0n);
            runner.expect(v.length).toBe(0);
        });

        runner.it('agent hash 0 matches all agents', () => {
            const agent1 = q_hash('agent1'), agent2 = q_hash('agent2');
            const claim  = q_hash('claim1');
            const q1 = buildEpistemicQuin(agent1, OP_KNOWS, 200n, claim, 0n);
            const q2 = buildEpistemicQuin(agent2, OP_KNOWS, 200n, claim, 0n);
            const v  = evaluateEpistemicFrame([q1, q2], 0n, 0n);
            runner.expect(v.length).toBe(2);
        });

        runner.it('empty input → zero verdicts', () => {
            runner.expect(evaluateEpistemicFrame([], 0n, 0n).length).toBe(0);
        });

        runner.it('COMMON_KNOWLEDGE quin with agent-hash=0 still included', () => {
            const claim = q_hash('shared-claim');
            const ck = buildEpistemicQuin(0n, OP_COMMON_KNOWLEDGE, 255n, claim, 0n);
            const v  = evaluateEpistemicFrame([ck], q_hash('some-agent'), 0n);
            runner.expect(v.length).toBe(1);
            runner.expect(v[0].status).toBe('Active');
        });

        runner.it('non-epistemic opcode is ignored', () => {
            const agent = q_hash('agent1'), claim = q_hash('claim1');
            const q = makeQuin(agent, 0x99n, claim, 0n);
            const v = evaluateEpistemicFrame([q], agent, 0n);
            runner.expect(v.length).toBe(0);
        });

        runner.it('opcode OP_KNOWS = 0x20', () => { runner.expect(Number(OP_KNOWS)).toBe(0x20); });
        runner.it('opcode OP_BELIEVES = 0x21', () => { runner.expect(Number(OP_BELIEVES)).toBe(0x21); });
        runner.it('opcode OP_COMMON_KNOWLEDGE = 0x22', () => { runner.expect(Number(OP_COMMON_KNOWLEDGE)).toBe(0x22); });
    });
}

export default register;
