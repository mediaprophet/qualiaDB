// WASM Bioinformatics tests.
// Covers align_sequences_wasm and validate_fasta_wasm from wasm_bridge.rs.

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Bioinformatics', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.describe('Sequence Alignment (align_sequences_wasm)', () => {

            runner.it('identical nucleotide sequences → 100% identity', () => {
                if (!mod.align_sequences_wasm) return;
                const r = mod.align_sequences_wasm({ query: 'ATCG', target: 'ATCG', mode: 'nucleotide' });
                runner.expect(r.identity_pct).toBeGreaterThanOrEqual(99.0);
            });

            runner.it('alignment returns required fields', () => {
                if (!mod.align_sequences_wasm) return;
                const r = mod.align_sequences_wasm({ query: 'ATCG', target: 'ATCG', mode: 'nucleotide' });
                runner.expect(r).toHaveProperty('score');
                runner.expect(r).toHaveProperty('identity_pct');
                runner.expect(r).toHaveProperty('num_matches');
                runner.expect(r).toHaveProperty('num_gaps');
                runner.expect(r).toHaveProperty('aligned_query');
                runner.expect(r).toHaveProperty('aligned_target');
            });

            runner.it('nucleotide mode produces positive score for matching seq', () => {
                if (!mod.align_sequences_wasm) return;
                const r = mod.align_sequences_wasm({ query: 'AAAA', target: 'AAAA', mode: 'nucleotide' });
                runner.expect(r.score).toBeGreaterThan(0);
            });

            runner.it('protein alignment mode accepted', () => {
                if (!mod.align_sequences_wasm) return;
                const r = mod.align_sequences_wasm({ query: 'MAST', target: 'MAST', mode: 'protein' });
                runner.expect(r.identity_pct).toBeGreaterThanOrEqual(99.0);
            });

            runner.it('divergent sequences score worse than identical ones', () => {
                if (!mod.align_sequences_wasm) return;
                const perfect = mod.align_sequences_wasm({ query: 'ATCGATCG', target: 'ATCGATCG', mode: 'nucleotide' });
                const diverge = mod.align_sequences_wasm({ query: 'ATCGATCG', target: 'TTTTTTTT', mode: 'nucleotide' });
                runner.expect(diverge.score).toBeLessThan(perfect.score);
            });
        });

        runner.describe('FASTA Validation (validate_fasta_wasm)', () => {

            runner.it('valid DNA FASTA passes', () => {
                if (!mod.validate_fasta_wasm) return;
                const r = mod.validate_fasta_wasm({ header: '>seq1 test', sequence: 'ATCGATCG' });
                runner.expect(r.is_valid).toBeTruthy();
            });

            runner.it('invalid chars detected', () => {
                if (!mod.validate_fasta_wasm) return;
                const r = mod.validate_fasta_wasm({ header: '>seq1', sequence: 'ATCG1234' });
                runner.expect(r.invalid_chars.length).toBeGreaterThan(0);
            });

            runner.it('valid FASTA has empty invalid_chars list', () => {
                if (!mod.validate_fasta_wasm) return;
                const r = mod.validate_fasta_wasm({ header: '>seq1', sequence: 'ATCG' });
                runner.expect(r.invalid_chars.length).toBe(0);
            });

            runner.it('alphabet field is returned', () => {
                if (!mod.validate_fasta_wasm) return;
                const r = mod.validate_fasta_wasm({ header: '>seq1', sequence: 'ATCG' });
                runner.expect(typeof r.alphabet).toBe('string');
            });
        });
    });
}

export default register;
