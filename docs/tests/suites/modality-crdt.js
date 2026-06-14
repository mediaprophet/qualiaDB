// LWW CRDT — mirrors crdt.rs LwwRegister + temporal expiry (pure JS).

export function register(runner) {
    // Last-Writer-Wins register: (subject, predicate, object, timestamp)
    // Mirrors LwwEntry { subject, predicate, object, timestamp_ms, tombstone }
    function lwwMerge(entries) {
        const map = new Map();
        for (const e of entries) {
            const key = `${e.subject}:${e.predicate}`;
            const existing = map.get(key);
            if (!existing || e.timestamp_ms > existing.timestamp_ms) {
                map.set(key, { ...e });
            }
        }
        // Filter tombstoned entries
        const result = [];
        for (const v of map.values()) {
            if (!v.tombstone) result.push(v);
        }
        return result;
    }

    function lwwResolve(a, b) {
        // Higher timestamp wins; on tie, higher object hash wins (deterministic)
        if (a.timestamp_ms !== b.timestamp_ms) return a.timestamp_ms > b.timestamp_ms ? a : b;
        return a.object > b.object ? a : b;
    }

    runner.describe('Modality: LWW CRDT', () => {

        runner.describe('LwwRegister merge semantics', () => {

            runner.it('single entry survives merge', () => {
                const entries = [{ subject: 1n, predicate: 2n, object: 3n, timestamp_ms: 1000, tombstone: false }];
                runner.expect(lwwMerge(entries).length).toBe(1);
            });

            runner.it('later timestamp wins for same (subject, predicate) key', () => {
                const early = { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 1000, tombstone: false };
                const late  = { subject: 1n, predicate: 2n, object: 20n, timestamp_ms: 2000, tombstone: false };
                const result = lwwMerge([early, late]);
                runner.expect(result.length).toBe(1);
                runner.expect(result[0].object).toBe(20n);
            });

            runner.it('earlier entry is discarded', () => {
                const early = { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 500,  tombstone: false };
                const late  = { subject: 1n, predicate: 2n, object: 20n, timestamp_ms: 2000, tombstone: false };
                const result = lwwMerge([late, early]);  // order shouldn't matter
                runner.expect(result[0].object).toBe(20n);
            });

            runner.it('tombstone removes entry', () => {
                const write = { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 1000, tombstone: false };
                const del   = { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 2000, tombstone: true  };
                runner.expect(lwwMerge([write, del]).length).toBe(0);
            });

            runner.it('different predicates are independent entries', () => {
                const a = { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 1000, tombstone: false };
                const b = { subject: 1n, predicate: 3n, object: 20n, timestamp_ms: 1000, tombstone: false };
                runner.expect(lwwMerge([a, b]).length).toBe(2);
            });

            runner.it('idempotent: merging the same set twice yields same result', () => {
                const entries = [
                    { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 1000, tombstone: false },
                    { subject: 1n, predicate: 3n, object: 20n, timestamp_ms: 2000, tombstone: false },
                ];
                const r1 = lwwMerge(entries);
                const r2 = lwwMerge([...entries, ...entries]);
                runner.expect(r2.length).toBe(r1.length);
            });
        });

        runner.describe('Tie-breaking', () => {

            runner.it('on equal timestamp, higher object hash wins (deterministic)', () => {
                const a = { subject: 1n, predicate: 2n, object: 5n, timestamp_ms: 1000, tombstone: false };
                const b = { subject: 1n, predicate: 2n, object: 9n, timestamp_ms: 1000, tombstone: false };
                const winner = lwwResolve(a, b);
                runner.expect(winner.object).toBe(9n);
            });

            runner.it('resolve is commutative', () => {
                const a = { subject: 1n, predicate: 2n, object: 5n, timestamp_ms: 1000, tombstone: false };
                const b = { subject: 1n, predicate: 2n, object: 9n, timestamp_ms: 1000, tombstone: false };
                runner.expect(lwwResolve(a, b).object).toBe(lwwResolve(b, a).object);
            });
        });

        runner.describe('Temporal expiry', () => {

            runner.it('entries with expiry_ms < now are filtered as expired', () => {
                const NOW = 5000;
                const entries = [
                    { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 4000, tombstone: false, expiry_ms: 3000 },
                    { subject: 1n, predicate: 3n, object: 20n, timestamp_ms: 4000, tombstone: false, expiry_ms: 9000 },
                ];
                const live = entries.filter(e => !e.expiry_ms || e.expiry_ms >= NOW);
                runner.expect(live.length).toBe(1);
                runner.expect(live[0].object).toBe(20n);
            });
        });
    });
}

export default register;
