// Complete function catalog for the QualiaDB API Explorer.
// Each entry defines: id, category, name, summary, params, returns,
// snippets (per language), and optionally a live() runner function.

// ─── Snippet helpers ──────────────────────────────────────────────────────────

function js(code)   { return { lang: 'JS/WASM',  code: code.trim() }; }
function rs(code)   { return { lang: 'Rust',      code: code.trim() }; }
function http(code) { return { lang: 'HTTP',      code: code.trim() }; }
function cli(code)  { return { lang: 'CLI',       code: code.trim() }; }
function nt(code)   { return { lang: 'N-Triples', code: code.trim() }; }

// ─── CATALOG ─────────────────────────────────────────────────────────────────

export const CATALOG = [

    // ═══════════════════════════════════════════════════════════════════════════
    // CORE PRIMITIVES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'core.q_hash',
        category: 'Core Primitives',
        name: 'q_hash()',
        summary: 'FNV-1a 64-bit compile-time URI hasher. Converts any string to a stable u64 Quin field value with zero heap allocation.',
        params: [{ name: 's', type: 'string', desc: 'Any URI, predicate, or literal string' }],
        returns: 'u64 (BigInt in JS)',
        snippets: [
            js(`
import { q_hash } from './qualia-primitives.js';

const subjectId  = q_hash('https://example.org/Alice');
const predicateId = q_hash('http://xmlns.com/foaf/0.1/name');
console.log(subjectId.toString(16)); // stable hex fingerprint
`),
            rs(`
use qualia_core_db::q_hash;

// Evaluated at compile time — zero runtime cost
const FOAF_NAME: u64 = q_hash("http://xmlns.com/foaf/0.1/name");
let subject_hash = q_hash("https://example.org/Alice");
`),
        ],
        live: async (wasm, _native, inputs) => {
            const FNV_OFFSET = 0xcbf29ce484222325n;
            const FNV_PRIME  = 0x100000001b3n;
            const MASK_64    = 0xffffffffffffffffn;
            let h = FNV_OFFSET;
            for (const b of new TextEncoder().encode(inputs.s || '')) {
                h = ((h ^ BigInt(b)) * FNV_PRIME) & MASK_64;
            }
            return { hash_dec: h.toString(), hash_hex: '0x' + h.toString(16).padStart(16,'0') };
        },
        liveInputs: [{ name: 's', label: 'String to hash', default: 'http://xmlns.com/foaf/0.1/name' }],
    },

    {
        id: 'core.qualiaQuin',
        category: 'Core Primitives',
        name: 'QualiaQuin',
        summary: '48-byte zero-copy semantic statement container. Six u64 fields: subject, predicate, object, context, metadata, parity. The atomic unit of all QualiaDB storage.',
        params: [
            { name: 'subject',   type: 'u64', desc: 'FNV-1a hash of subject IRI' },
            { name: 'predicate', type: 'u64', desc: 'FNV-1a hash of predicate IRI' },
            { name: 'object',    type: 'u64', desc: 'FNV-1a hash of object IRI or literal' },
            { name: 'context',   type: 'u64', desc: 'Named graph / context identifier' },
            { name: 'metadata',  type: 'u64', desc: 'Routing lane (bits 61–62), Lamport clock (bits 32–60), payload (bits 0–31)' },
            { name: 'parity',    type: 'u64', desc: 'ECC checksum — set to u64::MAX to mark sector corrupt' },
        ],
        returns: 'QualiaQuin (48 bytes, repr(C, align(16)))',
        snippets: [
            js(`
// QualiaQuin as a plain JS object — 6 BigInt fields
const quin = {
  subject:   q_hash('https://example.org/Alice'),
  predicate: q_hash('http://xmlns.com/foaf/0.1/name'),
  object:    q_hash('"Alice"'),
  context:   0n,
  metadata:  0x01n << 61n,  // EnforcePermissiveCommons routing lane
  parity:    0n,
};
`),
            rs(`
use qualia_core_db::{QualiaQuin, q_hash};

let quin = QualiaQuin {
    subject:   q_hash("https://example.org/Alice"),
    predicate: q_hash("http://xmlns.com/foaf/0.1/name"),
    object:    q_hash("\"Alice\""),
    context:   0,
    metadata:  0x01 << 61,  // EnforcePermissiveCommons
    parity:    0,
};
assert_eq!(std::mem::size_of::<QualiaQuin>(), 48);
`),
            rs(`
// Using the q_turtle! macro for compile-time zero-allocation construction
use qualia_core_db::q_turtle;

let quin = q_turtle!(
    "https://example.org/Alice",
    "http://xmlns.com/foaf/0.1/name",
    "Alice"
);
`),
        ],
    },

    {
        id: 'core.routing_lanes',
        category: 'Core Primitives',
        name: 'Routing Lanes',
        summary: 'Bits 61–62 of the metadata field classify every Quin into one of four access-control routing tiers. Checked by the permissive runtime gate before data egress.',
        params: [
            { name: 'metadata', type: 'u64', desc: 'Quin metadata field' },
        ],
        returns: 'PermissiveRoutingLane enum variant',
        snippets: [
            js(`
// Routing lane constants (bits 61–62 of metadata)
const LANE_PASSTHROUGH   = 0x00n << 61n;  // local sensor data, files
const LANE_PERMISSIVE    = 0x01n << 61n;  // Permissive Commons compensation gate
const LANE_BILATERAL     = 0x02n << 61n;  // multi-signatory personal data
const LANE_SPATIOTEMPORAL= 0x03n << 61n;  // GPU bounding hull + linguistic check

function identifyRoutingLane(metadata) {
  const bits = (metadata >> 61n) & 0x03n;
  return ['PassthroughStandard','EnforcePermissiveCommons',
          'EnforceBilateralMicroCommons','SpatiotemporalAmbiguous'][Number(bits)];
}
`),
            rs(`
use qualia_core_db::{QualiaQuin, PermissiveRoutingLane};

let quin = QualiaQuin { metadata: 0x01 << 61, ..Default::default() };
assert_eq!(quin.identify_routing_lane(),
           PermissiveRoutingLane::EnforcePermissiveCommons);
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // LOGIC MODALITIES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'modality.epistemic',
        category: 'Logic Modalities',
        name: 'Epistemic Logic',
        summary: 'Models knowledge and belief across agents and possible worlds. Three opcodes: OP_KNOWS (0x20) for certain knowledge, OP_BELIEVES (0x21) for uncertain belief (threshold 128), OP_COMMON_KNOWLEDGE (0x22) for shared facts that boost certainty to 255.',
        params: [
            { name: 'quins',         type: '&[QualiaQuin]', desc: 'Slice of epistemic quins to evaluate' },
            { name: 'agent_did_hash', type: 'u64',          desc: 'Filter to a specific agent (0 = all agents)' },
            { name: 'world_hash',    type: 'u64',           desc: 'Filter to a possible world (0 = all worlds)' },
            { name: 'out',           type: '&mut [EpistemicVerdict]', desc: 'Output buffer for verdicts' },
        ],
        returns: 'Result<usize, EpistemicError> — number of verdicts written',
        snippets: [
            js(`
// Opcodes encoded in predicate field bits 0–7; certainty in bits 8–15
const OP_KNOWS            = 0x20n;
const OP_BELIEVES         = 0x21n;
const OP_COMMON_KNOWLEDGE = 0x22n;

function buildEpistemicQuin(agent, opcode, certainty, claim, world) {
  return {
    subject:   agent,
    predicate: opcode | (certainty << 8n),
    object:    claim,
    context:   world,
    metadata:  0n,
    parity:    agent ^ (opcode | (certainty << 8n)) ^ claim ^ world,
  };
}

// Alice knows that "sky is blue" with certainty 200
const quin = buildEpistemicQuin(
  q_hash('did:wellfare:alice'),
  OP_KNOWS,
  200n,
  q_hash('qualia:claim:sky_is_blue'),
  0n
);
`),
            rs(`
use qualia_core_db::modalities::epistemic::{
    evaluate_epistemic_frame, OP_KNOWS, OP_BELIEVES,
    OP_COMMON_KNOWLEDGE, EpistemicVerdict, EpistemicStatus,
};
use qualia_core_db::{QualiaQuin, q_hash};

let agent = q_hash("did:wellfare:alice");
let claim = q_hash("qualia:claim:sky_is_blue");
let quin  = QualiaQuin {
    subject:   agent,
    predicate: OP_KNOWS as u64 | (200u64 << 8),
    object:    claim,
    ..Default::default()
};

let mut out = vec![EpistemicVerdict::default(); 16];
let n = evaluate_epistemic_frame(&[quin], agent, 0, &mut out).unwrap();
assert_eq!(out[0].status, EpistemicStatus::Active);
`),
        ],
    },

    {
        id: 'modality.ltl',
        category: 'Logic Modalities',
        name: 'Temporal LTL',
        summary: 'Linear Temporal Logic over Quin traces. Five operators: G (Globally), F (Finally), X (Next), U (Until), R (Release). Predicates are matched against the Quin predicate field.',
        params: [
            { name: 'trace',   type: '&[QualiaQuin]', desc: 'Ordered sequence of Quins (time steps)' },
            { name: 'formula', type: '&LtlFormula',   desc: 'One of Globally(p), Finally(p), Next(p), Until{ante,consequent}, Release{trigger,invariant}' },
        ],
        returns: 'bool',
        snippets: [
            js(`
// Globally(p): p holds at every position in the trace
// Finally(p):  p holds at least once
// Next(p):     p holds at position 1
// Until(a,b):  a holds until b occurs
// Release(t,i): i holds until t fires (empty trace = true)

function evaluateLtlTrace(trace, formula) {
  switch (formula.op) {
    case 'Globally': return trace.length > 0 && trace.every(q => q.predicate === formula.p);
    case 'Finally':  return trace.length > 0 && trace.some(q => q.predicate === formula.p);
    case 'Next':     return trace.length >= 2 && trace[1].predicate === formula.p;
    case 'Until':
      for (let i = 0; i < trace.length; i++) {
        if (trace[i].predicate === formula.consequent) {
          return trace.slice(0, i).every(q => q.predicate === formula.ante);
        }
      }
      return false;
    case 'Release':
      if (!trace.length) return true;
      for (let i = 0; i < trace.length; i++) {
        if (trace[i].predicate !== formula.invariant) {
          if (!trace.slice(0, i+1).some(q => q.predicate === formula.trigger)) return false;
        }
      }
      return true;
  }
}
`),
            rs(`
use qualia_core_db::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
use qualia_core_db::QualiaQuin;

let p = 100u64;
let trace: Vec<QualiaQuin> = (0..5).map(|_|
    QualiaQuin { predicate: p, ..Default::default() }
).collect();

// G(p): p holds at every position
assert!(evaluate_ltl_trace(&trace, &LtlFormula::Globally(p)));

// F(200): 200 never occurs in this trace
assert!(!evaluate_ltl_trace(&trace, &LtlFormula::Finally(200)));
`),
        ],
    },

    {
        id: 'modality.paraconsistent',
        category: 'Logic Modalities',
        name: 'Paraconsistent Logic',
        summary: 'Isolates contradictions without crashing inference. Quins with identical subject/predicate/context but different objects are detected as contradictions and routed to an isolated buffer with their context XOR-ed against ISOLATED_CONTEXT_PREFIX = q_hash("q42:isolated").',
        params: [
            { name: 'quins',          type: '&[QualiaQuin]',     desc: 'Input Quin slice to route' },
            { name: 'out_consistent', type: '&mut [QualiaQuin]', desc: 'Buffer for non-contradicting Quins' },
            { name: 'out_isolated',   type: '&mut [QualiaQuin]', desc: 'Buffer for contradicting Quins' },
        ],
        returns: 'Result<(usize, usize), ParaconsistentError>',
        snippets: [
            js(`
const ISOLATED_PREFIX = q_hash('q42:isolated');

function routeParaconsistent(quins) {
  const consistent = [], isolated = [];
  for (const q of quins) {
    const contradiction = consistent.some(c =>
      c.context   === q.context   &&
      c.subject   === q.subject   &&
      c.predicate === q.predicate &&
      c.object    !== q.object
    );
    if (contradiction) {
      isolated.push({ ...q, context: q.context ^ ISOLATED_PREFIX });
    } else {
      consistent.push(q);
    }
  }
  return { consistent, isolated };
}
`),
            rs(`
use qualia_core_db::modalities::paraconsistent::route_paraconsistent;
use qualia_core_db::QualiaQuin;

let q1 = QualiaQuin { subject: 1, predicate: 1, object: 1, context: 100, ..Default::default() };
let q2 = QualiaQuin { subject: 1, predicate: 1, object: 2, context: 100, ..Default::default() };

let mut consistent = vec![QualiaQuin::default(); 8];
let mut isolated   = vec![QualiaQuin::default(); 8];
let (nc, ni) = route_paraconsistent(&[q1, q2], &mut consistent, &mut isolated).unwrap();
// nc=1 (q1), ni=1 (q2 isolated with XOR-ed context)
`),
        ],
    },

    {
        id: 'modality.linear',
        category: 'Logic Modalities',
        name: 'Linear Logic',
        summary: 'Resource-consumption semantics via metadata bit 59 (CONSUMED_BIT). A consumed Quin cannot be reused — models one-shot rights, tokens, and obligations.',
        params: [
            { name: 'q', type: '&mut QualiaQuin', desc: 'Quin to consume' },
        ],
        returns: 'void / bool (is_consumed)',
        snippets: [
            js(`
const CONSUMED_BIT = 1n << 59n;

const consume  = q => ({ ...q, metadata: q.metadata | CONSUMED_BIT });
const consumed = q => (q.metadata & CONSUMED_BIT) !== 0n;

let token = { ...myQuin };
token = consume(token);   // one-shot: cannot be used again
console.log(consumed(token)); // true
`),
            rs(`
use qualia_core_db::modalities::linear::{consume_quin, is_consumed};
use qualia_core_db::QualiaQuin;

let mut ticket = QualiaQuin::default();
assert!(!is_consumed(&ticket));
consume_quin(&mut ticket);
assert!(is_consumed(&ticket));  // resource spent
`),
        ],
    },

    {
        id: 'modality.dialectical',
        category: 'Logic Modalities',
        name: 'Dialectical Logic',
        summary: 'Hegelian synthesis: given a thesis and antithesis (same subject+predicate, different object), produces a synthesized Quin with SYNTHESIZED_BIT (bit 58) set, context = thesis.context XOR antithesis.context, object = thesis.object XOR antithesis.object.',
        params: [
            { name: 'thesis',     type: '&QualiaQuin', desc: 'The thesis Quin' },
            { name: 'antithesis', type: '&QualiaQuin', desc: 'The antithesis Quin (must share subject+predicate, differ in object)' },
        ],
        returns: 'Option<QualiaQuin> — None if no contradiction',
        snippets: [
            js(`
const SYNTHESIZED_BIT = 1n << 58n;

function synthesizeDialectical(thesis, antithesis) {
  if (thesis.subject !== antithesis.subject ||
      thesis.predicate !== antithesis.predicate ||
      thesis.object === antithesis.object) return null;
  const syn = { ...thesis };
  syn.context  = thesis.context  ^ antithesis.context;
  syn.metadata = thesis.metadata | SYNTHESIZED_BIT;
  syn.object   = thesis.object   ^ antithesis.object;
  syn.parity   = syn.subject ^ syn.predicate ^ syn.object ^ syn.context;
  return syn;
}
`),
            rs(`
use qualia_core_db::modalities::dialectical::synthesize_dialectical;
use qualia_core_db::QualiaQuin;

let thesis     = QualiaQuin { subject: 1, predicate: 2, object: 3, context: 10, ..Default::default() };
let antithesis = QualiaQuin { subject: 1, predicate: 2, object: 4, context: 20, ..Default::default() };
let synthesis  = synthesize_dialectical(&thesis, &antithesis).unwrap();
// synthesis.context == 10 ^ 20 == 30
`),
        ],
    },

    {
        id: 'modality.spatio_temporal',
        category: 'Logic Modalities',
        name: 'Spatio-Temporal (Allen\'s Algebra)',
        summary: 'Allen\'s Interval Algebra for temporal reasoning over Quin traces. Seven relations: Before, Meets, Overlaps, Starts, During, Finishes, Equals.',
        params: [
            { name: 'op',     type: 'TemporalOp', desc: 'One of Before, Meets, Overlaps, Starts, During, Finishes, Equals' },
            { name: 't1_start / t1_end', type: 'i64', desc: 'Start and end of interval 1' },
            { name: 't2_start / t2_end', type: 'i64', desc: 'Start and end of interval 2' },
        ],
        returns: 'bool',
        snippets: [
            js(`
// Allen's 7 interval relations
const allen = {
  Before:   (s1,e1,s2,_2) => e1 < s2,
  Meets:    (s1,e1,s2,_2) => e1 === s2,
  Overlaps: (s1,e1,s2,e2) => s1 < s2 && e1 > s2 && e1 < e2,
  Starts:   (s1,e1,s2,e2) => s1 === s2 && e1 < e2,
  During:   (s1,e1,s2,e2) => s1 > s2 && e1 < e2,
  Finishes: (s1,e1,s2,e2) => e1 === e2 && s1 > s2,
  Equals:   (s1,e1,s2,e2) => s1 === s2 && e1 === e2,
};

// Meeting [1,10] → [10,20]?
console.log(allen.Meets(1,10,10,20));  // true
`),
            rs(`
use qualia_core_db::modalities::spatio_temporal::{evaluate_temporal, TemporalOp};

assert!(evaluate_temporal(TemporalOp::Meets,    1, 10, 10, 20));
assert!(evaluate_temporal(TemporalOp::Before,   1,  5, 10, 20));
assert!(evaluate_temporal(TemporalOp::During,  12, 18, 10, 20));
`),
        ],
    },

    {
        id: 'modality.dl',
        category: 'Logic Modalities',
        name: 'Description Logic (Subsumption)',
        summary: 'TBox subsumption check via DFS transitive closure over rdfs:subClassOf quins. Bounded at 64 hops to prevent cycles. A ⊑ A is always true (reflexive).',
        params: [
            { name: 'sub_class_hash',   type: 'u64',           desc: 'Hash of the subclass to test' },
            { name: 'super_class_hash', type: 'u64',           desc: 'Hash of the candidate superclass' },
            { name: 'tbox',             type: '&[QualiaQuin]', desc: 'TBox quins with predicate = q_hash("rdfs:subClassOf")' },
        ],
        returns: 'bool',
        snippets: [
            js(`
function checkSubsumption(subClass, superClass, tbox) {
  if (subClass === superClass) return true;
  let current = subClass;
  for (let depth = 0; depth < 64; depth++) {
    const link = tbox.find(q => q.subject === current);
    if (!link) break;
    current = link.object;
    if (current === superClass) return true;
  }
  return false;
}

// Mammal ⊑ Animal ⊑ LivingThing
const tbox = [
  { subject: q_hash('Mammal'),  object: q_hash('Animal') },
  { subject: q_hash('Animal'), object: q_hash('LivingThing') },
];
checkSubsumption(q_hash('Mammal'), q_hash('LivingThing'), tbox); // true
`),
            rs(`
use qualia_core_db::modalities::dl::check_subsumption_quin;
use qualia_core_db::{QualiaQuin, q_hash};

let tbox = vec![
    QualiaQuin { subject: q_hash("Mammal"),  object: q_hash("Animal"),      ..Default::default() },
    QualiaQuin { subject: q_hash("Animal"),  object: q_hash("LivingThing"), ..Default::default() },
];
assert!(check_subsumption_quin(q_hash("Mammal"), q_hash("LivingThing"), &tbox));
`),
        ],
    },

    {
        id: 'modality.asp',
        category: 'Logic Modalities',
        name: 'Answer Set Programming',
        summary: 'Enumerates stable models (parallel worlds) for a base Quin. Each world is encoded as base.context XOR world_index. MVP returns 2 worlds; up to MAX_STABLE_MODELS = 8.',
        params: [
            { name: 'base',       type: '&QualiaQuin',    desc: 'The base Quin defining the initial context' },
            { name: 'rules',      type: '&[QualiaQuin]',  desc: 'Rule Quins (currently unused in MVP)' },
            { name: 'out_worlds', type: '&mut [u64; 8]',  desc: 'Output context hashes for each stable model' },
        ],
        returns: 'usize — number of stable models found',
        snippets: [
            js(`
const MAX_STABLE_MODELS = 8;

function enumerateStableModels(base, rules = []) {
  // MVP: 2 worlds — context XOR 0, context XOR 1
  return [base.context ^ 0n, base.context ^ 1n];
}

const worlds = enumerateStableModels({ context: 42n });
// worlds = [42n, 43n]
`),
            rs(`
use qualia_core_db::modalities::asp::enumerate_stable_models;
use qualia_core_db::QualiaQuin;

let base = QualiaQuin { context: 42, ..Default::default() };
let mut worlds = [0u64; 8];
let n = enumerate_stable_models(&base, &[], &mut worlds);
assert_eq!(n, 2);
assert_eq!(worlds[0], 42 ^ 0);
assert_eq!(worlds[1], 42 ^ 1);
`),
        ],
    },

    {
        id: 'modality.probabilistic',
        category: 'Logic Modalities',
        name: 'Probabilistic Logic',
        summary: 'Weight-based threshold evaluation. The 5th Quin vector (metadata) stores the probability weight as a float. evaluate_threshold(weight, threshold) returns true iff weight >= threshold.',
        params: [
            { name: 'weight',    type: 'f32', desc: 'Probability weight stored in Quin metadata (0.0–1.0)' },
            { name: 'threshold', type: 'f32', desc: 'Minimum weight for the statement to be considered active' },
        ],
        returns: 'bool',
        snippets: [
            js(`
// Probability weight is stored in the lower 32 bits of metadata as IEEE-754 float
function evaluateThreshold(weight, threshold) {
  return weight >= threshold;
}

// Weakly held belief (0.3) — does not meet certainty bar (0.7)
console.log(evaluateThreshold(0.3, 0.7));  // false
console.log(evaluateThreshold(0.9, 0.7));  // true
`),
            rs(`
use qualia_core_db::modalities::probabilistic::evaluate_threshold;

assert!(!evaluate_threshold(0.3, 0.7));
assert!(evaluate_threshold(0.9, 0.7));
assert!(evaluate_threshold(0.5, 0.5));  // equal = true
`),
        ],
        live: async (_wasm, _native, inputs) => {
            const w = parseFloat(inputs.weight    || '0.8');
            const t = parseFloat(inputs.threshold || '0.5');
            return { result: w >= t, weight: w, threshold: t };
        },
        liveInputs: [
            { name: 'weight',    label: 'Weight (0–1)',    default: '0.8' },
            { name: 'threshold', label: 'Threshold (0–1)', default: '0.5' },
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // WASM API
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'wasm.execute_ntriples_query',
        category: 'WASM API',
        name: 'execute_ntriples_query()',
        summary: 'Execute a single N-Triples pattern query against flat QualiaQuin bytes in the browser. Returns JSON with matches array, vm_cycles, and op stats. db_bytes must be a multiple of 48.',
        params: [
            { name: 'query',       type: 'string',    desc: 'N-Triples pattern, e.g. "?s <http://…/name> ?o"' },
            { name: 'db_bytes',    type: 'Uint8Array', desc: 'Flat QualiaQuin bytes (N × 48). Use /cache upload to populate.' },
            { name: 'max_results', type: 'number',    desc: 'Output buffer size. Must be ≥ match count or returns OutputBufferFull error.' },
        ],
        returns: 'JSON string: { matches: [...], vm_cycles, direct_jump_ops, lexicon_lookup_ops }',
        snippets: [
            js(`
import init, { execute_ntriples_query } from './playground/qualia_core_db.js';
await init();

// Flat Quin bytes — load from /cache or build manually (48 bytes per Quin)
const db = await fetch('/my-dataset.q42').then(r => r.arrayBuffer());
const bytes = new Uint8Array(db);

const raw  = execute_ntriples_query('?s <http://xmlns.com/foaf/0.1/name> ?o', bytes, 256);
const data = JSON.parse(raw);
// data.matches = [{ s, p, o, c, m }, …]  (u64 as decimal strings)
// data.vm_cycles = 1234
`),
            http(`
POST http://127.0.0.1:4242/query
Content-Type: application/json
X-Qualia-Token: <your-token>

{
  "query": "?s <http://xmlns.com/foaf/0.1/name> ?o",
  "format": "json-ld"
}
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.execute_ntriples_query) return { error: 'WASM not loaded' };
            const raw = wasm.execute_ntriples_query(
                inputs.query || '?s ?p ?o', new Uint8Array(0), 256);
            return JSON.parse(raw);
        },
        liveInputs: [{ name: 'query', label: 'N-Triples pattern', default: '?s ?p ?o' }],
    },

    {
        id: 'wasm.parse_turtle_wasm',
        category: 'WASM API',
        name: 'parse_turtle_wasm()',
        summary: 'Parses a Turtle format string and converts it to an array of QualiaQuins represented as JSON objects. Demonstrates how RDF-like strings can be mapped to 64-bit Quin tokens via WASM.',
        params: [
            { name: 'payload', type: 'string', desc: 'A valid Turtle document string.' },
        ],
        returns: 'JSON string: Array of { subject, predicate, object }',
        snippets: [
            js(`
import init, { parse_turtle_wasm } from './playground/qualia_core_db.js';
await init();

const turtleString = "@prefix ex: <http://example.org/> . ex:Alice ex:knows ex:Bob .";
const result = parse_turtle_wasm(turtleString);
console.log(result);
// [{ subject: "...", predicate: "...", object: "..." }]
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.parse_turtle_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                return wasm.parse_turtle_wasm(inputs.payload);
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'Turtle String', default: '<http://ex.org/a> <http://ex.org/b> <http://ex.org/c> .' }],
    },

    {
        id: 'wasm.parse_n3logic_wasm',
        category: 'WASM API',
        name: 'parse_n3logic_wasm()',
        summary: 'Parses N3 Logic rules and triples, converting them into an array of QualiaQuins.',
        params: [
            { name: 'payload', type: 'string', desc: 'A valid N3 Logic string.' },
        ],
        returns: 'JSON string: Array of { subject, predicate, object }',
        snippets: [
            js(`
import init, { parse_n3logic_wasm } from './playground/qualia_core_db.js';
await init();

const n3String = "{ ?s ?p ?o } => { ?o ?p ?s } .";
const result = parse_n3logic_wasm(n3String);
console.log(result);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.parse_n3logic_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                return wasm.parse_n3logic_wasm(inputs.payload);
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'N3 Logic String', default: '{ ?s ?p ?o } => { ?s ?p ?o } .' }],
    },

    {
        id: 'wasm.parse_cbor_ld_wasm',
        category: 'WASM API',
        name: 'parse_cbor_ld_wasm()',
        summary: 'Parses a CBOR-LD binary array into a QualiaQuin representing dictionary-compressed lexicons. This validates binary ingestion paths.',
        params: [
            { name: 'payload', type: 'Uint8Array', desc: 'A valid CBOR-LD binary buffer.' },
        ],
        returns: 'JSON string: { subject, predicate, object, context }',
        snippets: [
            js(`
import init, { parse_cbor_ld_wasm } from './playground/qualia_core_db.js';
await init();

// CBOR Array [1000, 2000, 3000, 4000]
const cborBytes = new Uint8Array([0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
const result = parse_cbor_ld_wasm(cborBytes);
console.log(result);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.parse_cbor_ld_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                // Parse hex string input into Uint8Array for the live widget
                const hex = inputs.payload.replace(/\s+/g, '');
                const bytes = new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
                return wasm.parse_cbor_ld_wasm(bytes);
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'CBOR Hex Bytes', default: '84 19 03 E8 19 07 D0 19 0B B8 19 0F A0' }],
    },

    {
        id: 'wasm.parse_json_wasm',
        category: 'WASM API',
        name: 'parse_json_wasm()',
        summary: 'Parses a flat JSON-LD representation into an array of QualiaQuins.',
        params: [
            { name: 'payload', type: 'string', desc: 'JSON string containing an array of {s, p, o} objects.' },
        ],
        returns: 'JSON string: Array of { subject, predicate, object }',
        snippets: [
            js(`
import init, { parse_json_wasm } from './playground/qualia_core_db.js';
await init();

const jsonString = '[{"s": "Alice", "p": "knows", "o": "Bob"}]';
const result = parse_json_wasm(jsonString);
console.log(result);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.parse_json_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                return wasm.parse_json_wasm(inputs.payload);
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'JSON String', default: '[{"s": "Alice", "p": "knows", "o": "Bob"}]' }],
    },

    {
        id: 'wasm.compile_query_to_json',
        category: 'WASM API',
        name: 'compile_query_to_json()',
        summary: 'Compiles an N-Triples query pattern to a JSON representation of the Webizen VM bytecode program. Useful for debugging query compilation and understanding what the VM will execute.',
        params: [
            { name: 'query', type: 'string', desc: 'N-Triples pattern string' },
        ],        returns: 'JSON string describing the compiled bytecode program',
        snippets: [
            js(`
import init, { compile_query_to_json } from './playground/qualia_core_db.js';
await init();

const json = compile_query_to_json('?s <http://xmlns.com/foaf/0.1/name> ?o');
const program = JSON.parse(json);
console.log(program); // bytecode instruction listing
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.compile_query_to_json) return { error: 'WASM not loaded' };
            try { return JSON.parse(wasm.compile_query_to_json(inputs.query || '?s ?p ?o')); }
            catch (e) { return { raw: wasm.compile_query_to_json(inputs.query || '?s ?p ?o') }; }
        },
        liveInputs: [{ name: 'query', label: 'N-Triples pattern', default: '?s <http://xmlns.com/foaf/0.1/name> ?o' }],
    },

    {
        id: 'wasm.align_sequences_wasm',
        category: 'WASM API',
        name: 'align_sequences_wasm()',
        summary: 'Smith-Waterman / Needleman-Wunsch sequence alignment for nucleotide or protein sequences. Returns score, identity %, aligned sequences, and gap counts.',
        params: [
            { name: 'query',  type: 'string', desc: 'Query sequence (nucleotide: ATCG… / protein: amino acid one-letter codes)' },
            { name: 'target', type: 'string', desc: 'Target sequence to align against' },
            { name: 'mode',   type: '"nucleotide" | "protein"', desc: 'Alignment scoring matrix to use' },
        ],
        returns: '{ score, identity_pct, num_matches, num_gaps, aligned_query, aligned_target }',
        snippets: [
            js(`
import init, { align_sequences_wasm } from './playground/qualia_core_db.js';
await init();

const result = align_sequences_wasm({
  query:  'ATCGATCGTTAG',
  target: 'ATCGATCGAAAG',
  mode:   'nucleotide',
});
// { score: 28, identity_pct: 83.3, num_matches: 10, num_gaps: 0, … }
`),
            rs(`
use qualia_core_db::bioinformatics::{align_nucleotide, align_protein};

let result = align_nucleotide(b"ATCGATCG", b"ATCGATCG");
assert_eq!(result.num_matches, 8);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.align_sequences_wasm) return { error: 'Not in current WASM build' };
            return wasm.align_sequences_wasm({
                query: inputs.query || 'ATCGATCG', target: inputs.target || 'ATCGATCG',
                mode: inputs.mode || 'nucleotide',
            });
        },
        liveInputs: [
            { name: 'query',  label: 'Query sequence',  default: 'ATCGATCG' },
            { name: 'target', label: 'Target sequence', default: 'ATCGTTCG' },
            { name: 'mode',   label: 'Mode',            default: 'nucleotide', options: ['nucleotide','protein'] },
        ],
    },

    {
        id: 'wasm.compute_framingham_risk_wasm',
        category: 'WASM API',
        name: 'compute_framingham_risk_wasm()',
        summary: 'Computes the Framingham 10-year cardiovascular risk score. Returns risk percentage and category (Low/Intermediate/High).',
        params: [
            { name: 'age',                      type: 'u8',    desc: 'Patient age in years' },
            { name: 'sex_male',                 type: 'bool',  desc: 'true for male' },
            { name: 'total_cholesterol_mmol',   type: 'f64',   desc: 'Total cholesterol in mmol/L' },
            { name: 'hdl_cholesterol_mmol',     type: 'f64',   desc: 'HDL cholesterol in mmol/L' },
            { name: 'systolic_bp',              type: 'f64',   desc: 'Systolic blood pressure mm/Hg' },
            { name: 'bp_treated',               type: 'bool',  desc: 'On antihypertensive treatment' },
            { name: 'current_smoker',           type: 'bool',  desc: 'Current smoker' },
            { name: 'diabetic',                 type: 'bool',  desc: 'Diabetic' },
        ],
        returns: '{ risk_10yr_pct: f64, category: string }',
        snippets: [
            js(`
import init, { compute_framingham_risk_wasm } from './playground/qualia_core_db.js';
await init();

const risk = compute_framingham_risk_wasm({
  age: 55, sex_male: true,
  total_cholesterol_mmol: 5.8, hdl_cholesterol_mmol: 1.1,
  systolic_bp: 140.0, bp_treated: false,
  current_smoker: false, diabetic: false,
});
// { risk_10yr_pct: 12.4, category: "Intermediate" }
`),
            rs(`
use qualia_core_db::clinical_engine::{framingham_10yr_risk, FraminghamInput};

let result = framingham_10yr_risk(&FraminghamInput {
    age: 55, sex_male: true,
    total_cholesterol_mmol: 5.8, hdl_cholesterol_mmol: 1.1,
    systolic_bp: 140.0, bp_treated: false,
    current_smoker: false, diabetic: false,
});
println!("10yr risk: {:.1}% ({})", result.risk_10yr * 100.0,
         format!("{:?}", result.category));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.compute_framingham_risk_wasm) return { error: 'Not in current WASM build' };
            return wasm.compute_framingham_risk_wasm({
                age: parseInt(inputs.age||'55'), sex_male: inputs.sex_male==='true',
                total_cholesterol_mmol: parseFloat(inputs.tc||'5.8'),
                hdl_cholesterol_mmol:   parseFloat(inputs.hdl||'1.1'),
                systolic_bp:            parseFloat(inputs.sbp||'140'),
                bp_treated: false, current_smoker: false, diabetic: false,
            });
        },
        liveInputs: [
            { name: 'age',      label: 'Age',                    default: '55' },
            { name: 'sex_male', label: 'Male?',                  default: 'true', options: ['true','false'] },
            { name: 'tc',       label: 'Total cholesterol mmol', default: '5.8' },
            { name: 'hdl',      label: 'HDL cholesterol mmol',   default: '1.1' },
            { name: 'sbp',      label: 'Systolic BP mm/Hg',      default: '140' },
        ],
    },

    {
        id: 'wasm.compute_molecular_descriptors_wasm',
        category: 'WASM API',
        name: 'compute_molecular_descriptors_wasm()',
        summary: 'Computes molecular descriptors from a SMILES string: MW, formula, heavy atom count, H-bond donors/acceptors, rotatable bonds, ring counts, logP (Crippen), TPSA (Ertl), chiral centers, fraction Csp3.',
        params: [{ name: 'smiles', type: 'string', desc: 'SMILES notation of the molecule, e.g. "CC(=O)Oc1ccccc1C(=O)O" (aspirin)' }],
        returns: '{ molecular_weight, formula, heavy_atom_count, hb_donors, hb_acceptors, rotatable_bonds, aromatic_ring_count, ring_count, logp_crippen, tpsa_ertl, chiral_centers, fraction_csp3 }',
        snippets: [
            js(`
import init, { compute_molecular_descriptors_wasm } from './playground/qualia_core_db.js';
await init();

const desc = compute_molecular_descriptors_wasm({ smiles: 'CC(=O)Oc1ccccc1C(=O)O' });
// Aspirin: MW≈180, formula C9H8O4, logP≈1.19
console.log(\`\${desc.formula}  MW \${desc.molecular_weight.toFixed(2)}\`);
`),
            rs(`
use qualia_core_db::organic_chemistry::{parse_smiles, compute_descriptors};

let mol  = parse_smiles("CC(=O)Oc1ccccc1C(=O)O");
let desc = compute_descriptors(&mol);
println!("{} MW={:.2} logP={:.2}", desc.formula, desc.molecular_weight, desc.logp_crippen);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.compute_molecular_descriptors_wasm) return { error: 'Not in current WASM build' };
            return wasm.compute_molecular_descriptors_wasm({ smiles: inputs.smiles || 'CCO' });
        },
        liveInputs: [{ name: 'smiles', label: 'SMILES', default: 'CC(=O)Oc1ccccc1C(=O)O' }],
    },

    {
        id: 'wasm.validate_shacl_constraint_wasm',
        category: 'WASM API',
        name: 'validate_shacl_constraint_wasm()',
        summary: 'Validates a numeric value against a SHACL constraint. Supported types: MinInclusive, MaxInclusive, MinExclusive, MaxExclusive.',
        params: [
            { name: 'constraint_type', type: 'string', desc: 'One of MinInclusive, MaxInclusive, MinExclusive, MaxExclusive' },
            { name: 'value',           type: 'f64',    desc: 'The constraint bound value' },
            { name: 'target_value',    type: 'f64',    desc: 'The data value being validated' },
        ],
        returns: '{ passes: bool, constraint_type, value, target_value }',
        snippets: [
            js(`
import init, { validate_shacl_constraint_wasm } from './playground/qualia_core_db.js';
await init();

const result = validate_shacl_constraint_wasm({
  constraint_type: 'MinInclusive',
  value: 0.0,        // constraint bound
  target_value: 5.5, // the data value
});
// { passes: true, constraint_type: "MinInclusive", value: 0, target_value: 5.5 }
`),
            rs(`
use qualia_core_db::shacl_compiler::{ShaclCompiler, ShaclSeverity};

let compiler = ShaclCompiler::new();
let shape = compiler.compile(
    "ex:target", "ex:property",
    ShaclCompiler::parse_constraint_pub("MinInclusive", 0.0),
    ShaclSeverity::Violation,
);
assert!(shape.evaluate_numeric(5.5));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.validate_shacl_constraint_wasm) return { error: 'WASM not loaded' };
            return wasm.validate_shacl_constraint_wasm({
                constraint_type: inputs.constraint_type || 'MinInclusive',
                value:        parseFloat(inputs.value        || '0'),
                target_value: parseFloat(inputs.target_value || '5'),
            });
        },
        liveInputs: [
            { name: 'constraint_type', label: 'Constraint',    default: 'MinInclusive',
              options: ['MinInclusive','MaxInclusive','MinExclusive','MaxExclusive'] },
            { name: 'value',           label: 'Bound value',   default: '0' },
            { name: 'target_value',    label: 'Target value',  default: '5' },
        ],
    },

    {
        id: 'wasm.run_semantic_simulation',
        category: 'WASM API',
        name: 'run_semantic_simulation()',
        summary: 'Monte Carlo Value-at-Risk simulation. Runs geometric Brownian motion over simulation_steps × time_horizon days with 252 trading days/year to compute mean final price and 5th-percentile VaR.',
        params: [
            { name: 'initial_price',    type: 'f64', desc: 'Starting price' },
            { name: 'drift',            type: 'f64', desc: 'Annual drift μ (e.g. 0.05 = 5%)' },
            { name: 'volatility',       type: 'f64', desc: 'Annual volatility σ (e.g. 0.2 = 20%)' },
            { name: 'time_horizon',     type: 'i32', desc: 'Horizon in years' },
            { name: 'simulation_steps', type: 'i32', desc: 'Number of Monte Carlo paths' },
        ],
        returns: '{ mean: f64, value_at_risk: f64 }',
        snippets: [
            js(`
import init, { run_semantic_simulation } from './playground/qualia_core_db.js';
await init();

const result = run_semantic_simulation({
  initial_price: 100.0,
  drift:         0.07,
  volatility:    0.20,
  time_horizon:  1,
  simulation_steps: 10000,
});
console.log(\`Mean: \${result.mean.toFixed(2)}  VaR: \${result.value_at_risk.toFixed(2)}\`);
`),
            rs(`
use qualia_core_db::economics::run_monte_carlo_var;

let (mean, var) = run_monte_carlo_var(100.0, 0.07, 0.20, 1.0, 10_000, 252);
println!("Mean: {mean:.2}  5% VaR: {var:.2}");
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.run_semantic_simulation) return { error: 'Not in current WASM build' };
            return wasm.run_semantic_simulation({
                initial_price:    parseFloat(inputs.price  || '100'),
                drift:            parseFloat(inputs.drift  || '0.07'),
                volatility:       parseFloat(inputs.vol    || '0.20'),
                time_horizon:     parseInt(inputs.horizon  || '1'),
                simulation_steps: parseInt(inputs.steps    || '5000'),
            });
        },
        liveInputs: [
            { name: 'price',   label: 'Initial price',  default: '100' },
            { name: 'drift',   label: 'Annual drift μ', default: '0.07' },
            { name: 'vol',     label: 'Volatility σ',   default: '0.20' },
            { name: 'horizon', label: 'Horizon (yrs)',  default: '1' },
            { name: 'steps',   label: 'MC paths',       default: '5000' },
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // NATIVE DAEMON ENDPOINTS
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'daemon.health',
        category: 'Native Daemon',
        name: 'GET /health',
        summary: 'Health probe for the local native daemon. Returns engine name, status, and version. No authentication required. Daemon listens on 127.0.0.1:4242 by default.',
        params: [],
        returns: '{ status: "active", engine: "qualia-core-db", version: "0.0.x" }',
        snippets: [
            http(`
GET http://127.0.0.1:4242/health
`),
            js(`
const r    = await fetch('http://127.0.0.1:4242/health');
const body = await r.json();
// { status: "active", engine: "qualia-core-db", version: "0.0.8", webtorrent: { … } }
`),
            cli(`
# Start the daemon (dev mode — no token required)
qualia-cli daemon --dev --port 4242

# Probe in a second terminal
curl http://127.0.0.1:4242/health
`),
        ],
        live: async (_wasm, native) => {
            if (!native) return { error: 'Daemon offline — start with: qualia-cli daemon --dev' };
            const { body } = await native.health();
            return body;
        },
    },

    {
        id: 'daemon.query',
        category: 'Native Daemon',
        name: 'POST /query',
        summary: 'Execute an N-Triples pattern query against the native graph engine. Requires X-Qualia-Token header (omit in dev mode). Supports json-ld, n-triples, and q42 formats. Response includes X-Qualia-Compute-Cost: {matches}+{cycles} header.',
        params: [
            { name: 'query',  type: 'string',                           desc: 'N-Triples pattern, e.g. "?s <http://…/name> ?o"' },
            { name: 'format', type: '"json-ld" | "n-triples" | "q42"',  desc: 'Response serialisation format (default: json-ld)' },
        ],
        returns: 'JSON-LD: { "@context": {...}, "@graph": [...], match_count } or N-Triples text',
        snippets: [
            http(`
POST http://127.0.0.1:4242/query
Content-Type: application/json
X-Qualia-Token: <your-token>
Accept: application/ld+json

{
  "query":  "?s <http://xmlns.com/foaf/0.1/name> ?o",
  "format": "json-ld"
}

# Response headers include:
# X-Qualia-Compute-Cost: 5+142
# Content-Type: application/ld+json
`),
            js(`
const token = localStorage.getItem('qualia_x_token') || '';

const r = await fetch('http://127.0.0.1:4242/query', {
  method: 'POST',
  headers: {
    'Content-Type':   'application/json',
    'Accept':         'application/ld+json',
    'X-Qualia-Token': token,
  },
  body: JSON.stringify({
    query:  '?s <http://xmlns.com/foaf/0.1/name> ?o',
    format: 'json-ld',
  }),
});
const cost  = r.headers.get('X-Qualia-Compute-Cost'); // "5+142"
const graph = (await r.json())['@graph'];
`),
            cli(`
curl -X POST http://127.0.0.1:4242/query \\
  -H "Content-Type: application/json" \\
  -H "X-Qualia-Token: \$QUALIA_TOKEN" \\
  -d '{"query":"?s ?p ?o","format":"json-ld"}'
`),
        ],
        live: async (_wasm, native, inputs) => {
            if (!native) return { error: 'Daemon offline — start with: qualia-cli daemon --dev' };
            const { ok, body, computeCost, status } = await native.query(
                inputs.query || '?s ?p ?o', inputs.format || 'json-ld');
            return { status, ok, computeCost, body };
        },
        liveInputs: [
            { name: 'query',  label: 'N-Triples pattern', default: '?s ?p ?o' },
            { name: 'format', label: 'Format',            default: 'json-ld',
              options: ['json-ld', 'n-triples'] },
        ],
    },

    {
        id: 'daemon.websocket',
        category: 'Native Daemon',
        name: 'WS /qualia-bridge',
        summary: 'WebSocket bridge for native compute offload and browser benchmarks. On connect: HANDSHAKE_SUCCESS. Query frames use format=metrics (no JSON-LD payload). Dev daemon supports bench_load (binary flat QualiaQuin bytes).',
        params: [
            { name: 'query', type: 'string', desc: 'N-Triples pattern for type=query frames' },
            { name: 'format', type: 'string', desc: '"metrics" — returns match_count + vm_cycles only' },
        ],
        returns: 'HANDSHAKE_SUCCESS, then { type: "result", match_count, vm_cycles, direct_jump_ops } or bench_loaded',
        snippets: [
            js(`
const ws = new WebSocket('ws://127.0.0.1:4242/qualia-bridge');
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.type === 'HANDSHAKE_SUCCESS') {
    ws.send(JSON.stringify({
      type: 'query',
      id: 1,
      query: '<http://q.test/s/0> ?p ?o .',
      format: 'metrics',
    }));
  }
  if (msg.type === 'result') {
    // msg.match_count, msg.vm_cycles — no HTTP/JSON-LD overhead
  }
};
`),
            cli(`
# WebSocket test with websocat (install: cargo install websocat)
websocat ws://127.0.0.1:4242/qualia-bridge
# Immediately receives: {"type":"HANDSHAKE_SUCCESS","payload":{"mode":"NATIVE","version":"0.0.8"}}
`),
        ],
        live: async (_wasm, native) => {
            if (!native) return { error: 'Daemon offline — start with: qualia-cli daemon --dev' };
            return new Promise(resolve => {
                const ws = new WebSocket(native.base.replace('http://','ws://') + '/qualia-bridge');
                const t  = setTimeout(() => { ws.close(); resolve({ error: 'timeout' }); }, 3000);
                ws.onmessage = e => { clearTimeout(t); ws.close(); resolve(JSON.parse(e.data)); };
                ws.onerror   = () => { clearTimeout(t); resolve({ error: 'WebSocket connection refused' }); };
            });
        },
    },

    {
        id: 'daemon.chat_publish',
        category: 'Native Daemon',
        name: 'POST /chat/publish',
        summary: 'Append a signed relay envelope to a group-chat inbox. Messages are stored as JSONL under {storage}/ChatRelay/{session_id}/inbox.jsonl. Ed25519 signatures are verified when signature_hex is present.',
        params: [
            { name: 'session_id', type: 'string', desc: 'Chat session identifier' },
            { name: 'lamport', type: 'u64', desc: 'Monotonic Lamport clock for ordering' },
            { name: 'role', type: 'string', desc: '"user" | "assistant" | "system"' },
            { name: 'content', type: 'string', desc: 'Message body' },
            { name: 'author_did', type: 'string', desc: 'Principal or sub-agent DID' },
            { name: 'signature_hex', type: 'string', desc: 'Optional 64-byte Ed25519 signature (hex)' },
        ],
        returns: '{ ok: true, lamport: number }',
        snippets: [
            http(`
POST http://127.0.0.1:4242/chat/publish
Content-Type: application/json

{
  "session_id": "grp-abc123",
  "lamport": 42,
  "role": "assistant",
  "content": "Grounded summary with provenance.",
  "author_did": "did:qualia:subagent:…",
  "author_name": "Alice",
  "reply_to_fragment": null,
  "timestamp": 1717756800,
  "signature_hex": "<128-char hex>"
}
`),
            js(`
const envelope = {
  session_id: 'grp-abc123',
  lamport: 42,
  role: 'assistant',
  content: 'Grounded summary.',
  author_did: 'did:qualia:principal:…',
  timestamp: Math.floor(Date.now() / 1000),
  signature_hex: '',
};
const r = await fetch('http://127.0.0.1:4242/chat/publish', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(envelope),
});
`),
        ],
    },

    {
        id: 'daemon.chat_pull',
        category: 'Native Daemon',
        name: 'GET /chat/pull',
        summary: 'Pull relay messages for a session since a Lamport watermark. Used by desktop clients to sync group-chat inboxes without a central cloud broker.',
        params: [
            { name: 'session_id', type: 'string', desc: 'Chat session identifier (required query param)' },
            { name: 'since_lamport', type: 'u64', desc: 'Return messages with lamport > this value (default 0)' },
        ],
        returns: '{ messages: RelayEnvelope[], latest_lamport: number }',
        snippets: [
            http(`
GET http://127.0.0.1:4242/chat/pull?session_id=grp-abc123&since_lamport=0
`),
            js(`
const sessionId = 'grp-abc123';
const since = 0;
const r = await fetch(
  \`http://127.0.0.1:4242/chat/pull?session_id=\${sessionId}&since_lamport=\${since}\`
);
const { messages, latest_lamport } = await r.json();
`),
            cli(`
curl "http://127.0.0.1:4242/chat/pull?session_id=grp-abc123&since_lamport=10"
`),
        ],
        live: async (_wasm, native, inputs) => {
            if (!native) return { error: 'Daemon offline — start with: qualia-cli daemon --dev' };
            const sid = (inputs.session_id || 'default').trim();
            const since = inputs.since_lamport || '0';
            const url = `${native.base}/chat/pull?session_id=${encodeURIComponent(sid)}&since_lamport=${since}`;
            const r = await fetch(url);
            return { status: r.status, body: await r.json() };
        },
        liveInputs: [
            { name: 'session_id', label: 'Session ID', default: 'default' },
            { name: 'since_lamport', label: 'Since Lamport', default: '0' },
        ],
    },

    {
        id: 'daemon.torrent_seed',
        category: 'Native Daemon',
        name: 'POST /torrent/seed',
        summary: 'Register a .c.q42 ontology artifact for HTTP web-seeding on the Qualia daemon. Seeding runs in-process (seeder: qualia-daemon), not in the Flutter UI. Magnets include a ws= parameter pointing at /torrent/webseed/{hash}.',
        params: [
            { name: 'info_hash', type: 'string', desc: 'SHA-1 info hash (40 hex chars)' },
            { name: 'file_path', type: 'string', desc: 'Absolute path to the .c.q42 file' },
            { name: 'display_name', type: 'string', desc: 'Human-readable torrent name' },
            { name: 'ontology_id', type: 'string', desc: 'Workbench ontology identifier' },
        ],
        returns: '{ status: "ok", seed: SeedRecord, seeder: "qualia-daemon" }',
        snippets: [
            http(`
POST http://127.0.0.1:4242/torrent/seed
Content-Type: application/json

{
  "info_hash": "a1b2c3d4e5f6789012345678abcdef0123456789",
  "file_path": "C:/Users/me/.qualia/Index/ontologies/prov-o.c.q42",
  "display_name": "W3C PROV-O (compressed)",
  "ontology_id": "prov-o"
}
`),
            cli(`
# Reload seeds from workbench index after daemon boot
curl -X POST http://127.0.0.1:4242/torrent/sync
`),
        ],
    },

    {
        id: 'daemon.torrent_telemetry',
        category: 'Native Daemon',
        name: 'GET /torrent/telemetry',
        summary: 'Live WebTorrent seeder statistics from the Qualia daemon. Also embedded in GET /health under the webtorrent key.',
        params: [],
        returns: '{ seeder: "qualia-daemon", seeders, leechers, speed, status, uploaded_session_kb, active_ontologies, … }',
        snippets: [
            http(`
GET http://127.0.0.1:4242/torrent/telemetry
`),
            js(`
const r = await fetch('http://127.0.0.1:4242/torrent/telemetry');
const stats = await r.json();
// stats.seeder === "qualia-daemon"
`),
        ],
        live: async (_wasm, native) => {
            if (!native) return { error: 'Daemon offline — start with: qualia-cli daemon --dev' };
            const r = await fetch(`${native.base}/torrent/telemetry`);
            return { status: r.status, body: await r.json() };
        },
    },

    {
        id: 'daemon.torrent_webseed',
        category: 'Native Daemon',
        name: 'GET /torrent/webseed/{info_hash}',
        summary: 'Serve a registered .c.q42 file as an HTTP web seed (BEP-19). Supports Range requests for partial fetches. Referenced by magnet URIs via the ws= parameter.',
        params: [
            { name: 'info_hash', type: 'string', desc: 'SHA-1 info hash (path segment)' },
            { name: 'Range', type: 'header', desc: 'Optional bytes=start-end for partial content' },
        ],
        returns: 'application/octet-stream (200 or 206 Partial Content)',
        snippets: [
            http(`
GET http://127.0.0.1:4242/torrent/webseed/a1b2c3d4e5f6789012345678abcdef0123456789
Range: bytes=0-4095
`),
            js(`
// Magnet URI from workbench includes ws= pointing here:
// magnet:?xt=urn:btih:…&dn=PROV-O&ws=http%3A%2F%2F127.0.0.1%3A4242%2Ftorrent%2Fwebseed%2F…
const hash = 'a1b2c3d4e5f6789012345678abcdef0123456789';
const r = await fetch(\`http://127.0.0.1:4242/torrent/webseed/\${hash}\`);
const bytes = await r.arrayBuffer();
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // DESKTOP CHAT (Flutter FRB)
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'chat.sub_agent',
        category: 'Desktop Chat',
        name: 'getLocalAgentConfig()',
        summary: 'Returns the local principal\'s sub-agent binding for a chat session. Local LLM/Webizen agents are sub-agents of human participants — not independent chat actors. Each participant may use a different model/backend.',
        params: [{ name: 'sessionId', type: 'string', desc: 'Chat session ID' }],
        returns: 'ParticipantAgentConfig { principalDid, subAgentDid, modelId?, backend, outcomeSharing, updatedAt }',
        snippets: [
            js(`
import { getLocalAgentConfig } from './src/rust/api/chat_agents.dart';

const cfg = await getLocalAgentConfig(sessionId: 'grp-abc123');
// cfg.subAgentDid  → did:qualia:subagent:{principal_hash}:{session_hash}
// cfg.backend      → "local" | "remote" | "hybrid"
`),
            rs(`
use qualia_client_core::chat_agents;

let cfg = chat_agents::load_local_agent_config(&storage, &session_id)?;
// cfg.sub_agent_did is derived — not a peer participant
`),
        ],
    },

    {
        id: 'chat.outcome_sharing',
        category: 'Desktop Chat',
        name: 'updateAgentOutcomeSharing()',
        summary: 'Set explicit permissions for sharing Webizen-processed outcomes (summaries, grounded answers) with other group members. Raw prompts are never relayed — only processed results when policy permits.',
        params: [
            { name: 'sessionId', type: 'string', desc: 'Chat session ID' },
            { name: 'policy.visibility', type: 'string', desc: 'owner_only | session_participants | specific_dids' },
            { name: 'policy.allowPeerLlmContext', type: 'bool', desc: 'Peers may include this outcome in their LLM context' },
        ],
        returns: 'ParticipantAgentConfig',
        snippets: [
            js(`
import { updateAgentOutcomeSharing, OutcomeSharingPolicy } from './chat_agents.dart';

await updateAgentOutcomeSharing(
  sessionId: 'grp-abc123',
  policy: OutcomeSharingPolicy(
    visibility: 'session_participants',
    shareProvenance: true,
    shareModelAttribution: false,
    allowPeerLlmContext: true,
    allowedDids: [],
  ),
);
`),
        ],
    },

    {
        id: 'chat.sync_relay',
        category: 'Desktop Chat',
        name: 'syncChatRelay()',
        summary: 'Pull new messages from the daemon relay inbox and merge into the local session WAL. Call periodically or after sending to keep group chats in sync.',
        params: [{ name: 'sessionId', type: 'string?', desc: 'Session to sync (null = all active sessions)' }],
        returns: 'u64 — latest Lamport clock after sync',
        snippets: [
            js(`
import { syncChatRelay } from './src/rust/api/chat_graph.dart';

const latest = await syncChatRelay(sessionId: 'grp-abc123');
`),
            http(`
# Under the hood: GET /chat/pull?session_id=…&since_lamport=…
# then local WAL merge + signature validation
`),
        ],
    },

    {
        id: 'chat.group_session',
        category: 'Desktop Chat',
        name: 'createGroupChatSession()',
        summary: 'Create a multi-participant chat session with a stable session DID for ontology sharing and relay sync.',
        params: [
            { name: 'title', type: 'string?', desc: 'Display title' },
            { name: 'participantDids', type: 'string[]', desc: 'Initial participant DIDs' },
        ],
        returns: 'string — new session_id',
        snippets: [
            js(`
import { createGroupChatSession } from './src/rust/api/chat_session.dart';

const id = await createGroupChatSession(
  title: 'Clinical review',
  participantDids: ['did:qualia:…', 'did:qualia:…'],
);
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // ONTOLOGY WORKBENCH (Flutter FRB)
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'workbench.import_uri',
        category: 'Ontology Workbench',
        name: 'workbenchImportOntologyUri()',
        summary: 'Import an ontology from a remote URI, compress to .c.q42, compute SHA-1 info hash, and build a magnet URI with ws= web-seed parameter for Permissive Commons sharing.',
        params: [
            { name: 'uri', type: 'string', desc: 'Source ontology URL (Turtle, N-Triples, etc.)' },
            { name: 'ontologyId', type: 'string?', desc: 'Stable ID (auto-derived if omitted)' },
            { name: 'domain', type: 'string?', desc: 'Domain tag for share cards' },
            { name: 'title', type: 'string?', desc: 'Display title' },
        ],
        returns: 'WorkbenchImportResult { entry, compressRatio, sourceRemoved }',
        snippets: [
            js(`
import { workbenchImportOntologyUri } from './src/rust/api/ontology_workbench.dart';

const result = await workbenchImportOntologyUri(
  uri: 'https://www.w3.org/ns/prov-o',
  ontologyId: 'prov-o',
  domain: 'provenance',
  title: 'W3C PROV-O',
);
// result.entry.magnetUri includes ws=http://127.0.0.1:4242/torrent/webseed/…
`),
            cli(`
# After import, enable seeding via the Ontology Hub UI or:
curl -X POST http://127.0.0.1:4242/torrent/seed -H "Content-Type: application/json" \\
  -d '{"info_hash":"…","file_path":"…/prov-o.c.q42","display_name":"PROV-O","ontology_id":"prov-o"}'
`),
        ],
    },

    {
        id: 'workbench.set_seed',
        category: 'Ontology Workbench',
        name: 'setWorkbenchSeed()',
        summary: 'Toggle active seeding for a workbench ontology. Registers the .c.q42 with the Qualia daemon seeder and updates workbench.jsonl index.',
        params: [
            { name: 'ontologyId', type: 'string', desc: 'Workbench ontology ID' },
            { name: 'active', type: 'bool', desc: 'true to seed, false to unseed' },
        ],
        returns: 'WorkbenchEntry with updated seedActive and upload stats',
        snippets: [
            js(`
import { setWorkbenchSeed } from './src/rust/api/ontology_workbench.dart';

const entry = await setWorkbenchSeed(ontologyId: 'prov-o', active: true);
// Daemon serves via GET /torrent/webseed/{info_hash}
`),
        ],
    },

    {
        id: 'workbench.share_cards',
        category: 'Ontology Workbench',
        name: 'listOntologySharesForSession()',
        summary: 'List ontology share cards visible to a chat session DID. Cards respect per-ontology torrent policy (audience, allowed contact/session DIDs).',
        params: [{ name: 'sessionDid', type: 'string', desc: 'Chat session DID' }],
        returns: 'OntologyShareCard[] { ontologyId, title, domain, magnetUri, infoHashSha1, quinCount }',
        snippets: [
            js(`
import { listOntologySharesForSession } from './src/rust/api/ontology_workbench.dart';

const cards = await listOntologySharesForSession(sessionDid: sessionDid);
// Each card.magnetUri is ready for WebTorrent clients with ws= web seed
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // CLI
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'cli.daemon',
        category: 'CLI',
        name: 'qualia-cli daemon',
        summary: 'Start the native loopback daemon. Dev mode (--dev) skips token auth and allows any localhost origin — use for development and testing. Production mode requires a paired QUALIA_TOKEN env var.',
        params: [
            { name: '--dev',          type: 'flag',   desc: 'Skip token auth, allow all localhost origins' },
            { name: '--port',         type: 'u16',    desc: 'Port to listen on (default: 4242)' },
            { name: '--net-mode',     type: 'string', desc: 'offline | metered | unmetered (default: unmetered)' },
            { name: '--energy-mode',  type: 'string', desc: 'strict | opportunistic | unlimited (default: unlimited)' },
            { name: '--workers',      type: 'u16',    desc: 'Number of 512 MB sharding cells (default: 1)' },
        ],
        returns: 'HTTP server on 127.0.0.1:{port}',
        snippets: [
            cli(`
# Development — no token, accepts localhost origins
qualia-cli daemon --dev --port 4242

# Production — requires QUALIA_TOKEN set in environment
QUALIA_TOKEN=your-secret qualia-cli daemon --port 4242

# Metered connection, strict energy (e.g. on battery)
qualia-cli daemon --dev --net-mode metered --energy-mode strict
`),
        ],
    },

    {
        id: 'cli.ingest',
        category: 'CLI',
        name: 'qualia-cli ingest',
        summary: 'Ingest an N-Triples file into a .q42 SuperBlock binary + .q42.lex reverse-lexicon + .q42.bidx block-range index. The output is suitable for browser OPFS caching and the GH Pages WASM demo.',
        params: [
            { name: '--input',  type: 'path', desc: 'Path to .nt (N-Triples) input file' },
            { name: '--output', type: 'path', desc: 'Path for the output .q42 file (.lex and .bidx are auto-created alongside)' },
        ],
        returns: '.q42 (40960-byte SuperBlocks) + .q42.lex (lexicon) + .q42.bidx (block index)',
        snippets: [
            cli(`
# Ingest an N-Triples file
qualia-cli ingest --input data.nt --output data.q42

# Output:
#   data.q42       — SuperBlock binary (N × 40960 bytes)
#   data.q42.lex   — Reverse lexicon (hash → IRI string)
#   data.q42.bidx  — Block-range index for demand-paged HTTP Range requests
`),
            js(`
// Upload an ingested .q42 shard to the daemon cache
const bytes = await fetch('/data.q42').then(r => r.arrayBuffer());
await fetch('http://127.0.0.1:4242/cache?filename=data.q42', {
  method: 'POST',
  body: bytes,
});
`),
        ],
    },

    {
        id: 'cli.dump',
        category: 'CLI',
        name: 'qualia-cli dump',
        summary: 'Generate a mocked .q42 binary file with 3 sample Quins for testing. Produces exactly 144 bytes (3 × 48-byte Quins).',
        params: [
            { name: 'out_path', type: 'path', desc: 'Output path for the .q42 test file' },
        ],
        returns: '144-byte .q42 file with 3 mocked Quins',
        snippets: [
            cli(`
qualia-cli dump test_block.q42
# → Dumped 3 mocked Quins (144 bytes) to .q42 successfully.
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // COGAI CHUNKS
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'cogai.chunks_format',
        category: 'CogAI Chunks',
        name: 'CogAI Chunk Format (.chk text)',
        summary: 'W3C CogAI Community Group chunks-and-rules format (https://github.com/w3c-cg/cogai). A chunk is a named, typed collection of properties modelled on ACT-R cognitive architecture. QualiaDB ingests .chk text files and compiles them into QualiaQuins via the RetrieveByActivation, DecayMetadata, and Unless SHACL opcodes.',
        params: [
            { name: 'type',   type: 'string', desc: 'Chunk type (e.g. dog, memory, rule)' },
            { name: 'id',     type: 'string', desc: 'Optional chunk ID (auto-assigned if omitted)' },
            { name: 'props',  type: 'object', desc: 'Key-value properties: names, numbers, booleans, ISO8601 dates, quoted strings, comma-lists' },
        ],
        returns: 'Array of QualiaQuin (one per property + one type-assertion quin)',
        snippets: [
            nt(`
# CogAI .chk text format — ingest via qualia-cli ingest --input knowledge.chk

dog dog1 {
  name "Fido"
  age 4
  active true
}

# Multi-property chunk with @rdfmap
@rdfmap {
  dog http://example.com/ns/dog
  name http://xmlns.com/foaf/0.1/name
}

memory m1 {
  content "sky is blue"
  strength 0.9
  context world1
}

# Rule: conditions => actions
count {state start; from ?num1; to ?num2}
  => count {state counting}
`),
            rs(`
// After ingest, CogAI chunk properties become Quins:
// subject   = q_hash(chunk_id || chunk_type)
// predicate = q_hash(property_name)
// object    = inline integer/boolean OR q_hash(string_value)
// context   = q_hash(chunk_type)   ← named graph

use qualia_core_db::q_hash;

let chunk_hash = q_hash("dog1");
let name_pred  = q_hash("name");
let fido_hash  = q_hash("Fido");
// Results in one Quin: (dog1_hash, name_hash, fido_hash, dog_hash, 0, parity)
`),
            cli(`
# Ingest a CogAI .chk text file (distinct from QCHK binary profiles)
qualia-cli ingest --input knowledge.chk --output knowledge.q42

# ⚠ .chk extension is shared between two formats:
#   CogAI text chunks   → no magic bytes, plain text
#   QCHK binary profiles → starts with bytes 51 43 48 4B ("QCHK")
# qualia-cli auto-detects by inspecting offset 0.
`),
        ],
        live: async (_wasm, _native, inputs) => {
            // Parse the chunk text and show the resulting Quin structure
            const FNV_OFFSET = 0xcbf29ce484222325n;
            const FNV_PRIME  = 0x100000001b3n;
            const MASK_64    = 0xffffffffffffffffn;
            const ENC        = new TextEncoder();
            function q_hash(s) {
                let h = FNV_OFFSET;
                for (const b of ENC.encode(s)) h = ((h ^ BigInt(b)) * FNV_PRIME) & MASK_64;
                return h;
            }
            try {
                const text   = (inputs.chunk || '').trim();
                const header = text.match(/^(\S+)(?:\s+(\S+))?\s*\{/);
                if (!header) return { error: 'Invalid chunk — expected: type [id] { key value; ... }' };
                const type = header[1];
                const id   = header[2] || type;
                const bodyStart = text.indexOf('{') + 1;
                const bodyEnd   = text.lastIndexOf('}');
                const body      = text.slice(bodyStart, bodyEnd).trim();
                const quins = [{ field: 'type assertion', subject: '0x' + q_hash(id).toString(16), predicate: '0x' + q_hash('cogai:type').toString(16), object: '0x' + q_hash(type).toString(16) }];
                for (const line of body.split(/[;\n]+/)) {
                    const parts = line.trim().match(/^(\S+)\s+(.+)$/);
                    if (!parts) continue;
                    const key = parts[1];
                    const val = parts[2].trim().replace(/^"|"$/g, '');
                    quins.push({ field: key, subject: '0x' + q_hash(id).toString(16), predicate: '0x' + q_hash(key).toString(16), object: '0x' + q_hash(val).toString(16) });
                }
                return { type, id, quin_count: quins.length, quins };
            } catch (e) {
                return { error: e.message };
            }
        },
        liveInputs: [{ name: 'chunk', label: 'CogAI Chunk Text', default: 'memory m1 {\n  content "sky is blue"\n  strength "0.9"\n}' }],
    },

    {
        id: 'cogai.actr_opcodes',
        category: 'CogAI Chunks',
        name: 'ACT-R Opcodes (RetrieveByActivation / DecayMetadata)',
        summary: 'SHACL constraints that compile to ACT-R cognitive opcodes in the Webizen VM. qualia:retrieveByActivation → NativeRetrieveByActivation (yields to GPU Sieve / Core 2). qualia:decayMetadata → NativeDecayMetadata (yields to Core 2). qualia:unless → NativeUnless (executes inline on Core 1 as non-monotonic default logic). Activation levels are encoded in Quin metadata bits 0–31 as fixed-point u32.',
        params: [
            { name: 'activation', type: 'f32 in [0.0, 1.0]', desc: 'Chunk activation level — encoded as fixed-point u32 in metadata bits 0–31' },
            { name: 'decayRate',  type: 'f32',                desc: 'ACT-R base-level decay rate d (typically 0.5)' },
            { name: 'elapsedMs',  type: 'u64',                desc: 'Milliseconds since last access' },
        ],
        returns: 'Activation level after decay; encoded as u32 in Quin metadata',
        snippets: [
            rs(`
// SHACL shape that triggers ACT-R retrieval
// qualia:retrieveByActivation maps to NativeRetrieveByActivation SlgOpcode
// NativeRetrieveByActivation YIELDS to Core 2 (GPU Sieve) — returns None from execute_vm_frame

use qualia_core_db::shacl_compiler::{ShaclCompiler, ShaclConstraint, ShaclSeverity};

let compiler = ShaclCompiler::new();
let shape = compiler.compile(
    "cog:Memory",
    "cog:activate",
    ShaclConstraint::RetrieveByActivation,
    ShaclSeverity::Violation,
);
// shape.opcodes contains [NativeRetrieveByActivation, Halt]
`),
            rs(`
// Activation level encoded in Quin metadata bits 0-31 (fixed-point u32)
// Decay: level * exp(-rate * elapsed_s)   (ACT-R base-level learning)

use qualia_core_db::QualiaQuin;

const ACTIVATION_SCALE: u32 = 1_000_000;

fn encode_activation(level: f32) -> u64 {
    let clamped = level.clamp(0.0, 1.0);
    (clamped * ACTIVATION_SCALE as f32).round() as u64
}

fn decode_activation(metadata: u64) -> f32 {
    (metadata & 0xFFFF_FFFF) as f32 / ACTIVATION_SCALE as f32
}

fn decay_activation(level: f32, rate: f32, elapsed_ms: u64) -> f32 {
    (level * (-rate * elapsed_ms as f32 / 1000.0).exp()).max(0.0)
}
`),
        ],
        live: async (_wasm, _native, inputs) => {
            const level   = parseFloat(inputs.level   || '0.9');
            const rate    = parseFloat(inputs.rate    || '0.5');
            const elapsed = parseFloat(inputs.elapsed || '1000');
            const SCALE   = 1_000_000;
            const clamped = Math.max(0, Math.min(1, level));
            const encoded = Math.round(clamped * SCALE);
            const decayed = Math.max(0, clamped * Math.exp(-rate * elapsed / 1000));
            return {
                initial_activation:    clamped.toFixed(6),
                encoded_u32:           encoded,
                metadata_bits_0_31:    '0x' + encoded.toString(16).padStart(8, '0'),
                decayed_activation:    decayed.toFixed(6),
                decayed_encoded_u32:   Math.round(decayed * SCALE),
                core2_yield:           'NativeRetrieveByActivation + NativeDecayMetadata → GPU Sieve (returns None from execute_vm_frame)',
            };
        },
        liveInputs: [
            { name: 'level',   label: 'Initial activation (0.0–1.0)', default: '0.9' },
            { name: 'rate',    label: 'Decay rate d (ACT-R, typically 0.5)', default: '0.5' },
            { name: 'elapsed', label: 'Elapsed ms since last access', default: '1000' },
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // CAPABILITY PROFILES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'profiles.qchk_format',
        category: 'Capability Profiles',
        name: 'QCHK Binary Format (.chk binary)',
        summary: 'QualiaDB Capability Profile binary format. Declares the allowed engine operations and ontology namespaces for an agent session. Six named profiles: general, health, chemistry, research, legal, financial. Compiled via qualia-cli profile compile. Distinguished from CogAI text .chk files by the "QCHK" magic bytes at offset 0.',
        params: [
            { name: 'profile_id',   type: 'u64 (little-endian at offset 4)',  desc: 'q_hash("profile:<name>") — e.g. q_hash("profile:health")' },
            { name: 'payload_len',  type: 'u32 (little-endian at offset 12)', desc: 'Byte length of the JSON-LD payload' },
            { name: 'payload',      type: 'UTF-8 JSON-LD at offset 16',       desc: 'Profile declaration with allowed engines and ontology namespaces' },
        ],
        returns: 'Bound CapabilityProfile — restricts available SlgOpcodes and ontology namespaces for the session',
        snippets: [
            cli(`
# Compile a JSON-LD capability profile to a QCHK binary
qualia-cli profile compile health.jsonld --out health.chk

# List all known profile IDs and their q_hash values
qualia-cli profile list

# Decode and inspect a compiled .chk file
qualia-cli profile inspect health.chk

# Bind a profile during ingest (restricts opcodes to health-permitted set)
qualia-cli ingest --input patient-graph.ttl --output patient.q42 --profile health.chk

# ── .chk disambiguation ─────────────────────────────────────────────────
# QCHK binary: offset 0 = 0x51 0x43 0x48 0x4B ("QCHK") — binary profile
# CogAI text:  offset 0 = plain text (type char) — ACT-R chunks-and-rules
`),
            rs(`
// QCHK binary layout (profiles.rs)
//   0..4   Magic: b"QCHK"  (0x51 0x43 0x48 0x4B)
//   4..12  profile_id: u64 little-endian  = q_hash("profile:health")
//   12..16 payload_len: u32 little-endian
//   16..   JSON-LD payload (UTF-8)

use qualia_core_db::profiles::CapabilityProfile;

let profile = CapabilityProfile::load_from_chk(std::fs::read("health.chk")?)?;
assert_eq!(profile.profile_id, q_hash("profile:health"));
`),
            js(`
// Detect and parse a .chk file in the browser
async function loadChk(arrayBuffer) {
    const bytes = new Uint8Array(arrayBuffer);
    const magic = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
    if (magic === 'QCHK') {
        // Binary Capability Profile
        const view       = new DataView(arrayBuffer);
        const lo         = view.getUint32(4, true);
        const hi         = view.getUint32(8, true);
        const profileId  = (BigInt(hi) << 32n) | BigInt(lo);
        const payloadLen = view.getUint32(12, true);
        const payload    = new TextDecoder().decode(bytes.slice(16, 16 + payloadLen));
        return { kind: 'qchk', profileId: '0x' + profileId.toString(16), payload };
    } else {
        // CogAI Cognitive AI Chunks text file
        return { kind: 'cogai-text', text: new TextDecoder().decode(bytes) };
    }
}
`),
        ],
        live: async (_wasm, _native, inputs) => {
            const FNV_OFFSET = 0xcbf29ce484222325n;
            const FNV_PRIME  = 0x100000001b3n;
            const MASK_64    = 0xffffffffffffffffn;
            const ENC        = new TextEncoder();
            function q_hash(s) {
                let h = FNV_OFFSET;
                for (const b of ENC.encode(s)) h = ((h ^ BigInt(b)) * FNV_PRIME) & MASK_64;
                return h;
            }
            const profiles = ['general','health','chemistry','research','legal','financial'];
            return Object.fromEntries(profiles.map(p => [
                `profile:${p}`,
                '0x' + q_hash(`profile:${p}`).toString(16).padStart(16, '0'),
            ]));
        },
        liveInputs: [],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // RESOURCE CATALOG
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'resources.catalog',
        category: 'Resource Catalog',
        name: 'Resource Catalog',
        summary: 'Three resource types — LLMResource (GGUF models), OntologyResource (RDF namespaces + SHACL shapes), SPARQLResource (federated endpoints). Each serializes to QualiaQuins via to_quins(). YAML catalogs in resources/: llms.yaml, ontologies.yaml, sparql_endpoints.yaml. Download pipeline: YAML → reqwest stream → GGufSharder → WAL.',
        params: [
            { name: 'type',     type: 'string', desc: '"llms" | "ontologies" | "sparql"' },
            { name: 'resource', type: 'object', desc: 'Resource definition with id, name, uri/endpoint, and type-specific fields' },
        ],
        returns: 'Array of QualiaQuin — one type assertion + one per field',
        snippets: [
            cli(`
# List all LLM resources in the catalog
qualia-cli resources list llms

# List ontology resources
qualia-cli resources list ontologies

# Show full details for a specific resource
qualia-cli resources show phi3-mini-4k-instruct-q4

# Download a GGUF model (streams → GGufSharder → WAL pointer map)
qualia-cli resources download phi3-mini-4k-instruct-q4

# Download + ingest an ontology (→ .q42 + WAL provenance)
qualia-cli resources import-ontology prov-o
`),
            rs(`
use qualia_core_db::resource_catalog::{LLMResource, OntologyResource, SPARQLResource};

// Each resource type implements to_quins()
let llm = LLMResource {
    id: "phi3-mini-4k-instruct-q4".into(),
    name: "Phi-3-mini 4K (Q4_K_M)".into(),
    uri: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf".into(),
    size_bytes: 2_200_000_000,
    quantization: "Q4_K_M".into(),
    context_window: 4096,
};
let quins = llm.to_quins();
// → 6 quins: type, name, uri, size_bytes, quantization, context_window

let ont = OntologyResource {
    id: "prov-o".into(),
    name: "W3C PROV-O".into(),
    uri: "https://www.w3.org/ns/prov-o".into(),
    shacl_shapes: vec!["prov:Entity".into(), "prov:Activity".into()],
};
let ont_quins = ont.to_quins();
// → 3 base quins + 1 per SHACL shape
`),
            js(`
// Resource ID → subject hash
const resourceSubject = q_hash('phi3-mini-4k-instruct-q4');

// Type predicates
const TYPE_LLM      = q_hash('resource:llm');
const TYPE_ONTOLOGY = q_hash('resource:ontology');
const TYPE_SPARQL   = q_hash('resource:sparql');

// Numeric fields use inline integer encoding (type tag 0b001 << 60n)
const contextWindowQuin = {
    subject:   resourceSubject,
    predicate: q_hash('qualia:contextWindow'),
    object:    (1n << 60n) | 4096n,   // xsd:integer, value 4096
    context:   0n,
    metadata:  0n,
    parity:    0n,
};
`),
        ],
        live: async (_wasm, _native, inputs) => {
            const FNV_OFFSET = 0xcbf29ce484222325n;
            const FNV_PRIME  = 0x100000001b3n;
            const MASK_64    = 0xffffffffffffffffn;
            const ENC        = new TextEncoder();
            function q_hash(s) {
                let h = FNV_OFFSET;
                for (const b of ENC.encode(s)) h = ((h ^ BigInt(b)) * FNV_PRIME) & MASK_64;
                return h;
            }
            const id = (inputs.resource_id || 'phi3-mini-4k-instruct-q4').trim();
            const subject = q_hash(id);
            return {
                resource_id:         id,
                subject_hash:        '0x' + subject.toString(16).padStart(16, '0'),
                type_predicate:      '0x' + q_hash('qualia:resourceType').toString(16).padStart(16, '0'),
                type_llm_hash:       '0x' + q_hash('resource:llm').toString(16).padStart(16, '0'),
                type_ontology_hash:  '0x' + q_hash('resource:ontology').toString(16).padStart(16, '0'),
                type_sparql_hash:    '0x' + q_hash('resource:sparql').toString(16).padStart(16, '0'),
                shacl_shape_pred:    '0x' + q_hash('qualia:shaclShape').toString(16).padStart(16, '0'),
            };
        },
        liveInputs: [{ name: 'resource_id', label: 'Resource ID', default: 'phi3-mini-4k-instruct-q4' }],
    },
];

// ─── Index helpers ─────────────────────────────────────────────────────────────

export const CATEGORIES = [...new Set(CATALOG.map(e => e.category))];

export function getById(id) {
    return CATALOG.find(e => e.id === id) || null;
}

export function getByCategory(cat) {
    return CATALOG.filter(e => e.category === cat);
}
