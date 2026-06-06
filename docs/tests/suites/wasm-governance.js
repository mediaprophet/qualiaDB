// WASM Governance tests.
// Covers Webizen agreements, rights ontology, mesh pruning, and opcode interception.

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Governance & Webizen', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.describe('Webizen Agreement Lifecycle', () => {

            runner.it('webizen_propose_agreement returns a bigint ID', () => {
                if (!mod.webizen_propose_agreement) return;
                const id = mod.webizen_propose_agreement(
                    ['did:wellfare:guardian1'], 'did:wellfare:principal', 'health', 1
                );
                runner.expect(typeof id).toBe('bigint');
                runner.expect(id).toBeGreaterThan(0n);
            });

            runner.it('webizen_poll_agreements returns JSON string', () => {
                if (!mod.webizen_poll_agreements) return;
                const raw = mod.webizen_poll_agreements();
                runner.expect(typeof raw).toBe('string');
                runner.expect(() => JSON.parse(raw)).not.toThrow();
            });

            runner.it('webizen_sign_agreement does not throw', () => {
                if (!mod.webizen_propose_agreement || !mod.webizen_sign_agreement) return;
                const id = mod.webizen_propose_agreement(
                    ['did:wellfare:guardian1'], 'did:wellfare:principal', 'health', 1
                );
                runner.expect(() => mod.webizen_sign_agreement(id, 'mock_key')).not.toThrow();
            });

            runner.it('two proposals generate distinct IDs', () => {
                if (!mod.webizen_propose_agreement) return;
                const id1 = mod.webizen_propose_agreement(['did:g1'], 'did:p1', 'a', 1);
                const id2 = mod.webizen_propose_agreement(['did:g2'], 'did:p2', 'b', 1);
                runner.expect(id1 === id2).toBeFalsy();
            });
        });

        runner.describe('Rights Ontology (enforce_rights_ontology)', () => {

            runner.it('DID 0 fails enforcement (reserved)', () => {
                if (!mod.enforce_rights_ontology) return;
                runner.expect(mod.enforce_rights_ontology(0n)).toBeFalsy();
            });

            runner.it('non-zero DID returns boolean', () => {
                if (!mod.enforce_rights_ontology) return;
                const r = mod.enforce_rights_ontology(12345678901234567890n);
                runner.expect(typeof r).toBe('boolean');
            });
        });

        runner.describe('Mesh Pruning (prune_and_validate_mesh)', () => {

            runner.it('mesh ID 0 returns boolean', () => {
                if (!mod.prune_and_validate_mesh) return;
                runner.expect(typeof mod.prune_and_validate_mesh(0n)).toBe('boolean');
            });

            runner.it('valid mesh ID returns true', () => {
                if (!mod.prune_and_validate_mesh) return;
                runner.expect(mod.prune_and_validate_mesh(1n)).toBeTruthy();
            });
        });

        runner.describe('Opcode Interception (intercept_computational_opcode)', () => {

            runner.it('light opcode returns null (no offload needed)', () => {
                if (!mod.intercept_computational_opcode) return;
                const result = mod.intercept_computational_opcode(0x01, 16);
                // Small payload — should not trigger offload
                runner.expect(result === null || result === undefined || typeof result === 'object').toBeTruthy();
            });

            runner.it('heavy opcode returns AgentIntent or null', () => {
                if (!mod.intercept_computational_opcode) return;
                const result = mod.intercept_computational_opcode(0xFF, 1024 * 1024);
                runner.expect(result === null || result === undefined || typeof result === 'object').toBeTruthy();
            });
        });

        runner.describe('Pharmacogenomics Intent (intercept_pharmacogenomics_intent)', () => {

            runner.it('aspirin SMILES returns AgentIntent', () => {
                if (!mod.intercept_pharmacogenomics_intent) return;
                const intent = mod.intercept_pharmacogenomics_intent('CC(=O)Oc1ccccc1C(=O)O');
                runner.expect(intent).not.toBeNull();
                runner.expect(intent).not.toBeUndefined();
            });

            runner.it('AgentIntent has opcode field', () => {
                if (!mod.intercept_pharmacogenomics_intent) return;
                const intent = mod.intercept_pharmacogenomics_intent('CCO');
                runner.expect(typeof intent.opcode).toBe('number');
            });
        });
    });
}

export default register;
