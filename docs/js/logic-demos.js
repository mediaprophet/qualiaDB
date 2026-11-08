/**
 * Shared N3Logic + SHACL + Modality demo definitions for playground and logic-showcase pages.
 * Uses real WASM exports (`parse_n3logic_wasm`, `validate_shacl_constraint_wasm`, `forward_chain_wasm`)
 * alongside `vibe-wasm` (`vibe:0.3/modality`, `vibe:0.3/qgraph`, `vibe:0.3/nquin`).
 */

export const N3_RULE_ARROWS = [
    { arrow: '=>', name: 'Strict', color: 'emerald', desc: 'Classical modus ponens — hard forward chaining' },
    { arrow: '~>', name: 'Defeasible', color: 'amber', desc: 'Default rule — can be overridden by a defeater' },
    { arrow: '^>', name: 'Defeater', color: 'rose', desc: 'Maps to DEFEATER_BIT — cancels matching defeasible norms' },
    { arrow: '-o', name: 'Linear', color: 'cyan', desc: 'Linear logic — premise is consumed when the rule fires' },
];

export const N3_PRESETS = {
    strict: `@prefix q42: <https://webizen.org/q42#> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .

{ ?x a foaf:Person } => { ?x a q42:Mortal } .
:socrates a foaf:Person .`,
    defeasible: `@prefix q42: <https://webizen.org/q42#> .

{ ?agent q42:role q42:Guardian } ~> { ?agent q42:may q42:accessRecords } .
{ ?agent q42:role q42:Minor } ~> { ?agent q42:mayNot q42:accessRecords } .`,
    defeater: `@prefix q42: <https://webizen.org/q42#> .

{ ?party q42:signed q42:Agreement } ~> { ?party q42:may q42:shareData } .
{ ?party q42:revoked q42:Consent } ^> { ?party q42:may q42:shareData } .`,
    linear: `@prefix q42: <https://webizen.org/q42#> .

{ ?token q42:status q42:Active } -o { ?token q42:status q42:Consumed } .
:sessionToken q42:status q42:Active .`,
    deontic: `@prefix q42: <https://webizen.org/q42#> .

{ ?g q42:hasRole q42:Guardian ; q42:must q42:obtainConsent } => { ?g q42:obligated q42:consentFlow } .
{ ?g q42:hasRole q42:Guardian } ~> { ?g q42:may q42:viewHealthRecord } .
:guardian1 q42:hasRole q42:Guardian .`,
};

export const SHACL_WASM_CONSTRAINTS = [
    { id: 'minInclusive', label: 'sh:minInclusive', example: 18, test: 21 },
    { id: 'maxInclusive', label: 'sh:maxInclusive', example: 120, test: 55 },
    { id: 'minExclusive', label: 'sh:minExclusive', example: 0, test: 0.1 },
    { id: 'maxExclusive', label: 'sh:maxExclusive', example: 100, test: 99 },
    { id: 'minCount', label: 'sh:minCount', example: 1, test: 2 },
    { id: 'maxCount', label: 'sh:maxCount', example: 5, test: 3 },
    { id: 'minLength', label: 'sh:minLength', example: 3, test: 8 },
    { id: 'maxLength', label: 'sh:maxLength', example: 64, test: 12 },
];

export const SHACL_QUALIA_EXTENSIONS = [
    {
        group: 'Deontic norms',
        icon: 'fa-gavel',
        color: 'rose',
        items: [
            { name: 'DeonticObligate', desc: 'Validates active obligation Quins (OP_OBLIGATE 0x10)' },
            { name: 'DeonticPermit', desc: 'Permissive norms — may/can actions' },
            { name: 'DeonticForbid', desc: 'Prohibition norms — must not / forbid' },
            { name: 'DeonticNotExpired', desc: 'Temporal expiry on contract-bound norms' },
        ],
    },
    {
        group: 'Epistemic state',
        icon: 'fa-brain',
        color: 'purple',
        items: [
            { name: 'EpistemicKnowledge', desc: 'Agent knows claim above min certainty (OP_KNOWS 0x20)' },
            { name: 'EpistemicBelief', desc: 'Doxastic belief with certainty threshold' },
            { name: 'CommonKnowledge', desc: 'Multi-agent common knowledge propagation' },
        ],
    },
    {
        group: 'CogAI / ACT-R',
        icon: 'fa-lightbulb',
        color: 'amber',
        items: [
            { name: 'RetrieveByActivation', desc: 'Chunk retrieval ranked by activation' },
            { name: 'DecayMetadata', desc: 'Lamport-clock decay on chunk metadata' },
            { name: 'Unless', desc: 'Defeasible unless-sentinel (DEFEATER_BIT linkage)' },
        ],
    },
    {
        group: 'Client & infra',
        icon: 'fa-server',
        color: 'cyan',
        items: [
            { name: 'LogConfiguration', desc: 'Logging buffer and flush limits' },
            { name: 'SystemTrayConfiguration', desc: 'Desktop tray menu constraints' },
            { name: 'SecurityConfiguration', desc: 'Sanctuary mode and lane keys' },
            { name: 'NetworkConfiguration', desc: 'Daemon port and federation caps' },
        ],
    },
    {
        group: 'Specialized libraries',
        icon: 'fa-flask',
        color: 'emerald',
        items: [
            { name: 'MoleculeConfiguration', desc: 'SMILES / InChI validation shapes' },
            { name: 'ClinicalDecisionConfiguration', desc: 'FHIR observation bounds' },
            { name: 'FinancialModelConfiguration', desc: 'VaR and position limit shapes' },
            { name: 'CryptographicConfiguration', desc: 'Key length and algorithm policy' },
        ],
    },
];

export const FORWARD_CHAIN_PRESETS = {
    penguin: {
        label: 'Penguin defeasible',
        facts: ['bird', 'penguin'],
        rules: [
            { head: 'flies', body: ['bird'], defeaters: ['penguin'] },
            { head: 'swims', body: ['penguin'], defeaters: [] },
        ],
    },
    guardian: {
        label: 'Guardian consent',
        facts: ['guardian', 'signed_agreement'],
        rules: [
            { head: 'may_access', body: ['guardian', 'signed_agreement'], defeaters: [] },
            { head: 'must_renew', body: ['guardian'], defeaters: ['signed_agreement'] },
        ],
    },
};

export const LOGIC_CATEGORIES = [
    { id: 'deontic', label: 'Deontic Norms (0x10–0x12)', icon: 'fa-scale-balanced', canvasKind: 'deontic' },
    { id: 'spatio-temporal', label: 'Allen Intervals & Spatio-Temporal', icon: 'fa-arrows-left-right', canvasKind: 'allen' },
    { id: 'ltl', label: 'Temporal LTL (0x40–0x44)', icon: 'fa-infinity', canvasKind: 'ltl' },
    { id: 'epistemic', label: 'Epistemic Kripke (0x20–0x22)', icon: 'fa-eye', canvasKind: 'epistemic' },
    { id: 'paraconsistent', label: 'Paraconsistent Belnap F₄ (0x30–0x32)', icon: 'fa-shield-virus', canvasKind: 'paraconsistent' },
    { id: 'inference', label: 'N3 / SHACL / Linear / Probabilistic', icon: 'fa-code-branch', canvasKind: 'inference' },
];

export const LOGIC_DEMOS = [
    {
        id: 'deontic-guardianship',
        category: 'deontic',
        canvasKind: 'deontic',
        title: 'Deontic Normative Triad & Defeater Override (0x10–0x12)',
        desc: 'Compiles OBLIGATE (0x10), PERMIT (0x11), and FORBID (0x12) norms with DEFEATER_BIT (bit 63) override in vibe-wasm and evaluates active vs defeated verdicts.',
        n3Preset: N3_PRESETS.deontic,
        vibeCode: `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let pred = modality.deontic.pack_norm_predicate(4201, 16, 0);\n  let def_pred = modality.deontic.pack_norm_predicate(4201, 17, 1);\n  let op = modality.deontic.norm_opcode(pred);\n  let par = qgraph.staging.seal_quin(101, pred, 202, 303, 1700000000);\n  let ok = nquin.wire.verify_wire_parity(101, pred, 202, 303, 1700000000, par);\n  return [op, pred, def_pred, ok];\n}`,
    },
    {
        id: 'allen-lattice',
        category: 'spatio-temporal',
        canvasKind: 'allen',
        title: 'Allen Interval Algebra & 13-Relation Lattice',
        desc: 'Interactive dual timeline evaluating modality.allen.interval_relation(sA, eA, sB, eB) in vibe-wasm at 60 FPS across all 13 interval relations.',
        n3Preset: N3_PRESETS.strict,
        vibeCode: `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let rel_before = modality.allen.interval_relation(2, 7, 10, 18);\n  let rel_overlaps = modality.allen.interval_relation(5, 14, 10, 20);\n  let par = nquin.wire.compute_wire_parity(2, 7, 10, 18, rel_before);\n  let ok = nquin.wire.verify_wire_parity(2, 7, 10, 18, rel_before, par);\n  return [rel_before, rel_overlaps, par, ok];\n}`,
    },
    {
        id: 'ltl-automaton',
        category: 'ltl',
        canvasKind: 'ltl',
        title: 'Temporal LTL Trace Automaton (0x40–0x44)',
        desc: 'Evaluates Globally G(φ), Finally F(ψ), Next X(φ), Until (φ U ψ), and Release (ψ R φ) across a 6-tick NQuin state trace.',
        n3Preset: N3_PRESETS.defeasible,
        vibeCode: `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let step = modality.allen.interval_relation(0, 6, 1, 7);\n  let par = qgraph.staging.seal_quin(64, 31, 1, 1, step);\n  let ok = qgraph.staging.verify_sealed_quin(64, 31, 1, 1, step, par);\n  return [step, par, ok];\n}`,
    },
    {
        id: 'epistemic-kripke',
        category: 'epistemic',
        canvasKind: 'epistemic',
        title: 'Epistemic Kripke Possible-Worlds & Common Knowledge (0x20–0x22)',
        desc: 'Models KNOWS (0x20), BELIEVES (0x21), and COMMON_KNOWLEDGE (0x22) across possible worlds w₀…w₃ with certainty bands [8..15].',
        n3Preset: N3_PRESETS.defeasible,
        vibeCode: `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let knows_pred = 32 + (255 * 256);\n  let par = qgraph.staging.seal_quin(701, knows_pred, 9001, 1, 255);\n  let ok = nquin.wire.verify_wire_parity(701, knows_pred, 9001, 1, 255, par);\n  return [knows_pred, par, ok];\n}`,
    },
    {
        id: 'paraconsistent-belnap',
        category: 'paraconsistent',
        canvasKind: 'paraconsistent',
        title: 'Paraconsistent Belnap F₄ Bilattice Router (0x30–0x32)',
        desc: 'Isolates contradictory assertions (Both ⊤) into a quarantined sub-context (context ⊕ q_hash("q42:isolated")) to prevent classical explosion.',
        n3Preset: N3_PRESETS.defeater,
        vibeCode: `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let iso_ctx = 5042;\n  let par = qgraph.staging.seal_quin(1, 48, 200, iso_ctx, 1);\n  let ok = qgraph.staging.verify_sealed_quin(1, 48, 200, iso_ctx, 1, par);\n  return [iso_ctx, par, ok];\n}`,
    },
    {
        id: 'inference-n3-shacl',
        category: 'inference',
        canvasKind: 'inference',
        title: 'N3 Defeasible Proof DAG, Fuzzy Curve & Linear Petri-Net',
        desc: 'Combines forward_chain_wasm, validate_shacl_constraint_wasm, and linear token consumption (-o) with CBOR-LD Tag 4200 receipts.',
        n3Preset: N3_PRESETS.linear,
        vibeCode: `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let par = qgraph.staging.seal_quin(42, 16, 1024, 7, 99);\n  let ok = nquin.wire.verify_wire_parity(42, 16, 1024, 7, 99, par);\n  return [par, ok];\n}`,
    },
];

export function getLogicDemosByCategory(catId) {
    if (!catId || catId === 'all') return LOGIC_DEMOS;
    return LOGIC_DEMOS.filter((d) => d.category === catId);
}

export function getLogicDemoById(id) {
    return LOGIC_DEMOS.find((d) => d.id === id) || LOGIC_DEMOS[0];
}

/** Client-side N3 rule surface parse (WASM parse_n3logic_wasm emits static triples only). */
export function detectN3Rules(text) {
    const rules = [];
    const re = /\{([^}]*)\}\s*(=>|~>|\^>|-o)\s*\{([^}]*)\}/g;
    let m;
    while ((m = re.exec(text)) !== null) {
        const arrow = m[2];
        const meta = N3_RULE_ARROWS.find((r) => r.arrow === arrow) ?? { arrow, name: 'Rule', desc: '' };
        rules.push({
            arrow,
            type: meta.name,
            premise: m[1].trim().replace(/\s+/g, ' '),
            conclusion: m[3].trim().replace(/\s+/g, ' '),
            desc: meta.desc,
        });
    }
    return rules;
}

export function validateShaclConstraint(mod, constraint_type, value, target_value) {
    if (!mod?.validate_shacl_constraint_wasm) {
        throw new Error('validate_shacl_constraint_wasm not available in this WASM build');
    }
    return mod.validate_shacl_constraint_wasm({ constraint_type, value, target_value });
}

export function runForwardChain(mod, input) {
    if (!mod?.forward_chain_wasm) {
        throw new Error('forward_chain_wasm not available in this WASM build');
    }
    return mod.forward_chain_wasm(input);
}

export function parseN3Triples(mod, text) {
    if (!mod?.parse_n3logic_wasm) return [];
    const out = mod.parse_n3logic_wasm(text);
    return out && Array.isArray(out) ? out : [];
}

export function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
