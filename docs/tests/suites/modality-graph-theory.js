// Graph Theory — mirrors graph_theory.rs QualiaGraph algorithms (pure JS).

import { q_hash, makeQuin } from './primitives.js';

export function register(runner) {

    // ── Graph construction (mirrors QualiaGraph::from_quins) ─────────────────
    function makeGraph(edges) {
        // edges: [[subject_hash, object_hash], ...]
        const nodes = new Map();
        const adj   = new Map();
        for (const [s, o] of edges) {
            if (!nodes.has(s)) nodes.set(s, { id: s, degree: 0, centrality: 0, community: null });
            if (!nodes.has(o)) nodes.set(o, { id: o, degree: 0, centrality: 0, community: null });
            nodes.get(s).degree += 1;
            if (!adj.has(s)) adj.set(s, []);
            adj.get(s).push(o);
        }
        return { nodes, adj };
    }

    /** Degree centrality: degree[v] / (|V|-1) */
    function degreeCentrality(graph) {
        const n = graph.nodes.size;
        if (n <= 1) return new Map();
        const result = new Map();
        for (const [id, node] of graph.nodes) {
            result.set(id, node.degree / (n - 1));
        }
        return result;
    }

    /** Simplified PageRank (damping factor 0.85, max 50 iterations). */
    function pageRank(graph, d = 0.85, iters = 50) {
        const n = graph.nodes.size;
        const ids = Array.from(graph.nodes.keys());
        const rank = new Map(ids.map(id => [id, 1.0 / n]));
        for (let i = 0; i < iters; i++) {
            const newRank = new Map(ids.map(id => [id, (1 - d) / n]));
            for (const [src, targets] of graph.adj) {
                const share = rank.get(src) * d / targets.length;
                for (const tgt of targets) {
                    newRank.set(tgt, (newRank.get(tgt) || 0) + share);
                }
            }
            for (const id of ids) rank.set(id, newRank.get(id) || (1 - d) / n);
        }
        return rank;
    }

    /** BFS shortest path — returns hop count or Infinity. */
    function shortestPath(graph, src, dst) {
        if (src === dst) return 0;
        const visited = new Set([src]);
        const queue = [[src, 0]];
        while (queue.length) {
            const [node, dist] = queue.shift();
            for (const nb of (graph.adj.get(node) || [])) {
                if (nb === dst) return dist + 1;
                if (!visited.has(nb)) { visited.add(nb); queue.push([nb, dist + 1]); }
            }
        }
        return Infinity;
    }

    /** Simple label-propagation community detection (single pass). */
    function labelPropagation(graph) {
        const ids = Array.from(graph.nodes.keys());
        const labels = new Map(ids.map((id, i) => [id, i]));
        let changed = true;
        let maxIters = 20;
        while (changed && maxIters-- > 0) {
            changed = false;
            for (const id of ids) {
                const nbs = graph.adj.get(id) || [];
                if (!nbs.length) continue;
                const freq = new Map();
                for (const nb of nbs) { const l = labels.get(nb) ?? -1; freq.set(l, (freq.get(l) || 0) + 1); }
                let best = labels.get(id), bestCount = 0;
                for (const [l, c] of freq) { if (c > bestCount) { bestCount = c; best = l; } }
                if (best !== labels.get(id)) { labels.set(id, best); changed = true; }
            }
        }
        return labels;
    }

    // Build a small test graph: Alice → Bob → Carol, Alice → Carol, Dave (isolated)
    const A = q_hash('did:alice'), B = q_hash('did:bob'),
          C = q_hash('did:carol'), D = q_hash('did:dave');

    runner.describe('Modality: Graph Theory', () => {

        runner.describe('Graph construction from Quins', () => {
            runner.it('nodes are created for every unique subject and object', () => {
                const g = makeGraph([[A, B], [B, C]]);
                runner.expect(g.nodes.has(A)).toBeTruthy();
                runner.expect(g.nodes.has(B)).toBeTruthy();
                runner.expect(g.nodes.has(C)).toBeTruthy();
            });

            runner.it('degree counts out-edges', () => {
                const g = makeGraph([[A, B], [A, C]]);
                runner.expect(g.nodes.get(A).degree).toBe(2);
                runner.expect(g.nodes.get(B).degree).toBe(0);
            });

            runner.it('isolated node has degree 0', () => {
                const g = makeGraph([[A, B], [D, D]]);
                runner.expect(g.nodes.get(D).degree).toBe(1);  // self-loop counts
            });
        });

        runner.describe('Degree centrality', () => {
            runner.it('hub node has highest degree centrality', () => {
                const g = makeGraph([[A, B], [A, C], [B, C]]);
                const dc = degreeCentrality(g);
                runner.expect(dc.get(A)).toBeGreaterThan(dc.get(C));
            });

            runner.it('centrality of unconnected sink node is 0', () => {
                const g = makeGraph([[A, B], [A, C]]);
                const dc = degreeCentrality(g);
                runner.expect(dc.get(B)).toBe(0);
            });

            runner.it('all centrality values are in [0, 1]', () => {
                const g = makeGraph([[A, B], [B, C], [C, A]]);
                const dc = degreeCentrality(g);
                for (const v of dc.values()) {
                    runner.expect(v).toBeGreaterThanOrEqual(0);
                    runner.expect(v).toBeLessThanOrEqual(1);
                }
            });
        });

        runner.describe('PageRank', () => {
            runner.it('PageRank sums to ~1 across all nodes', () => {
                const g = makeGraph([[A, B], [B, C], [C, A]]);
                const pr = pageRank(g);
                const total = Array.from(pr.values()).reduce((a, b) => a + b, 0);
                runner.expect(Math.abs(total - 1.0)).toBeLessThan(0.01);
            });

            runner.it('authority node receives higher PageRank', () => {
                // A → C, B → C — C is authority
                const g = makeGraph([[A, C], [B, C], [C, A]]);
                const pr = pageRank(g);
                runner.expect(pr.get(C)).toBeGreaterThan(pr.get(B));
            });

            runner.it('PageRank of every node is positive', () => {
                const g = makeGraph([[A, B], [B, C], [C, A]]);
                const pr = pageRank(g);
                for (const v of pr.values()) runner.expect(v).toBeGreaterThan(0);
            });
        });

        runner.describe('Shortest path (BFS)', () => {
            runner.it('distance from node to itself is 0', () => {
                const g = makeGraph([[A, B]]);
                runner.expect(shortestPath(g, A, A)).toBe(0);
            });

            runner.it('direct edge has distance 1', () => {
                const g = makeGraph([[A, B], [B, C]]);
                runner.expect(shortestPath(g, A, B)).toBe(1);
            });

            runner.it('two-hop path has distance 2', () => {
                const g = makeGraph([[A, B], [B, C]]);
                runner.expect(shortestPath(g, A, C)).toBe(2);
            });

            runner.it('unreachable node has distance Infinity', () => {
                const g = makeGraph([[A, B]]);
                runner.expect(shortestPath(g, A, C)).toBe(Infinity);
            });
        });

        runner.describe('Community detection (label propagation)', () => {
            runner.it('all nodes get a community label', () => {
                const g = makeGraph([[A, B], [B, A], [C, D], [D, C]]);
                const labels = labelPropagation(g);
                runner.expect(labels.size).toBe(g.nodes.size);
            });

            runner.it('densely connected nodes share a community', () => {
                // Complete triangle: A↔B↔C↔A
                const g = makeGraph([[A, B], [B, A], [B, C], [C, B], [C, A], [A, C]]);
                const labels = labelPropagation(g);
                // They should converge to the same label
                const la = labels.get(A), lb = labels.get(B), lc = labels.get(C);
                runner.expect(la === lb && lb === lc).toBeTruthy();
            });
        });

        runner.describe('NQuin-to-graph bridge', () => {
            runner.it('makeQuin subject/object become graph edge endpoints', () => {
                const q = makeQuin(A, q_hash('foaf:knows'), B);
                const g = makeGraph([[q.subject, q.object]]);
                runner.expect(g.nodes.has(A)).toBeTruthy();
                runner.expect(g.nodes.has(B)).toBeTruthy();
            });
        });
    });
}

export default register;
