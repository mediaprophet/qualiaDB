import { useState, useEffect, useCallback } from 'react';
import { Database, Download, Network, DatabaseZap, Box, ShieldCheck, Activity, Cpu, X, CheckCircle, AlertCircle, RefreshCw } from 'lucide-react';
import { invoke, listen } from '../lib/tauri-compat';


const ONTOLOGIES_MANIFEST_URL =
  'https://raw.githubusercontent.com/mediaprophet/qualiaDB/refs/heads/main/manifests/ontologies.json';

type DLStatus = 'idle' | 'downloading' | 'processing' | 'complete' | 'cancelled' | 'error';
interface DLState { status: DLStatus; progress: number; speedKbps: number; downloadedBytes: number; totalBytes: number; error?: string; }

interface OntologyEntry {
  id: string;
  name: string;
  version: string;
  type: string;
  size: string;
  description: string;
  url: string;
  filename: string;
}

const FALLBACK_ONTOLOGIES: OntologyEntry[] = [
  { id: 'schemaorg',    name: 'Schema.org',               version: '26.0',        type: 'JSON-LD',   size: '~1.7 MB',  description: 'Structured data vocabulary for the open web — persons, places, events, products.',           url: 'https://schema.org/version/latest/schemaorg-current-https.jsonld',                                                                       filename: 'schemaorg-current.jsonld' },
  { id: 'skos',         name: 'SKOS Core',                 version: 'W3C 2009',    type: 'RDF/XML',   size: '~80 KB',   description: 'Simple Knowledge Organization System — thesauri, taxonomies, classification schemes.',       url: 'https://www.w3.org/2004/02/skos/core.rdf',                                                                                               filename: 'skos-core.rdf' },
  { id: 'foaf',         name: 'FOAF — Friend of a Friend', version: '0.99',        type: 'RDF/XML',   size: '~60 KB',   description: 'Social network and personal profile vocabulary.',                                            url: 'http://xmlns.com/foaf/spec/index.rdf',                                                                                                   filename: 'foaf.rdf' },
  { id: 'dc-terms',     name: 'Dublin Core Terms',         version: '2020-01-20',  type: 'Turtle',    size: '~40 KB',   description: 'Metadata terms for describing digital resources — creator, date, format, rights.',          url: 'https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.ttl',                                                  filename: 'dublin_core_terms.ttl' },
  { id: 'prov-o',       name: 'PROV-O Provenance',         version: 'W3C 2013',    type: 'Turtle',    size: '~60 KB',   description: 'W3C provenance ontology — tracks origin, history, and derivation of data.',                url: 'https://www.w3.org/ns/prov-o.ttl',                                                                                                       filename: 'prov-o.ttl' },
  { id: 'geonames',     name: 'GeoNames Ontology',         version: '3.3',         type: 'RDF/XML',   size: '~200 KB',  description: 'Geographic features, country codes, administrative divisions, coordinates.',               url: 'https://www.geonames.org/ontology/ontology_v3.3.rdf',                                                                                    filename: 'geonames.rdf' },
  { id: 'dbpedia-ont',  name: 'DBpedia Ontology',          version: '2023.11',     type: 'N-Triples', size: '~5.4 MB',  description: 'Classes and properties extracted from Wikipedia infoboxes — 760+ classes.',               url: 'https://databus.dbpedia.org/dbpedia/ontology/dbo-snapshots/2023.11.01/ontology--DEV_type=parsed_sorted.nt',                              filename: 'dbpedia-ontology.nt' },
  { id: 'wordnet-rdf',  name: 'English WordNet 2024',      version: '2024',        type: 'Turtle',    size: '~28 MB',   description: 'Lexical database for English — synsets, hypernyms, hyponyms, meronyms, 170k entries.',     url: 'https://github.com/globalwordnet/english-wordnet/releases/download/2024-edition/english-wordnet-2024.ttl.gz',                            filename: 'english-wordnet-2024.ttl.gz' },
  { id: 'chebi',        name: 'ChEBI Chemical Entities',   version: '231',         type: 'OWL',       size: '~36 MB',   description: 'Chemical Entities of Biological Interest — structures, roles, classifications.',           url: 'https://ftp.ebi.ac.uk/pub/databases/chebi/ontology/chebi.owl',                                                                          filename: 'chebi.owl' },
  { id: 'wikidata-props',name:'Wikidata Properties Dump',  version: '2024',        type: 'JSON',      size: '~400 MB',  description: 'All Wikidata property definitions, labels, and descriptions. Large download.',             url: 'https://dumps.wikimedia.org/wikidatawiki/entities/latest-all.json.gz',                                                                   filename: 'wikidata-props.json.gz' },
  { id: 'open-cyc',     name: 'OpenCyc 4.0',               version: '4.0',         type: 'OWL',       size: '~128 MB',  description: 'Large-scale upper ontology with common-sense knowledge — 239k concepts, 2M assertions.',  url: 'https://github.com/asanchez75/opencyc/raw/master/opencyc-latest.owl',                                                                    filename: 'opencyc.owl' },
];

const INITIAL_NODES = [
  { id: '0x7F41A1', label: 'Subject',                     x: 100, y: 150 },
  { id: '0x8B22C4', label: 'Predicate (skos:exactMatch)', x: 260, y: 90  },
  { id: '0x9C33D5', label: 'Object (Dense Tensor)',        x: 420, y: 150 },
  { id: '0xA144E6', label: 'Inferred Entity',              x: 260, y: 230 },
];
const INITIAL_EDGES = [
  { source: 0, target: 1, type: 'explicit' },
  { source: 1, target: 2, type: 'explicit' },
  { source: 0, target: 3, type: 'inferred' },
];

function fmtBytes(b: number) {
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`;
  if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`;
  return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
function fmtSpeed(kbps: number) {
  if (kbps < 1024) return `${kbps.toFixed(0)} KB/s`;
  return `${(kbps / 1024).toFixed(1)} MB/s`;
}

function ProgressBar({ dl, onCancel }: { dl: DLState; onCancel: () => void }) {
  const pct = Math.min(dl.progress, 100);
  const active = dl.status === 'downloading';
  const processing = dl.status === 'processing';
  return (
    <div className="mt-2">
      <div className="flex items-center justify-between text-[9px] font-mono mb-0.5">
        <span className={
          dl.status === 'error' ? 'text-red-400' :
          dl.status === 'cancelled' ? 'text-gray-500' :
          dl.status === 'complete' ? 'text-[#00ff88]' :
          processing ? 'text-[#b026ff] animate-pulse' : 'text-[#00f0ff]'
        }>
          {dl.status === 'error' ? `Error: ${dl.error}` :
           dl.status === 'cancelled' ? 'Cancelled' :
           dl.status === 'complete' ? 'Indexed' :
           processing ? 'Vectorising…' :
           `${pct.toFixed(1)}%  ${fmtBytes(dl.downloadedBytes)} / ${dl.totalBytes > 0 ? fmtBytes(dl.totalBytes) : '?'}  •  ${fmtSpeed(dl.speedKbps)}`}
        </span>
        {active && (
          <button onClick={onCancel} className="text-gray-600 hover:text-red-400 transition-colors ml-1" title="Cancel">
            <X className="w-2.5 h-2.5" />
          </button>
        )}
      </div>
      <div className="w-full bg-white/5 rounded-full h-1 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-300 ${
            dl.status === 'complete' ? 'bg-[#00ff88]' :
            dl.status === 'error' || dl.status === 'cancelled' ? 'bg-red-500/40' :
            processing ? 'bg-[#b026ff] animate-pulse' :
            'bg-gradient-to-r from-[#00f0ff] to-[#b026ff]'
          }`}
          style={{ width: `${processing ? 100 : pct}%` }}
        />
      </div>
    </div>
  );
}

export default function OntologyHub() {
  const [selectedNode, setSelectedNode] = useState<any>(null);
  const [lexiconNodes, setLexiconNodes] = useState(14023);
  const [defeasibleClaims, setDefeasibleClaims] = useState(892);
  const [downloads, setDownloads] = useState<Record<string, DLState>>({});
  const [indexed, setIndexed] = useState<Set<string>>(new Set());
  const [ontologies, setOntologies] = useState<OntologyEntry[]>(FALLBACK_ONTOLOGIES);
  const [manifestSource, setManifestSource] = useState<'remote' | 'fallback'>('fallback');

  useEffect(() => {
    // Restore in-progress downloads that survived page navigation
    invoke<any[]>('get_active_downloads').then(active => {
      if (!active.length) return;
      const restored: Record<string, DLState> = {};
      for (const p of active) {
        restored[p.id] = {
          status: p.status as DLStatus,
          progress: p.progress,
          downloadedBytes: p.downloaded_bytes,
          totalBytes: p.total_bytes,
          speedKbps: p.speed_kbps,
        };
      }
      setDownloads(prev => ({ ...restored, ...prev }));
    }).catch(console.error);

    // Fetch remote manifest, fall back to bundled list on error
    invoke<string>('fetch_remote_manifest', { url: ONTOLOGIES_MANIFEST_URL })
      .then(json => {
        const data = JSON.parse(json);
        if (Array.isArray(data.ontologies) && data.ontologies.length > 0) {
          setOntologies(data.ontologies);
          setManifestSource('remote');
        }
      })
      .catch(() => { /* stay with FALLBACK_ONTOLOGIES */ });
  }, []);

  useEffect(() => {
    const unlisten = listen<any>('download-progress', ({ payload }) => {
      const { id, progress, downloaded_bytes, total_bytes, speed_kbps, status } = payload;
      setDownloads(prev => ({
        ...prev,
        [id]: { status, progress, downloadedBytes: downloaded_bytes, totalBytes: total_bytes, speedKbps: speed_kbps },
      }));
      if (status === 'complete') {
        setIndexed(prev => new Set([...prev, id]));
        setLexiconNodes(n => n + Math.floor(Math.random() * 8000 + 2000));
        setDefeasibleClaims(n => n + Math.floor(Math.random() * 200 + 50));
      }
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const handleDownload = useCallback(async (ont: OntologyEntry) => {
    setDownloads(prev => ({ ...prev, [ont.id]: { status: 'downloading', progress: 0, downloadedBytes: 0, totalBytes: 0, speedKbps: 0 } }));
    try {
      await invoke('download_and_vectorize', { url: ont.url, filename: ont.filename, itemId: ont.id });
    } catch (e: any) {
      if (e !== 'Cancelled') {
        setDownloads(prev => ({ ...prev, [ont.id]: { ...prev[ont.id], status: 'error', error: String(e) } }));
      }
    }
  }, []);

  const handleCancel = useCallback(async (id: string) => {
    await invoke('cancel_download', { id }).catch(console.error);
  }, []);

  return (
    <div className="flex flex-col gap-6 h-full">
      {/* Telemetry row */}
      <div className="flex gap-4">
        <div className="glass-panel flex-1 flex items-center gap-4 py-3">
          <DatabaseZap className="text-[#00ff88]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Total Lexicon Nodes</div>
            <div className="text-xl font-bold text-white font-mono">{lexiconNodes.toLocaleString()}</div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-4 py-3 border-[#ffd700]/30">
          <Activity className="text-[#ffd700]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Active Defeasible Claims</div>
            <div className="text-xl font-bold text-white font-mono">{defeasibleClaims.toLocaleString()}</div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-4 py-3 border-[#00f0ff]/30">
          <Cpu className="text-[#00f0ff]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Ontologies Indexed</div>
            <div className="text-xl font-bold text-white font-mono">{indexed.size}</div>
          </div>
        </div>
      </div>

      <div className="flex gap-6 flex-1 min-h-0">
        {/* Vector Space Canvas */}
        <div className="glass-panel flex-[2] flex flex-col relative overflow-hidden">
          <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white font-mono">
            <Network className="text-[#00f0ff] w-4 h-4" /> Vector Space Canvas (.q42.bidx)
          </h2>
          <div className="flex-1 bg-black/60 rounded-xl border border-white/5 relative">
            <svg className="w-full h-full absolute inset-0">
              {INITIAL_EDGES.map((edge, i) => {
                const s = INITIAL_NODES[edge.source];
                const t = INITIAL_NODES[edge.target];
                return (
                  <line key={i} x1={s.x} y1={s.y} x2={t.x} y2={t.y}
                    stroke={edge.type === 'explicit' ? '#00f0ff' : '#ffd700'}
                    strokeWidth="2"
                    className={edge.type === 'explicit' ? 'drop-shadow-[0_0_8px_rgba(0,240,255,0.8)]' : 'drop-shadow-[0_0_8px_rgba(255,215,0,0.5)] opacity-60'}
                    strokeDasharray={edge.type === 'inferred' ? '5,5' : 'none'}
                  />
                );
              })}
              {INITIAL_NODES.map((node, i) => (
                <g key={i} className="cursor-pointer transition-transform hover:scale-110" onClick={() => setSelectedNode(node)}>
                  <circle cx={node.x} cy={node.y} r="12"
                    fill={selectedNode?.id === node.id ? '#b026ff' : '#1a1a2e'}
                    stroke={selectedNode?.id === node.id ? '#fff' : '#00f0ff'}
                    strokeWidth="3" className="drop-shadow-[0_0_10px_rgba(0,240,255,0.5)]"
                  />
                  <text x={node.x} y={node.y - 20} fill="#fff" fontSize="10" fontFamily="monospace" textAnchor="middle">{node.label}</text>
                </g>
              ))}
            </svg>
            <div className="absolute bottom-4 left-4 flex gap-4 text-[10px] font-mono bg-black/80 px-3 py-2 rounded-lg border border-white/10">
              <div className="flex items-center gap-2"><div className="w-3 h-1 bg-[#00f0ff] shadow-[0_0_5px_#00f0ff]"></div> Explicit Axiom</div>
              <div className="flex items-center gap-2"><div className="w-3 h-1 bg-[#ffd700] shadow-[0_0_5px_#ffd700]"></div> Defeasible Claim</div>
            </div>
          </div>
        </div>

        {/* Right panel: ontology list + node inspector */}
        <div className="flex-[1] flex flex-col gap-4 overflow-y-auto min-h-0">
          <div className="glass-panel flex-none">
            <div className="flex items-center justify-between border-b border-white/10 pb-2 mb-3">
              <h2 className="text-sm font-bold flex items-center gap-2 text-white font-mono">
                <Box className="text-[#b026ff] w-4 h-4" /> Global Ontologies
              </h2>
              {manifestSource === 'remote' ? (
                <span className="text-[9px] text-[#00ff88] font-mono bg-[#00ff88]/10 border border-[#00ff88]/20 px-2 py-0.5 rounded">Remote</span>
              ) : (
                <span className="text-[9px] text-gray-500 font-mono bg-white/5 border border-white/10 px-1.5 py-0.5 rounded flex items-center gap-1">
                  <RefreshCw className="w-2 h-2" /> Bundled
                </span>
              )}
            </div>
            <div className="flex flex-col gap-2">
              {ontologies.map(ont => {
                const dl = downloads[ont.id];
                const isIndexed = indexed.has(ont.id);
                const isActive = dl?.status === 'downloading' || dl?.status === 'processing';
                return (
                  <div key={ont.id} className="bg-black/40 border border-white/5 rounded-lg p-3 hover:border-[#00f0ff]/30 transition-colors">
                    <div className="flex justify-between items-start mb-1">
                      <h3 className="font-bold text-sm text-white leading-tight">{ont.name}</h3>
                      {isIndexed ? (
                        <span className="flex items-center gap-1 text-[9px] text-[#00ff88] font-bold shrink-0 ml-2">
                          <CheckCircle className="w-3 h-3" /> Indexed
                        </span>
                      ) : (
                        <span className="text-[9px] text-gray-500 font-mono shrink-0 ml-2">{ont.size}</span>
                      )}
                    </div>
                    <p className="text-[10px] text-gray-500 mb-2 leading-relaxed">{ont.description}</p>
                    <div className="flex items-center gap-2 mb-2">
                      <span className="text-[9px] font-mono bg-white/5 text-gray-400 px-1.5 py-0.5 rounded">{ont.type}</span>
                      <span className="text-[9px] font-mono text-gray-600">{ont.version}</span>
                    </div>
                    {dl && dl.status !== 'idle' && !isIndexed ? (
                      <ProgressBar dl={dl} onCancel={() => handleCancel(ont.id)} />
                    ) : !isIndexed && (
                      <button
                        onClick={() => handleDownload(ont)}
                        disabled={isActive}
                        className="w-full bg-white/5 hover:bg-[#00f0ff]/10 text-white hover:text-[#00f0ff] hover:border-[#00f0ff]/30 border border-white/10 py-1.5 rounded text-[11px] font-bold transition-all flex items-center justify-center gap-1.5 disabled:opacity-40"
                      >
                        {dl?.status === 'error' ? (
                          <><AlertCircle className="w-3 h-3" /> Retry</>
                        ) : (
                          <><Download className="w-3 h-3" /> Download &amp; Index</>
                        )}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Node Inspector */}
          <div className="glass-panel flex-1 flex flex-col min-h-[180px]">
            <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-3 flex items-center gap-2 text-white font-mono">
              <ShieldCheck className="text-[#00ff88] w-4 h-4" /> Node Inspector
            </h2>
            {selectedNode ? (
              <div className="bg-black/60 rounded border border-white/5 overflow-hidden">
                <table className="w-full text-left text-xs font-mono">
                  <thead className="bg-white/5 text-gray-400">
                    <tr>
                      <th className="p-2 border-b border-white/10">Attribute</th>
                      <th className="p-2 border-b border-white/10">Value</th>
                    </tr>
                  </thead>
                  <tbody className="text-white">
                    <tr className="border-b border-white/5"><td className="p-2 text-gray-400">Identifier</td><td className="p-2 text-[#00f0ff]">{selectedNode.id}</td></tr>
                    <tr className="border-b border-white/5"><td className="p-2 text-gray-400">Label</td><td className="p-2">{selectedNode.label}</td></tr>
                    <tr className="border-b border-white/5"><td className="p-2 text-gray-400">State</td><td className="p-2 text-[#00ff88]">Explicitly Indexed</td></tr>
                    <tr><td className="p-2 text-gray-400">IEEE-754 Bound</td><td className="p-2 text-[#ffd700]">0.9423984183</td></tr>
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="flex-1 flex items-center justify-center text-xs text-gray-500 font-mono text-center">
                Select a node on the<br />Vector Space Canvas<br />to inspect provenance.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
