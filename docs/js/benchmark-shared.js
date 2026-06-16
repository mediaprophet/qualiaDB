/**
 * Shared helpers for QualiaDB benchmark pages (CI JSON + browser demos).
 */

export const PLATFORM_METRIC_GROUPS = [
    {
        id: 'graph',
        label: 'Graph Primitives',
        icon: 'fa-project-diagram',
        description: 'Indexed lookup, two-hop adjacency, predicate scan, ingestion.',
        metrics: ['point', 'twohop', 'filter', 'ingestion', 'cyclic', 'jitter'],
    },
    {
        id: 'sentinel',
        label: 'Sentinel & Governance',
        icon: 'fa-shield-halved',
        description: 'Intent intercept, provenance validation, deontic escrow, partition routing.',
        metrics: ['intercept', 'provenance_val', 'obligation_escrow', 'nym_partition'],
    },
    {
        id: 'streaming',
        label: 'Lazy & Streaming',
        icon: 'fa-bolt',
        description: 'Time-to-first-query, sync, SuperBlock lazy load, P2P stream.',
        metrics: ['ttfq', 'sync', 'wordnet_streaming', 'wordnet_p2p_stream', 'wordnet_compression'],
    },
    {
        id: 'ontology',
        label: 'Ontology & SHACL',
        icon: 'fa-book',
        description: 'SHACL throughput, defeasible lexical rights on WordNet-scale data.',
        metrics: ['wordnet_shacl', 'wordnet_defeasible'],
    },
];

export const METRIC_LABELS = {
    point: 'Point lookup',
    twohop: 'Two-hop adjacency',
    filter: 'Predicate filter',
    ingestion: 'Cold ingestion',
    cyclic: 'Cyclic query guard',
    jitter: 'Latency jitter',
    intercept: 'Intent intercept',
    provenance_val: 'Provenance validation',
    obligation_escrow: 'Obligation escrow',
    nym_partition: 'NYM partition',
    ttfq: 'Time to first query',
    sync: 'CRDT sync',
    wordnet_streaming: 'WordNet streaming',
    wordnet_p2p_stream: 'P2P SuperBlock stream',
    wordnet_compression: 'WordNet compression',
    wordnet_shacl: 'SHACL throughput',
    wordnet_defeasible: 'Defeasible lexical rights',
};

export function formatMs(val) {
    if (val === null || val === undefined) return '—';
    if (typeof val === 'string') {
        if (val.startsWith('ERROR') || val === 'OOM') return '—';
        return val;
    }
    const n = Number(val);
    if (!isFinite(n) || n < 0) return '—';
    if (n < 0.001) return '< 0.001';
    if (n < 1) return n.toFixed(3);
    return n.toFixed(2);
}

export function formatMicros(stat) {
    if (!stat?.p50 && stat?.p50 !== 0) return '—';
    const us = stat.p50;
    if (us < 1000) return `${us.toFixed(us < 10 ? 2 : 1)} µs`;
    return `${(us / 1000).toFixed(3)} ms`;
}

export function formatMetricName(key) {
    return METRIC_LABELS[key] || key.replace(/_/g, ' ');
}

export async function fetchJson(path) {
    const res = await fetch(path);
    if (!res.ok) throw new Error(`${path} → HTTP ${res.status}`);
    return res.json();
}

export function renderPlatformSuite(target, data) {
    if (!target || !data) return;
    const metrics = data.metrics || {};
    const stats = data.qualia_latency_stats || {};
    const scaling = data.qualia_scaling_stats || {};
    const env = data.execution_environment;
    const updated = data.last_updated || data.timestamp || env?.collected_at;

    let html = `
        <div class="mb-6 rounded-2xl border border-amber-900/40 bg-amber-950/30 px-5 py-4 text-sm text-amber-200">
            <div class="flex items-start gap-3">
                <i class="fa-solid fa-circle-info mt-0.5 text-amber-400"></i>
                <div>${data.note || 'Platform suite from qualia-cli bench --suite full.'}
                ${data.comparison_scope?.apples_to_apples === false
        ? ' Competitor columns are reference values — not same-machine runs.'
        : ''}</div>
            </div>
        </div>`;

    if (env) {
        html += `
        <div class="glass-strong rounded-3xl border border-slate-700 p-6 mb-8 grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div><div class="text-xs text-slate-500 uppercase mb-1">Runner</div><div class="font-mono">${env.runner || '—'}</div></div>
            <div><div class="text-xs text-slate-500 uppercase mb-1">Engine</div><div class="font-mono text-amber-300">${env.engine_version ? `v${env.engine_version}` : '—'}</div></div>
            <div><div class="text-xs text-slate-500 uppercase mb-1">Memory ceiling</div><div class="font-mono">${data.memory_limit_enforced || (env?.memory_ceiling_mb ? `${env.memory_ceiling_mb} MB` : '—')}</div></div>
        </div>`;
    }

    for (const group of PLATFORM_METRIC_GROUPS) {
        const rows = group.metrics
            .filter((k) => metrics[k])
            .map((k) => {
                const m = metrics[k];
                const micro = stats[k] || stats[`${k}_10k_quins`] || null;
                return `
                <tr class="engine-row">
                    <td class="font-medium">${formatMetricName(k)}</td>
                    <td class="text-right font-mono text-amber-300">${m.qualia ?? '—'}</td>
                    <td class="text-right font-mono text-slate-400">${m.oxi ?? '—'}</td>
                    <td class="text-right font-mono text-slate-400">${m.surreal ?? '—'}</td>
                    <td class="text-right font-mono text-emerald-400">${micro ? formatMicros(micro) : '—'}</td>
                </tr>`;
            })
            .join('');

        if (!rows) continue;

        html += `
        <div class="mb-8">
            <div class="section-label mb-2 px-1">${group.label.toUpperCase()}</div>
            <p class="text-sm text-slate-500 mb-4 px-1">${group.description}</p>
            <div class="glass-strong overflow-hidden rounded-3xl border border-slate-700">
                <table class="data-table w-full">
                    <thead class="bg-[#0f1623]">
                        <tr>
                            <th class="text-left">Operation</th>
                            <th class="text-right">Qualia (live)</th>
                            <th class="text-right">Oxigraph (ref)</th>
                            <th class="text-right">Surreal (ref)</th>
                            <th class="text-right">µbench p50</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-800">${rows}</tbody>
                </table>
            </div>
        </div>`;
    }

    const scaleKeys = Object.keys(scaling).sort((a, b) => Number(a) - Number(b));
    if (scaleKeys.length) {
        html += `
        <div class="mb-8">
            <div class="section-label mb-4 px-1">SCALING (QUALIA µBENCH)</div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">`;
        for (const key of scaleKeys) {
            const row = scaling[key];
            html += `
            <div class="glass p-5">
                <div class="text-2xl font-semibold font-mono text-blue-300">${Number(key).toLocaleString()}</div>
                <div class="text-xs text-slate-500 mb-3">subjects · RSS ${row.rss_after_materialize_mb?.toFixed(1) ?? '—'} MB</div>
                <div class="space-y-1 text-sm font-mono">
                    <div class="flex justify-between"><span class="text-slate-500">point</span><span>${formatMicros(row.point)}</span></div>
                    <div class="flex justify-between"><span class="text-slate-500">twohop</span><span>${formatMicros(row.twohop)}</span></div>
                    <div class="flex justify-between"><span class="text-slate-500">filter</span><span>${formatMicros(row.filter)}</span></div>
                </div>
            </div>`;
        }
        html += `</div></div>`;
    }

    if (updated) {
        html += `<div class="text-xs text-slate-500 font-mono px-1">CI suite updated ${new Date(updated).toLocaleString()}</div>`;
    }

    target.innerHTML = html;
}