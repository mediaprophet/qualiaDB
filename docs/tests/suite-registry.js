/**
 * Single source of truth for docs/tests suite registration.
 * Used by runner.js (browser) and run-headless.mjs (CI / MCP run_docs_tests).
 */

import { register as regPrimitives } from './suites/primitives.js';
import { register as regEpistemic } from './suites/modality-epistemic.js';
import { register as regLtl } from './suites/modality-ltl.js';
import { register as regParaconsistent } from './suites/modality-paraconsistent.js';
import { register as regLinear } from './suites/modality-linear.js';
import { register as regDialectical } from './suites/modality-dialectical.js';
import { register as regSpatioTemporal } from './suites/modality-spatio-temporal.js';
import { register as regDl } from './suites/modality-dl.js';
import { register as regAsp } from './suites/modality-asp.js';
import { register as regProbabilistic } from './suites/modality-probabilistic.js';
import { register as regCogAi } from './suites/modality-cogai.js';
import { register as regAgency } from './suites/modality-agency.js';
import { register as regComorbidity } from './suites/modality-comorbidity.js';
import { register as regDicom } from './suites/modality-dicom.js';
import { register as regDeontic } from './suites/modality-deontic.js';
import { register as regControlTheory } from './suites/modality-control-theory.js';
import { register as regCrdt } from './suites/modality-crdt.js';
import { register as regNeuroSymbolic } from './suites/modality-neuro-symbolic.js';
import { register as regArgumentation } from './suites/modality-argumentation.js';
import { register as regGraphTheory } from './suites/modality-graph-theory.js';
import { register as regIntervalReason } from './suites/modality-interval-reasoning.js';
import { register as regDiffusion } from './suites/modality-diffusion.js';
import { register as regRdfFormats } from './suites/rdf-formats-dispatch.js';
import { register as regOntology } from './suites/ontology-alignment.js';

import { register as regQueryEngine } from './suites/wasm-query-engine.js';
import { register as regBioinformatics } from './suites/wasm-bioinformatics.js';
import { register as regClinical } from './suites/wasm-clinical.js';
import { register as regChemistry } from './suites/wasm-chemistry.js';
import { register as regEconomics } from './suites/wasm-economics.js';
import { register as regShacl } from './suites/wasm-shacl.js';
import { register as regGovernance } from './suites/wasm-governance.js';
import { register as regWasmIngest } from './suites/wasm-ingest.js';
import { register as regDataFormats } from './suites/wasm-data-formats.js';
import { register as regProfiles } from './suites/wasm-profiles.js';
import { register as regResources } from './suites/wasm-resources.js';
import { register as regRdfStar } from './suites/wasm-rdf-star.js';
import { register as regSolvers } from './suites/wasm-solvers.js';
import { register as regFinanceExtras } from './suites/wasm-finance-extras.js';

import { register as regNativeDaemon } from './suites/native-daemon.js';
import { register as regNativeQuery } from './suites/native-query.js';
import { register as regNativeLive } from './suites/native-live.js';
import { register as regNativeChat } from './suites/native-chat.js';
import { register as regNativeTorrent } from './suites/native-torrent.js';
import { register as regComparison } from './suites/native-comparison.js';

/** @typedef {'logic'|'wasm'|'native'|'both'} RunMode */

/**
 * Register all suites for the given mode into `runner`.
 * @param {import('./test-runner.js').TestRunner} runner
 * @param {{ mode: RunMode, wasm: object|null, native: object|null, isMobile: boolean }} ctx
 * @param {RunMode} mode
 */
export function registerSuites(runner, ctx, mode) {
    const c = { ...ctx, mode };

    // Pure JS logic (all modes)
    regPrimitives(runner, c);
    regEpistemic(runner, c);
    regLtl(runner, c);
    regParaconsistent(runner, c);
    regLinear(runner, c);
    regDialectical(runner, c);
    regSpatioTemporal(runner, c);
    regDl(runner, c);
    regAsp(runner, c);
    regProbabilistic(runner, c);
    regCogAi(runner, c);
    regAgency(runner, c);
    regComorbidity(runner, c);
    regDicom(runner, c);
    regDeontic(runner, c);
    regControlTheory(runner, c);
    regCrdt(runner, c);
    regNeuroSymbolic(runner, c);
    regArgumentation(runner, c);
    regGraphTheory(runner, c);
    regIntervalReason(runner, c);
    regDiffusion(runner, c);
    regRdfFormats(runner, c);
    regOntology(runner, c);

    if (mode === 'wasm' || mode === 'both') {
        regQueryEngine(runner, c);
        regBioinformatics(runner, c);
        regClinical(runner, c);
        regChemistry(runner, c);
        regEconomics(runner, c);
        regShacl(runner, c);
        regGovernance(runner, c);
        regWasmIngest(runner, c);
        regDataFormats(runner, c);
        regProfiles(runner, c);
        regResources(runner, c);
        regRdfStar(runner, c);
        regSolvers(runner, c);
        regFinanceExtras(runner, c);
    }

    if (mode === 'native' || mode === 'both') {
        regNativeDaemon(runner, c);
        regNativeQuery(runner, c);
        regNativeLive(runner, c);
        regNativeChat(runner, c);
        regNativeTorrent(runner, c);
    }

    if (mode === 'both') {
        regComparison(runner, c);
    }
}

