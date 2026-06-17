import { OntologyEngine, esc } from './ontology-engine.js';
import { formatOpfsCacheLabel } from '../playground/vfs.js';

const engine = new OntologyEngine();
let lastLookup = null;
let graphNodes = [];
let graphEdges = [];

const UI_PROFILES = {
    wordnet: {
        title: 'Princeton WordNet',
        subtitle: 'Princeton WordNet 3.1 — ~5.56M triples, lemmas and synset relations',
        searchLabel: 'Search Word',
        searchPlaceholder: 'e.g., dog, happy, run',
        searchButton: 'Lookup Word',
        stat1: 'Lexicon Entries',
        stat2: 'Est. Synsets',
        stat3: 'Relation Types',
        stat4: 'Hypernym Depth',
        categoryTitle: 'POS Browser',
        categories: [
            { id: 'noun', label: 'Nouns', icon: 'fa-cube', color: 'blue' },
            { id: 'verb', label: 'Verbs', icon: 'fa-bolt', color: 'emerald' },
            { id: 'adjective', label: 'Adjectives', icon: 'fa-palette', color: 'purple' },
            { id: 'adverb', label: 'Adverbs', icon: 'fa-wand-magic-sparkles', color: 'amber' },
        ],
        quickExamples: ['dog', 'happy', 'run', 'beautiful', 'computer'],
        graphPlaceholder: 'Enter word to visualize',
        graphToggles: [
            { id: 'show-hypernyms', label: 'Hypernyms', key: 'hypernyms', color: '#3b82f6', default: true },
            { id: 'show-hyponyms', label: 'Hyponyms', key: 'hyponyms', color: '#10b981', default: true },
            { id: 'show-synonyms', label: 'Synonyms', key: 'synonyms', color: '#8b5cf6', default: true },
            { id: 'show-similar', label: 'Similar', key: 'similar', color: '#ec4899', default: false },
        ],
        relationMap: [
            ['hypernyms', 'Hypernyms', 'relation-hypernym'],
            ['hyponyms', 'Hyponyms', 'relation-hyponym'],
            ['synonyms', 'Synonyms', 'relation-synonym'],
            ['similar', 'Similar', 'relation-similar'],
            ['lemmas', 'Lemmas', 'relation-part'],
        ],
        sparqlExamples: [
            { id: 'lemma', label: 'Find lemma matches', icon: 'purple' },
            { id: 'hypernyms', label: 'Hypernym pattern', icon: 'blue' },
            { id: 'hyponyms', label: 'Hyponym pattern', icon: 'emerald' },
            { id: 'wildcard', label: 'Wildcard scan', icon: 'amber' },
        ],
        defaultSearch: 'dog',
        ingestNote: 'powershell scripts/fetch_wordnet_release.ps1  (or ingest_princeton_wordnet.ps1 to build)',
    },
    schemaorg: {
        title: 'Schema.org',
        subtitle: 'Schema.org 30.0 vocabulary — classes, properties, and domains',
        searchLabel: 'Search Type or Property',
        searchPlaceholder: 'e.g., Person, Organization, name',
        searchButton: 'Lookup Term',
        stat1: 'Lexicon Entries',
        stat2: 'Est. Types',
        stat3: 'Relation Types',
        stat4: 'Superclass Depth',
        categoryTitle: 'Vocabulary Browser',
        categories: [
            { id: 'classes', label: 'Classes', icon: 'fa-shapes', color: 'blue' },
            { id: 'properties', label: 'Properties', icon: 'fa-tag', color: 'emerald' },
            { id: 'types', label: 'Type Families', icon: 'fa-sitemap', color: 'purple' },
        ],
        quickExamples: ['Person', 'Organization', 'Event', 'CreativeWork', 'name'],
        graphPlaceholder: 'Enter type or property to visualize',
        graphToggles: [
            { id: 'show-superclass', label: 'Superclasses', key: 'superClass', color: '#3b82f6', default: true },
            { id: 'show-subclasses', label: 'Subclasses', key: 'subClasses', color: '#10b981', default: true },
            { id: 'show-domains', label: 'Domains', key: 'domains', color: '#8b5cf6', default: true },
            { id: 'show-ranges', label: 'Ranges', key: 'ranges', color: '#ec4899', default: false },
        ],
        relationMap: [
            ['superClass', 'Superclass', 'relation-hypernym'],
            ['subClasses', 'Subclasses', 'relation-hyponym'],
            ['domains', 'Domains', 'relation-synonym'],
            ['ranges', 'Ranges', 'relation-similar'],
            ['labels', 'Labels', 'relation-part'],
            ['comments', 'Comments', 'relation-part'],
        ],
        sparqlExamples: [
            { id: 'type', label: 'Inspect a type', icon: 'purple' },
            { id: 'subclass', label: 'Subclass pattern', icon: 'blue' },
            { id: 'property', label: 'Property lookup', icon: 'emerald' },
            { id: 'wildcard', label: 'Wildcard scan', icon: 'amber' },
        ],
        defaultSearch: 'Person',
        ingestNote: 'bash scripts/prepare_schemaorg_benchmark.sh 30.0 current-https',
    },
    w3c: {
        title: 'W3C Vocabulary',
        subtitle: 'W3C vocabulary schemas (class/property TBox only — typically 1–4 KB per .q42, not instance data)',
        searchLabel: 'Search Term or Class',
        searchPlaceholder: 'e.g., Concept, Shape, Activity',
        searchButton: 'Lookup Term',
        stat1: 'Lexicon Entries',
        stat2: 'Est. Terms',
        stat3: 'Relation Types',
        stat4: 'Superclass Depth',
        categoryTitle: 'Vocabulary Browser',
        categories: [
            { id: 'terms', label: 'Key Terms', icon: 'fa-star', color: 'blue' },
            { id: 'classes', label: 'Classes', icon: 'fa-shapes', color: 'emerald' },
            { id: 'properties', label: 'Properties', icon: 'fa-tag', color: 'purple' },
        ],
        quickExamples: ['Concept', 'Shape', 'Activity', 'Dataset', 'Sensor'],
        graphPlaceholder: 'Enter class or property to visualize',
        graphToggles: [
            { id: 'show-superclass', label: 'Superclasses', key: 'superClass', color: '#3b82f6', default: true },
            { id: 'show-subclasses', label: 'Subclasses', key: 'subClasses', color: '#10b981', default: true },
            { id: 'show-domains', label: 'Domains', key: 'domains', color: '#8b5cf6', default: true },
            { id: 'show-labels', label: 'Labels', key: 'labels', color: '#ec4899', default: false },
        ],
        relationMap: [
            ['superClass', 'Superclass', 'relation-hypernym'],
            ['subClasses', 'Subclasses', 'relation-hyponym'],
            ['domains', 'Domains', 'relation-synonym'],
            ['ranges', 'Ranges', 'relation-similar'],
            ['labels', 'Labels', 'relation-part'],
            ['definitions', 'Definitions', 'relation-part'],
        ],
        sparqlExamples: [
            { id: 'type', label: 'Inspect a term', icon: 'purple' },
            { id: 'subclass', label: 'Subclass pattern', icon: 'blue' },
            { id: 'label', label: 'Label lookup', icon: 'emerald' },
            { id: 'wildcard', label: 'Wildcard scan', icon: 'amber' },
        ],
        defaultSearch: 'Concept',
        ingestNote: 'bash scripts/prepare_bundled_ontologies.sh w3c',
    },
};

const SPARQL_TEMPLATES = {
    wordnet: {
        lemma: `PREFIX wn: <https://en-wordnet.oed.com/schema/>
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
        wildcard: `SELECT ?s ?p ?o WHERE {
  ?s ?p ?o
}`,
    },
    schemaorg: {
        type: `PREFIX schema: <https://schema.org/>
SELECT ?p ?o WHERE {
  <https://schema.org/Person> ?p ?o
}`,
        subclass: `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?super WHERE {
  <https://schema.org/Person> rdfs:subClassOf ?super .
}`,
        property: `SELECT ?s ?p ?o WHERE {
  ?s ?p "name"
}`,
        wildcard: `SELECT ?s ?p ?o WHERE {
  ?s ?p ?o
}`,
    },
    w3c: {
        type: `SELECT ?p ?o WHERE {
  <http://www.w3.org/2004/02/skos/core#Concept> ?p ?o
}`,
        subclass: `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?super WHERE {
  <http://www.w3.org/2004/02/skos/core#Concept> rdfs:subClassOf ?super .
}`,
        label: `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?s ?label WHERE {
  ?s rdfs:label ?label .
}`,
        wildcard: `SELECT ?s ?p ?o WHERE {
  ?s ?p ?o
}`,
    },
};

function $(id) { return document.getElementById(id); }

function datasetUiHints() {
    const ds = engine.activeDataset ?? {};
    const base = ui();
    return {
        ...base,
        title: ds.label ?? base.title,
        quickExamples: ds.quickExamples?.length ? ds.quickExamples : base.quickExamples,
        defaultSearch: ds.defaultSearch ?? base.defaultSearch,
    };
}

function w3cSparqlTemplates() {
    const ds = engine.activeDataset ?? {};
    const term = ds.defaultSearch ?? 'Resource';
    const iri = engine.normalizeIri(term);
    const fullIri = iri.startsWith('http') ? iri : `${ds.namespace ?? ''}${term}`;
    return {
        type: `SELECT ?p ?o WHERE {\n  <${fullIri}> ?p ?o\n}`,
        subclass: `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\nSELECT ?super WHERE {\n  <${fullIri}> rdfs:subClassOf ?super .\n}`,
        label: `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\nSELECT ?s ?label WHERE {\n  ?s rdfs:label "${term}" .\n}`,
        wildcard: `SELECT ?s ?p ?o WHERE {\n  ?s ?p ?o\n}`,
    };
}

function ui() {
    return UI_PROFILES[engine.profile] ?? UI_PROFILES.wordnet;
}

function setStatus(text, ok = true) {
    const el = $('engine-status');
    if (!el) return;
    el.textContent = text;
    el.className = ok
        ? 'text-xs px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25'
        : 'text-xs px-3 py-1 rounded-full bg-red-500/15 text-red-300 border border-red-500/25';
}

function setLoading(show, msg = 'Loading ontology…') {
    const overlay = $('loading-overlay');
    if (!overlay) return;
    overlay.style.display = show ? 'flex' : 'none';
    const label = $('loading-label');
    if (label) label.textContent = msg;
}

let _opfsWatchTimer = null;

function watchOpfsCache(vfs) {
    if (_opfsWatchTimer) clearInterval(_opfsWatchTimer);
    if (!vfs?.opfsCache?.prefetching) return;
    _opfsWatchTimer = setInterval(() => {
        const c = vfs.opfsCache;
        if (!c.prefetching) {
            clearInterval(_opfsWatchTimer);
            _opfsWatchTimer = null;
        }
        setStatus(`Dataset mounted · WASM ready${formatOpfsCacheLabel(c)}`);
    }, 1500);
}

function shortLabel(value) {
    if (!value) return '';
    if (value.startsWith('http')) {
        const tail = value.split(/[/#]/).pop() ?? value;
        return tail.length > 28 ? tail.slice(0, 25) + '…' : tail;
    }
    return value.length > 32 ? value.slice(0, 29) + '…' : value;
}

function renderRelationTags(relations) {
    const blocks = [];
    for (const [key, label, cls] of ui().relationMap) {
        const items = relations[key] ?? [];
        if (!items.length) continue;
        const tags = items.slice(0, 12).map(w =>
            `<span class="relation-tag ${cls}">${esc(shortLabel(w))}</span>`
        ).join('');
        const more = items.length > 12
            ? `<span class="text-xs text-white/40">+${items.length - 12} more</span>`
            : '';
        blocks.push(`<div><div class="text-xs text-white/50 mb-1">${label}</div><div class="flex flex-wrap gap-1">${tags}${more}</div></div>`);
    }
    return blocks.join('');
}

function updateStats(stats, depth = null) {
    const p = ui();
    $('stat-1').textContent = stats.terms ? stats.terms.toLocaleString() : '—';
    $('stat-2').textContent = stats.entities ? stats.entities.toLocaleString() : '—';
    $('stat-3').textContent = String(stats.relations ?? '—');
    $('stat-4').textContent = depth != null ? String(depth) : (stats.depth ?? '—');
    $('stat-1-label').textContent = p.stat1;
    $('stat-2-label').textContent = p.stat2;
    $('stat-3-label').textContent = p.stat3;
    $('stat-4-label').textContent = p.stat4;
}

function updateDatasetInfo(stats) {
    const el = $('dataset-info');
    if (!el) return;
    const d = engine.activeDataset;
    if (!d) {
        el.classList.add('hidden');
        return;
    }
    const profile = d.profile ?? 'wordnet';
    const estTriples = stats?.triples ? `~${stats.triples.toLocaleString()} est. triples` : '';
    const blocks = stats?.blocks ? `${stats.blocks.toLocaleString()} blocks` : '';
    const sizeHint = profile === 'w3c' || profile === 'purl'
        ? 'Vocabulary schema (TBox) — small file size is expected.'
        : profile === 'fibo'
        ? 'FIBO domain slice — financial ontology TBox + axioms.'
        : profile === 'schemaorg'
        ? 'Full Schema.org release vocabulary (~18k triples).'
        : 'Full Princeton WordNet 3.1 knowledge graph (~127 MB .q42, 5.56M triples). Build via scripts/ingest_princeton_wordnet.ps1.';
    el.innerHTML = `
        <div class="flex flex-wrap items-start gap-x-3 gap-y-1">
            <span class="text-white/80 font-medium">${esc(d.label ?? d.id)}</span>
            ${blocks ? `<span class="text-white/40">·</span><span>${blocks}</span>` : ''}
            ${estTriples ? `<span class="text-white/40">·</span><span>${estTriples}</span>` : ''}
        </div>
        <p class="text-xs text-white/50 mt-1">${esc(d.description ?? sizeHint)}</p>
        <p class="text-xs text-white/40 mt-1">${esc(sizeHint)}</p>`;
    el.classList.remove('hidden');
}

function renderDatasetPicker() {
    const container = $('dataset-picker');
    if (!container || !engine.datasets.length) return;

    const primary = engine.datasets.filter(d => !['w3c', 'purl', 'fibo', 'w3c-archives'].includes(d.group));
    const w3c = engine.datasets.filter(d => d.group === 'w3c');
    const purl = engine.datasets.filter(d => d.group === 'purl');
    const fibo = engine.datasets.filter(d => d.group === 'fibo');
    const w3cArchives = engine.datasets.filter(d => d.group === 'w3c-archives');

    const card = (d) => {
        const active = d.id === engine.activeDataset?.id;
        const profile = d.profile ?? 'wordnet';
        const icon = d.icon ?? (profile === 'schemaorg' ? '🌐' : profile === 'w3c' ? '📘' : '📚');
        return `
            <button type="button"
                class="dataset-btn flex-1 min-w-[140px] px-4 py-3 rounded-2xl text-left transition-all ${active
                    ? 'bg-blue-600/30 border-2 border-blue-500/60'
                    : 'bg-white/5 border border-white/10 hover:bg-white/10'}"
                data-dataset="${esc(d.id)}">
                <div class="text-lg mb-1">${icon}</div>
                <div class="text-sm font-semibold">${esc(d.label ?? d.id)}</div>
                <div class="text-xs text-white/50 mt-0.5">${esc(d.description?.slice(0, 72) ?? profile)}</div>
            </button>`;
    };

    let html = `<div class="flex flex-wrap gap-3 w-full">${primary.map(card).join('')}</div>`;

    const renderSelect = (group, id, title, count) => {
        if (!count) return '';
        const activeId = engine.activeDataset?.id ?? '';
        const items = group === 'w3c' ? w3c
            : group === 'purl' ? purl
            : group === 'fibo' ? fibo
            : w3cArchives;
        return `
            <div class="glass-strong rounded-2xl p-4 w-full mt-1">
                <div class="text-xs text-white/60 mb-2 flex items-center gap-2">
                    <i class="fa-solid fa-layer-group text-blue-400"></i>
                    <span>${title} (${count})</span>
                </div>
                <select id="${id}" class="w-full bg-zinc-900 border border-white/20 rounded-xl px-3 py-2 text-sm">
                    <option value="">Select a ${title.toLowerCase()}…</option>
                    ${items.map(d => `<option value="${esc(d.id)}" ${d.id === activeId ? 'selected' : ''}>${d.icon ?? '📘'} ${esc(d.label ?? d.id)}</option>`).join('')}
                </select>
            </div>`;
    };

    html += renderSelect('w3c', 'w3c-select', 'W3C Vocabularies (schema only)', w3c.length);
    html += renderSelect('w3c-archives', 'w3c-archives-select', 'W3C Archives (schema only)', w3cArchives.length);
    html += renderSelect('purl', 'purl-select', 'PURL.org Vocabularies', purl.length);
    html += renderSelect('fibo', 'fibo-select', 'FIBO Domains (EDMC)', fibo.length);

    container.innerHTML = html;

    container.querySelectorAll('.dataset-btn').forEach(btn => {
        btn.addEventListener('click', () => switchDataset(btn.dataset.dataset));
    });

    for (const selId of ['w3c-select', 'w3c-archives-select', 'purl-select', 'fibo-select']) {
        const sel = $(selId);
        if (sel) {
            sel.addEventListener('change', () => {
                if (sel.value) switchDataset(sel.value);
            });
        }
    }
}

function applyProfileUi() {
    const p = datasetUiHints();
    $('page-title').textContent = 'Ontology Demo';
    $('page-subtitle').innerHTML = `
        Browse <strong>${esc(p.title)}</strong> and other mounted vocabularies live in your browser via
        <code class="text-cyan-400">qualia_core_db_bg.wasm</code> over
        <code class="text-cyan-400">.q42</code> volumes.`;
    $('search-label').textContent = p.searchLabel;
    $('entity-search').placeholder = p.searchPlaceholder;
    $('search-button-label').textContent = p.searchButton;
    $('category-title').textContent = p.categoryTitle;
    $('graph-term').placeholder = p.graphPlaceholder;
    $('ingest-note').textContent = p.ingestNote;

    const quickDiv = $('quick-examples');
    if (quickDiv) {
        quickDiv.innerHTML = p.quickExamples.map(term =>
            `<button type="button" onclick="quickLookup('${esc(term)}')" class="text-xs px-3 py-1 bg-white/5 hover:bg-white/10 rounded-full">${esc(term)}</button>`
        ).join('');
    }

    const catGrid = $('category-grid');
    if (catGrid) {
        const colors = {
            blue: 'bg-blue-500/20 hover:bg-blue-500/30 text-blue-400',
            emerald: 'bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-400',
            purple: 'bg-purple-500/20 hover:bg-purple-500/30 text-purple-400',
            amber: 'bg-amber-500/20 hover:bg-amber-500/30 text-amber-400',
        };
        catGrid.innerHTML = p.categories.map(c => `
            <button type="button" onclick="filterByCategory('${c.id}', this)"
                class="category-btn px-3 py-2 ${colors[c.color]} rounded-xl text-sm font-medium">
                <i class="fa-solid ${c.icon} mr-1"></i> ${esc(c.label)}
            </button>`).join('');
    }

    const sparqlBtns = $('sparql-examples');
    if (sparqlBtns) {
        const iconColors = { purple: 'text-purple-400', blue: 'text-blue-400', emerald: 'text-emerald-400', amber: 'text-amber-400' };
        sparqlBtns.innerHTML = p.sparqlExamples.map(ex => `
            <button type="button" onclick="showSparqlExample('${ex.id}')"
                class="w-full text-left px-3 py-2 bg-white/5 hover:bg-white/10 rounded-xl text-sm">
                <i class="fa-solid fa-code mr-2 ${iconColors[ex.icon]}"></i> ${esc(ex.label)}
            </button>`).join('');
    }

    const toggles = $('graph-toggles');
    if (toggles) {
        toggles.innerHTML = p.graphToggles.map(t => `
            <label class="flex items-center gap-2">
                <input type="checkbox" id="${t.id}" data-rel-key="${t.key}" ${t.default ? 'checked' : ''} class="rounded graph-toggle">
                <span>${esc(t.label)}</span>
            </label>`).join('');
    }
}

async function switchDataset(datasetId) {
    if (!datasetId || datasetId === engine.activeDataset?.id) return;
    setLoading(true, `Mounting ${datasetId}…`);
    try {
        const stats = await engine.mountDataset(datasetId);
        const url = new URL(location.href);
        url.searchParams.set('dataset', datasetId);
        history.replaceState({}, '', url);
        renderDatasetPicker();
        applyProfileUi();
        updateStats(stats);
        updateDatasetInfo(stats);
        setStatus(`${stats.label} · ${stats.blocks.toLocaleString()} blocks · WASM ready${formatOpfsCacheLabel(engine.vfs?.opfsCache)}`);
        watchOpfsCache(engine.vfs);
        const hints = datasetUiHints();
        showSparqlExample(hints.sparqlExamples[0]?.id ?? 'wildcard');
        $('entity-search').value = hints.defaultSearch;
        $('graph-term').value = hints.defaultSearch;
        $('category-words').innerHTML = '<div class="text-xs text-white/60 text-center py-4">Select a category to browse samples</div>';
        $('entity-result').classList.add('hidden');
        await lookupEntity();
    } catch (e) {
        console.error(e);
        setStatus(e.message || 'Failed to mount dataset', false);
    } finally {
        setLoading(false);
    }
}

window.lookupEntity = async function lookupEntity() {
    if (!engine.vfs) return;
    const term = $('entity-search').value.trim();
    if (!term) return;

    setLoading(true, `Looking up "${term}"…`);
    try {
        const result = await engine.lookupEntity(term);
        lastLookup = result;
        const resultDiv = $('entity-result');
        const entityLabel = engine.profile === 'wordnet' ? 'lemma' : 'term';

        if (!result.found || !result.entities.length) {
            resultDiv.classList.remove('hidden');
            $('result-term').textContent = term;
            $('result-type').textContent = 'not found';
            $('result-summary').textContent = `No ${entityLabel} matched in the mounted graph.`;
            $('result-relations').innerHTML = '';
            return;
        }

        const entity = result.entities[0];
        resultDiv.classList.remove('hidden');
        $('result-term').textContent = shortLabel(entity.iri) || term;
        $('result-type').textContent = entity.pos;
        $('result-summary').textContent = entity.gloss || `${entity.edgeCount} relations in graph.`;
        $('result-relations').innerHTML = renderRelationTags(entity.relations);

        const depth = await engine.hierarchyDepth(term);
        updateStats(engine.getStats(), depth);
        $('graph-term').value = term;
        visualizeGraphFromLookup(result);
    } catch (e) {
        console.error(e);
        alert(e.message || String(e));
    } finally {
        setLoading(false);
    }
};

window.quickLookup = function quickLookup(term) {
    $('entity-search').value = term;
    lookupEntity();
};

window.filterByCategory = async function filterByCategory(category, btn) {
    document.querySelectorAll('.category-btn').forEach(b => b.classList.remove('active'));
    if (btn) btn.classList.add('active');

    const wordsDiv = $('category-words');
    const samples = engine.getCategorySamples(category);
    wordsDiv.innerHTML = '<div class="text-xs text-white/60 text-center py-4">Checking samples…</div>';

    const rows = [];
    for (const sample of samples) {
        let found = false;
        if (engine.profile === 'schemaorg' || engine.profile === 'w3c') {
            const iri = engine.normalizeIri(sample);
            const hit = await engine.query(`<${iri}> ?p ?o`, 1);
            found = hit.matches.length > 0;
            if (!found) {
                const labelHit = await engine.query(`?s ?p "${sample}"`, 1);
                found = labelHit.matches.length > 0;
            }
            if (!found) {
                const rdfsHit = await engine.query(`?s <http://www.w3.org/2000/01/rdf-schema#label> "${sample}"`, 1);
                found = rdfsHit.matches.length > 0;
            }
        } else {
            const hit = await engine.query(`?s ?p "${sample}"`, 1);
            found = hit.matches.length > 0;
        }
        rows.push(`
            <div class="flex items-center justify-between p-2 bg-white/5 hover:bg-white/10 rounded-xl cursor-pointer ${found ? '' : 'opacity-40'}"
                 onclick="quickLookup('${esc(sample)}')">
                <span class="text-sm">${esc(sample)}</span>
                <span class="text-xs text-white/40">${found ? 'in graph' : 'missing'}</span>
            </div>`);
    }
    wordsDiv.innerHTML = rows.join('');
};

window.showSparqlExample = function showSparqlExample(kind) {
    const templates = engine.profile === 'w3c'
        ? w3cSparqlTemplates()
        : (SPARQL_TEMPLATES[engine.profile] ?? SPARQL_TEMPLATES.wordnet);
    const query = templates[kind] ?? templates.wildcard;
    $('sparql-output').classList.remove('hidden');
    $('sparql-code').textContent = query;
    $('sparql-editor').value = query;
};

window.runSparqlQuery = async function runSparqlQuery() {
    const sparql = $('sparql-editor').value.trim();
    if (!sparql) return;
    $('sparql-output').classList.remove('hidden');
    $('sparql-code').textContent = sparql;

    setLoading(true, 'Scanning mounted .q42…');
    try {
        const result = await engine.querySparql(sparql, 100);
        const lines = result.matches.map(m =>
            `S: ${engine.labelFor(m.s)}  P: ${engine.labelFor(m.p)}  O: ${engine.labelFor(m.o)}`
        );
        $('sparql-results').textContent = lines.length
            ? `Pattern: ${result.bgp}\n\n${lines.join('\n')}\n\n(${result.matches.length} matches, ${result.vm_cycles} VM cycles)`
            : `Pattern: ${result.bgp}\n\nNo matches.`;
    } catch (e) {
        $('sparql-results').textContent = e.message || String(e);
    } finally {
        setLoading(false);
    }
};

function visualizeGraphFromLookup(lookup) {
    if (!lookup?.found || !lookup.entities.length) return;
    const entity = lookup.entities[0];
    const rel = entity.relations;
    const centerId = shortLabel(entity.iri) || lookup.term;

    graphNodes = [{ id: centerId, x: 500, y: 200, level: 0, color: '#ffffff' }];
    graphEdges = [];

    document.querySelectorAll('.graph-toggle').forEach(input => {
        if (!input.checked) return;
        const key = input.dataset.relKey;
        const toggle = ui().graphToggles.find(t => t.key === key);
        if (!toggle) return;
        const items = rel[key] ?? [];
        items.slice(0, 8).forEach((item, i) => {
            const id = shortLabel(item) || `${key}-${i}`;
            const angle = (i / Math.max(items.length, 1)) * Math.PI * 2;
            graphNodes.push({
                id,
                x: 500 + Math.cos(angle) * 180,
                y: 200 + Math.sin(angle) * 120,
                level: 1,
                color: toggle.color,
            });
            graphEdges.push({ from: centerId, to: id, color: toggle.color, type: key });
        });
    });
    drawGraph();
}

window.visualizeGraph = async function visualizeGraph() {
    const term = $('graph-term').value.trim();
    if (!term) return;
    $('entity-search').value = term;
    await lookupEntity();
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
        renderDatasetPicker();
        applyProfileUi();
        updateStats(stats);
        updateDatasetInfo(stats);
        setStatus(`${stats.label} · ${stats.blocks.toLocaleString()} blocks · WASM ready${formatOpfsCacheLabel(engine.vfs?.opfsCache)}`);
        watchOpfsCache(engine.vfs);
        const hints = datasetUiHints();
        showSparqlExample(hints.sparqlExamples[0]?.id ?? 'wildcard');
        $('entity-search').value = hints.defaultSearch;
        $('graph-term').value = hints.defaultSearch;
        await lookupEntity();
    } catch (e) {
        console.error(e);
        setStatus(e.message || 'Engine failed to load', false);
        $('stat-1').textContent = '!';
    } finally {
        setLoading(false);
    }
}

document.addEventListener('DOMContentLoaded', boot);

// Back-compat aliases for wordnet.html if it still loads this module
window.lookupWord = lookupEntity;