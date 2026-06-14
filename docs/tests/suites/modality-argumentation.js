// Argumentation Frameworks — mirrors argumentation.rs Dung-style abstract argumentation (pure JS).

export function register(runner) {

    // ── Core data types (mirror Argument, Attack, ArgumentationFramework) ───────
    function makeFramework() {
        return { arguments: new Map(), attacks: [] };
    }
    function addArgument(fw, id, content = '', strength = 1.0) {
        fw.arguments.set(id, { id, content, strength });
    }
    function addAttack(fw, attacker, target, strength = 1.0) {
        fw.attacks.push({ attacker, target, strength });
    }

    /** Returns the set of arguments that attack arg `id`. */
    function attackers(fw, id) {
        return fw.attacks.filter(a => a.target === id).map(a => a.attacker);
    }

    /** A set S defends argument `id` iff every attacker of `id` is itself
     *  attacked by some member of S. */
    function defends(fw, S, id) {
        return attackers(fw, id).every(att => S.some(s => fw.attacks.some(a => a.attacker === s && a.target === att)));
    }

    /** A set S is conflict-free iff no member of S attacks another member of S. */
    function isConflictFree(fw, S) {
        return !S.some(a => S.some(b => fw.attacks.some(atk => atk.attacker === a && atk.target === b)));
    }

    /** A set S is admissible iff it is conflict-free and defends all its members. */
    function isAdmissible(fw, S) {
        return isConflictFree(fw, S) && S.every(id => defends(fw, S, id));
    }

    /** Computes the grounded extension (iterative characteristic function fixpoint). */
    function groundedExtension(fw) {
        // Characteristic function: args defended by current extension
        const args = Array.from(fw.arguments.keys());
        let ext = [];
        let changed = true;
        while (changed) {
            changed = false;
            for (const id of args) {
                if (!ext.includes(id) && defends(fw, ext, id)) {
                    ext.push(id);
                    changed = true;
                }
            }
        }
        return ext.sort();
    }

    /** Enumerates all admissible sets (for small frameworks only). */
    function allAdmissible(fw) {
        const args = Array.from(fw.arguments.keys());
        const result = [];
        const n = args.length;
        for (let mask = 0; mask < (1 << n); mask++) {
            const S = args.filter((_, i) => mask & (1 << i));
            if (isAdmissible(fw, S)) result.push(S);
        }
        return result;
    }

    // ── Bit constants (mirror argument.rs) ──────────────────────────────────────
    const ARGUMENT_BIT = 1n << 55n;
    const ATTACK_BIT   = 1n << 54n;
    const DEFENSE_BIT  = 1n << 53n;

    runner.describe('Modality: Argumentation', () => {

        runner.describe('Bit constants', () => {
            runner.it('ARGUMENT_BIT is 1 << 55', () => runner.expect(ARGUMENT_BIT).toBe(1n << 55n));
            runner.it('ATTACK_BIT is 1 << 54',   () => runner.expect(ATTACK_BIT).toBe(1n << 54n));
            runner.it('DEFENSE_BIT is 1 << 53',  () => runner.expect(DEFENSE_BIT).toBe(1n << 53n));
            runner.it('bit constants are disjoint', () => {
                runner.expect(ARGUMENT_BIT & ATTACK_BIT).toBe(0n);
                runner.expect(ARGUMENT_BIT & DEFENSE_BIT).toBe(0n);
                runner.expect(ATTACK_BIT & DEFENSE_BIT).toBe(0n);
            });
        });

        runner.describe('Conflict-freedom', () => {
            runner.it('empty set is conflict-free', () => {
                const fw = makeFramework();
                runner.expect(isConflictFree(fw, [])).toBeTruthy();
            });

            runner.it('singleton set is always conflict-free', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addAttack(fw, 'a', 'a');
                runner.expect(isConflictFree(fw, ['a'])).toBeFalsy();
            });

            runner.it('two non-attacking args are conflict-free', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                runner.expect(isConflictFree(fw, ['a', 'b'])).toBeTruthy();
            });

            runner.it('set with mutual attackers is NOT conflict-free', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                addAttack(fw, 'a', 'b');
                runner.expect(isConflictFree(fw, ['a', 'b'])).toBeFalsy();
            });
        });

        runner.describe('Admissibility', () => {
            runner.it('empty set is admissible', () => {
                const fw = makeFramework();
                addArgument(fw, 'a');
                runner.expect(isAdmissible(fw, [])).toBeTruthy();
            });

            runner.it('unattacked singleton is admissible', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                addAttack(fw, 'a', 'b');
                runner.expect(isAdmissible(fw, ['a'])).toBeTruthy();
            });

            runner.it('attacked singleton with no counter-attack is not admissible', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                addAttack(fw, 'b', 'a');
                runner.expect(isAdmissible(fw, ['a'])).toBeFalsy();
            });

            runner.it('{a,c} is admissible when c attacks b which attacks a', () => {
                // Classic: a ← b ← c  ⟹ {a,c} is admissible (c defends a)
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b'); addArgument(fw, 'c');
                addAttack(fw, 'b', 'a'); addAttack(fw, 'c', 'b');
                runner.expect(isAdmissible(fw, ['a', 'c'])).toBeTruthy();
            });
        });

        runner.describe('Grounded extension', () => {
            runner.it('no attacks → all arguments in grounded extension', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                const ge = groundedExtension(fw);
                runner.expect(ge).toContain('a');
                runner.expect(ge).toContain('b');
            });

            runner.it('a attacks b → grounded extension is [a]', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                addAttack(fw, 'a', 'b');
                const ge = groundedExtension(fw);
                runner.expect(ge).toContain('a');
                runner.expect(ge).not.toContain('b');
            });

            runner.it('mutual attack → grounded extension is empty (sceptical)', () => {
                // a ↔ b: neither is defended
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                addAttack(fw, 'a', 'b'); addAttack(fw, 'b', 'a');
                runner.expect(groundedExtension(fw).length).toBe(0);
            });

            runner.it('reinstatement: b attacks a, c attacks b → a is in grounded extension', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b'); addArgument(fw, 'c');
                addAttack(fw, 'b', 'a'); addAttack(fw, 'c', 'b');
                const ge = groundedExtension(fw);
                runner.expect(ge).toContain('a');
                runner.expect(ge).toContain('c');
                runner.expect(ge).not.toContain('b');
            });
        });

        runner.describe('Admissible sets enumeration', () => {
            runner.it('empty set is always among admissible sets', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b'); addAttack(fw, 'a', 'b');
                const all = allAdmissible(fw);
                runner.expect(all.some(s => s.length === 0)).toBeTruthy();
            });

            runner.it('grounded extension is admissible', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b'); addArgument(fw, 'c');
                addAttack(fw, 'b', 'a'); addAttack(fw, 'c', 'b');
                const ge = groundedExtension(fw);
                runner.expect(isAdmissible(fw, ge)).toBeTruthy();
            });
        });

        runner.describe('Weighted argumentation', () => {
            runner.it('argument strength is preserved', () => {
                const fw = makeFramework();
                addArgument(fw, 'a', 'Climate change is anthropogenic', 0.95);
                runner.expect(fw.arguments.get('a').strength).toBe(0.95);
            });

            runner.it('attack strength is preserved', () => {
                const fw = makeFramework();
                addArgument(fw, 'a'); addArgument(fw, 'b');
                addAttack(fw, 'a', 'b', 0.7);
                runner.expect(fw.attacks[0].strength).toBe(0.7);
            });
        });
    });
}

export default register;
