// Interval Reasoning — mirrors interval_reasoning.rs Allen's Interval Algebra (pure JS).
// Allen (1983): 13 base temporal relations between intervals.

export function register(runner) {

    // ── Core interval type ────────────────────────────────────────────────────
    function interval(start, end) {
        if (start > end) throw new Error('start must be <= end');
        return { start, end, duration: end - start };
    }

    // ── Allen's 13 base relations ─────────────────────────────────────────────
    // Mirrors AllenRelation enum in interval_reasoning.rs
    function allen(a, b) {
        if (a.end < b.start)                         return 'Before';
        if (b.end < a.start)                         return 'After';
        if (a.end === b.start)                       return 'Meets';
        if (b.end === a.start)                       return 'MetBy';
        if (a.start < b.start && a.end < b.end && a.end > b.start) return 'Overlaps';
        if (b.start < a.start && b.end < a.end && b.end > a.start) return 'OverlappedBy';
        if (a.start === b.start && a.end < b.end)   return 'Starts';
        if (b.start === a.start && b.end < a.end)   return 'StartedBy';
        if (a.start > b.start && a.end < b.end)     return 'During';
        if (b.start > a.start && b.end < a.end)     return 'Contains';
        if (a.end === b.end && a.start > b.start)   return 'Ends';
        if (b.end === a.end && b.start > a.start)   return 'EndedBy';
        return 'Equals';  // a.start === b.start && a.end === b.end
    }

    // ── Interval operations ────────────────────────────────────────────────────
    function contains(iv, point) { return point >= iv.start && point <= iv.end; }
    function overlaps(a, b)      { return a.start <= b.end && b.start <= a.end; }
    function intersection(a, b) {
        if (!overlaps(a, b)) return null;
        return interval(Math.max(a.start, b.start), Math.min(a.end, b.end));
    }
    function union_(a, b)        { return interval(Math.min(a.start, b.start), Math.max(a.end, b.end)); }
    function gap(a, b) {
        if (overlaps(a, b)) return null;
        return a.end < b.start ? b.start - a.end : a.start - b.end;
    }

    // ── Allen relation inverse table ───────────────────────────────────────────
    const INVERSE = {
        Before: 'After', After: 'Before',
        Meets: 'MetBy', MetBy: 'Meets',
        Overlaps: 'OverlappedBy', OverlappedBy: 'Overlaps',
        Starts: 'StartedBy', StartedBy: 'Starts',
        During: 'Contains', Contains: 'During',
        Ends: 'EndedBy', EndedBy: 'Ends',
        Equals: 'Equals',
    };

    runner.describe('Modality: Interval Reasoning', () => {

        runner.describe('Interval construction', () => {
            runner.it('duration = end - start', () => {
                const iv = interval(10, 30);
                runner.expect(iv.duration).toBe(20);
            });

            runner.it('point interval has duration 0', () => {
                runner.expect(interval(5, 5).duration).toBe(0);
            });
        });

        runner.describe('Allen\'s 13 base relations', () => {
            runner.it('Before: A ends before B starts', () => {
                runner.expect(allen(interval(1, 3), interval(5, 8))).toBe('Before');
            });
            runner.it('After: A starts after B ends', () => {
                runner.expect(allen(interval(5, 8), interval(1, 3))).toBe('After');
            });
            runner.it('Meets: A.end === B.start', () => {
                runner.expect(allen(interval(1, 5), interval(5, 8))).toBe('Meets');
            });
            runner.it('MetBy: B.end === A.start', () => {
                runner.expect(allen(interval(5, 8), interval(1, 5))).toBe('MetBy');
            });
            runner.it('Overlaps: A and B overlap, A starts first', () => {
                runner.expect(allen(interval(1, 6), interval(4, 9))).toBe('Overlaps');
            });
            runner.it('OverlappedBy: B starts first and overlaps A', () => {
                runner.expect(allen(interval(4, 9), interval(1, 6))).toBe('OverlappedBy');
            });
            runner.it('Starts: A starts B (same start, A shorter)', () => {
                runner.expect(allen(interval(1, 5), interval(1, 9))).toBe('Starts');
            });
            runner.it('StartedBy: A started by B (same start, A longer)', () => {
                runner.expect(allen(interval(1, 9), interval(1, 5))).toBe('StartedBy');
            });
            runner.it('During: A is strictly inside B', () => {
                runner.expect(allen(interval(3, 6), interval(1, 9))).toBe('During');
            });
            runner.it('Contains: B is strictly inside A', () => {
                runner.expect(allen(interval(1, 9), interval(3, 6))).toBe('Contains');
            });
            runner.it('Ends: A ends B (same end, A shorter)', () => {
                runner.expect(allen(interval(5, 9), interval(1, 9))).toBe('Ends');
            });
            runner.it('EndedBy: A ended by B (same end, A longer)', () => {
                runner.expect(allen(interval(1, 9), interval(5, 9))).toBe('EndedBy');
            });
            runner.it('Equals: A equals B', () => {
                runner.expect(allen(interval(1, 9), interval(1, 9))).toBe('Equals');
            });
        });

        runner.describe('Relation inverse consistency', () => {
            const cases = [
                [interval(1, 3), interval(5, 8)],
                [interval(1, 5), interval(5, 8)],
                [interval(1, 6), interval(4, 9)],
                [interval(1, 5), interval(1, 9)],
                [interval(3, 6), interval(1, 9)],
            ];
            runner.it('allen(a, b) is the inverse of allen(b, a)', () => {
                for (const [a, b] of cases) {
                    runner.expect(INVERSE[allen(a, b)]).toBe(allen(b, a));
                }
            });
        });

        runner.describe('Interval point containment', () => {
            runner.it('point inside interval is contained', () => {
                runner.expect(contains(interval(10, 20), 15)).toBeTruthy();
            });
            runner.it('boundary points are contained', () => {
                const iv = interval(10, 20);
                runner.expect(contains(iv, 10)).toBeTruthy();
                runner.expect(contains(iv, 20)).toBeTruthy();
            });
            runner.it('point outside interval is not contained', () => {
                runner.expect(contains(interval(10, 20), 5)).toBeFalsy();
            });
        });

        runner.describe('Interval operations', () => {
            runner.it('overlapping intervals have a non-null intersection', () => {
                const ix = intersection(interval(1, 7), interval(5, 10));
                runner.expect(ix).not.toBeNull();
                runner.expect(ix.start).toBe(5);
                runner.expect(ix.end).toBe(7);
            });

            runner.it('non-overlapping intervals have null intersection', () => {
                runner.expect(intersection(interval(1, 3), interval(5, 8))).toBeNull();
            });

            runner.it('union spans the full range', () => {
                const u = union_(interval(1, 5), interval(3, 9));
                runner.expect(u.start).toBe(1);
                runner.expect(u.end).toBe(9);
            });

            runner.it('gap between non-overlapping intervals is positive', () => {
                runner.expect(gap(interval(1, 4), interval(7, 10))).toBe(3);
            });

            runner.it('gap between overlapping intervals is null', () => {
                runner.expect(gap(interval(1, 7), interval(5, 10))).toBeNull();
            });
        });

        runner.describe('Temporal constraint satisfaction', () => {
            runner.it('scheduling: a meeting that starts after a task ends', () => {
                const task = interval(9 * 60, 10 * 60);    // 09:00 – 10:00
                const meeting = interval(10 * 60, 11 * 60); // 10:00 – 11:00
                runner.expect(allen(task, meeting)).toBe('Meets');
            });

            runner.it('concurrent sessions overlap', () => {
                const s1 = interval(100, 300);
                const s2 = interval(200, 400);
                runner.expect(allen(s1, s2)).toBe('Overlaps');
            });

            runner.it('nested event is During its parent', () => {
                const conference = interval(0, 480);  // all day (min)
                const keynote    = interval(60, 120);
                runner.expect(allen(keynote, conference)).toBe('During');
            });
        });
    });
}

export default register;
