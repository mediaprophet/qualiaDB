use crate::NQuin;

/// Computation-Tree Logic (CTL) — BRANCHING-time temporal logic over a transition
/// system, distinct from the LINEAR-time `temporal_ltl`. Transitions are
/// `(state →next→ state')` edges (predicate == `next`); a state satisfies a
/// proposition when `(state, holds, prop)` is present. Bounded, zero-heap.

/// Max states explored by the bounded zero-heap CTL reachability.
pub const MAX_CTL_STATES: usize = 256;

#[inline]
fn satisfies(graph: &[NQuin], state: u64, holds: u64, prop: u64) -> bool {
    graph.iter().any(|q| q.subject == state && q.predicate == holds && q.object == prop)
}

/// **EF φ** — from `start`, SOME path eventually reaches a state satisfying `prop`.
/// Zero-heap BFS over `next` edges.
pub fn exists_finally(graph: &[NQuin], start: u64, prop: u64, next: u64, holds: u64) -> bool {
    if satisfies(graph, start, holds, prop) {
        return true;
    }
    let mut stack = [0u64; MAX_CTL_STATES];
    let mut slen = 1usize;
    stack[0] = start;
    let mut visited = [0u64; MAX_CTL_STATES];
    let mut vlen = 1usize;
    visited[0] = start;
    while slen > 0 {
        slen -= 1;
        let node = stack[slen];
        for q in graph {
            if q.subject != node || q.predicate != next {
                continue;
            }
            let s2 = q.object;
            if satisfies(graph, s2, holds, prop) {
                return true;
            }
            let mut seen = false;
            for &v in visited.iter().take(vlen) {
                if v == s2 {
                    seen = true;
                    break;
                }
            }
            if !seen && vlen < MAX_CTL_STATES && slen < MAX_CTL_STATES {
                visited[vlen] = s2;
                vlen += 1;
                stack[slen] = s2;
                slen += 1;
            }
        }
    }
    false
}

/// **AG φ** — EVERY state reachable from `start` (including `start`) satisfies the
/// invariant `inv`. Zero-heap BFS.
pub fn always_globally(graph: &[NQuin], start: u64, inv: u64, next: u64, holds: u64) -> bool {
    if !satisfies(graph, start, holds, inv) {
        return false;
    }
    let mut stack = [0u64; MAX_CTL_STATES];
    let mut slen = 1usize;
    stack[0] = start;
    let mut visited = [0u64; MAX_CTL_STATES];
    let mut vlen = 1usize;
    visited[0] = start;
    while slen > 0 {
        slen -= 1;
        let node = stack[slen];
        for q in graph {
            if q.subject != node || q.predicate != next {
                continue;
            }
            let s2 = q.object;
            let mut seen = false;
            for &v in visited.iter().take(vlen) {
                if v == s2 {
                    seen = true;
                    break;
                }
            }
            if seen {
                continue;
            }
            if !satisfies(graph, s2, holds, inv) {
                return false; // a reachable state violates the invariant
            }
            if vlen < MAX_CTL_STATES && slen < MAX_CTL_STATES {
                visited[vlen] = s2;
                vlen += 1;
                stack[slen] = s2;
                slen += 1;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(from: u64, to: u64) -> NQuin {
        let mut q = NQuin { subject: from, predicate: crate::q_hash("ctl:next"), object: to, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }
    fn label(state: u64, prop: u64) -> NQuin {
        let mut q = NQuin { subject: state, predicate: crate::q_hash("ctl:holds"), object: prop, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn ef_and_ag() {
        let next = crate::q_hash("ctl:next");
        let holds = crate::q_hash("ctl:holds");
        let goal = 100u64;
        let safe = 200u64;
        // 1 → 2 → 3; goal holds at 3; safe holds at 1,2,3.
        let graph = [t(1, 2), t(2, 3), label(3, goal), label(1, safe), label(2, safe), label(3, safe)];
        assert!(exists_finally(&graph, 1, goal, next, holds), "EF goal: state 3 is reachable");
        assert!(always_globally(&graph, 1, safe, next, holds), "AG safe: all reachable states are safe");
        // Break the invariant at state 2.
        let graph2 = [t(1, 2), t(2, 3), label(1, safe), label(3, safe)];
        assert!(!always_globally(&graph2, 1, safe, next, holds), "AG fails when a reachable state lacks the invariant");
        assert!(!exists_finally(&graph2, 1, goal, next, holds), "EF goal is false when no reachable state has it");
    }
}
