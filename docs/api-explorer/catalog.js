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
// { status: "active", engine: "qualia-core-db", version: "0.0.30", webtorrent: { … } }
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
# Immediately receives: {"type":"HANDSHAKE_SUCCESS","payload":{"mode":"NATIVE","version":"0.0.30"}}
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
        summary: 'Register a unified v3 .q42 ontology for HTTP web-seeding on the Qualia daemon. Seeding runs in-process (seeder: qualia-daemon). Magnets include a ws= parameter pointing at /torrent/webseed/{hash}. Separate .c.q42 transport files are obsolete — LZ4 SuperBlocks live inside the .q42.',
        params: [
            { name: 'info_hash', type: 'string', desc: 'SHA-1 info hash (40 hex chars)' },
            { name: 'file_path', type: 'string', desc: 'Absolute path to the unified .q42 volume' },
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
  "file_path": "C:/Users/me/.qualia/Index/prov-o.q42",
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
        summary: 'Serve a registered unified .q42 as an HTTP web seed (BEP-19). Supports Range requests for LZ4 SuperBlocks. Referenced by magnet URIs via the ws= parameter.',
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
        summary: 'Import an ontology from a remote URI into a unified v3 .q42, compute SHA-1 info hash, and build a magnet URI with ws= for Permissive Commons sharing.',
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
  -d '{"info_hash":"…","file_path":"…/prov-o.q42","display_name":"PROV-O","ontology_id":"prov-o"}'
`),
        ],
    },

    {
        id: 'workbench.set_seed',
        category: 'Ontology Workbench',
        name: 'setWorkbenchSeed()',
        summary: 'Toggle active seeding for a workbench ontology. Registers the unified v3 .q42 with the Qualia daemon seeder and updates workbench.jsonl index.',
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
        summary: 'Ingest RDF into a unified Q42 v3 volume (Q42\\0 header, embedded Q42LEX, BIDX, FIDX, PIDX, LZ4 SuperBlocks). Sidecar .q42.lex / .q42.bidx files are not written.',
        params: [
            { name: '--input',  type: 'path', desc: 'Path to .nt / .ttl / .rdf input' },
            { name: '--output', type: 'path', desc: 'Path for the unified .q42 volume' },
        ],
        returns: 'one .q42 file (256-byte v3 header + embedded lexicon/indexes + LZ4 SuperBlocks)',
        snippets: [
            cli(`
# Ingest N-Triples into a unified v3 volume
qualia-cli ingest --input data.nt --output data.q42
qualia-cli q42 inspect data.q42
qualia-cli q42 verify data.q42

# Output is a single file:
#   data.q42  — Q42\\0 v3 · Q42LEX + BIDX + FIDX + PIDX + LZ4 SuperBlocks
# Sidecar .q42.lex / .q42.bidx are obsolete.
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
        summary: 'Write a tiny unified v3 .q42 for tests. Not a flat 144-byte Quin dump — inspect/verify expect Q42\\0.',
        params: [
            { name: 'out_path', type: 'path', desc: 'Output path for the .q42 test volume' },
        ],
        returns: 'unified v3 .q42 (header + lexicon + one SuperBlock)',
        snippets: [
            cli(`
qualia-cli dump test_block.q42
qualia-cli q42 inspect test_block.q42
# → unified v3 volume; use q42 verify, not a 144-byte size check.
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
        summary: 'SHACL constraints that compile to ACT-R cognitive opcodes in the Webizen VM. qualia:retrieveByActivation → NativeRetrieveByActivation. qualia:decayMetadata → NativeDecayMetadata. qualia:unless → NativeUnless. All ACT-R opcodes execute inline on Core 1 (NativeUnless acts as non-monotonic default logic). Activation levels are encoded in Quin metadata bits 0–31 as fixed-point u32.',
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
                core1_inline:          'NativeRetrieveByActivation + NativeDecayMetadata → executes inline on Core 1',
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
        name: 'QCHK Binary Format (.qchk binary)',
        summary: 'QualiaDB Capability Profile binary format. Declares the allowed engine operations and ontology namespaces for an agent session. Six named profiles: general, health, chemistry, research, legal, financial. Compiled via qualia-cli profile compile. Canonical extension: .qchk. Legacy .chk QCHK files are distinguished from CogAI text .chk files by the "QCHK" magic bytes at offset 0.',
        params: [
            { name: 'profile_id',   type: 'u64 (little-endian at offset 4)',  desc: 'q_hash("profile:<name>") — e.g. q_hash("profile:health")' },
            { name: 'payload_len',  type: 'u32 (little-endian at offset 12)', desc: 'Byte length of the JSON-LD payload' },
            { name: 'payload',      type: 'UTF-8 JSON-LD at offset 16',       desc: 'Profile declaration with allowed engines and ontology namespaces' },
        ],
        returns: 'Bound CapabilityProfile — restricts available SlgOpcodes and ontology namespaces for the session',
        snippets: [
            cli(`
# Compile a JSON-LD capability profile to a QCHK binary
qualia-cli profile compile health.jsonld --out health.qchk

# List all known profile IDs and their q_hash values
qualia-cli profile list

# Decode and inspect a compiled .qchk file
qualia-cli profile inspect health.qchk

# Bind a profile during ingest (restricts opcodes to health-permitted set)
qualia-cli ingest --input patient-graph.ttl --output patient.q42 --profile health.qchk

# ── .qchk / legacy .chk disambiguation ──────────────────────────────────
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

let profile = CapabilityProfile::load_from_chk(std::fs::read("health.qchk")?)?;
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

    // ═══════════════════════════════════════════════════════════════════════════
    // LOGIC MODALITIES — uncataloged additions
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'modality.deontic',
        category: 'Logic Modalities',
        name: 'Deontic Logic (O/P/F)',
        summary: 'Obligation (0x10), Permission (0x11), Forbidding (0x12) norm compilation and evaluation. Norms are packed into a NQuin predicate field. An "unless" defeater sets bit 63 to cancel matching obligations at evaluation time.',
        params: [
            { name: 'party',          type: 'u64',    desc: 'DID hash of the norm subject (compiled from did: IRI via q_hash)' },
            { name: 'opcode',         type: 'u8',     desc: 'Obligation=0x10 / Permission=0x11 / Forbidding=0x12' },
            { name: 'property_path',  type: 'u64',    desc: 'q_hash of the rights property path e.g. q42:disclose' },
            { name: 'action_object',  type: 'u64',    desc: 'Target resource or action hash' },
            { name: 'contract',       type: 'u64',    desc: 'Contract identifier stored in context field' },
            { name: 'expiry_unix32',  type: 'u32',    desc: 'Epoch expiry timestamp — 0 = never expires' },
            { name: 'is_defeater',    type: 'bool',   desc: 'When true, sets bit 63 — marks this as an unless-clause' },
        ],
        returns: 'NQuin — compiled norm record with ECC parity',
        snippets: [
            js(`
// Compile an obligation: Alice MUST disclose confidential data under NDA
const OP_OBLIGATE = 0x10n;
const DEFEATER_BIT = 1n << 63n;

function compileNorm(party, opcode, path, obj, contract, expiry, isDefeater) {
    let pred = ((BigInt(path) << 8n) & ~DEFEATER_BIT) | opcode;
    if (isDefeater) pred |= DEFEATER_BIT;
    return { subject: party, predicate: pred, object: obj, context: contract,
             metadata: BigInt(expiry), parity: 0n };
}

const alice   = q_hash('did:alice');
const disclose = q_hash('q42:disclose');
const norm    = compileNorm(alice, OP_OBLIGATE, disclose,
    q_hash('q42:data:confidential'), q_hash('contract:nda'), 4_000_000_000, false);

// Add an unless-defeater: obligation is cancelled if Alice is a certified auditor
const auditor = q_hash('q42:role:certified-auditor');
const unless  = compileNorm(alice, 0x11n, disclose, auditor,
    q_hash('contract:nda'), 4_000_000_000, true);
`),
            rs(`
use qualia_core_db::deontic_logic::compile_n3_rule_to_norm;
use qualia_core_db::modalities::logic::n3_parser::Rule;

// Compile an N3 rule to a deontic norm Quin
let rule = Rule { /* ... parsed from N3 logic */ };
let norm_quin = compile_n3_rule_to_norm(&rule, contract_hash, expiry_unix32);
`),
        ],
        live: async (_wasm, _native, inputs) => {
            const FNV_OFFSET = 0xcbf29ce484222325n;
            const FNV_PRIME  = 0x100000001b3n;
            const MASK64     = 0xffffffffffffffffn;
            function qh(s) {
                let h = FNV_OFFSET;
                for (const b of new TextEncoder().encode(s)) h = ((h ^ BigInt(b)) * FNV_PRIME) & MASK64;
                return h;
            }
            const opcodeMap = { 'Obligate': 0x10n, 'Permit': 0x11n, 'Forbid': 0x12n };
            const opcode = opcodeMap[inputs.opcode] ?? 0x10n;
            const DEFEATER_BIT = 1n << 63n;
            const party  = qh(inputs.party  || 'did:alice');
            const path   = qh(inputs.target || 'q42:disclose');
            let pred = ((path << 8n) & ~DEFEATER_BIT) | opcode;
            return {
                predicate_hex: '0x' + pred.toString(16).padStart(16, '0'),
                opcode_hex: '0x' + opcode.toString(16),
                party_hash:  '0x' + party.toString(16).padStart(16, '0'),
            };
        },
        liveInputs: [
            { name: 'party',  label: 'Party DID',        default: 'did:alice' },
            { name: 'target', label: 'Property path',    default: 'q42:disclose' },
            { name: 'opcode', label: 'Opcode',           default: 'Obligate', options: ['Obligate', 'Permit', 'Forbid'] },
        ],
    },

    {
        id: 'modality.agency',
        category: 'Logic Modalities',
        name: 'Agency & Fiduciary Stamps',
        summary: 'Encodes the Principal ≠ Thing separation required by the Webizen governance model. stamp_fiduciary_metadata() embeds principal DID hash and agent routing lane bits into a NQuin metadata field. verify_human_agency() validates an Ed25519 signature over the author-scoped Merkle root.',
        params: [
            { name: 'principal_did_hash', type: 'u64', desc: 'q_hash of the principal DID — owner of the data' },
            { name: 'agent_did_hash',     type: 'u64', desc: 'q_hash of the acting agent DID' },
        ],
        returns: 'void — modifies NQuin metadata field in-place',
        snippets: [
            rs(`
use qualia_core_db::agency::{stamp_fiduciary_metadata, verify_human_agency};
use qualia_core_db::q_hash;

let principal = q_hash("did:wellfare:alice");
let agent     = q_hash("did:wellfare:agent:care-assistant");

// Stamp the NQuin so routing lanes carry fiduciary metadata
stamp_fiduciary_metadata(&mut quin, principal, agent);

// Later — verify the Merkle root signature
let result = verify_human_agency(&frame, principal, &verifying_key, &signature);
assert!(result.is_ok());
`),
            js(`
// JS-side: fiduciary lane bits are in metadata bits 61-62
const LANE_FIDUCIARY = 0b10n << 61n;
const principalHash  = q_hash('did:wellfare:alice');

quin.metadata = (quin.metadata & ~(0b11n << 61n)) | LANE_FIDUCIARY;
quin.metadata = (quin.metadata & 0xFFFFFFFF00000000n) | (principalHash & 0xFFFFFFFFn);
`),
        ],
    },

    {
        id: 'modality.comorbidity',
        category: 'Logic Modalities',
        name: 'Comorbidity Exacerbation Quins',
        summary: 'Encodes organ-specific comorbidity risk compounding as RDF-Star nested Quin pairs. compile_exacerbation_quins() emits 2 NQuins: a primary directed edge (condition A → condition B) and a nested annotation carrying severity and patient context.',
        params: [
            { name: 'ante_condition',  type: 'u64', desc: 'Antecedent condition hash e.g. q_hash("condition:smoking")' },
            { name: 'cons_condition',  type: 'u64', desc: 'Consequent condition hash e.g. q_hash("condition:pneumonia")' },
            { name: 'patient_context', type: 'u64', desc: 'Patient graph context hash' },
            { name: 'severity',        type: 'f32', desc: 'Exacerbation severity 0.0 – 1.0' },
        ],
        returns: '[NQuin; 2] — primary edge + nested severity annotation',
        snippets: [
            rs(`
use qualia_core_db::{comorbidity_eval::compile_exacerbation_quins, q_hash};

let mut out = [NQuin::default(); 2];
let n = compile_exacerbation_quins(
    q_hash("condition:smoking"),
    q_hash("condition:pneumonia"),
    q_hash("patient:p001"),
    0.72,   // severity
    &mut out,
);
// out[0] = primary edge: smoking → pneumonia in patient context
// out[1] = nested annotation: bit 62 set, severity packed into metadata
`),
            js(`
// Comorbidity edges are evaluated with deontic logic to gate treatment protocols.
// A severity of 1.0 triggers obligate-high-risk norms downstream.
const smoker    = q_hash('condition:smoking');
const pneumonia = q_hash('condition:pneumonia');
const NESTED_BIT = 1n << 62n;

// Primary edge
const edge = { subject: smoker, predicate: q_hash('q42:exacerbates'), object: pneumonia };
// Nested annotation carries severity × 1000 in metadata low-32
const annotation = { subject: smoker | NESTED_BIT, predicate: q_hash('q42:severity'),
                      object: pneumonia, metadata: BigInt(Math.round(0.72 * 1000)) };
`),
        ],
    },

    {
        id: 'modality.dicom',
        category: 'Logic Modalities',
        name: 'DICOM Split-Ingest',
        summary: 'Ingests DICOM imaging studies by separating blob pixel data from semantic NQuin metadata. Each DICOM tag maps to a NQuin predicate via q_hash. Pixel blobs are stored by OPFS content-address; the NQuin object field holds only the hash pointer.',
        params: [
            { name: 'study_uid',   type: 'string', desc: 'DICOM Study Instance UID' },
            { name: 'modality',    type: 'string', desc: 'CT / MR / US / XR' },
            { name: 'blob_hash',   type: 'u64',    desc: 'OPFS content-address (q_hash of pixel bytes)' },
            { name: 'series_date', type: 'u64',    desc: 'Unix timestamp of study acquisition' },
        ],
        returns: 'Vec<NQuin> — semantic annotation quins (no pixel data inline)',
        snippets: [
            js(`
// DICOM split-ingest pattern in the browser
const studyHash  = q_hash('urn:dicom:study:1.2.840.10008.5.1.4.1');
const modalityH  = q_hash('dicom:modality:CT');
const pixelData  = await file.arrayBuffer();
const blobHash   = await crypto.subtle.digest('SHA-256', pixelData);
const blobRef    = q_hash(new Uint8Array(blobHash));

// Semantic NQuins — stored in graph; pixel data stays in OPFS
const quins = [
    makeQuin(studyHash, q_hash('rdf:type'),          q_hash('dicom:ImagingStudy')),
    makeQuin(studyHash, q_hash('dicom:modality'),     modalityH),
    makeQuin(studyHash, q_hash('dicom:pixelDataRef'), blobRef),   // pointer only
];
`),
            rs(`
// DicomMetadata::from_iri_hash() resolves tag predicates from q_hash values
use qualia_core_db::dicom::DicomMetadata;
let tag = DicomMetadata::from_iri_hash(q_hash("dicom:modality"));
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // GOVERNANCE & WEBIZEN
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'governance.propose_agreement',
        category: 'Governance & Webizen',
        name: 'webizen_propose_agreement()',
        summary: 'Initiates a multi-party Webizen agreement. Returns a Lamport-stamped agreement ID (u64). All listed guardians must call sign_agreement() to ratify before the agreement activates.',
        params: [
            { name: 'guardians',            type: 'string[]', desc: 'DID strings of required co-signers' },
            { name: 'principal',            type: 'string',   desc: 'Principal DID who the agreement serves' },
            { name: 'domain',               type: 'string',   desc: 'Domain string — "health", "finance", "legal"' },
            { name: 'required_signatures',  type: 'number',   desc: 'Minimum signature threshold (M-of-N)' },
        ],
        returns: 'BigInt — agreement ID (u64 Lamport-stamped)',
        snippets: [
            js(`
import init, { webizen_propose_agreement } from './qualia_core_db.js';
await init();

const agreementId = webizen_propose_agreement(
    ['did:wellfare:guardian1', 'did:wellfare:guardian2'],
    'did:wellfare:alice',
    'health',
    2   // require both guardians
);
console.log('Agreement ID:', agreementId.toString());
`),
            cli(`q42 governance propose \\
    --guardian did:wellfare:guardian1 \\
    --guardian did:wellfare:guardian2 \\
    --principal did:wellfare:alice \\
    --domain health \\
    --threshold 2`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.webizen_propose_agreement) return { error: 'WASM not loaded or function unavailable' };
            const id = wasm.webizen_propose_agreement(
                [inputs.guardian || 'did:wellfare:guardian1'],
                inputs.principal || 'did:wellfare:alice',
                inputs.domain    || 'health',
                1
            );
            return { agreement_id: String(id), agreement_id_hex: '0x' + id.toString(16) };
        },
        liveInputs: [
            { name: 'guardian',  label: 'Guardian DID',  default: 'did:wellfare:guardian1' },
            { name: 'principal', label: 'Principal DID', default: 'did:wellfare:alice' },
            { name: 'domain',    label: 'Domain',        default: 'health' },
        ],
    },

    {
        id: 'governance.sign_agreement',
        category: 'Governance & Webizen',
        name: 'webizen_sign_agreement()',
        summary: 'Co-signs a pending Webizen agreement by agreement ID. When the required_signatures threshold is met the agreement is activated and its deontic norms become enforceable.',
        params: [
            { name: 'agreement_id', type: 'BigInt', desc: 'Agreement ID from webizen_propose_agreement()' },
            { name: 'signing_key',  type: 'string', desc: 'Signer public key (base64) or DID key reference' },
        ],
        returns: 'void',
        snippets: [
            js(`
import init, { webizen_propose_agreement, webizen_sign_agreement } from './qualia_core_db.js';
await init();

const id = webizen_propose_agreement(['did:wellfare:g1'], 'did:wellfare:alice', 'health', 1);
webizen_sign_agreement(id, 'mock_key_or_did_key_ref');
`),
        ],
    },

    {
        id: 'governance.enforce_rights',
        category: 'Governance & Webizen',
        name: 'enforce_rights_ontology()',
        summary: 'Enforces the N3Logic Rights Ontology against a principal DID hash. DID hash 0 is always denied (reserved). Returns false if the principal lacks the required rights clearance for the current graph context.',
        params: [
            { name: 'principal_did_hash', type: 'BigInt (u64)', desc: 'FNV-1a hash of the principal DID string' },
        ],
        returns: 'bool — true if rights are satisfied',
        snippets: [
            js(`
import init, { enforce_rights_ontology } from './qualia_core_db.js';
await init();

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME  = 0x100000001b3n;
function q_hash(s) {
    let h = FNV_OFFSET;
    for (const b of new TextEncoder().encode(s)) h = ((h ^ BigInt(b)) * FNV_PRIME) & 0xffffffffffffffffn;
    return h;
}

const allowed = enforce_rights_ontology(q_hash('did:wellfare:alice'));
console.log('Access granted:', allowed);
`),
            rs(`
use qualia_core_db::{webizen::enforce_rights_ontology, q_hash};
let ok = enforce_rights_ontology(q_hash("did:wellfare:alice"));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.enforce_rights_ontology) return { error: 'WASM not loaded or function unavailable' };
            const hash = BigInt(inputs.did_hash || '0');
            return { enforced: wasm.enforce_rights_ontology(hash) };
        },
        liveInputs: [
            { name: 'did_hash', label: 'Principal DID hash (u64 decimal)', default: '12345678901234567890' },
        ],
    },

    {
        id: 'governance.prune_mesh',
        category: 'Governance & Webizen',
        name: 'prune_and_validate_mesh()',
        summary: 'Prunes expired or invalidated Quins from a named semantic mesh and verifies its Merkle integrity. Mesh ID 0 is reserved and always returns false.',
        params: [
            { name: 'mesh_id', type: 'BigInt (u64)', desc: 'Semantic mesh identifier' },
        ],
        returns: 'bool — true if the mesh is valid after pruning',
        snippets: [
            js(`
import init, { prune_and_validate_mesh } from './qualia_core_db.js';
await init();
const valid = prune_and_validate_mesh(1n);
console.log('Mesh valid:', valid);
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // WASM API — uncataloged additions
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'wasm.validate_fhir_observation',
        category: 'WASM API',
        name: 'validate_fhir_observation_wasm()',
        summary: 'Validates a FHIR Observation resource against LOINC reference ranges. Returns an interpretation code: N (Normal), H (High), L (Low), A (Abnormal).',
        params: [
            { name: 'loinc_code',      type: 'string',    desc: 'LOINC code e.g. "2093-3" for total cholesterol' },
            { name: 'value',           type: 'f64',       desc: 'Measured numeric value' },
            { name: 'unit_ucum',       type: 'string',    desc: 'UCUM unit string e.g. "mmol/L"' },
            { name: 'reference_low',   type: 'f64|null',  desc: 'Lower bound of normal range (optional)' },
            { name: 'reference_high',  type: 'f64|null',  desc: 'Upper bound of normal range (optional)' },
        ],
        returns: '{ is_valid: bool, status: string, interpretation_code: string }',
        snippets: [
            js(`
import init, { validate_fhir_observation_wasm } from './qualia_core_db.js';
await init();

const result = validate_fhir_observation_wasm({
    loinc_code:     '2093-3',        // Total cholesterol
    value:          5.2,
    unit_ucum:      'mmol/L',
    reference_low:  3.5,
    reference_high: 5.0,
});
// { is_valid: true, status: '...', interpretation_code: 'H' }
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.validate_fhir_observation_wasm) return { error: 'WASM not loaded' };
            return wasm.validate_fhir_observation_wasm({
                loinc_code:     inputs.loinc  || '2093-3',
                value:          parseFloat(inputs.value) || 5.2,
                unit_ucum:      inputs.unit   || 'mmol/L',
                reference_low:  parseFloat(inputs.low)  || 3.5,
                reference_high: parseFloat(inputs.high) || 5.0,
            });
        },
        liveInputs: [
            { name: 'loinc', label: 'LOINC code',  default: '2093-3' },
            { name: 'value', label: 'Value',        default: '5.2' },
            { name: 'unit',  label: 'UCUM unit',    default: 'mmol/L' },
            { name: 'low',   label: 'Ref low',      default: '3.5' },
            { name: 'high',  label: 'Ref high',     default: '5.0' },
        ],
    },

    {
        id: 'wasm.check_drug_interactions',
        category: 'WASM API',
        name: 'check_drug_interactions_wasm()',
        summary: 'Checks pharmacological interactions between a list of medications using q_hash fingerprinting against a compiled interaction graph.',
        params: [
            { name: 'medications', type: 'string[]', desc: 'Medication names — case-insensitive, hashed via q_hash internally' },
        ],
        returns: 'Array<{ mechanism: string, severity: string }>',
        snippets: [
            js(`
import init, { check_drug_interactions_wasm } from './qualia_core_db.js';
await init();

const interactions = check_drug_interactions_wasm({
    medications: ['warfarin', 'aspirin', 'ibuprofen'],
});
for (const ix of interactions) {
    console.log(\`\${ix.severity}: \${ix.mechanism}\`);
}
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.check_drug_interactions_wasm) return { error: 'WASM not loaded' };
            const meds = (inputs.meds || 'warfarin, aspirin').split(',').map(s => s.trim());
            return wasm.check_drug_interactions_wasm({ medications: meds });
        },
        liveInputs: [
            { name: 'meds', label: 'Medications (comma-separated)', default: 'warfarin, aspirin, ibuprofen' },
        ],
    },

    {
        id: 'wasm.evaluate_lipinski',
        category: 'WASM API',
        name: 'evaluate_lipinski_wasm()',
        summary: 'Evaluates Lipinski Rule of Five, Veber, Ghose, and Egan drug-likeness filters for oral bioavailability from a SMILES string.',
        params: [
            { name: 'smiles', type: 'string', desc: 'SMILES molecular representation' },
        ],
        returns: '{ lipinski_passes, lipinski_violations, veber_passes, ghose_passes, egan_passes, mw, logp, tpsa, hbd, hba, rot_bonds }',
        snippets: [
            js(`
import init, { evaluate_lipinski_wasm } from './qualia_core_db.js';
await init();

const r = evaluate_lipinski_wasm({ smiles: 'CC(=O)Oc1ccccc1C(=O)O' }); // aspirin
console.log('Lipinski passes:', r.lipinski_passes, '| MW:', r.mw.toFixed(1));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.evaluate_lipinski_wasm) return { error: 'WASM not loaded' };
            return wasm.evaluate_lipinski_wasm({ smiles: inputs.smiles || 'CC(=O)Oc1ccccc1C(=O)O' });
        },
        liveInputs: [{ name: 'smiles', label: 'SMILES', default: 'CC(=O)Oc1ccccc1C(=O)O' }],
    },

    {
        id: 'wasm.detect_functional_groups',
        category: 'WASM API',
        name: 'detect_functional_groups_wasm()',
        summary: 'Identifies functional groups (hydroxyl, carbonyl, amine, carboxyl, etc.) in a molecule from SMILES. Returns detected groups and pKa estimates for ionisable sites.',
        params: [
            { name: 'smiles', type: 'string', desc: 'SMILES string' },
        ],
        returns: '{ functional_groups: string[], pka_estimates: [group, pka, is_acid][] }',
        snippets: [
            js(`
import init, { detect_functional_groups_wasm } from './qualia_core_db.js';
await init();

const r = detect_functional_groups_wasm({ smiles: 'CCO' }); // ethanol
console.log('Groups:', r.functional_groups);
console.log('pKa:', r.pka_estimates);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.detect_functional_groups_wasm) return { error: 'WASM not loaded' };
            return wasm.detect_functional_groups_wasm({ smiles: inputs.smiles || 'CCO' });
        },
        liveInputs: [{ name: 'smiles', label: 'SMILES', default: 'CCO' }],
    },

    {
        id: 'wasm.compute_reaction_metrics',
        category: 'WASM API',
        name: 'compute_reaction_metrics_wasm()',
        summary: 'Computes green chemistry metrics: atom economy, E-factor, process mass intensity (PMI), and reaction mass efficiency (RME).',
        params: [
            { name: 'reactant_smiles', type: 'string[]', desc: 'SMILES of each reactant' },
            { name: 'product_smiles',  type: 'string',   desc: 'SMILES of desired product' },
            { name: 'yield_fraction',  type: 'f64',      desc: 'Reaction yield 0.0–1.0' },
            { name: 'solvent_kg',      type: 'f64',      desc: 'Solvent + auxiliary mass in kg' },
            { name: 'product_kg',      type: 'f64',      desc: 'Collected product mass in kg' },
        ],
        returns: '{ atom_economy_pct, e_factor, process_mass_intensity, reaction_mass_efficiency_pct, yield_corrected_ae_pct }',
        snippets: [
            js(`
import init, { compute_reaction_metrics_wasm } from './qualia_core_db.js';
await init();

const m = compute_reaction_metrics_wasm({
    reactant_smiles: ['CCO'],        // ethanol
    product_smiles:  'CC(=O)O',     // acetic acid
    yield_fraction:  0.85,
    solvent_kg:      10.0,
    product_kg:      1.0,
});
console.log('Atom economy:', m.atom_economy_pct.toFixed(1) + '%');
console.log('E-factor:',     m.e_factor.toFixed(2));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.compute_reaction_metrics_wasm) return { error: 'WASM not loaded' };
            return wasm.compute_reaction_metrics_wasm({
                reactant_smiles: [inputs.reactant || 'CCO'],
                product_smiles:  inputs.product   || 'CC(=O)O',
                yield_fraction:  parseFloat(inputs.yld)     || 0.85,
                solvent_kg:      parseFloat(inputs.solvent) || 10.0,
                product_kg:      parseFloat(inputs.prod_kg) || 1.0,
            });
        },
        liveInputs: [
            { name: 'reactant', label: 'Reactant SMILES', default: 'CCO' },
            { name: 'product',  label: 'Product SMILES',  default: 'CC(=O)O' },
            { name: 'yld',      label: 'Yield',           default: '0.85' },
            { name: 'solvent',  label: 'Solvent kg',      default: '10' },
            { name: 'prod_kg',  label: 'Product kg',      default: '1' },
        ],
    },

    {
        id: 'wasm.compute_thermochemistry',
        category: 'WASM API',
        name: 'compute_thermochemistry_wasm()',
        summary: 'Calculates Gibbs free energy (ΔG = ΔH − TΔS), equilibrium constant (Keq), Henderson-Hasselbalch pH, and Arrhenius rate constant. Supply only the fields you need — others may be null.',
        params: [
            { name: 'delta_h_j_mol',           type: 'f64',     desc: 'Enthalpy change in J/mol' },
            { name: 'delta_s_j_mol_k',         type: 'f64',     desc: 'Entropy change in J/(mol·K)' },
            { name: 'temp_k',                  type: 'f64',     desc: 'Temperature in Kelvin' },
            { name: 'pka',                     type: 'f64|null', desc: 'pKa for Henderson-Hasselbalch (optional)' },
            { name: 'conc_base',               type: 'f64|null', desc: '[A⁻] concentration — only when pka is set' },
            { name: 'conc_acid',               type: 'f64|null', desc: '[HA] concentration — only when pka is set' },
            { name: 'activation_energy_j_mol', type: 'f64|null', desc: 'Activation energy Ea in J/mol for Arrhenius' },
            { name: 'pre_exponential_a',       type: 'f64|null', desc: 'Pre-exponential factor A for Arrhenius' },
        ],
        returns: '{ gibbs_energy_j_mol, equilibrium_constant, ph: f64|null, rate_constant: f64|null }',
        snippets: [
            js(`
import init, { compute_thermochemistry_wasm } from './qualia_core_db.js';
await init();

// Gibbs + Keq for an exothermic reaction at 298 K
const r = compute_thermochemistry_wasm({
    delta_h_j_mol:   -100_000,
    delta_s_j_mol_k: 50,
    temp_k:          298.15,
    pka: null, conc_base: null, conc_acid: null,
    activation_energy_j_mol: null, pre_exponential_a: null,
});
console.log('ΔG =', r.gibbs_energy_j_mol.toFixed(0), 'J/mol');
console.log('Keq =', r.equilibrium_constant.toExponential(2));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.compute_thermochemistry_wasm) return { error: 'WASM not loaded' };
            return wasm.compute_thermochemistry_wasm({
                delta_h_j_mol:   parseFloat(inputs.dh) || -100000,
                delta_s_j_mol_k: parseFloat(inputs.ds) || 50,
                temp_k:          parseFloat(inputs.t)  || 298.15,
                pka: null, conc_base: null, conc_acid: null,
                activation_energy_j_mol: null, pre_exponential_a: null,
            });
        },
        liveInputs: [
            { name: 'dh', label: 'ΔH (J/mol)',   default: '-100000' },
            { name: 'ds', label: 'ΔS (J/mol·K)', default: '50' },
            { name: 't',  label: 'Temp (K)',      default: '298.15' },
        ],
    },

    {
        id: 'wasm.validate_fasta',
        category: 'WASM API',
        name: 'validate_fasta_wasm()',
        summary: 'Validates a FASTA sequence record, auto-detects the alphabet (DNA / RNA / Protein), and reports any invalid characters.',
        params: [
            { name: 'header',   type: 'string', desc: 'FASTA header line (without the leading >)' },
            { name: 'sequence', type: 'string', desc: 'Sequence string — case insensitive' },
        ],
        returns: '{ is_valid: bool, alphabet: string, invalid_chars: char[] }',
        snippets: [
            js(`
import init, { validate_fasta_wasm } from './qualia_core_db.js';
await init();

const r = validate_fasta_wasm({
    header:   'seq1 | Homo sapiens | BRCA1 exon 11',
    sequence: 'ATCGATCGATCGTAGCTAGC',
});
console.log(r.is_valid, r.alphabet); // true, "Dna"
`),
            rs(`
use qualia_core_db::domains::biological::bioinformatics::validate_fasta_record;

let rec = validate_fasta_record("seq1 | BRCA1", b"ATCGATCG");
assert!(rec.is_valid);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.validate_fasta_wasm) return { error: 'WASM not loaded' };
            return wasm.validate_fasta_wasm({
                header:   inputs.header || 'seq1',
                sequence: inputs.seq    || 'ATCGATCGATCG',
            });
        },
        liveInputs: [
            { name: 'header', label: 'Header', default: 'seq1 | Homo sapiens' },
            { name: 'seq',    label: 'Sequence', default: 'ATCGATCGATCG' },
        ],
    },

    {
        id: 'wasm.simulate_gbm_path',
        category: 'WASM API',
        name: 'simulate_gbm_path_wasm()',
        summary: 'Simulates a single Geometric Brownian Motion price path using the exact closed-form solution (S(t) = S₀·exp((μ−σ²/2)t + σ√t·Z)). Returns the path, final price, and min/max.',
        params: [
            { name: 'initial_price', type: 'f64', desc: 'Starting asset price S₀' },
            { name: 'drift',         type: 'f64', desc: 'Expected annual return μ' },
            { name: 'volatility',    type: 'f64', desc: 'Annual volatility σ' },
            { name: 'time_horizon',  type: 'f64', desc: 'Time in years T' },
            { name: 'steps',         type: 'u32', desc: 'Number of time steps (capped at 252)' },
        ],
        returns: '{ final_price: f64, min_price: f64, max_price: f64, path: f64[] }',
        snippets: [
            js(`
import init, { simulate_gbm_path_wasm } from './qualia_core_db.js';
await init();

const r = simulate_gbm_path_wasm({
    initial_price: 100.0,
    drift:         0.08,   // 8% annual return
    volatility:    0.20,   // 20% volatility
    time_horizon:  1.0,
    steps:         252,    // daily steps
});
console.log('Final price:', r.final_price.toFixed(2));
console.log('Path length:', r.path.length);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.simulate_gbm_path_wasm) return { error: 'WASM not loaded' };
            return wasm.simulate_gbm_path_wasm({
                initial_price: parseFloat(inputs.price) || 100,
                drift:         parseFloat(inputs.drift) || 0.08,
                volatility:    parseFloat(inputs.vol)   || 0.2,
                time_horizon:  parseFloat(inputs.t)     || 1.0,
                steps:         parseInt(inputs.steps)   || 252,
            });
        },
        liveInputs: [
            { name: 'price', label: 'Initial price', default: '100' },
            { name: 'drift', label: 'Drift μ',       default: '0.08' },
            { name: 'vol',   label: 'Volatility σ',  default: '0.2' },
            { name: 't',     label: 'Years T',       default: '1' },
            { name: 'steps', label: 'Steps',         default: '252' },
        ],
    },

    {
        id: 'wasm.black_scholes',
        category: 'WASM API',
        name: 'black_scholes_wasm()',
        summary: 'Black-Scholes European option pricing with full Greeks (delta, gamma, vega, theta, rho). Uses the standard closed-form solution for European calls and puts.',
        params: [
            { name: 'spot',       type: 'f64',  desc: 'Underlying asset price S' },
            { name: 'strike',     type: 'f64',  desc: 'Option strike price K' },
            { name: 'rate',       type: 'f64',  desc: 'Risk-free interest rate r (annual, e.g. 0.05)' },
            { name: 'vol',        type: 'f64',  desc: 'Implied volatility σ (annual, e.g. 0.2)' },
            { name: 'time_years', type: 'f64',  desc: 'Time to expiration in years T' },
            { name: 'is_call',    type: 'bool', desc: 'true for call option, false for put' },
        ],
        returns: '{ price, delta, gamma, vega, theta, rho }',
        snippets: [
            js(`
import init, { black_scholes_wasm } from './qualia_core_db.js';
await init();

// ATM call: S=K=100, r=5%, σ=20%, T=1yr
const opt = black_scholes_wasm({
    spot: 100, strike: 100, rate: 0.05, vol: 0.2, time_years: 1.0, is_call: true,
});
console.log('Price:', opt.price.toFixed(2));
console.log('Delta:', opt.delta.toFixed(4));
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.black_scholes_wasm) {
                // Pure JS fallback — standard Black-Scholes
                const S = parseFloat(inputs.spot)   || 100;
                const K = parseFloat(inputs.strike) || 100;
                const r = parseFloat(inputs.rate)   || 0.05;
                const v = parseFloat(inputs.vol)    || 0.2;
                const T = parseFloat(inputs.t)      || 1.0;
                const call = inputs.type !== 'put';
                function phi(x) {
                    const a1=0.254829592, a2=-0.284496736, a3=1.421413741,
                          a4=-1.453152027, a5=1.061405429, p=0.3275911;
                    const sign = x < 0 ? -1 : 1;
                    x = Math.abs(x);
                    const t2 = 1/(1+p*x);
                    const y = 1 - (((((a5*t2+a4)*t2)+a3)*t2+a2)*t2+a1)*t2*Math.exp(-x*x);
                    return 0.5*(1+sign*y);
                }
                const d1 = (Math.log(S/K)+(r+v*v/2)*T)/(v*Math.sqrt(T));
                const d2 = d1 - v*Math.sqrt(T);
                const price = call
                    ? S*phi(d1) - K*Math.exp(-r*T)*phi(d2)
                    : K*Math.exp(-r*T)*phi(-d2) - S*phi(-d1);
                const delta = call ? phi(d1) : phi(d1)-1;
                const nd1   = Math.exp(-d1*d1/2)/Math.sqrt(2*Math.PI);
                const gamma = nd1/(S*v*Math.sqrt(T));
                const vega  = S*nd1*Math.sqrt(T)/100;
                const theta = (-(S*nd1*v)/(2*Math.sqrt(T)) - r*K*Math.exp(-r*T)*(call?phi(d2):phi(-d2)))/365;
                const rho   = call ? K*T*Math.exp(-r*T)*phi(d2)/100 : -K*T*Math.exp(-r*T)*phi(-d2)/100;
                return { price: +price.toFixed(4), delta: +delta.toFixed(4),
                         gamma: +gamma.toFixed(6), vega: +vega.toFixed(4),
                         theta: +theta.toFixed(4), rho: +rho.toFixed(4),
                         note: 'computed via JS fallback' };
            }
            return wasm.black_scholes_wasm({
                spot:       parseFloat(inputs.spot)   || 100,
                strike:     parseFloat(inputs.strike) || 100,
                rate:       parseFloat(inputs.rate)   || 0.05,
                vol:        parseFloat(inputs.vol)    || 0.2,
                time_years: parseFloat(inputs.t)      || 1.0,
                is_call:    inputs.type !== 'put',
            });
        },
        liveInputs: [
            { name: 'spot',   label: 'Spot S',    default: '100' },
            { name: 'strike', label: 'Strike K',  default: '100' },
            { name: 'rate',   label: 'Rate r',    default: '0.05' },
            { name: 'vol',    label: 'Vol σ',     default: '0.2' },
            { name: 't',      label: 'Years T',   default: '1' },
            { name: 'type',   label: 'Type',      default: 'call', options: ['call', 'put'] },
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTROL THEORY
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'control.pid_step',
        category: 'Control Theory',
        name: 'compute_pid_step_wasm()',
        summary: 'Stateless PID controller step. Returns the control output, updated error, and updated integral for chaining into the next step. Based on control_feedback.rs PidParameters presets (conservative_power_system / aggressive_response).',
        params: [
            { name: 'setpoint',      type: 'f64', desc: 'Target value' },
            { name: 'current_value', type: 'f64', desc: 'Current measured value' },
            { name: 'prev_error',    type: 'f64', desc: 'Error from the previous step (0 for first step)' },
            { name: 'integral',      type: 'f64', desc: 'Accumulated integral (0 for first step)' },
            { name: 'kp',            type: 'f64', desc: 'Proportional gain' },
            { name: 'ki',            type: 'f64', desc: 'Integral gain' },
            { name: 'kd',            type: 'f64', desc: 'Derivative gain' },
            { name: 'dt',            type: 'f64', desc: 'Time step in seconds' },
        ],
        returns: '{ output: f64, new_error: f64, new_integral: f64 }',
        snippets: [
            js(`
import init, { compute_pid_step_wasm } from './qualia_core_db.js';
await init();

// Simulate 10 steps of power grid frequency stabilisation
let error = 0, integral = 0;
let value = 80;  // target = 100 (e.g. 100 Hz)
for (let i = 0; i < 10; i++) {
    const r = compute_pid_step_wasm({
        setpoint: 100, current_value: value,
        prev_error: error, integral,
        kp: 0.5, ki: 0.1, kd: 0.05, dt: 1.0,
    });
    value += r.output;
    error    = r.new_error;
    integral = r.new_integral;
    console.log(\`step \${i+1}: value=\${value.toFixed(2)}\`);
}
`),
            rs(`
use qualia_core_db::modalities::control_feedback::{ControlState, PidParameters};

let params = PidParameters::conservative_power_system();
let mut ctrl = ControlState::new(100.0, 80.0, params);
ctrl.update(85.0, 1_000);   // new_value=85, time_ms=1000
`),
        ],
        live: async (wasm, _native, inputs) => {
            // Always available — pure JS fallback
            const sp  = parseFloat(inputs.sp)  || 100;
            const cv  = parseFloat(inputs.cv)  || 80;
            const kp  = parseFloat(inputs.kp)  || 0.5;
            const ki  = parseFloat(inputs.ki)  || 0.1;
            const kd  = parseFloat(inputs.kd)  || 0.05;
            const err = sp - cv;
            if (wasm?.compute_pid_step_wasm) {
                return wasm.compute_pid_step_wasm({ setpoint: sp, current_value: cv,
                    prev_error: 0, integral: 0, kp, ki, kd, dt: 1.0 });
            }
            return { output: +(kp * err).toFixed(4), new_error: +err.toFixed(4),
                     new_integral: +err.toFixed(4), note: 'P-only JS fallback' };
        },
        liveInputs: [
            { name: 'sp', label: 'Setpoint',      default: '100' },
            { name: 'cv', label: 'Current value', default: '80' },
            { name: 'kp', label: 'Kp',            default: '0.5' },
            { name: 'ki', label: 'Ki',            default: '0.1' },
            { name: 'kd', label: 'Kd',            default: '0.05' },
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // SOLVERS
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'solvers.sat',
        category: 'Solvers',
        name: 'solve_sat_wasm()',
        summary: 'Bounded DPLL SAT solver. Accepts clauses as arrays of signed integer literals (positive = variable true, negative = variable negated). Returns satisfiability and a satisfying assignment.',
        params: [
            { name: 'clauses', type: 'number[][]', desc: 'Array of clauses; each clause is an array of integer literals' },
        ],
        returns: '{ satisfiable: bool, assignment: Record<string, bool> }',
        snippets: [
            js(`
import init, { solve_sat_wasm } from './qualia_core_db.js';
await init();

// (x1 ∨ x2 ∨ ¬x3) ∧ (¬x1 ∨ x3) ∧ (x2 ∨ ¬x3)
const result = solve_sat_wasm({
    clauses: [[1, 2, -3], [-1, 3], [2, -3]],
});
console.log('SAT:', result.satisfiable);
console.log('Assignment:', result.assignment);
`),
            rs(`
use qualia_core_db::solvers::symbolic_logic::{BoundedSatSolver, Clause, Literal, SolverConfig};

let mut solver = BoundedSatSolver::new(SolverConfig::default());
solver.add_clause(Clause { literals: [Literal { var: 1, positive: true }, ...] })?;
let state = solver.solve()?;
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.solve_sat_wasm) return { error: 'WASM not loaded — solve_sat_wasm not yet exported' };
            try {
                return wasm.solve_sat_wasm({ clauses: JSON.parse(inputs.clauses || '[[1,2]]') });
            } catch (e) { return { error: e.message }; }
        },
        liveInputs: [
            { name: 'clauses', label: 'Clauses (JSON array of arrays)', default: '[[1,2,-3],[-1,3],[2,-3]]' },
        ],
    },

    {
        id: 'solvers.forward_chain',
        category: 'Solvers',
        name: 'forward_chain_wasm()',
        summary: 'Forward-chaining defeasible inference engine. Derives all provable conclusions from facts and rules, respecting defeater cancellation via ForwardChainingDefeasible in symbolic_logic.rs.',
        params: [
            { name: 'facts', type: 'string[]',                    desc: 'Initial fact propositions (string atoms)' },
            { name: 'rules', type: '{head, body, defeaters}[]',   desc: 'Inference rules — head fires when all body facts hold and no defeater fires' },
        ],
        returns: '{ inferred: string[] }',
        snippets: [
            js(`
import init, { forward_chain_wasm } from './qualia_core_db.js';
await init();

const kb = {
    facts: ['bird', 'penguin'],
    rules: [
        { head: 'flies',    body: ['bird'],   defeaters: ['penguin'] },
        { head: 'swims',    body: ['penguin'], defeaters: [] },
    ],
};
const r = forward_chain_wasm(kb);
console.log(r.inferred); // ['swims'] — penguin defeater cancels 'flies'
`),
            rs(`
use qualia_core_db::solvers::symbolic_logic::{ForwardChainingDefeasible, DefeasibleRule, Fact};

let mut engine = ForwardChainingDefeasible::new(SolverConfig::default());
engine.add_fact(Fact { proposition: q_hash("bird"), ... })?;
let state = engine.infer()?;
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.forward_chain_wasm) return { error: 'WASM not loaded — forward_chain_wasm not yet exported' };
            try { return wasm.forward_chain_wasm(JSON.parse(inputs.kb)); }
            catch (e) { return { error: e.message }; }
        },
        liveInputs: [
            { name: 'kb', label: 'Knowledge base (JSON)', default: '{"facts":["bird","penguin"],"rules":[{"head":"flies","body":["bird"],"defeaters":["penguin"]},{"head":"swims","body":["penguin"],"defeaters":[]}]}' },
        ],
    },

    {
        id: 'solvers.ode_decay',
        category: 'Solvers',
        name: 'solve_ode_exponential_decay_wasm()',
        summary: 'RK4 fourth-order Runge-Kutta solver for the canonical exponential decay ODE: dy/dt = −k·y. Returns full time and value arrays. Backed by RungeKutta4Static in solvers/calculus/mod.rs.',
        params: [
            { name: 'k',       type: 'f64', desc: 'Decay constant (must be > 0)' },
            { name: 'y0',      type: 'f64', desc: 'Initial value y(t₀)' },
            { name: 't0',      type: 'f64', desc: 'Start time' },
            { name: 't_final', type: 'f64', desc: 'End time' },
            { name: 'dt',      type: 'f64', desc: 'Time step (smaller = more accurate)' },
        ],
        returns: '{ t_values: f64[], y_values: f64[], final_y: f64 }',
        snippets: [
            js(`
import init, { solve_ode_exponential_decay_wasm } from './qualia_core_db.js';
await init();

// Radioactive decay: half-life T½ = ln2/k
const r = solve_ode_exponential_decay_wasm({
    k: 0.5, y0: 100, t0: 0, t_final: 5, dt: 0.1,
});
console.log('Final value:', r.final_y.toFixed(2)); // ≈ 8.21 (analytical: 100·e^{-2.5})
`),
            rs(`
use qualia_core_db::solvers::calculus::{RungeKutta4Static, SolverConfig};

let mut rk4 = RungeKutta4Static::new(0.1, SolverConfig::default());
let result = rk4.integrate(&|_t, y| [-0.5 * y[0], 0.0, 0.0, 0.0], 0.0, [100.0, 0.0, 0.0, 0.0], 5.0)?;
`),
        ],
        live: async (wasm, _native, inputs) => {
            const k  = parseFloat(inputs.k)  || 0.5;
            const y0 = parseFloat(inputs.y0) || 100;
            const tf = parseFloat(inputs.tf) || 5;
            if (wasm?.solve_ode_exponential_decay_wasm) {
                return wasm.solve_ode_exponential_decay_wasm({ k, y0, t0: 0, t_final: tf, dt: 0.1 });
            }
            // Analytical fallback
            const final_y = y0 * Math.exp(-k * tf);
            return { final_y: +final_y.toFixed(4), note: 'analytical solution (WASM not loaded)' };
        },
        liveInputs: [
            { name: 'k',  label: 'Decay rate k',   default: '0.5' },
            { name: 'y0', label: 'Initial value',  default: '100' },
            { name: 'tf', label: 'Final time',     default: '5' },
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // SEMANTIC WEB
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'semantic.rdf_star',
        category: 'Semantic Web',
        name: 'RDF-Star (Nested Triples)',
        summary: 'RDF-Star support via the RdfStarParser and RdfStarSerializer traits in rdf_star.rs. Embedded triples are stored as virtual NQuin IDs — bit 62 of the subject field signals a nested-triple reference. The virtual ID is the FNV-1a hash of the serialised (s, p, o) components.',
        params: [
            { name: 'embedded_triple', type: '(u64, u64, u64)', desc: 'Subject, predicate, object of the triple being annotated' },
            { name: 'annotation_pred', type: 'u64',             desc: 'Predicate of the meta-statement' },
            { name: 'annotation_obj',  type: 'u64',             desc: 'Object of the meta-statement' },
        ],
        returns: 'u64 — virtual subject ID (bit 62 set)',
        snippets: [
            rs(`
use qualia_core_db::rdf_star::RdfStarParser;

// Parse an embedded triple << :Alice :knows :Bob >> :certainty 0.9
let virtual_id = parser.parse_embedded_triple(input)?;
// virtual_id has bit 62 set — signals nested reference in NQuin subject field
`),
            nt(`
# RDF-Star (N-Triples*) — annotating a triple with a certainty score
<< <https://example.org/Alice> <http://xmlns.com/foaf/0.1/knows> <https://example.org/Bob> >>
    <https://example.org/certainty> "0.9"^^<http://www.w3.org/2001/XMLSchema#decimal> .
`),
            js(`
// Nested triple bit convention — bit 62 marks a virtual RDF-Star subject
const NESTED_BIT = 1n << 62n;
const virtualId  = q_hash('alice:knows:bob') | NESTED_BIT;

const annotation = {
    subject:   virtualId,              // << alice knows bob >>
    predicate: q_hash('ex:certainty'),
    object:    q_hash('"0.9"^^xsd:decimal'),
};
`),
        ],
    },

    {
        id: 'semantic.wal',
        category: 'Semantic Web',
        name: 'WAL (Write-Ahead Log)',
        summary: 'Crash-safe append-only WAL in wal.rs. append_mutation() synchronously fsync-flushes each NQuin. recover() replays uncommitted quins after restart. checkpoint_to_dag() commits the WAL into a Merkle DAG node and returns a 32-byte SHA3-256 root hash.',
        params: [
            { name: 'path', type: '&Path', desc: 'File path — typically data_dir/wal.log' },
        ],
        returns: 'WalLog struct with open, append_mutation, recover, truncate, checkpoint_to_dag',
        snippets: [
            rs(`
use qualia_core_db::wal::WalLog;

// Open (or create) the WAL
let mut wal = WalLog::open("data/wal.log")?;

// Append a mutation — synchronous flush, zero memory allocation
wal.append_mutation(&quin)?;

// Crash recovery — replay all uncommitted quins
let pending = wal.recover()?;
for q in &pending { graph.insert(q); }

// Checkpoint: write a Merkle DAG node and truncate the WAL
let root_hash = wal.checkpoint_to_dag(&mut dag_store, author_did, timestamp_ms)?;
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // MISSING LOGIC MODALITY CATALOG ENTRIES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'modality.argumentation',
        category: 'Logic Modalities',
        name: 'Argumentation (Dung Frameworks)',
        summary: 'Abstract argumentation framework (Dung 1995): arguments, attack relations, admissible sets, grounded extension, and preferred extensions. Used for Peace Infrastructure debate resolution. Bit constants ARGUMENT_BIT (55), ATTACK_BIT (54), DEFENSE_BIT (53) mark NQuin predicate roles.',
        params: [
            { name: 'arguments', type: 'Map<u64, Argument>',  desc: 'Argument set keyed by ID' },
            { name: 'attacks',   type: 'Vec<Attack>',          desc: 'Attack relation: {attacker, target, strength}' },
        ],
        returns: 'Vec<u64> — grounded extension (minimal complete semantics)',
        snippets: [
            rs(`
use qualia_core_db::modalities::argumentation::{ArgumentationFramework, Argument, Attack, AttackType};

let mut fw = ArgumentationFramework::new();
let a1 = Argument::new(1, "Climate change is anthropogenic".into(), vec![], conclusion_quin);
let a2 = Argument::new(2, "Solar variability is the cause".into(), vec![], rebuttal_quin);
fw.arguments.insert(1, a1);
fw.arguments.insert(2, a2);
fw.attacks.push(Attack { attacker: 1, target: 2, attack_type: AttackType::Rebuttal, strength: 0.9 });
fw.attacks.push(Attack { attacker: 2, target: 1, attack_type: AttackType::Undercut,  strength: 0.3 });
// Grounded extension: argument 1 wins (higher strength prevails)
`),
            js(`
// JS reference — grounded extension (characteristic function fixpoint)
const fw = { arguments: new Map([[1, { id: 1, strength: 0.9 }], [2, { id: 2, strength: 0.3 }]]),
              attacks: [{ attacker: 1, target: 2 }] };
`),
        ],
    },

    {
        id: 'modality.graph_theory',
        category: 'Logic Modalities',
        name: 'Graph Theory (QualiaGraph)',
        summary: 'Graph algorithms over NQuin-derived edges: degree centrality, PageRank, BFS shortest path, and label-propagation community detection. QualiaGraph::from_quins() treats each NQuin as a directed edge (subject → object).',
        params: [
            { name: 'quins', type: '&[NQuin]', desc: 'Slice of NQuins — each becomes a directed edge subject→object' },
        ],
        returns: 'QualiaGraph with nodes HashMap<u64, GraphNode> and edges HashMap<(u64,u64), GraphEdge>',
        snippets: [
            rs(`
use qualia_core_db::modalities::graph_theory::QualiaGraph;

let mut graph = QualiaGraph::from_quins(&quins);
graph.calculate_betweenness_centrality();
// graph.nodes: each has .centrality_score, .degree, .community_id
let communities = graph.detect_communities();
`),
            js(`
// Build graph from NQuin array
const edges = quins.map(q => [q.subject, q.object]);
const adj   = new Map();
for (const [s, o] of edges) {
    if (!adj.has(s)) adj.set(s, []);
    adj.get(s).push(o);
}
// Degree centrality
const n = new Set([...edges.flat()]).size;
for (const [src, nbrs] of adj) {
    const dc = nbrs.length / (n - 1);
    console.log(\`\${src.toString(16)}: DC=\${dc.toFixed(3)}\`);
}
`),
        ],
    },

    {
        id: 'modality.interval_reasoning',
        category: 'Logic Modalities',
        name: 'Interval Reasoning (Allen Algebra)',
        summary: 'Allen\'s 13 base temporal relations (Before, After, Meets, MetBy, Overlaps, OverlappedBy, Starts, StartedBy, During, Contains, Ends, EndedBy, Equals) plus intersection, union, and gap operations. Used for WAL checkpoint scheduling and temporal graph constraint satisfaction.',
        params: [
            { name: 'a',   type: 'TemporalInterval', desc: '{id, start, end, duration} — all i64 Unix timestamps or relative ticks' },
            { name: 'b',   type: 'TemporalInterval', desc: 'Second interval to relate' },
        ],
        returns: 'AllenRelation — one of the 13 base relations',
        snippets: [
            rs(`
use qualia_core_db::modalities::interval_reasoning::{TemporalInterval, AllenRelation};

let task    = TemporalInterval::new(1, 9 * 3600, 10 * 3600); // 09:00–10:00
let meeting = TemporalInterval::new(2, 10 * 3600, 11 * 3600); // 10:00–11:00

assert_eq!(task.allen_relation(&meeting), AllenRelation::Meets);

let ix = task.intersection(&meeting); // None — they only touch
let gap = task.gap(&meeting);         // Some(0)
`),
            js(`
// Allen relation: determines temporal ordering
function allen(a, b) {
    if (a.end < b.start)             return 'Before';
    if (b.end < a.start)             return 'After';
    if (a.end === b.start)           return 'Meets';
    if (a.start === b.start && a.end < b.end) return 'Starts';
    if (a.start > b.start && a.end < b.end)   return 'During';
    if (a.start === b.start && a.end === b.end) return 'Equals';
    // ... all 13 cases
}
`),
        ],
    },

    {
        id: 'modality.diffusion',
        category: 'Logic Modalities',
        name: 'Discrete Diffusion (GPU Cellular Automaton)',
        summary: 'GPU-accelerated diffusion passes over NQuin graphs via WGSL compute shaders. trigger_diffusion(graph_id) is the synchronous CLI gate (returns false for empty IDs). execute_diffusion_pass() dispatches an async wgpu pipeline that reads NQuins from a storage buffer, applies diffusion.wgsl per-thread, and writes results back. Each NQuin diffusion edge packs its weight in metadata bits 31:0.',
        params: [
            { name: 'graph_id', type: 'string',  desc: 'Named graph to diffuse — empty string is a no-op' },
            { name: 'graph',    type: '&mut [NQuin]', desc: 'Mutable slice dispatched to the GPU compute pass' },
        ],
        returns: 'trigger_diffusion → bool; execute_diffusion_pass → Result<(), String>',
        snippets: [
            rs(`
use qualia_core_db::modalities::diffusion::{trigger_diffusion, execute_diffusion_pass};

// Synchronous gate
if trigger_diffusion("semantic-mesh-001") {
    // Async GPU pass (Tokio runtime required)
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(execute_diffusion_pass(&mut graph_quins)).expect("GPU diffusion failed");
}
`),
            js(`
// Diffusion weight convention: stored in NQuin metadata bits 31:0
const DIFFUSION_BIT = 1n << 48n;
function isDiffusionEdge(q) { return (q.predicate & DIFFUSION_BIT) !== 0n; }
function getWeight(q)       { return Number(q.metadata & 0xFFFFFFFFn) / 1_000_000; }

// CPU-side one-step propagation (mirrors GPU pass):
function diffuseStep(graph, values) {
    const next = new Map(values);
    for (const q of graph.filter(isDiffusionEdge)) {
        const w = getWeight(q);
        next.set(q.object, (next.get(q.object) ?? 0) + w * (values.get(q.subject) ?? 0));
    }
    return next;
}
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // MISSING WASM EXPORT ENTRIES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'wasm.predict_receptor_binding',
        category: 'WASM API',
        name: 'predict_receptor_binding_wasm()',
        summary: 'Quantum DFT receptor binding affinity prediction using the Physics-Informed Neural Network (PINN) pipeline. Returns a binding affinity in kcal/mol (negative = tighter binding per IUPAC convention). Backed by LocalLlmAgent + wgpu tensor pass rather than an external ML runtime.',
        params: [],
        returns: 'f64 — binding affinity in kcal/mol (negative values indicate binding)',
        snippets: [
            js(`
import init, { predict_receptor_binding_wasm } from './qualia_core_db.js';
await init();

const affinity = predict_receptor_binding_wasm();
// kcal/mol convention: negative = favourable binding
console.log('Binding affinity:', affinity.toFixed(2), 'kcal/mol');
console.log('Interpretation:', affinity < -7 ? 'strong binder' : 'weak binder');
`),
        ],
        live: async (wasm) => {
            if (!wasm?.predict_receptor_binding_wasm) return { error: 'WASM not loaded' };
            const aff = wasm.predict_receptor_binding_wasm();
            return { binding_affinity_kcal_mol: aff, interpretation: aff < -7 ? 'strong binder' : 'moderate' };
        },
        liveInputs: [],
    },

    {
        id: 'wasm.resolve_lww',
        category: 'WASM API',
        name: 'resolve_lww_wasm()',
        summary: 'Merges two conflicting NQuin records from offline-first sync using Last-Writer-Wins semantics. The Lamport clock is in metadata bits 63:32; on clock tie, higher object hash wins for deterministic resolution.',
        params: [
            { name: 'local',  type: 'QuinJson', desc: '{subject, predicate, object, context, metadata, parity}' },
            { name: 'remote', type: 'QuinJson', desc: 'Conflicting remote NQuin record' },
        ],
        returns: 'QuinJson — the winning record',
        snippets: [
            js(`
import init, { resolve_lww_wasm } from './qualia_core_db.js';
await init();

// Offline-first sync: Alice edited on phone (clock 1002), Bob on desktop (clock 1005)
const local  = { subject: 1n, predicate: 2n, object: 10n, context: 0n, metadata: 1002n << 32n, parity: 0n };
const remote = { subject: 1n, predicate: 2n, object: 20n, context: 0n, metadata: 1005n << 32n, parity: 0n };
const winner = resolve_lww_wasm(local, remote);
console.log('Winner object:', winner.object); // 20n — remote wins (higher clock)
`),
        ],
        live: async (wasm, _n, inputs) => {
            if (!wasm?.resolve_lww_wasm) return { error: 'WASM not loaded' };
            const lc = BigInt(inputs.local_clock  || '1002') << 32n;
            const rc = BigInt(inputs.remote_clock || '1005') << 32n;
            const local  = { subject: 1n, predicate: 2n, object: 10n, context: 0n, metadata: lc, parity: 0n };
            const remote = { subject: 1n, predicate: 2n, object: 20n, context: 0n, metadata: rc, parity: 0n };
            return wasm.resolve_lww_wasm(local, remote);
        },
        liveInputs: [
            { name: 'local_clock',  label: 'Local Lamport clock',  default: '1002' },
            { name: 'remote_clock', label: 'Remote Lamport clock', default: '1005' },
        ],
    },

    {
        id: 'wasm.engine_version',
        category: 'WASM API',
        name: 'get_engine_version()',
        summary: 'Returns the qualia-core-db crate version baked in at compile time. Identical to the version field returned by the native daemon\'s GET /health endpoint.',
        params: [],
        returns: 'string — semver string e.g. "0.0.30"',
        snippets: [
            js(`
import init, { get_engine_version } from './qualia_core_db.js';
await init();
console.log(get_engine_version()); // "0.0.30"
`),
        ],
        live: async (wasm) => {
            if (!wasm?.get_engine_version) return { error: 'WASM not loaded' };
            return { version: wasm.get_engine_version() };
        },
        liveInputs: [],
    },

    {
        id: 'wasm.engine_info',
        category: 'WASM API',
        name: 'get_engine_info()',
        summary: 'Returns structured engine metadata: version, engine name ("qualia-core-db"), build target ("wasm32"), and the full WASM_CAPABILITY_REGISTRY array. Useful for feature-detection in browser clients.',
        params: [],
        returns: '{ version, engine, target, capabilities: string[] }',
        snippets: [
            js(`
import init, { get_engine_info } from './qualia_core_db.js';
await init();
const info = get_engine_info();
console.log(info.engine);         // "qualia-core-db"
console.log(info.capabilities);   // ["SHACL", "QueryEngine", "N3Parser", ...]
`),
        ],
        live: async (wasm) => {
            if (!wasm?.get_engine_info) return { error: 'WASM not loaded' };
            return wasm.get_engine_info();
        },
        liveInputs: [],
    },

    {
        id: 'wasm.list_capabilities',
        category: 'WASM API',
        name: 'list_capabilities_wasm()',
        summary: 'Returns the WASM capability registry as a JSON array of string identifiers. Each string corresponds to a feature compiled into this WASM binary. Use this for runtime feature detection before calling optional exports.',
        params: [],
        returns: 'string[] — capability identifiers e.g. ["SHACL", "QueryEngine", "BlackScholes", ...]',
        snippets: [
            js(`
import init, { list_capabilities_wasm } from './qualia_core_db.js';
await init();
const caps = list_capabilities_wasm();
if (caps.includes('BlackScholes')) {
    // Safe to call black_scholes_wasm()
}
`),
        ],
        live: async (wasm) => {
            if (!wasm?.list_capabilities_wasm) return { error: 'WASM not loaded' };
            return wasm.list_capabilities_wasm();
        },
        liveInputs: [],
    },

    {
        id: 'wasm.initialize_webgpu',
        category: 'WASM API',
        name: 'initialize_webgpu_engine()',
        summary: 'Boots the native WebGPU LLM engine from a model Uint8Array. Dual-format: canonical P64 (`p64\\0`) is validated and exposed through a synthetic tensor index; GGUF uses its native metadata index. Resident weights and the KV arena are initialized before decode.',
        params: [
            { name: 'model_data', type: 'Uint8Array', desc: 'Raw GGUF bytes or a canonical P64 v3 container' },
        ],
        returns: 'Promise<void>',
        snippets: [
            js(`
import init, { initialize_webgpu_engine, compileGgufToP64, p64FormatVersion } from '../playground/qualia_core_db.js';
import { loadOrCompileP64 } from '../js/opfs-model-cache.js';
await init();

// AOT: compile GGUF → P64 once, cache in OPFS, warm-boot thereafter.
const { bytes } = await loadOrCompileP64(
  'https://huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct-GGUF/resolve/main/smollm2-360m-instruct-q4_k_m.gguf',
  'SmolLM2-360M-Instruct-Q4_K_M',
  { compile: compileGgufToP64, formatVersion: p64FormatVersion() },
);
await initialize_webgpu_engine(bytes);
console.log('Engine resident — ready to generate');
`),
        ],
    },

    {
        id: 'wasm.compileGgufToP64',
        category: 'WASM API',
        name: 'compileGgufToP64()',
        summary: 'Ahead-of-time compiler: turns a GGUF Uint8Array into canonical P64 v3 with page-aligned weight blobs, hyperparameters, tokenizer, manifold records, and CRC-32C integrity. The historical compileGgufToQ42 export remains as an alias.',
        params: [
            { name: 'gguf', type: 'Uint8Array', desc: 'Raw GGUF model bytes' },
            { name: 'page_log2', type: 'u16', desc: 'Page-alignment exponent (14 = 16 KB pages, the default)' },
        ],
        returns: 'Uint8Array — the P64 container',
        snippets: [
            js(`
import init, { compileGgufToP64, p64FormatVersion } from '../playground/qualia_core_db.js';
await init();
const gguf = new Uint8Array(await (await fetch('model.gguf')).arrayBuffer());
const p64 = compileGgufToP64(gguf, 14);
console.log('P64 v' + p64FormatVersion() + ', ' + (p64.length / 1048576).toFixed(1) + ' MB');
`),
        ],
    },

    {
        id: 'wasm.p64FormatVersion',
        category: 'WASM API',
        name: 'p64FormatVersion()',
        summary: 'Returns the P64 container version this build emits and consumes. The historical q42FormatVersion export returns the same value.',
        params: [],
        returns: 'number — current P64 format version',
        snippets: [
            js(`import init, { p64FormatVersion } from '../playground/qualia_core_db.js';\nawait init();\nconsole.log('P64 v' + p64FormatVersion());`),
        ],
    },

    {
        id: 'wasm.inferWasmStreaming',
        category: 'WASM API',
        name: 'inferWasmStreaming()',
        summary: 'Streaming autoregressive decode on the resident WebGPU engine (single-submit forward per token, ~5.9 tok/s on SmolLM2-360M). Calls a callback with each token delta as it is produced. The prompt must already include any chat-template tokens. inferWasmAsync is the alias the demos use.',
        params: [
            { name: 'prompt', type: 'string', desc: 'Full prompt (include chat-template markers if applicable)' },
            { name: 'on_token', type: 'Function', desc: 'Called with each streamed token-delta string' },
        ],
        returns: 'Promise<string> — the full generated text',
        snippets: [
            js(`
import init, { inferWasmStreaming } from '../playground/qualia_core_db.js';
// (initialize_webgpu_engine must have run first)
let out = '';
const full = await inferWasmStreaming('The capital of France is', (delta) => {
  out += delta;
  document.getElementById('output').textContent = out;
});
console.log(full);
`),
        ],
    },

    {
        id: 'wasm.infer',
        category: 'WASM API',
        name: 'infer_wasm()',
        summary: 'Non-streaming browser LLM inference on the native WebGPU path (gguf_bridge → llm_agent decode). Requires initialize_webgpu_engine() first with GGUF or P64. For token-by-token UX use inferWasmStreaming().',
        params: [
            { name: 'prompt', type: 'string', desc: 'User prompt (include chat-template markers if applicable)' },
        ],
        returns: 'Promise<string> — generated text',
        snippets: [
            js(`
import init, { initialize_webgpu_engine, infer_wasm } from '../playground/qualia_core_db.js';
await init();
const modelBytes = await (await fetch('models/SmolLM2-360M-Instruct-Q4_K_M.gguf')).arrayBuffer();
await initialize_webgpu_engine(new Uint8Array(modelBytes));
console.log(await infer_wasm('The capital of France is'));
`),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // SPARQL LIBRARY CAPABILITIES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'sparql.select',
        category: 'SPARQL Engine',
        name: 'SPARQL SELECT / ASK / DESCRIBE',
        summary: 'Full SPARQL 1.1 SELECT queries over the native graph engine. The sparql_library/ module compiles queries through a 5-stage pipeline: lexer → AST (sparql_ast.rs) → planner (sparql_planner.rs) → executor (sparql_executor.rs) → results. On the browser, compile_query_to_json() exposes bytecode inspection; execution runs via the native daemon at localhost:4242/query.',
        params: [
            { name: 'query', type: 'string', desc: 'SPARQL 1.1 query string' },
        ],
        returns: '{ columns: string[], rows: Record<string, RdfTerm>[] }',
        snippets: [
            http('POST /query\nContent-Type: application/sparql-query\n\nSELECT ?s ?p ?o WHERE {\n  ?s ?p ?o .\n  FILTER (?p = <https://example.org/name>)\n}\nLIMIT 100'),
            rs(`
use qualia_core_db::sparql_library::{SparqlParser, SparqlExecutor};

let ast = SparqlParser::parse("SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10")?;
let plan = SparqlPlanner::plan(&ast)?;
let results = SparqlExecutor::execute(&plan, &graph)?;
`),
            cli('q42 sparql "SELECT ?s ?o WHERE { ?s foaf:knows ?o }"'),
        ],
    },

    {
        id: 'sparql.update',
        category: 'SPARQL Engine',
        name: 'SPARQL UPDATE (INSERT / DELETE)',
        summary: 'SPARQL 1.1 Update: INSERT DATA, DELETE DATA, INSERT/DELETE with WHERE pattern. All mutations are WAL-logged before committing to the graph. sparql_update.rs handles parsing; the executor validates permissions via N3Logic rights ontology before writing.',
        params: [
            { name: 'update', type: 'string', desc: 'SPARQL Update string' },
        ],
        returns: 'void (writes to WAL + graph)',
        snippets: [
            http('POST /query\nContent-Type: application/sparql-update\n\nINSERT DATA {\n  <https://example.org/alice> <http://xmlns.com/foaf/0.1/knows> <https://example.org/bob> .\n}'),
            http('POST /query\nContent-Type: application/sparql-update\n\nDELETE WHERE { <https://example.org/alice> ?p ?o . }'),
            rs(`
use qualia_core_db::sparql_library::sparql_update::SparqlUpdate;
SparqlUpdate::execute("INSERT DATA { <did:alice> <foaf:knows> <did:bob> . }", &mut graph)?;
`),
        ],
    },

    {
        id: 'sparql.aggregates',
        category: 'SPARQL Engine',
        name: 'SPARQL Aggregates (COUNT / SUM / GROUP BY)',
        summary: 'SPARQL 1.1 aggregate functions: COUNT, SUM, AVG, MIN, MAX, GROUP_CONCAT. sparql_aggregates.rs implements stack-allocated aggregation with GROUP BY support. External sorting via external_sort.rs handles result sets larger than the SlgArena ceiling.',
        params: [
            { name: 'query', type: 'string', desc: 'SPARQL query with aggregation' },
        ],
        returns: '{ group_key, aggregate_value }[]',
        snippets: [
            http('POST /query\nContent-Type: application/sparql-query\n\nSELECT ?type (COUNT(?s) AS ?count)\nWHERE { ?s rdf:type ?type }\nGROUP BY ?type\nORDER BY DESC(?count)'),
            http('POST /query\nContent-Type: application/sparql-query\n\nSELECT (AVG(?score) AS ?avg_score)\nWHERE { ?patient q42:framinghamScore ?score }'),
        ],
    },

    {
        id: 'sparql.filters',
        category: 'SPARQL Engine',
        name: 'SPARQL FILTER / REGEX / BIND',
        summary: 'SPARQL 1.1 FILTER clause evaluation via sparql_filter.rs. Supports arithmetic comparisons, string functions (regex, contains, str, strlen), type testing (isLiteral, isIRI, lang), and BIND for computed variables. Filter expressions are compiled to bytecode for efficient repeated evaluation.',
        params: [
            { name: 'query', type: 'string', desc: 'SPARQL query with FILTER/BIND' },
        ],
        returns: 'Same as SELECT result set, filtered',
        snippets: [
            http('POST /query\nContent-Type: application/sparql-query\n\nSELECT ?name ?score\nWHERE {\n  ?p foaf:name ?name ;\n     q42:riskScore ?score .\n  FILTER (?score > 10.0 && regex(?name, "^Alice"))\n  BIND (?score * 1.1 AS ?adjustedScore)\n}'),
        ],
    },

    {
        id: 'sparql.federated',
        category: 'SPARQL Engine',
        name: 'SPARQL Federated Queries (SERVICE)',
        summary: 'SPARQL 1.1 Federated Query via SERVICE keyword. sparql_federated.rs proxies sub-queries to remote SPARQL endpoints and merges results locally. Outbound traffic to Remote backends requires a signed VC from the Principal (enforced by the Webizen VM pre-flight).',
        params: [
            { name: 'query',       type: 'string', desc: 'SPARQL query with SERVICE keyword' },
            { name: 'remote_url',  type: 'string', desc: 'Remote SPARQL endpoint URI inside SERVICE clause' },
        ],
        returns: 'Joined result set from local + remote sources',
        snippets: [
            http('POST /query\nContent-Type: application/sparql-query\n\nSELECT ?name ?remoteProp\nWHERE {\n  ?person foaf:name ?name .\n  SERVICE <https://dbpedia.org/sparql> {\n    ?person dbo:birthPlace ?remoteProp .\n  }\n}'),
            rs(`
use qualia_core_db::sparql_library::sparql_federated::FederatedQueryPlanner;
let plan = FederatedQueryPlanner::plan(&ast, &["https://dbpedia.org/sparql"])?;
`),
        ],
    },

    {
        id: 'sparql.did_context',
        category: 'SPARQL Engine',
        name: 'DID-Scoped SPARQL Contexts',
        summary: 'sparql_did.rs adds per-principal query context: every query runs against the principal\'s named graph partition. The DID hash is used as the default graph context, so ?s ?p ?o only matches triples the principal is authorized to see. Cross-principal reads require explicit GRAPH clauses with deontic permissions.',
        params: [
            { name: 'principal_did', type: 'string', desc: 'DID string — resolves to graph context via q_hash' },
            { name: 'query',         type: 'string', desc: 'SPARQL query' },
        ],
        returns: 'Filtered result set scoped to principal\'s authorized partition',
        snippets: [
            http('POST /query\nX-Principal-DID: did:wellfare:alice\nContent-Type: application/sparql-query\n\nSELECT ?s ?p ?o\nWHERE { GRAPH <did:wellfare:alice> { ?s ?p ?o } }'),
            rs(`
use qualia_core_db::sparql_library::sparql_did::DidQueryContext;
let ctx = DidQueryContext::new(q_hash("did:wellfare:alice"));
let results = SparqlExecutor::execute_with_context(&plan, &graph, &ctx)?;
`),
        ],
    },

    {
        id: 'sparql.websocket',
        category: 'SPARQL Engine',
        name: 'SPARQL WebSocket Streaming',
        summary: 'sparql_websocket.rs provides real-time result streaming for long-running or live queries. The client sends a SPARQL query over a WebSocket connection; results are pushed as JSON-Lines as they are found rather than waiting for the full result set.',
        params: [
            { name: 'query', type: 'string', desc: 'SPARQL SELECT query to stream' },
        ],
        returns: 'JSON-Lines stream — one result binding per message',
        snippets: [
            js(`
const ws = new WebSocket('ws://localhost:4242/query/stream');
ws.send(JSON.stringify({
    query: 'SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10000',
    format: 'json-lines',
}));
ws.onmessage = ({ data }) => {
    const row = JSON.parse(data);
    console.log(row.s, row.p, row.o);
};
`),
        ],
    },

    {
        id: 'sparql.multimedia',
        category: 'SPARQL Engine',
        name: 'SPARQL-MM (Multimedia Extensions)',
        summary: 'sparql_mm.rs implements W3C SPARQL-MM functions for querying video, image, and audio metadata stored as NQuins. Functions include ma:spatialRelation, ma:frameOfReference, ma:temporalClipping for media fragment handling. Useful for DICOM and multimodal AI pipelines.',
        params: [
            { name: 'query', type: 'string', desc: 'SPARQL query with SPARQL-MM function calls' },
        ],
        returns: 'Media-aware result set',
        snippets: [
            http('POST /query\nContent-Type: application/sparql-query\n\nPREFIX ma: <http://www.w3.org/ns/ma-ont#>\nSELECT ?clip ?frame\nWHERE {\n  ?video ma:hasFragment ?clip .\n  ?clip  ma:temporalClipping ?frame .\n  FILTER (ma:temporalRelation(?clip, ?frame) = "during")\n}'),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // SPECIALIZED SCIENTIFIC LIBRARIES (native-only — no WASM bridge)
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'specialized.cryptographic',
        category: 'Specialized Libraries',
        name: 'Cryptographic Library (ZK / ML-DSA / Ed25519)',
        summary: 'specialized_libs/cryptographic_library.rs implements quantum-resistant post-quantum signatures (ML-DSA / Dilithium), ZK-SNARK and ZK-STARK proofs, Ed25519 key management, and AES-GCM encryption — all as zero-allocation Rust structs. These run natively only; no WASM bridge to avoid shipping key material to the browser.',
        params: [
            { name: 'key_type', type: 'enum', desc: 'Ed25519 / MlDsa65 / MlDsa87' },
        ],
        returns: 'KeyManager, SignatureEngine, EncryptionEngine, ProofEngine',
        snippets: [
            rs(`
use qualia_core_db::specialized_libs::cryptographic_library::{KeyManager, SignatureEngine};

// Generate ML-DSA (post-quantum) key pair
let km = KeyManager::new();
let (sk, vk) = km.generate_key_pair_ml_dsa()?;

// Sign a NQuin batch root hash
let sig = SignatureEngine::sign_ml_dsa(&payload_bytes, &sk)?;
assert!(SignatureEngine::verify_ml_dsa(&payload_bytes, &sig, &vk)?);
`),
            cli('q42 key generate --type ml-dsa-65 --output my-key.q42k'),
            cli('q42 sign --key my-key.q42k --payload graph.nq'),
        ],
    },

    {
        id: 'specialized.physics',
        category: 'Specialized Libraries',
        name: 'Physics Simulation (Thermodynamics / DFT)',
        summary: 'specialized_libs/physics_simulation.rs implements MCMC thermodynamics sampling, Density Functional Theory (DFT) ground state computation, classical particle system dynamics, and quantum mechanics simulations. Uses stack-allocated arrays to stay within the SlgArena 42 MB ceiling.',
        params: [
            { name: 'temperature_k', type: 'f64', desc: 'System temperature in Kelvin' },
            { name: 'n_atoms',       type: 'usize', desc: 'Number of atoms in the simulation' },
        ],
        returns: 'SimulationState { energy, entropy, temperature, pressure }',
        snippets: [
            rs(`
use qualia_core_db::specialized_libs::physics_simulation::{
    ThermodynamicsEngine, DftCalculator
};

// MCMC thermodynamics at 300 K
let mut thermo = ThermodynamicsEngine::new(300.0, 64)?;
let state = thermo.run_mcmc(10_000)?;
println!("Free energy: {:.3} eV", state.gibbs_free_energy);

// DFT ground state
let dft = DftCalculator::new(atoms)?;
let (energy, density) = dft.compute_ground_state()?;
`),
            cli('q42 science thermodynamics --temp 300 --atoms 64 --steps 10000'),
        ],
    },

    {
        id: 'specialized.machine_learning',
        category: 'Specialized Libraries',
        name: 'Machine Learning (Edge Neural Inference)',
        summary: 'specialized_libs/machine_learning.rs implements stack-allocated neural network layers (dense, attention, normalization) that operate directly on NQuin feature vectors. ModelManager handles model lifecycle; InferenceEngine runs forward passes; TrainingEngine does on-device fine-tuning via gradient descent. Distinct from the GGUF LLM pipeline — these are domain-specific networks.',
        params: [
            { name: 'model_path', type: 'string', desc: 'Path to compiled model weights (q42 model format)' },
        ],
        returns: 'InferenceResult { predictions: Vec<f32>, confidence: f32 }',
        snippets: [
            rs(`
use qualia_core_db::specialized_libs::machine_learning::{ModelManager, InferenceEngine};

let mut mgr = ModelManager::new();
let model   = mgr.load_model("models/risk-classifier.q42m")?;
let engine  = InferenceEngine::new(model);

// Run inference on a NQuin feature vector
let features = extract_quin_features(&patient_quins);
let result   = engine.infer(&features)?;
println!("Risk class: {} (confidence {:.1}%)", result.class_label, result.confidence * 100.0);
`),
            cli('q42 ml infer --model risk-classifier.q42m --graph patient-graph.nq'),
        ],
    },

    {
        id: 'specialized.statistical',
        category: 'Specialized Libraries',
        name: 'Statistical Computing',
        summary: 'specialized_libs/statistical_computing.rs implements Bayesian inference (MCMC, variational), frequentist hypothesis testing (t-test, chi-squared, ANOVA), survival analysis (Kaplan-Meier, Cox PH), and advanced distribution sampling — all stack-allocated for no-heap operation.',
        params: [
            { name: 'data', type: '&[f64]', desc: 'Slice of observations' },
        ],
        returns: 'StatisticalSummary { mean, variance, p_value, confidence_interval }',
        snippets: [
            rs(`
use qualia_core_db::specialized_libs::statistical_computing::{
    BayesianInference, HypothesisTesting, SurvivalAnalysis
};

// Bayesian posterior estimation
let posterior = BayesianInference::mcmc_posterior(&observed, &prior, 5000)?;

// Frequentist t-test
let (t_stat, p_value) = HypothesisTesting::two_sample_t_test(&group_a, &group_b)?;

// Kaplan-Meier survival curve
let km = SurvivalAnalysis::kaplan_meier(&times, &events)?;
`),
            cli('q42 stats t-test --group-a data_a.csv --group-b data_b.csv'),
        ],
    },

    {
        id: 'specialized.engineering',
        category: 'Specialized Libraries',
        name: 'Engineering Analysis (FEA / CFD / Reliability)',
        summary: 'specialized_libs/engineering_analysis.rs implements Finite Element Analysis (structural static + dynamic), Computational Fluid Dynamics (Burgers equation, laminar flow), thermal analysis, and structural reliability indexing (β-index, Monte Carlo first-order). Used by environmental monitoring and infrastructure applications.',
        params: [
            { name: 'mesh_nodes', type: 'usize', desc: 'Number of FEA mesh nodes' },
        ],
        returns: 'AnalysisResult { displacements, stresses, safety_factor }',
        snippets: [
            rs(`
use qualia_core_db::specialized_libs::engineering_analysis::{
    StructuralAnalyzer, FluidAnalyzer, ThermalAnalyzer
};

// Static structural FEA
let structure = StructuralAnalyzer::new(mesh)?;
let result = structure.solve_static_linear(&loads, &boundary_conditions)?;
println!("Max displacement: {:.4} mm", result.max_displacement * 1000.0);

// 1D Burgers CFD
let cfd = FluidAnalyzer::new_1d_burgers(viscosity, grid_points)?;
let flow = cfd.solve(initial_condition, dt, time_steps)?;
`),
            cli('q42 science fea --mesh structure.obj --loads loads.json --output results.json'),
        ],
    },

    {
        id: 'specialized.quantum_biology',
        category: 'Specialized Libraries',
        name: 'Quantum Biology',
        summary: 'specialized_libs/quantum_biology.rs models non-trivial quantum effects in biological systems: photosynthetic quantum coherence (FMO complex), enzyme quantum tunneling, radical pair mechanism (avian magnetoreception), and quantum Zeno effects in protein folding. Uses Lindblad master equation for open quantum systems.',
        params: [
            { name: 'complex_type', type: 'enum', desc: 'FmoPhotosynthesis / EnzymeTunneling / RadicalPair' },
        ],
        returns: 'QuantumBioResult { coherence_time_ps, transfer_efficiency, quantum_advantage }',
        snippets: [
            rs(`
use qualia_core_db::specialized_libs::quantum_biology::{FmoPhotosynthesisModel, QuantumTunneling};

// FMO photosynthetic complex — quantum coherence window
let fmo = FmoPhotosynthesisModel::new(7)?; // 7-site bacteriochlorophyll network
let result = fmo.simulate_energy_transfer(300.0, 500e-12)?; // 300K, 500ps
println!("Quantum advantage: {:.2}x over classical transfer", result.quantum_advantage);

// Enzyme quantum tunneling through a barrier
let tunnel = QuantumTunneling::proton_transfer(barrier_height_ev, barrier_width_angstrom)?;
`),
            cli('q42 science quantum-bio --model fmo --sites 7 --temperature 300'),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN ONTOLOGIES
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'domain.geospatial',
        category: 'Domain Ontologies',
        name: 'Geospatial Domain (GeoSPARQL + KML)',
        summary: 'domains/geospatial/spatial.rs bridges GeoSPARQL geometry types (Point, LineString, Polygon, MultiPolygon) and KML features to NQuin predicates. Spatial relations (within, intersects, distance) use q_hash-keyed geometry quins with WKT literal objects. Used with KmlBridge for ingesting geographic data.',
        params: [
            { name: 'wkt_geometry', type: 'string', desc: 'WKT geometry string e.g. "POINT(-122.4194 37.7749)"' },
        ],
        returns: 'NQuin — geometry triple with q_hash("geo:hasGeometry") predicate',
        snippets: [
            rs(`
use qualia_core_db::domains::geospatial::spatial::GeospatialBridge;

// Ingest a GeoJSON Point as NQuin
let quin = GeospatialBridge::point_to_quin(
    q_hash("did:place:sf-city-hall"),
    -122.4194,
    37.7749,
    q_hash("ctx:geospatial-graph"),
)?;

// Spatial relation: is Point within Polygon?
let within = GeospatialBridge::check_within(&point_quin, &polygon_quin)?;
`),
            http('POST /query\nContent-Type: application/sparql-query\n\nPREFIX geo: <http://www.opengis.net/ont/geosparql#>\nSELECT ?place\nWHERE {\n  ?place geo:hasGeometry ?g .\n  FILTER (geo:sfWithin(?g, "POLYGON((-123 37, -121 37, -121 38, -123 38, -123 37))"^^geo:wktLiteral))\n}'),
            nt('<https://example.org/sf-city-hall> <http://www.opengis.net/ont/geosparql#hasGeometry> "POINT(-122.4194 37.7749)"^^<http://www.opengis.net/ont/geosparql#wktLiteral> .'),
        ],
    },

    {
        id: 'domain.mathematical',
        category: 'Domain Ontologies',
        name: 'Mathematical Domain (Geometric Algebra)',
        summary: 'domains/mathematical/geometric.rs implements a SIMD-accelerated Geometric Algebra kernel (G3 Clifford algebra) with multivector operations: grade projection, geometric product, outer product, inner product, and reversion. Runs on native targets with AVX2/NEON; falls back to scalar on WASM.',
        params: [
            { name: 'multivector', type: '[f32; 8]', desc: 'G3 multivector coefficients [scalar, e1, e2, e3, e12, e13, e23, e123]' },
        ],
        returns: 'GeometricProduct — another multivector',
        snippets: [
            rs(`
use qualia_core_db::domains::mathematical::geometric::{Multivector, GeometricAlgebra};

// G3 Clifford algebra: represent a rotation by angle θ around axis n
let n = Multivector::vector(0.0, 1.0, 0.0);    // y-axis
let theta: f32 = std::f32::consts::PI / 4.0;    // 45 degrees
let rotor = GeometricAlgebra::rotor_from_angle_axis(theta, &n);
let v = Multivector::vector(1.0, 0.0, 0.0);     // x unit vector
let rotated = GeometricAlgebra::sandwich_product(&rotor, &v); // RṽR†
`),
        ],
    },

    {
        id: 'domain.physical',
        category: 'Domain Ontologies',
        name: 'Physical Domain (Thermodynamics)',
        summary: 'domains/physical/thermodynamics.rs provides first-principles thermodynamic property calculations: ideal gas PVT relations, enthalpy/entropy from heat capacities, Carnot efficiency, and equation-of-state (van der Waals) corrections. Results are serialised as NQuin metric quins with QUDT unit predicates.',
        params: [
            { name: 'temperature_k', type: 'f64', desc: 'Temperature in Kelvin' },
            { name: 'pressure_pa',   type: 'f64', desc: 'Pressure in Pascals' },
            { name: 'moles',         type: 'f64', desc: 'Amount of substance in mol' },
        ],
        returns: 'ThermodynamicState { volume, enthalpy, entropy, gibbs_free_energy }',
        snippets: [
            rs(`
use qualia_core_db::domains::physical::thermodynamics::ThermodynamicCalculator;

// Ideal gas: PV = nRT
let state = ThermodynamicCalculator::ideal_gas_state(298.15, 101_325.0, 1.0)?;
println!("Volume: {:.4} m³", state.volume);

// Carnot efficiency
let eta = ThermodynamicCalculator::carnot_efficiency(500.0, 300.0); // T_hot, T_cold
println!("Carnot η = {:.1}%", eta * 100.0);
`),
            cli('q42 science thermodynamics --mode ideal-gas --temp 298.15 --pressure 101325 --moles 1'),
        ],
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // SOLVER CATALOG ENTRIES (linear algebra + optimization — currently native-only)
    // ═══════════════════════════════════════════════════════════════════════════

    {
        id: 'solvers.linear_algebra',
        category: 'Solvers',
        name: 'Linear Algebra (LU / Eigen / Tensor)',
        summary: 'StaticLuDecomposition (Doolittle LU with partial pivoting), FixedLanczosEigensolver (sparse eigenvalues), and ConstTensorContractor (Einstein summation) — all stack-allocated with a 4×4 matrix ceiling to stay within the zero-allocation ABI. Native-only: these are currently disabled from the WASM build due to build complexity.',
        params: [
            { name: 'matrix', type: 'Matrix4x4', desc: '4×4 f64 array [[f64; 4]; 4]' },
            { name: 'b',      type: 'Vector4',   desc: 'Right-hand side [f64; 4]' },
        ],
        returns: 'Vector4 — solution x to Ax = b',
        snippets: [
            rs(`
use qualia_core_db::solvers::linear_algebra::{StaticLuDecomposition, Matrix4x4, Vector4};
use qualia_core_db::solvers::SolverConfig;

let mut lu = StaticLuDecomposition::new(SolverConfig::default());
let m = Matrix4x4 { data: [[2.0,1.0,0.0,0.0],[4.0,3.0,0.0,0.0],[0.0,0.0,1.0,2.0],[0.0,0.0,3.0,4.0]] };
let b = Vector4 { data: [5.0, 11.0, 8.0, 18.0] };
let x = lu.solve(&m, &b)?;
// x ≈ [1.0, 3.0, 4.0, 2.0]
`),
        ],
    },

    {
        id: 'solvers.optimization',
        category: 'Solvers',
        name: 'Optimization (Nelder-Mead / Newton-Raphson / L-M)',
        summary: 'Three constrained optimization solvers: NelderMeadSimplex (derivative-free, 4-param), BoundedNewtonRaphson (1-param with bounds), and LevenbergMarquardtStack (nonlinear least squares). All use stack-allocated state and are bounded by SolverConfig::max_iterations. Native-only in current build.',
        params: [
            { name: 'initial_point', type: '[f64; 4]',        desc: 'Starting guess' },
            { name: 'objective',     type: 'fn([f64;4])->f64', desc: 'Objective function to minimize' },
        ],
        returns: 'OptimizationResult { optimal_point, optimal_value, iterations }',
        snippets: [
            rs(`
use qualia_core_db::solvers::optimization::{NelderMeadSimplex, BoundedNewtonRaphson};

// Nelder-Mead: minimize f(x,y) = (x-1)² + (y-2)²
let mut nm = NelderMeadSimplex::new([0.0, 0.0, 0.0, 0.0], SolverConfig::default());
let result = nm.minimize(&|p| (p[0]-1.0).powi(2) + (p[1]-2.0).powi(2))?;

// Newton-Raphson: find root of f(x) = x³ - x - 2 near x=2
let mut nr = BoundedNewtonRaphson::new(2.0, -10.0, 10.0, SolverConfig::default());
let root = nr.find_root(&|x| x.powi(3) - x - 2.0, &|x| 3.0*x.powi(2) - 1.0)?;
`),
        ],
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

