/**
 * Shared N3Logic + SHACL demo helpers for playground and logic-showcase pages.
 * Uses real WASM exports: parse_n3logic_wasm, validate_shacl_constraint_wasm, forward_chain_wasm.
 */

export const N3_RULE_ARROWS = [
    { arrow: '=>', name: 'Strict', color: 'emerald', desc: 'Classical modus ponens — hard forward chaining' },
    { arrow: '~>', name: 'Defeasible', color: 'amber', desc: 'Default rule — can be overridden by a defeater' },
    { arrow: '^>', name: 'Defeater', color: 'rose', desc: 'Maps to DEFEATER_BIT — cancels matching defeasible norms' },
    { arrow: '-o', name: 'Linear', color: 'cyan', desc: 'Linear logic — premise is consumed when the rule fires' },
];

export const N3_PRESETS = {
    strict: `@prefix q42: <https://qualia.network/q42#> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .

{ ?x a foaf:Person } => { ?x a q42:Mortal } .
:socrates a foaf:Person .`,
    defeasible: `@prefix q42: <https://qualia.network/q42#> .

{ ?agent q42:role q42:Guardian } ~> { ?agent q42:may q42:accessRecords } .
{ ?agent q42:role q42:Minor } ~> { ?agent q42:mayNot q42:accessRecords } .`,
    defeater: `@prefix q42: <https://qualia.network/q42#> .

{ ?party q42:signed q42:Agreement } ~> { ?party q42:may q42:shareData } .
{ ?party q42:revoked q42:Consent } ^> { ?party q42:may q42:shareData } .`,
    linear: `@prefix q42: <https://qualia.network/q42#> .

{ ?token q42:status q42:Active } -o { ?token q42:status q42:Consumed } .
:sessionToken q42:status q42:Active .`,
    deontic: `@prefix q42: <https://qualia.network/q42#> .

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