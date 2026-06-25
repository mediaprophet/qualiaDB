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

/// **EX φ** — SOME immediate successor of `start` satisfies `prop`.
pub fn exists_next(graph: &[NQuin], start: u64, prop: u64, next: u64, holds: u64) -> bool {
    graph.iter().any(|q| q.subject == start && q.predicate == next && satisfies(graph, q.object, holds, prop))
}

/// **AX φ** — ALL immediate successors of `start` satisfy `prop` (vacuously true if none).
pub fn always_next(graph: &[NQuin], start: u64, prop: u64, next: u64, holds: u64) -> bool {
    graph
        .iter()
        .filter(|q| q.subject == start && q.predicate == next)
        .all(|q| satisfies(graph, q.object, holds, prop))
}

/// **E[φ U ψ]** — SOME path on which `phi` holds at every state until `psi` becomes true.
/// Zero-heap BFS constrained to `phi`-states.
pub fn exists_until(graph: &[NQuin], start: u64, phi: u64, psi: u64, next: u64, holds: u64) -> bool {
    if satisfies(graph, start, holds, psi) {
        return true;
    }
    if !satisfies(graph, start, holds, phi) {
        return false;
    }
    let mut stack = [0u64; MAX_CTL_STATES];
    let mut sl = 1usize;
    stack[0] = start;
    let mut vis = [0u64; MAX_CTL_STATES];
    let mut vl = 1usize;
    vis[0] = start;
    while sl > 0 {
        sl -= 1;
        let node = stack[sl];
        for q in graph {
            if q.subject != node || q.predicate != next {
                continue;
            }
            let s2 = q.object;
            if satisfies(graph, s2, holds, psi) {
                return true;
            }
            if satisfies(graph, s2, holds, phi) && !vis[..vl].contains(&s2) && vl < MAX_CTL_STATES && sl < MAX_CTL_STATES {
                vis[vl] = s2;
                vl += 1;
                stack[sl] = s2;
                sl += 1;
            }
        }
    }
    false
}

/// Collect the states reachable from `start` (inclusive) along `next` edges into `out`. Returns
/// the count. Bounded + zero-heap.
fn reachable_states(graph: &[NQuin], start: u64, next: u64, out: &mut [u64; MAX_CTL_STATES]) -> usize {
    let mut vl = 1usize;
    out[0] = start;
    let mut i = 0usize;
    while i < vl {
        let node = out[i];
        i += 1;
        for q in graph {
            if q.subject == node && q.predicate == next {
                let s2 = q.object;
                if !out[..vl].contains(&s2) && vl < MAX_CTL_STATES {
                    out[vl] = s2;
                    vl += 1;
                }
            }
        }
    }
    vl
}

#[inline]
fn idx_of(states: &[u64], s: u64) -> Option<usize> {
    states.iter().position(|&x| x == s)
}

/// **EG φ** — SOME path from `start` on which the invariant `prop` holds forever. Greatest-fixpoint
/// labelling (Emerson-Clarke): keep a `prop`-state alive while it retains a successor that is also
/// alive; `EG` holds iff `start` survives. Bounded + zero-heap.
pub fn exists_globally(graph: &[NQuin], start: u64, prop: u64, next: u64, holds: u64) -> bool {
    let mut states = [0u64; MAX_CTL_STATES];
    let n = reachable_states(graph, start, next, &mut states);
    let mut alive = [false; MAX_CTL_STATES];
    for i in 0..n {
        alive[i] = satisfies(graph, states[i], holds, prop);
    }
    loop {
        let mut changed = false;
        for i in 0..n {
            if !alive[i] {
                continue;
            }
            // Does state i have a successor that is alive?
            let mut has_alive_succ = false;
            for q in graph {
                if q.subject == states[i] && q.predicate == next {
                    if let Some(j) = idx_of(&states[..n], q.object) {
                        if alive[j] {
                            has_alive_succ = true;
                            break;
                        }
                    }
                }
            }
            if !has_alive_succ {
                alive[i] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    idx_of(&states[..n], start).map(|i| alive[i]).unwrap_or(false)
}

/// **AF φ** — on ALL paths from `start`, `prop` eventually holds. Least-fixpoint labelling: a state
/// is `AF` if it satisfies `prop`, or it has ≥1 successor and ALL successors are `AF` (a `prop`-free
/// cycle or `prop`-free deadlock falsifies it). Bounded + zero-heap.
pub fn all_finally(graph: &[NQuin], start: u64, prop: u64, next: u64, holds: u64) -> bool {
    let mut states = [0u64; MAX_CTL_STATES];
    let n = reachable_states(graph, start, next, &mut states);
    let mut is_af = [false; MAX_CTL_STATES];
    for i in 0..n {
        is_af[i] = satisfies(graph, states[i], holds, prop);
    }
    loop {
        let mut changed = false;
        for i in 0..n {
            if is_af[i] {
                continue;
            }
            let mut any_succ = false;
            let mut all_af = true;
            for q in graph {
                if q.subject == states[i] && q.predicate == next {
                    any_succ = true;
                    match idx_of(&states[..n], q.object) {
                        Some(j) if is_af[j] => {}
                        _ => {
                            all_af = false;
                            break;
                        }
                    }
                }
            }
            if any_succ && all_af {
                is_af[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    idx_of(&states[..n], start).map(|i| is_af[i]).unwrap_or(false)
}

/// **A[φ U ψ]** — on EVERY path from `start`, `phi` holds at each state until `psi` becomes true
/// (and `psi` is reached on every path). Least-fixpoint labelling. Bounded + zero-heap. Completes
/// the CTL operator set (EX, AX, EF, AF, EG, AG, EU, AU).
pub fn all_until(graph: &[NQuin], start: u64, phi: u64, psi: u64, next: u64, holds: u64) -> bool {
    let mut states = [0u64; MAX_CTL_STATES];
    let n = reachable_states(graph, start, next, &mut states);
    let mut au = [false; MAX_CTL_STATES];
    for i in 0..n {
        au[i] = satisfies(graph, states[i], holds, psi);
    }
    loop {
        let mut changed = false;
        for i in 0..n {
            if au[i] || !satisfies(graph, states[i], holds, phi) {
                continue;
            }
            let mut any_succ = false;
            let mut all_au = true;
            for q in graph {
                if q.subject == states[i] && q.predicate == next {
                    any_succ = true;
                    match idx_of(&states[..n], q.object) {
                        Some(j) if au[j] => {}
                        _ => {
                            all_au = false;
                            break;
                        }
                    }
                }
            }
            if any_succ && all_au {
                au[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    idx_of(&states[..n], start).map(|i| au[i]).unwrap_or(false)
}

/// Does the alive state at `start_idx` lie on a cycle within the alive set (reach itself)?
fn alive_reaches_self(graph: &[NQuin], states: &[u64], n: usize, alive: &[bool], start_idx: usize, next: u64) -> bool {
    let target = states[start_idx];
    let mut stack = [0usize; MAX_CTL_STATES];
    let mut sl = 0usize;
    let mut vis = [false; MAX_CTL_STATES];
    // Seed with the immediate alive successors of start_idx.
    for q in graph {
        if q.subject == states[start_idx] && q.predicate == next {
            if let Some(j) = idx_of(&states[..n], q.object) {
                if alive[j] {
                    if states[j] == target {
                        return true;
                    }
                    if !vis[j] && sl < MAX_CTL_STATES {
                        vis[j] = true;
                        stack[sl] = j;
                        sl += 1;
                    }
                }
            }
        }
    }
    while sl > 0 {
        sl -= 1;
        let cur = stack[sl];
        for q in graph {
            if q.subject == states[cur] && q.predicate == next {
                if let Some(j) = idx_of(&states[..n], q.object) {
                    if alive[j] {
                        if states[j] == target {
                            return true;
                        }
                        if !vis[j] && sl < MAX_CTL_STATES {
                            vis[j] = true;
                            stack[sl] = j;
                            sl += 1;
                        }
                    }
                }
            }
        }
    }
    false
}

/// **Fair EG φ** — an infinite path from `start` on which `prop` holds forever AND a `fair` state
/// is visited infinitely often. The fairness constraint eliminates unrealistic infinite loops that
/// make no progress. True iff some `fair` state — reachable from `start` within the `prop`-states
/// that have an infinite `prop`-future — lies on a cycle. Bounded + zero-heap.
pub fn fair_globally(graph: &[NQuin], start: u64, prop: u64, fair: u64, next: u64, holds: u64) -> bool {
    let mut states = [0u64; MAX_CTL_STATES];
    let n = reachable_states(graph, start, next, &mut states);
    let mut alive = [false; MAX_CTL_STATES];
    for i in 0..n {
        alive[i] = satisfies(graph, states[i], holds, prop);
    }
    loop {
        let mut changed = false;
        for i in 0..n {
            if !alive[i] {
                continue;
            }
            let mut has = false;
            for q in graph {
                if q.subject == states[i] && q.predicate == next {
                    if let Some(j) = idx_of(&states[..n], q.object) {
                        if alive[j] {
                            has = true;
                            break;
                        }
                    }
                }
            }
            if !has {
                alive[i] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for i in 0..n {
        if alive[i] && satisfies(graph, states[i], holds, fair) && alive_reaches_self(graph, &states, n, &alive, i, next) {
            return true;
        }
    }
    false
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

    #[test]
    fn ex_ax_eu_eg_af_operators() {
        let next = crate::q_hash("ctl:next");
        let holds = crate::q_hash("ctl:holds");
        let (p, goal, a) = (100u64, 200u64, 50u64);

        // EX / AX: 1→2, 1→3; p at 2 only.
        let g = [t(1, 2), t(1, 3), label(2, p)];
        assert!(exists_next(&g, 1, p, next, holds), "EX: successor 2 has p");
        assert!(!always_next(&g, 1, p, next, holds), "AX fails: successor 3 lacks p");
        assert!(always_next(&g, 2, p, next, holds), "no successors → AX vacuously true");

        // E[a U goal]: a holds along 1→2 until goal at 3.
        let g2 = [t(1, 2), t(2, 3), label(1, a), label(2, a), label(3, goal)];
        assert!(exists_until(&g2, 1, a, goal, next, holds));
        let g3 = [t(1, 2), t(2, 3), label(1, a), label(3, goal)]; // a breaks at 2
        assert!(!exists_until(&g3, 1, a, goal, next, holds));
        // A[a U goal]: on the only path, a holds until goal → AU holds; broken chain → fails.
        assert!(all_until(&g2, 1, a, goal, next, holds));
        assert!(!all_until(&g3, 1, a, goal, next, holds), "a breaks before goal");

        // EG p: 1→2→2 loop, p everywhere → an infinite p-path exists.
        let g4 = [t(1, 2), t(2, 2), label(1, p), label(2, p)];
        assert!(exists_globally(&g4, 1, p, next, holds));
        let g5 = [t(1, 2), label(1, p)]; // successor 2 lacks p → no infinite p-path
        assert!(!exists_globally(&g5, 1, p, next, holds));

        // AF goal: 1→2→2 loop with goal at 2 → every path reaches goal.
        let g6 = [t(1, 2), t(2, 2), label(2, goal)];
        assert!(all_finally(&g6, 1, goal, next, holds));
        let g7 = [t(1, 2), t(2, 2), label(1, goal)]; // goal-free 2-loop
        assert!(!all_finally(&g7, 2, goal, next, holds), "a goal-free cycle never reaches goal");
    }

    #[test]
    fn fair_eg_requires_a_fair_state_on_the_cycle() {
        let next = crate::q_hash("ctl:next");
        let holds = crate::q_hash("ctl:holds");
        let (p, fair) = (100u64, 300u64);
        // 1→2→2 loop, p everywhere, fair at 2 (on the cycle) → fair-EG holds.
        let g = [t(1, 2), t(2, 2), label(1, p), label(2, p), label(2, fair)];
        assert!(fair_globally(&g, 1, p, fair, next, holds));
        // fair only at 1 (NOT on the 2-cycle) → no fair infinite path.
        let g2 = [t(1, 2), t(2, 2), label(1, p), label(2, p), label(1, fair)];
        assert!(!fair_globally(&g2, 1, p, fair, next, holds), "the cycle has no fair state");
        // plain EG still holds (the unfair loop is an infinite p-path).
        assert!(exists_globally(&g2, 1, p, next, holds));
    }
}
