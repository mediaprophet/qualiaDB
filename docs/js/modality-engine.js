/**
 * Unified modality engine for the Modalities Observatory showcase.
 * JS reference implementations mirror docs/tests/suites/modality-*.js and Rust modules.
 */

import { q_hash, makeQuin } from '../tests/suites/primitives.js';

export { q_hash, makeQuin };

// ─── Deontic ─────────────────────────────────────────────────────────────────
const OP_OBLIGATE = 0x10n, OP_PERMIT = 0x11n, OP_FORBID = 0x12n;
const DEFEATER_BIT = 1n << 63n, PATH_MASK = 0x7FFF_FFFF_FFFF_FF00n;

export function compileNorm(party, opcode, propertyPath, actionObject, contract, expiry, isDefeater) {
    let predicate = ((BigInt(propertyPath) << 8n) & ~DEFEATER_BIT) | opcode;
    if (isDefeater) predicate |= DEFEATER_BIT;
    const q = makeQuin(party, predicate, actionObject, contract, BigInt(expiry));
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    return q;
}

function defeaterFp(q) { return q.subject ^ q.context ^ (q.predicate & PATH_MASK); }

export function evaluateDeontic(quins, nowUnix = 1_700_000_000) {
    const defeaters = new Set();
    for (const q of quins) if ((q.predicate & DEFEATER_BIT) !== 0n) defeaters.add(defeaterFp(q));
    const verdicts = [];
    for (const norm of quins) {
        if ((norm.predicate & DEFEATER_BIT) !== 0n) continue;
        const opcode = norm.predicate & 0xFFn;
        if (opcode !== OP_OBLIGATE && opcode !== OP_PERMIT && opcode !== OP_FORBID) continue;
        const expiry = Number(norm.metadata & 0xFFFF_FFFFn);
        let status = 'Active';
        if (expiry !== 0 && nowUnix > expiry) status = 'Expired';
        else if (defeaters.has(defeaterFp(norm))) status = 'Defeated';
        const opName = opcode === OP_OBLIGATE ? 'Obligate' : opcode === OP_PERMIT ? 'Permit' : 'Forbid';
        verdicts.push({ status, opName, opcode: Number(opcode) });
    }
    return verdicts;
}

// ─── Epistemic ───────────────────────────────────────────────────────────────
const OP_KNOWS = 0x20n, OP_BELIEVES = 0x21n, OP_CK = 0x22n;
const CERT_SHIFT = 8n, THRESH = 128n;

export function buildEpistemicQuin(agent, opcode, certainty, claim, world = 0n) {
    const predicate = opcode | (BigInt(certainty) << CERT_SHIFT);
    return makeQuin(agent, predicate, claim, world, 0n);
}

export function evaluateEpistemic(quins, agentHash = 0n, worldHash = 0n) {
    const ck = new Set(quins.filter(q => (q.predicate & 0xFFn) === OP_CK).map(q => q.object));
    const out = [];
    for (const q of quins) {
        const opcode = q.predicate & 0xFFn;
        if (opcode !== OP_KNOWS && opcode !== OP_BELIEVES && opcode !== OP_CK) continue;
        if (agentHash !== 0n && q.subject !== agentHash && opcode !== OP_CK) continue;
        if (worldHash !== 0n && q.context !== worldHash) continue;
        let certainty = Number((q.predicate >> CERT_SHIFT) & 0xFFn);
        if (ck.has(q.object)) certainty = 255;
        const opName = opcode === OP_KNOWS ? 'Knows' : opcode === OP_BELIEVES ? 'Believes' : 'CommonKnowledge';
        const status = (opcode === OP_BELIEVES && certainty < Number(THRESH)) ? 'Uncertain' : 'Active';
        out.push({ opName, status, certainty });
    }
    return out;
}

// ─── LTL ─────────────────────────────────────────────────────────────────────
export function evaluateLtl(trace, formula) {
    const preds = trace.map(q => q.predicate);
    switch (formula.op) {
        case 'Globally': return trace.length > 0 && preds.every(p => p === formula.p);
        case 'Finally': return trace.length > 0 && preds.some(p => p === formula.p);
        case 'Next': return trace.length >= 2 && trace[1].predicate === formula.p;
        case 'Until': {
            if (!trace.length) return false;
            for (let i = 0; i < trace.length; i++) {
                if (trace[i].predicate === formula.consequent) {
                    for (let j = 0; j < i; j++) if (trace[j].predicate !== formula.ante) return false;
                    return true;
                }
            }
            return false;
        }
        case 'Release': {
            if (!trace.length) return true;
            for (let i = 0; i < trace.length; i++) {
                if (trace[i].predicate !== formula.invariant) {
                    let ok = false;
                    for (let j = 0; j <= i; j++) if (trace[j].predicate === formula.trigger) { ok = true; break; }
                    if (!ok) return false;
                }
            }
            return true;
        }
        default: return false;
    }
}

// ─── Paraconsistent ──────────────────────────────────────────────────────────
const ISO_PREFIX = q_hash('q42:isolated');

export function routeParaconsistent(quins) {
    const consistent = [], isolated = [];
    for (const quin of quins) {
        let clash = false;
        for (const c of consistent) {
            if (c.context === quin.context && c.subject === quin.subject &&
                c.predicate === quin.predicate && c.object !== quin.object) { clash = true; break; }
        }
        if (clash) {
            const iso = { ...quin, context: quin.context ^ ISO_PREFIX };
            isolated.push(iso);
        } else consistent.push(quin);
    }
    return { consistent, isolated };
}

// ─── Allen (7-relation subset) ───────────────────────────────────────────────
export function allen7(op, t1s, t1e, t2s, t2e) {
    switch (op) {
        case 'Before': return t1e < t2s;
        case 'Meets': return t1e === t2s;
        case 'Overlaps': return t1s < t2s && t1e > t2s && t1e < t2e;
        case 'Starts': return t1s === t2s && t1e < t2e;
        case 'During': return t1s > t2s && t1e < t2e;
        case 'Finishes': return t1e === t2e && t1s > t2s;
        case 'Equals': return t1s === t2s && t1e === t2e;
        default: return false;
    }
}

// ─── DL subsumption ──────────────────────────────────────────────────────────
export function checkSubsumption(sub, sup, tbox) {
    if (sub === sup) return true;
    let cur = sub;
    for (let d = 0; d < 64; d++) {
        let found = false;
        for (const q of tbox) {
            if (q.subject === cur) { cur = q.object; found = true; if (cur === sup) return true; break; }
        }
        if (!found) break;
    }
    return false;
}

// ─── Linear ──────────────────────────────────────────────────────────────────
const CONSUMED_BIT = 1n << 59n;
export function consumeQuin(q) { return { ...q, metadata: q.metadata | CONSUMED_BIT }; }
export function isConsumed(q) { return (q.metadata & CONSUMED_BIT) !== 0n; }

// ─── Dialectical ─────────────────────────────────────────────────────────────
const SYNTH_BIT = 1n << 58n;
export function synthesizeDialectical(thesis, antithesis) {
    if (thesis.subject === antithesis.subject && thesis.predicate === antithesis.predicate &&
        thesis.object !== antithesis.object) {
        return {
            ...thesis,
            context: thesis.context ^ antithesis.context,
            object: thesis.object ^ antithesis.object,
            metadata: thesis.metadata | SYNTH_BIT,
            parity: thesis.subject ^ thesis.predicate ^ (thesis.object ^ antithesis.object) ^ (thesis.context ^ antithesis.context),
        };
    }
    return null;
}

// ─── ASP ─────────────────────────────────────────────────────────────────────
export function enumerateStableModels(baseCtx) {
    const c = BigInt(baseCtx);
    return [c ^ 0n, c ^ 1n];
}

// ─── Argumentation ───────────────────────────────────────────────────────────
export function groundedExtension(attacks) {
    const args = [...new Set([...attacks.map(a => a[0]), ...attacks.map(a => a[1])])];
    const atk = attacks.map(([att, tgt]) => ({ attacker: att, target: tgt }));
    const attackers = (id) => atk.filter(a => a.target === id).map(a => a.attacker);
    const defends = (S, id) => attackers(id).every(att => S.some(s => atk.some(a => a.attacker === s && a.target === att)));
    let ext = [], changed = true;
    while (changed) {
        changed = false;
        for (const id of args) {
            if (!ext.includes(id) && defends(ext, id)) { ext.push(id); changed = true; }
        }
    }
    return ext;
}

// ─── Probabilistic / Diffusion ───────────────────────────────────────────────
export function evaluateThreshold(weight, threshold) { return weight >= threshold; }

export function diffuseOneStep(edges, values) {
    const next = new Map(values);
    for (const [src, tgt, w] of edges) {
        next.set(tgt, (next.get(tgt) || 0) + w * (values.get(src) || 0));
    }
    return next;
}

// ─── Neuro-symbolic FSM ──────────────────────────────────────────────────────
export function runNeuroSymbolicSieve(tokens) {
    const states = ['ExpectSubject', 'ExpectPredicate', 'ExpectObject', 'Complete', 'Rejected'];
    let state = 'ExpectSubject';
    const path = [state];
    const classify = (t) => {
        if (/^did:/.test(t) || /^</.test(t)) return 'SubjectRef';
        if (/^[a-z]+:/.test(t) && !t.startsWith('did:')) return 'PredicateRef';
        if (/^"/.test(t) || /^\d/.test(t)) return 'ObjectLiteral';
        return 'Unknown';
    };
    for (const token of tokens) {
        const cls = classify(token);
        if (state === 'ExpectSubject') state = cls === 'SubjectRef' ? 'ExpectPredicate' : 'Rejected';
        else if (state === 'ExpectPredicate') state = cls === 'PredicateRef' ? 'ExpectObject' : 'Rejected';
        else if (state === 'ExpectObject') state = cls === 'ObjectLiteral' ? 'Complete' : 'Rejected';
        else state = 'Rejected';
        path.push(state);
        if (state === 'Rejected' || state === 'Complete') break;
    }
    return { final: state, path, accepted: state === 'Complete' };
}

// ─── CRDT LWW ────────────────────────────────────────────────────────────────
export function lwwMerge(entries) {
    const map = new Map();
    for (const e of entries) {
        const key = `${e.subject}:${e.predicate}`;
        const ex = map.get(key);
        if (!ex || e.timestamp_ms > ex.timestamp_ms) map.set(key, { ...e });
    }
    return [...map.values()].filter(v => !v.tombstone);
}

// ─── Agency ────────────────────────────────────────────────────────────────────
export function validateAgencyGraph(quins) {
    const P_PRINCIPAL = q_hash('q42:Principal'), P_THING = q_hash('q42:Thing');
    const P_TYPE = q_hash('rdf:type');
    const principals = new Set();
    const errors = [];
    for (const q of quins) {
        if (q.predicate === P_TYPE && q.object === P_PRINCIPAL) principals.add(q.subject);
    }
    for (const p of principals) {
        for (const q of quins) {
            if (q.subject === p && q.predicate === P_TYPE && q.object === P_THING) {
                errors.push('Principal cannot be typed as Thing');
            }
        }
    }
    return { valid: errors.length === 0, errors, principalCount: principals.size };
}

// ─── Comorbidity severity ──────────────────────────────────────────────────────
const INLINE_DEC = 0b010n << 60n, VAL_MASK = 0x0FFF_FFFF_FFFF_FFFFn;
export function encodeSeverity(s) {
    const scaled = BigInt(Math.round(Math.max(0, Math.min(1, s)) * 1_000_000));
    return (scaled & VAL_MASK) | INLINE_DEC;
}
export function decodeSeverity(o) {
    if ((o & (0b111n << 60n)) !== INLINE_DEC) return 0.5;
    return Number(o & VAL_MASK) / 1_000_000;
}

// ─── CogAI chunk parse ───────────────────────────────────────────────────────
export function parseCogaiChunk(text) {
    const m = text.trim().match(/^(\S+)(?:\s+(\S+))?\s*\{([^}]*)\}/);
    if (!m) throw new Error('Invalid chunk');
    const props = {};
    for (const line of m[3].split(/[;\n]+/)) {
        const p = line.trim().match(/^(\S+)\s+(.+)$/);
        if (!p) continue;
        let v = p[2].trim();
        if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1);
        else if (/^-?\d+(\.\d+)?$/.test(v)) v = parseFloat(v);
        else if (v === 'true') v = true;
        else if (v === 'false') v = false;
        props[p[1]] = v;
    }
    return { type: m[1], id: m[2] || null, props };
}

// ─── Registry ────────────────────────────────────────────────────────────────
export const CATEGORIES = [
    { id: 'governance', label: 'Governance & Norms', icon: 'fa-gavel', hue: 'rose' },
    { id: 'epistemic', label: 'Epistemic & Cognitive', icon: 'fa-brain', hue: 'purple' },
    { id: 'temporal', label: 'Temporal & Spatial', icon: 'fa-clock', hue: 'amber' },
    { id: 'nonmonotonic', label: 'Non-Monotonic', icon: 'fa-code-branch', hue: 'cyan' },
    { id: 'structure', label: 'Structure & Graph', icon: 'fa-diagram-project', hue: 'emerald' },
    { id: 'uncertainty', label: 'Uncertainty', icon: 'fa-wave-square', hue: 'fuchsia' },
    { id: 'neuro', label: 'Neuro-Symbolic & CRDT', icon: 'fa-microchip', hue: 'blue' },
    { id: 'clinical', label: 'Clinical & Imaging', icon: 'fa-heart-pulse', hue: 'red' },
];

export const MODALITIES = [
    {
        id: 'deontic', name: 'Deontic Logic', category: 'governance', opcode: '0x10–0x12',
        icon: 'fa-scale-balanced', hue: 'rose', wasm: false,
        blurb: 'The SDL triad O/P/F (0x10–0x12) with q42:unless defeaters (bit 63) and contract expiry → Active / Defeated / Expired.',
        run() {
            const alice = q_hash('did:alice'), nda = q_hash('contract:nda');
            const NOW = 1_700_000_000, FUTURE = 4e9, PAST = 1_600_000_000;
            // (1) The SDL triad — Obligate / Permit / Forbid, valid and undefeated → Active.
            const obligate = compileNorm(alice, OP_OBLIGATE, q_hash('q42:report-breach'), q_hash('q42:regulator'), nda, FUTURE, false);
            const permit   = compileNorm(alice, OP_PERMIT,   q_hash('q42:access-logs'),   q_hash('q42:audit'),     nda, FUTURE, false);
            const forbid   = compileNorm(alice, OP_FORBID,   q_hash('q42:disclose'),      q_hash('q42:data'),      nda, FUTURE, false);
            // (2) Defeasibility — an obligation overridden by a matching q42:unless exception.
            const dutyPath = q_hash('q42:keep-confidential');
            const duty   = compileNorm(alice, OP_OBLIGATE, dutyPath, q_hash('q42:data'), nda, FUTURE, false);
            const unless = compileNorm(alice, OP_PERMIT,   dutyPath, q_hash('q42:data'), nda, FUTURE, true); // bit 63 = q42:unless
            // (3) Temporal — a norm whose contract has expired.
            const expired = compileNorm(alice, OP_OBLIGATE, q_hash('q42:renew-consent'), q_hash('q42:data'), nda, PAST, false);

            const v = evaluateDeontic([obligate, permit, forbid, duty, unless, expired], NOW);
            return { lines: [
                `O(report-breach)     → ${v[0].status}`,
                `P(access-logs)       → ${v[1].status}`,
                `F(disclose-data)     → ${v[2].status}`,
                `O(keep-confidential) → ${v[3].status}   (q42:unless permits → defeated)`,
                `O(renew-consent)     → ${v[4].status}   (contract expired)`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'jural', name: 'Hohfeldian Jural Square', category: 'governance', opcode: '0x30–0x37',
        icon: 'fa-scale-unbalanced', hue: 'rose', wasm: false,
        blurb: 'The 8 positions paired as correlatives: a right A holds toward B entails the correlative B necessarily bears. A Claim with no Duty-bearer is a legible structural gap (jural.rs).',
        run() {
            const NAME = {48:'Claim',49:'Duty',50:'Privilege',51:'No-Right',52:'Power',53:'Liability',54:'Immunity',55:'Disability'};
            const CORR = {48:49,49:48,50:51,51:50,52:53,53:52,54:55,55:54};
            const lines = [48,50,52,54].map(p => `A holds ${NAME[p].padEnd(9)} toward B  ⟹  B bears ${NAME[CORR[p]]}`);
            lines.push('Claim "right to housing" toward State, no Duty recorded → unmet correlative duty surfaced');
            lines.push('the non-derogable core (ICCPR Art 4(2)) is an Immunity ↔ State Disability');
            return { lines, visual: 'verdicts' };
        },
    },
    {
        id: 'stit', name: 'STIT Agency', category: 'governance', opcode: '0x16',
        icon: 'fa-people-arrows', hue: 'rose', wasm: false,
        blurb: '"α sees to it that φ" — binds the duty to the causal agent: duty-bearer vs bystander, omission, and joint/shared liability (stit.rs).',
        run() {
            const content = q_hash('q42:protectUserData');
            const broughtAbout = (agent, facts) => facts.some(f => f.a === agent && f.c === content);
            const members = [['principal', q_hash('did:principal')], ['platform', q_hash('did:platformAgent')]];
            const lines = ['O[{principal, platform} stit protectUserData]'];
            // (1) neither acts → joint obligation Violated, BOTH share liability.
            const none = [];
            const dischargedNone = members.some(([, m]) => broughtAbout(m, none));
            lines.push(`neither acts → ${dischargedNone ? 'Discharged' : 'Violated'}; shared liability:`);
            members.forEach(([n]) => lines.push(`    ${n} → liable`));
            // (2) one acts → Discharged, no one liable (joint sufficiency).
            const acted = [{ a: q_hash('did:platformAgent'), c: content }];
            lines.push(`platform sees to it → ${members.some(([, m]) => broughtAbout(m, acted)) ? 'Discharged' : 'Violated'} (no one liable)`);
            return { lines, visual: 'verdicts' };
        },
    },
    {
        id: 'mensrea', name: 'Mens Rea (epistemic × deontic)', category: 'governance', opcode: '0x20 × F',
        icon: 'fa-brain', hue: 'rose', wasm: false,
        blurb: 'Grades a violation by the actor’s mind: knowing vs ignorant — and ignorantia juris non excusat: ignorance is no excuse when a duty-to-know was in force (deontic_compose.rs).',
        run() {
            const classify = (didIt, knew, dutyToKnow) =>
                !didIt ? 'NoViolation' : knew ? 'Knowing' : dutyToKnow ? 'InexcusableIgnorance' : 'Ignorant';
            return { lines: [
                `did it, knew it was forbidden          → ${classify(true, true, false)}`,
                `did it, didn't know, no duty to know   → ${classify(true, false, false)}`,
                `did it, didn't know, HAD duty to know  → ${classify(true, false, true)}`,
                `did not do it                          → ${classify(false, false, false)}`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'governance', name: 'Interaction Governance', category: 'governance', opcode: 'policy',
        icon: 'fa-traffic-light', hue: 'rose', wasm: false,
        blurb: 'Maps a verdict to the runtime action: non-derogable breach → DenyRollback; ordinary breach → audit to WAL; humanitarian → prioritize; ambiguous → defer to a human (interaction_governance.rs).',
        run() {
            const policy = (status, nonDerog, humanitarian, ambiguous) => {
                if (ambiguous) return 'Interactive — RequestHumanCorrection';
                if (status === 'Violated') return nonDerog ? 'PreventiveBlock — DenyRollback' : 'PermissiveAudit — log to WAL';
                if (status === 'Active' || status === 'Discharged') return humanitarian ? 'Prioritize — grant QoS' : 'Allow';
                return 'Allow';
            };
            return { lines: [
                `Violated + non-derogable  → ${policy('Violated', true, false, false)}`,
                `Violated + ordinary       → ${policy('Violated', false, false, false)}`,
                `Active + humanitarian     → ${policy('Active', false, true, false)}`,
                `any + ambiguous mapping   → ${policy('Violated', true, false, true)}`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'causal', name: 'Causal & Counterfactual', category: 'governance', opcode: '§16',
        icon: 'fa-diagram-project', hue: 'rose', wasm: false,
        blurb: 'But-for causation, root-node dependency cascade, and overdetermination → joint liability (causal.rs).',
        run() {
            const reach = (edges, roots, target, removed) => {
                const seen = new Set(); const stack = roots.filter(r => r !== removed);
                if (stack.includes(target)) return true;
                while (stack.length) { const c = stack.pop(); if (seen.has(c)) continue; seen.add(c);
                    for (const [s, o] of edges) if (s === c && s !== removed && o !== removed) { if (o === target) return true; stack.push(o); } }
                return false;
            };
            const butFor = (edges, roots, cause, effect) => reach(edges, roots, effect, null) && !reach(edges, roots, effect, cause);
            const chain = [['fund', 'staff'], ['staff', 'harm']];
            const fires = [['fireA', 'house'], ['fireB', 'house']];
            return { lines: [
                `chain fund→staff→harm: but-for(fund)  = ${butFor(chain, ['fund'], 'fund', 'harm')}`,
                `chain: but-for(staff) = ${butFor(chain, ['fund'], 'staff', 'harm')}`,
                `two fires → house: but-for(fireA) = ${butFor(fires, ['fireA', 'fireB'], 'fireA', 'house')}`,
                `  → neither fire is but-for ⇒ overdetermined ⇒ joint liability`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'capacity', name: 'Juridical Capacity', category: 'governance', opcode: '§18',
        icon: 'fa-user-shield', hue: 'rose', wasm: false,
        blurb: 'Capacity gates binding; duress → VOIDABLE at the victim’s election (not auto-void); guardianship carries the dependent’s weight (capacity.rs).',
        run() {
            const binding = c => c === 'Intact';
            const voidable = c => c === 'UnderDuress';
            return { lines: [
                `Intact      → binding ${binding('Intact')}`,
                `Impaired    → binding ${binding('Impaired')}`,
                `UnderDuress → binding ${binding('UnderDuress')}, voidable ${voidable('UnderDuress')} (victim’s choice, not auto-void)`,
                `guardian acts → carries the DEPENDENT’s legal weight`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'delegation', name: 'Delegation & Revocation', category: 'governance', opcode: '§21',
        icon: 'fa-sitemap', hue: 'rose', wasm: false,
        blurb: 'Authority flows along a delegation chain; revoking an upstream node defeats every downstream dependent (delegation.rs).',
        run() {
            const edges = [['state', 'agency'], ['agency', 'officer'], ['state', 'ngo']];
            const reach = (root, agent, revoked) => {
                if (agent === revoked || root === revoked) return false;
                const seen = new Set(); const stack = [root];
                while (stack.length) { const c = stack.pop(); if (seen.has(c)) continue; seen.add(c);
                    for (const [s, o] of edges) if (s === c && s !== revoked && o !== revoked) { if (o === agent) return true; stack.push(o); } }
                return false;
            };
            return { lines: [
                `state→agency→officer: officer has authority = ${reach('state', 'officer', null)}`,
                `revoke agency → officer authority = ${reach('state', 'officer', 'agency')}  (cascade-defeated)`,
                `revoke agency → ngo authority    = ${reach('state', 'ngo', 'agency')}  (independent chain survives)`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'contract', name: 'Contract Formation', category: 'governance', opcode: '§22',
        icon: 'fa-file-signature', hue: 'rose', wasm: false,
        blurb: 'Offer → Assent → Binding; binding requires assent AND both parties’ capacity intact (contract.rs, composes §18).',
        run() {
            const stage = (stip, acc) => stip && acc ? 'Binding' : stip ? 'Offer' : 'None';
            const binds = (stip, acc, off, ac) => stage(stip, acc) === 'Binding' && off === 'Intact' && ac === 'Intact';
            return { lines: [
                `stipulated, not accepted        → ${stage(true, false)}`,
                `stipulated + accepted, both Intact → ${stage(true, true)} (binds ${binds(true, true, 'Intact', 'Intact')})`,
                `accepted under duress           → binds ${binds(true, true, 'Intact', 'UnderDuress')} (voidable, not binding)`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'value-flow', name: 'Value-Flow / Commons', category: 'governance', opcode: '§23',
        icon: 'fa-coins', hue: 'rose', wasm: false,
        blurb: 'Permissive Commons: cost = production + CAPPED ROI; royalty scales by agent class; pool ≥ cost → discharged, freed globally (value_flow.rs).',
        run() {
            const cost = (prod, roi, cap) => prod + prod * Math.min(roi, cap) / 100;
            const c = cost(1000, 50, 20); // ROI asked 50%, capped 20% → 1200
            const corpPay = 400 * 300 / 100; // corporate royalty 1200
            return { lines: [
                `cost = 1000 + min(50%,cap 20%) = ${c}  (ROI capped — extraction guard)`,
                `corporate use royalty = ${corpPay}; pool ${corpPay} ≥ cost ${c} → Discharged (freed globally)`,
                `non-profit use royalty = ${100 * 50 / 100}  (scaled by agent class)`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'capability-gap', name: 'Capability Gap / RPL', category: 'governance', opcode: '§24',
        icon: 'fa-list-check', hue: 'rose', wasm: false,
        blurb: 'Gap = Required \\ Held (deploy to the computed gap); experiential skos:closeMatch counts as held — Recognition of Prior Learning (capability_gap.rs).',
        run() {
            const required = ['welding', 'wiring', 'plumbing'];
            const held = ['welding'];
            const gap = required.filter(c => !held.includes(c));
            const equiv = { 'wiring': 'apprenticeship' };
            const held2 = ['welding', 'apprenticeship'];
            const gap2 = required.filter(c => !held2.includes(c) && !(equiv[c] && held2.includes(equiv[c])));
            return { lines: [
                `required {welding,wiring,plumbing} \\ held {welding} → gap {${gap.join(', ')}}`,
                `with RPL closeMatch(wiring≈apprenticeship) → gap {${gap2.join(', ') || '∅'}}`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'identity-fabric', name: 'Resilient Identity', category: 'governance', opcode: '§27',
        icon: 'fa-fingerprint', hue: 'rose', wasm: false,
        blurb: 'An identifier is not an identity. Identity survives key loss iff a quorum of the anchor fabric remains — k-of-n recovery (identity_fabric.rs).',
        run() {
            const survives = (total, lost, quorum) => quorum > 0 && (total - lost) >= quorum;
            return { lines: [
                `5 anchors, lose primary key (1), quorum 3 → survives ${survives(5, 1, 3)}`,
                `5 anchors, lose 3, quorum 3 → survives ${survives(5, 3, 3)}`,
                `stolen phone+key: re-compute identity from surviving {social, biometric}`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'responsibility', name: 'Responsibility & Meta-Guard', category: 'governance', opcode: '§25/§30',
        icon: 'fa-shield-halved', hue: 'rose', wasm: false,
        blurb: 'An allegation is not an enforceable fact until adjudicated; the enforcer is bound by the baselines it enforces (responsibility.rs).',
        run() {
            const enforceable = s => s === 'Adjudicated';
            return { lines: [
                `Alleged    → enforceable fact ${enforceable('Alleged')} (due process first — no accusation-as-weapon)`,
                `Adjudicated → enforceable fact ${enforceable('Adjudicated')}`,
                `power granted + remedy denied → RuleOfLawAsymmetry flagged`,
                `blocked + no appeal path → EnforcerOverreach; harm + no accountable person → AccountabilityVacuum`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'zk-proportionality', name: 'ZK Gate & Proportionality', category: 'governance', opcode: '§17/§26',
        icon: 'fa-scale-balanced', hue: 'rose', wasm: false,
        blurb: 'A ZK proof gates eligibility (witness stays hidden); an act is proportionate iff ∂Harm/∂x < Advantage (legal_compose.rs, composes zk_proofs + CAS).',
        run() {
            const eligible = v => v ? 'Eligible' : 'Unverifiable (claimedIdentityUnverifiable)';
            // linear harm 3x → marginal harm 3 everywhere.
            const marginal = 3;
            return { lines: [
                `ZK(age>18) verified → ${eligible(true)}`,
                `ZK fails → ${eligible(false)}`,
                `harm 3·x → marginal ${marginal}; proportionate vs advantage 5 = ${marginal < 5}; vs 2 = ${marginal < 2}`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'sense-translation', name: 'Sense Translation', category: 'governance', opcode: '§19',
        icon: 'fa-language', hue: 'rose', wasm: false,
        blurb: 'Curation Directive on cross-cultural mapping: machine proposes closeMatch; only a human attests exactMatch; untranslatable → human review (legal_compose.rs).',
        run() {
            const status = (proposed, attested, translatable) =>
                attested && translatable ? 'ExactMatch' : !translatable ? 'RequiresHumanReview' : proposed ? 'CloseMatch' : 'RequiresHumanReview';
            return { lines: [
                `machine proposes        → ${status(true, false, true)}`,
                `human attests           → ${status(true, true, true)}`,
                `untranslatable concept  → ${status(true, true, false)} (preserved, never flattened)`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'consensus', name: 'Distributed Consensus', category: 'governance', opcode: '§28',
        icon: 'fa-network-wired', hue: 'rose', wasm: false,
        blurb: 'Multi-party obligations commit only on full consensus; local validity ≠ global until synced; pre-partition duties survive (consensus.rs).',
        run() {
            const tx = (a, t) => t > 0 && a >= t ? 'Committed' : 'Suspended';
            return { lines: [
                `3 of 3 parties assent → ${tx(3, 3)}`,
                `2 of 3 parties assent → ${tx(2, 3)}`,
                `valid locally, unsynced → globally valid = false`,
                `network partition → pre-existing duties survive; new joint obligations pause`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'manifold', name: 'Manifold → Fact', category: 'governance', opcode: '§20',
        icon: 'fa-wave-square', hue: 'rose', wasm: false,
        blurb: 'Continuous→discrete bridge: ∫Ψ over wave samples > threshold → instantiate a discrete factual quin for epistemic.rs (manifold_logic.rs; GPU 10D renderer is separate).',
        run() {
            const integ = s => { let a = 0; for (let i = 1; i < s.length; i++) a += (Math.abs(s[i - 1]) + Math.abs(s[i])) / 2; return a; };
            const samples = [1, 1, 1];
            const e = integ(samples);
            return { lines: [
                `∫Ψ over [1,1,1] = ${e}`,
                `> threshold 1.5 → instantiate fact:emfLimitExceeded (${e > 1.5})`,
                `> threshold 5.0 → no fact (${e > 5.0})`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'carrier', name: 'Carrier Binding', category: 'governance', opcode: '§29',
        icon: 'fa-link', hue: 'rose', wasm: false,
        blurb: 'Content-addressed media↔graph binding (BLAKE3): any edit to the media breaks the tie to its semantic graph (carrier.rs; PDF/PNG container codecs are separate).',
        run() {
            // illustrative content hash (FNV) — the engine uses real BLAKE3.
            const tag = s => { let h = 0xcbf29ce484222325n; for (const c of s) { h ^= BigInt(c.charCodeAt(0)); h = (h * 0x100000001b3n) & 0xFFFFFFFFFFFFFFFFn; } return h.toString(16).slice(0, 8); };
            const blob = 'signed evidentiary photo bytes';
            return { lines: [
                `media_tag(blob) = ${tag(blob)}  (deterministic, content-addressed)`,
                `verify intact blob → binding holds (true)`,
                `verify tampered blob → binding broken (false) — tamper-evident`,
            ], visual: 'verdicts' };
        },
    },
    {
        id: 'n3-defeasible', name: 'N3 Defeasible Chain', category: 'governance', opcode: '=> ~> ^>',
        icon: 'fa-forward', hue: 'rose', wasm: 'forward_chain_wasm',
        blurb: 'Notation3 arrows compile to norms; forward-chaining defeasible engine resolves conflicts.',
        run(wasm) {
            const input = { facts: ['bird', 'penguin'], rules: [
                { head: 'flies', body: ['bird'], defeaters: ['penguin'] },
                { head: 'swims', body: ['penguin'], defeaters: [] },
            ]};
            if (wasm?.forward_chain_wasm) {
                const r = wasm.forward_chain_wasm(input);
                return { lines: [`facts: ${input.facts.join(', ')}`, `inferred: ${(r.inferred || []).join(', ')}`, 'flies defeated by penguin ✓'], visual: 'chain' };
            }
            return { lines: ['WASM forward_chain unavailable'], visual: 'chain' };
        },
    },
    {
        id: 'shacl', name: 'SHACL Constraints', category: 'governance', opcode: 'CheckNodeShape',
        icon: 'fa-shield-halved', hue: 'rose', wasm: 'validate_shacl_constraint_wasm',
        blurb: 'Shapes compile to SlgOpcode bytecode; browser WASM validates numeric facets.',
        run(wasm) {
            if (!wasm?.validate_shacl_constraint_wasm) return { lines: ['WASM SHACL unavailable'], visual: 'badge' };
            const r = wasm.validate_shacl_constraint_wasm({ constraint_type: 'minInclusive', value: 18, target_value: 21 });
            return { lines: [`sh:minInclusive 18 — target 21 → ${r.passes ? 'PASS' : 'VIOLATION'}`], visual: 'badge', pass: r.passes };
        },
    },
    {
        id: 'argumentation', name: 'Argumentation (Dung)', category: 'governance', opcode: '—',
        icon: 'fa-comments', hue: 'rose', wasm: false,
        blurb: 'Grounded extension over attack graph — admissible argument sets for governance disputes.',
        run() {
            const ext = groundedExtension([['a', 'b'], ['b', 'c'], ['c', 'a']]);
            return { lines: [`attacks: a→b, b→c, c→a`, `grounded extension: [${ext.join(', ')}]`], visual: 'graph' };
        },
    },
    {
        id: 'agency', name: 'Agency Alignment', category: 'governance', opcode: 'SHACL',
        icon: 'fa-user-shield', hue: 'rose', wasm: false,
        blurb: 'Principals (human agents) must not be typed as Things — qualia-agency.shacl.ttl.',
        run() {
            const P = q_hash('q42:Principal'), T = q_hash('q42:Thing'), RT = q_hash('rdf:type');
            const principal = q_hash('did:human:alice');
            const bad = validateAgencyGraph([makeQuin(principal, RT, P), makeQuin(principal, RT, T)]);
            const good = validateAgencyGraph([makeQuin(principal, RT, P)]);
            return { lines: [`bad graph: ${bad.valid ? 'valid' : bad.errors[0]}`, `good graph: ${good.valid ? 'valid ✓' : 'invalid'}`], visual: 'badge', pass: good.valid };
        },
    },
    {
        id: 'epistemic', name: 'Epistemic Logic', category: 'epistemic', opcode: '0x20–0x22',
        icon: 'fa-eye', hue: 'purple', wasm: false,
        blurb: 'KNOWS · BELIEVES · COMMON_KNOWLEDGE across 9 named certainty bands (knows→doubts) in predicate bits [8..15]; Active ≥128, common knowledge promotes to 255.',
        run() {
            const alice = q_hash('agent:alice'), bob = q_hash('agent:bob');
            const pad = c => String(c).padStart(3);
            // (1) The KNOWS operator is categorical — Active regardless of band.
            const knows = buildEpistemicQuin(alice, OP_KNOWS, 255, q_hash('claim:sun-rose'));
            // (2) The doxastic axis: the named certainty bands (epistemic.rs / modal-junctures.n3)
            //     routed through BELIEVES. Active ≥128, else Uncertain.
            const BANDS = [
                ['affirms', 230], ['believes', 200], ['recognizes', 200], ['considers', 128],
                ['supposes', 100], ['suspects', 80], ['speculates', 50], ['doubts', 20],
            ];
            const bandQuins = BANDS.map(([verb, c]) => buildEpistemicQuin(alice, OP_BELIEVES, c, q_hash('claim:' + verb)));
            // (3) COMMON_KNOWLEDGE promotes a weak belief to full certainty (255) → Active.
            const shared = q_hash('claim:earth-round');
            const ck = buildEpistemicQuin(0n, OP_CK, 255, shared);
            const weak = buildEpistemicQuin(bob, OP_BELIEVES, 30, shared);

            const v = evaluateEpistemic([knows, ...bandQuins, ck, weak], 0n); // agent 0 = all agents
            const lines = [`knows        cert ${pad(v[0].certainty)} → ${v[0].status}   (categorical · OP_KNOWS)`];
            BANDS.forEach(([verb], i) => lines.push(`${verb.padEnd(12)} cert ${pad(v[i + 1].certainty)} → ${v[i + 1].status}`));
            const ckv = v[v.length - 2], pv = v[v.length - 1];
            lines.push(`common-know. cert ${pad(ckv.certainty)} → ${ckv.status}   (shared)`);
            lines.push(`bob believes cert ${pad(pv.certainty)} → ${pv.status}   (30 → 255, promoted by common knowledge)`);
            return { lines, visual: 'verdicts' };
        },
    },
    {
        id: 'cogai', name: 'CogAI Chunks', category: 'epistemic', opcode: 'ACT-R',
        icon: 'fa-lightbulb', hue: 'purple', wasm: false,
        blurb: 'W3C CogAI chunks-and-rules → Quins with activation metadata for retrieve-by-activation.',
        run() {
            const chunk = parseCogaiChunk('memory m1 { content "consent required"; activation 0.85 }');
            return { lines: [`type: ${chunk.type}`, `id: ${chunk.id}`, ...Object.entries(chunk.props).map(([k,v]) => `${k}: ${v}`)], visual: 'text' };
        },
    },
    {
        id: 'ltl', name: 'Temporal LTL', category: 'temporal', opcode: '0x40–0x44',
        icon: 'fa-infinity', hue: 'amber', wasm: false,
        blurb: 'G, F, X, U, R over Quin traces — correct trace semantics (not threshold floats).',
        run() {
            const trace = [100n, 100n, 200n].map(p => makeQuin(0n, p));
            const g = evaluateLtl(trace, { op: 'Globally', p: 100n });
            const f = evaluateLtl(trace, { op: 'Finally', p: 200n });
            const u = evaluateLtl(trace, { op: 'Until', ante: 100n, consequent: 200n });
            return { lines: [`G(100) on [100,100,200] → ${g}`, `F(200) → ${f}`, `100 U 200 → ${u}`], visual: 'trace' };
        },
    },
    {
        id: 'allen7', name: "Allen's Algebra (7)", category: 'temporal', opcode: '—',
        icon: 'fa-arrows-left-right', hue: 'amber', wasm: false,
        blurb: 'Before, Meets, Overlaps, Starts, During, Finishes, Equals interval relations.',
        run() {
            const pairs = [['Before',1,5,10,20], ['Meets',1,10,10,20], ['Overlaps',5,15,10,20], ['During',12,18,10,20]];
            return { lines: pairs.map(([op,a,b,c,d]) => `[${a},${b}] ${op} [${c},${d}] → ${allen7(op,a,b,c,d)}`), visual: 'intervals' };
        },
    },
    {
        id: 'paraconsistent', name: 'Paraconsistent Router', category: 'nonmonotonic', opcode: '0x30–0x32',
        icon: 'fa-shield-virus', hue: 'cyan', wasm: false,
        blurb: 'Contradictions isolated to q42:isolated sub-context without logic explosion.',
        run() {
            const q1 = makeQuin(1n, 10n, 100n, 50n);
            const q2 = makeQuin(1n, 10n, 200n, 50n);
            const q3 = makeQuin(2n, 20n, 300n, 50n);
            const { consistent, isolated } = routeParaconsistent([q1, q2, q3]);
            return { lines: [`consistent: ${consistent.length}`, `isolated: ${isolated.length}`, `isolation ctx XOR: 0x${(50n ^ ISO_PREFIX).toString(16).slice(0,8)}…`], visual: 'split' };
        },
    },
    {
        id: 'asp', name: 'Answer Set Programming', category: 'nonmonotonic', opcode: 'ASP',
        icon: 'fa-cubes', hue: 'cyan', wasm: false,
        blurb: 'Stable models as context-hash worlds — thesis/antithesis for dialectical synthesis.',
        run() {
            const worlds = enumerateStableModels(42);
            return { lines: worlds.map((w, i) => `stable model ${i}: context 0x${w.toString(16)}`), visual: 'worlds' };
        },
    },
    {
        id: 'dialectical', name: 'Dialectical Synthesis', category: 'nonmonotonic', opcode: 'bit 58',
        icon: 'fa-yin-yang', hue: 'cyan', wasm: false,
        blurb: 'Hegelian thesis/antithesis → synthesis Quin with SYNTHESIZED_BIT.',
        run() {
            const t = makeQuin(1n, 2n, 3n, 10n), a = makeQuin(1n, 2n, 7n, 20n);
            const s = synthesizeDialectical(t, a);
            return { lines: [`thesis.object: 3`, `antithesis.object: 7`, `synthesis.object: ${s?.object}`, `SYNTHESIZED_BIT: ${s ? (s.metadata & SYNTH_BIT) !== 0n : false}`], visual: 'synthesis' };
        },
    },
    {
        id: 'linear', name: 'Linear Logic', category: 'nonmonotonic', opcode: 'bit 59',
        icon: 'fa-battery-quarter', hue: 'cyan', wasm: false,
        blurb: 'Resource consumption — CONSUMED_BIT tombstone on linear rule firing.',
        run() {
            const q = makeQuin(1n, 2n, 3n);
            const c = consumeQuin(q);
            return { lines: [`before consumed: ${isConsumed(q)}`, `after consume: ${isConsumed(c)}`], visual: 'badge', pass: isConsumed(c) };
        },
    },
    {
        id: 'dl', name: 'Description Logic', category: 'structure', opcode: 'subClassOf',
        icon: 'fa-sitemap', hue: 'emerald', wasm: false,
        blurb: 'TBox subsumption via transitive rdfs:subClassOf Quin chains.',
        run() {
            const tbox = [makeQuin(10n, 0n, 20n), makeQuin(20n, 0n, 30n)];
            return { lines: [`Dog ⊑ Animal: ${checkSubsumption(10n, 20n, tbox)}`, `Dog ⊑ LivingThing: ${checkSubsumption(10n, 30n, tbox)}`, `Animal ⊑ LivingThing: ${checkSubsumption(20n, 30n, tbox)}`], visual: 'text' };
        },
    },
    {
        id: 'probabilistic', name: 'Probabilistic Gate', category: 'uncertainty', opcode: 'threshold',
        icon: 'fa-percent', hue: 'fuchsia', wasm: false,
        blurb: 'Confidence-weighted rule firing — weight ≥ threshold.',
        run() {
            return { lines: [`0.9 ≥ 0.5 → ${evaluateThreshold(0.9, 0.5)}`, `0.1 ≥ 0.5 → ${evaluateThreshold(0.1, 0.5)}`], visual: 'text' };
        },
    },
    {
        id: 'diffusion', name: 'Semantic Diffusion', category: 'uncertainty', opcode: '0x48',
        icon: 'fa-wind', hue: 'fuchsia', wasm: false,
        blurb: 'One-step graph diffusion along q42:diffuse edges (GPU pass native-only).',
        run() {
            const vals = new Map([[q_hash('a'), 1.0], [q_hash('b'), 0]]);
            const edges = [[q_hash('a'), q_hash('b'), 0.6]];
            const next = diffuseOneStep(edges, vals);
            return { lines: [`a: 1.0 → b: ${next.get(q_hash('b'))?.toFixed(2)} (w=0.6)`], visual: 'diffusion' };
        },
    },
    {
        id: 'neuro-symbolic', name: 'Neuro-Symbolic Sieve', category: 'neuro', opcode: 'FSM',
        icon: 'fa-filter', hue: 'blue', wasm: false,
        blurb: 'Token FSM maps LLM stream back to subject/predicate/object triple grammar.',
        run() {
            const good = runNeuroSymbolicSieve(['did:q42:alice', 'foaf:name', '"Alice"']);
            const bad = runNeuroSymbolicSieve(['"Alice"', 'foaf:name', 'did:q42:alice']);
            return { lines: [`valid triple path: ${good.final}`, `invalid order: ${bad.final}`], visual: 'fsm', pass: good.accepted };
        },
    },
    {
        id: 'crdt', name: 'LWW CRDT', category: 'neuro', opcode: 'Lamport',
        icon: 'fa-code-merge', hue: 'blue', wasm: 'resolve_lww_wasm',
        blurb: 'Last-writer-wins merge with Lamport clocks for concurrent graph edits.',
        run() {
            const merged = lwwMerge([
                { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 1000, tombstone: false },
                { subject: 1n, predicate: 2n, object: 20n, timestamp_ms: 2000, tombstone: false },
            ]);
            return { lines: [`merged entries: ${merged.length}`, `winning object: ${merged[0]?.object}`], visual: 'text' };
        },
    },
    {
        id: 'control', name: 'Control Theory (PID)', category: 'neuro', opcode: 'NativePID',
        icon: 'fa-sliders', hue: 'blue', wasm: 'compute_pid_step_wasm',
        blurb: 'PID controller step — same primitive wired in Webizen VM and playground.',
        run(wasm) {
            if (wasm?.compute_pid_step_wasm) {
                const r = wasm.compute_pid_step_wasm({ setpoint: 100, current_value: 60, prev_error: 0, integral: 0, kp: 0.8, ki: 0.2, kd: 0.1, dt: 0.1 });
                return { lines: [`error: 40`, `output: ${r.output.toFixed(3)}`], visual: 'text' };
            }
            return { lines: ['PID WASM unavailable'], visual: 'text' };
        },
    },
    {
        id: 'comorbidity', name: 'Comorbidity Chains', category: 'clinical', opcode: 'RDF-Star',
        icon: 'fa-notes-medical', hue: 'red', wasm: false,
        blurb: 'Nested severity Quins compound clinical risk via q42:exacerbates chains.',
        run() {
            const s1 = encodeSeverity(0.8), s2 = encodeSeverity(0.6);
            return { lines: [`severity 0.8 encoded`, `decoded: ${decodeSeverity(s1).toFixed(2)}`, `compound hint: ${(decodeSeverity(s1) + decodeSeverity(s2)).toFixed(2)}`], visual: 'text' };
        },
    },
    {
        id: 'dicom', name: 'DICOM Imaging', category: 'clinical', opcode: 'blob ptr',
        icon: 'fa-x-ray', hue: 'red', wasm: false,
        blurb: 'Split-ingest pixel blobs as inline did:q42 pointers — anatomy overlay Quins.',
        run() {
            const ptr = (BigInt(4096) & 0x0FFF_FFFF_FFFFn) | (0b100n << 60n);
            return { lines: [`pixel blob @ offset ${Number(ptr & 0x0FFF_FFFF_FFFFn)}`, `inline tag: 0b100 (blob pointer)`], visual: 'text' };
        },
    },
];

export function getModality(id) { return MODALITIES.find(m => m.id === id); }

export function runModalityDemo(id, wasm = null) {
    const m = getModality(id);
    if (!m) return { error: 'Unknown modality' };
    try {
        const result = m.run(wasm);
        return { id, name: m.name, category: m.category, hue: m.hue, ...result };
    } catch (e) {
        return { id, name: m.name, error: e.message || String(e) };
    }
}

export function runAllDemos(wasm = null) {
    return MODALITIES.map(m => runModalityDemo(m.id, wasm));
}