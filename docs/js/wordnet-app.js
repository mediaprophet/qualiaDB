import { WordNetEngine, esc } from './wordnet-engine.js';

const engine = new WordNetEngine();
let lastLookup = null;
let graphNodes = [];
let graphEdges = [];

const SPARQL_TEMPLATES = {
    synonyms: `PREFIX wn: <https://en-wordnet.oed.com/schema/>
SELECT ?s ?p WHERE {
  ?s ?p "dog"
}`,
    hypernyms: `PREFIX wn: <https://en-wordnet.oed.com/schema/>
SELECT ?parent WHERE {
  ?synset wn:hypernym ?parent .
}`,
    hyponyms: `PREFIX wn: <https://en-wordnet.oed.com/schema/>
SELECT ?child WHERE {
  ?synset wn:hyponym ?child .
}`,
    stats: `SELECT ?s ?p ?o WHERE {
  ?s ?p ?o
}`,
};

function $(id) { return document.getElementById(id); }

function setStatus(text, ok = true) {
    const el = $('engine-status');
    if (!el) return;
    el.textContent = text;
    el.className = ok
        ? 'text-xs px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25'
        : 'text-xs px-3 py-1 rounded-full bg-red-500/15 text-red-300 border border-red-500/25';
}

function setLoading(show, msg = 'Loading Open English WordNet…') {
    const overlay = $('loading-overlay');
    if (!overlay) return;
    overlay.style.display = show ? 'flex' : 'none';
    const label = $('loading-label');
    if (label) label.textContent = msg;
}

function renderRelationTags(relations) {
    const blocks = [];
    const map = [
        ['hypernyms', 'Hypernyms', 'relation-hypernym'],
        ['hyponyms', 'Hyponyms', 'relation-hyponym'],
        ['synonyms', 'Synonyms', 'relation-synonym'],
        ['similar', 'Similar', 'relation-similar'],
        ['lemmas', 'Lemmas', 'relation-part'],
    ];
    for (const [key, label, cls] of map) {
        const items = relations[key] ?? [];
        if (!items.length) continue;
        const tags = items.slice(0, 12).map(w =>
            `<span class="relation-tag ${cls}">${esc(shortLabel(w))}</span>`
        ).join('');
        const more = items.length > 12 ? `<span class="text-xs text-white/40">+${items.length - 12} more</span>` : '';
        blocks.push(`<div><div class="text-xs text-white/50 mb-1">${label}</div><div class="flex flex-wrap gap-1">${tags}${more}</div></div>`);
    }
    return blocks.join('');
}

function shortLabel(value) {
    if (!value) return '';
    if (value.startsWith('http')) {
        const tail = value.split(/[/#]/).pop() ?? value;
        return tail.length > 28 ? tail.slice(0, 25) + '…' : tail;
    }
    return value.length > 32 ? value.slice(0, 29) + '…' : value;
}

function updateStats(stats, depth = null) {
    $('stat-words').textContent = stats.words ? stats.words.toLocaleString() : '—';
    $('stat-synsets').textContent = stats.synsets ? stats.synsets.toLocaleString() : '—';
    $('stat-relations').textContent = String(stats.relations ?? '—');
    $('stat-depth').textContent = depth != null ? String(depth) : (stats.depth ?? '—');
}

window.lookupWord = async function lookupWord() {
    if (!engine.vfs) return;
    const word = $('word-search').value.trim();
    if (!word) return;

    setLoading(true, `Looking up "${word}"…`);
    try {
        const result = await engine.lookupWord(word);
        lastLookup = result;
        const resultDiv = $('word-result');

        if (!result.found || !result.synsets.length) {
            resultDiv.classList.remove('hidden');
            $('result-word').textContent = word;
            $('result-pos').textContent = 'not found';
            $('result-definition').textContent = 'No lemma matched in the mounted WordNet graph.';
            $('result-relations').innerHTML = '';
            return;
        }

        const syn = result.synsets[0];
        resultDiv.classList.remove('hidden');
        $('result-word').textContent = word;
        $('result-pos').textContent = syn.pos;
        $('result-definition').textContent = syn.gloss || `Synset with ${syn.edgeCount} relations in graph.`;
        $('result-relations').innerHTML = renderRelationTags(syn.relations);

        const depth = await engine.hypernymDepth(word);
        updateStats(engine.getStats(), depth);
        $('graph-word').value = word;
        visualizeGraphFromLookup(result);
    } catch (e) {
        console.error(e);
        alert(e.message || String(e));
    } finally {
        setLoading(false);
    }
};

window.quickLookup = function quickLookup(word) {
    $('word-search').value = word;
    lookupWord();
};

window.filterByCategory = async function filterByCategory(category, btn) {
    document.querySelectorAll('.category-btn').forEach(b => b.classList.remove('active'));
    if (btn) btn.classList.add('active');

    const wordsDiv = $('category-words');
    const samples = engine.getCategorySamples(category);
    wordsDiv.innerHTML = '<div class="text-xs text-white/60 text-center py-4">Checking samples…</div>';

    const rows = [];
    for (const word of samples) {
        const hit = await engine.query(`?s ?p "${word}"`, 1);
        const found = hit.matches.length > 0;
        rows.push(`
            <div class="flex items-center justify-between p-2 bg-white/5 hover:bg-white/10 rounded-xl cursor-pointer ${found ? '' : 'opacity-40'}"
                 onclick="quickLookup('${word}')">
                <span class="text-sm">${esc(word)}</span>
                <span class="text-xs text-white/40">${found ? 'in graph' : 'missing'}</span>
            </div>`);
    }
    wordsDiv.innerHTML = rows.join('');
};

window.showSparqlExample = function showSparqlExample(kind) {
    const query = SPARQL_TEMPLATES[kind] ?? SPARQL_TEMPLATES.stats;
    $('sparql-output').classList.remove('hidden');
    $('sparql-code').textContent = query;
    $('sparql-editor').value = query;
};

window.runSparqlQuery = async function runSparqlQuery() {
    const sparql = $('sparql-editor').value.trim();
    if (!sparql) return;
    $('sparql-output').classList.remove('hidden');
    $('sparql-code').textContent = sparql;

    setLoading(true, 'Executing SPARQL BGP…');
    try {
        const result = await engine.querySparql(sparql, 100);
        const lines = result.matches.map(m =>
            `S: ${engine.labelFor(m.s)}  P: ${engine.labelFor(m.p)}  O: ${engine.labelFor(m.o)}`
        );
        $('sparql-results').textContent = lines.length
            ? `BGP: ${result.bgp}\n\n${lines.join('\n')}\n\n(${result.matches.length} matches, ${result.vm_cycles} VM cycles)`
            : `BGP: ${result.bgp}\n\nNo matches.`;
    } catch (e) {
        $('sparql-results').textContent = e.message || String(e);
    } finally {
        setLoading(false);
    }
};

function visualizeGraphFromLookup(lookup) {
    if (!lookup?.found || !lookup.synsets.length) return;
    const syn = lookup.synsets[0];
    const rel = syn.relations;
    const showH = $('show-hypernyms').checked;
    const showHy = $('show-hyponyms').checked;
    const showS = $('show-synonyms').checked;
    const showSim = $('show-similar').checked;

    graphNodes = [{ id: lookup.word, x: 500, y: 200, level: 0, color: '#ffffff' }];
    graphEdges = [];

    const addEdges = (items, color, type, enabled) => {
        if (!enabled) return;
        items.slice(0, 8).forEach((item, i) => {
            const id = shortLabel(item) || `${type}-${i}`;
            const angle = (i / Math.max(items.length, 1)) * Math.PI * 2;
            graphNodes.push({
                id,
                x: 500 + Math.cos(angle) * 180,
                y: 200 + Math.sin(angle) * 120,
                level: 1,
                color,
            });
            graphEdges.push({ from: lookup.word, to: id, color, type });
        });
    };

    addEdges(rel.hypernyms, '#3b82f6', 'hypernym', showH);
    addEdges(rel.hyponyms, '#10b981', 'hyponym', showHy);
    addEdges(rel.synonyms, '#8b5cf6', 'synonym', showS);
    addEdges(rel.similar, '#ec4899', 'similar', showSim);
    drawGraph();
}

window.visualizeGraph = async function visualizeGraph() {
    const word = $('graph-word').value.trim();
    if (!word) return;
    $('word-search').value = word;
    await lookupWord();
};

function drawGraph() {
    const canvas = $('graph-canvas');
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    graphEdges.forEach(edge => {
        const from = graphNodes.find(n => n.id === edge.from);
        const to = graphNodes.find(n => n.id === edge.to);
        if (!from || !to) return;
        ctx.beginPath();
        ctx.moveTo(from.x, from.y);
        ctx.lineTo(to.x, to.y);
        ctx.strokeStyle = edge.color;
        ctx.lineWidth = 2;
        ctx.globalAlpha = 0.6;
        ctx.stroke();
        ctx.globalAlpha = 1;
    });

    graphNodes.forEach(node => {
        ctx.beginPath();
        ctx.arc(node.x, node.y, 20, 0, Math.PI * 2);
        ctx.fillStyle = node.color;
        ctx.fill();
        ctx.strokeStyle = 'rgba(255,255,255,0.3)';
        ctx.lineWidth = 2;
        ctx.stroke();
        ctx.fillStyle = '#ffffff';
        ctx.font = '12px Inter';
        ctx.textAlign = 'center';
        ctx.fillText(node.id, node.x, node.y + 35);
    });
}

window.startNtIngest = function startNtIngest() {
    const file = $('nt-file')?.files?.[0];
    if (!file) {
        alert('Select an N-Triples (.nt) file first.');
        return;
    }
    const status = $('ingest-status');
    status.textContent = 'Ingesting into browser vault…';
    const worker = new Worker(new URL('../playground/ingest_worker.js', import.meta.url));
    worker.onmessage = ({ data }) => {
        if (data.type === 'progress') {
            status.textContent = `${data.triples.toLocaleString()} triples, ${data.blocks} blocks…`;
        } else if (data.type === 'done') {
            status.textContent = `Done — reload to query ${data.triples.toLocaleString()} triples from OPFS.`;
        } else if (data.type === 'error') {
            status.textContent = 'Error: ' + data.message;
        }
    };
    worker.postMessage({ type: 'ingest', file });
};

async function boot() {
    setLoading(true);
    try {
        const stats = await engine.init();
        updateStats(stats);
        setStatus(`${stats.label} · ${stats.blocks.toLocaleString()} blocks · WASM ready`);
        showSparqlExample('synonyms');
        $('graph-word').value = 'dog';
        await lookupWord();
    } catch (e) {
        console.error(e);
        setStatus(e.message || 'Engine failed to load', false);
        $('stat-words').textContent = '!';
    } finally {
        setLoading(false);
    }
}

document.addEventListener('DOMContentLoaded', boot);