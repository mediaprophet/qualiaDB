// Neuro-Symbolic Sieve — mirrors neuro_symbolic_sieve.rs FSM grammar-constraint logic (pure JS).
// The sieve maps stochastic LLM token streams back into deterministic formal structures.

export function register(runner) {

    // Mirrors SymbolClass enum
    const SymbolClass = {
        SubjectRef:  0,
        PredicateRef: 1,
        ObjectLiteral: 2,
        Unknown: 3,
    };

    // Mirrors FsmState enum
    const FsmState = {
        ExpectSubject:    'ExpectSubject',
        ExpectPredicate:  'ExpectPredicate',
        ExpectObject:     'ExpectObject',
        Complete:         'Complete',
        Rejected:         'Rejected',
    };

    function classifyToken(token) {
        if (/^did:/.test(token) || /^<[^>]+>$/.test(token)) return SymbolClass.SubjectRef;
        if (/^[a-z]+:[a-zA-Z]+$/.test(token) && !token.startsWith('did:')) return SymbolClass.PredicateRef;
        if (/^".*"$/.test(token) || /^\d/.test(token)) return SymbolClass.ObjectLiteral;
        return SymbolClass.Unknown;
    }

    // FSM transition — mirrors NeuroSymbolicSieve::step()
    function fsmStep(state, token) {
        const cls = classifyToken(token);
        if (state === FsmState.ExpectSubject) {
            if (cls === SymbolClass.SubjectRef)   return FsmState.ExpectPredicate;
            return FsmState.Rejected;
        }
        if (state === FsmState.ExpectPredicate) {
            if (cls === SymbolClass.PredicateRef) return FsmState.ExpectObject;
            return FsmState.Rejected;
        }
        if (state === FsmState.ExpectObject) {
            if (cls === SymbolClass.ObjectLiteral || cls === SymbolClass.SubjectRef) return FsmState.Complete;
            return FsmState.Rejected;
        }
        return FsmState.Rejected;
    }

    function runSieve(tokens) {
        let state = FsmState.ExpectSubject;
        for (const tok of tokens) {
            state = fsmStep(state, tok);
            if (state === FsmState.Rejected) break;
        }
        return state;
    }

    // Logit masking: given a vocabulary and current state, which tokens are valid next?
    function allowedMask(vocabTokens, state) {
        return vocabTokens.map(tok => {
            const cls = classifyToken(tok);
            if (state === FsmState.ExpectSubject)   return cls === SymbolClass.SubjectRef;
            if (state === FsmState.ExpectPredicate) return cls === SymbolClass.PredicateRef;
            if (state === FsmState.ExpectObject)    return cls === SymbolClass.ObjectLiteral || cls === SymbolClass.SubjectRef;
            return false;
        });
    }

    runner.describe('Modality: Neuro-Symbolic Sieve', () => {

        runner.describe('Token classification (classifyToken)', () => {

            runner.it('DID URI → SubjectRef', () => {
                runner.expect(classifyToken('did:wellfare:alice')).toBe(SymbolClass.SubjectRef);
            });

            runner.it('angle-bracket IRI → SubjectRef', () => {
                runner.expect(classifyToken('<http://example.org/alice>')).toBe(SymbolClass.SubjectRef);
            });

            runner.it('prefixed name → PredicateRef', () => {
                runner.expect(classifyToken('foaf:knows')).toBe(SymbolClass.PredicateRef);
            });

            runner.it('quoted string → ObjectLiteral', () => {
                runner.expect(classifyToken('"Hello World"')).toBe(SymbolClass.ObjectLiteral);
            });

            runner.it('numeric literal → ObjectLiteral', () => {
                runner.expect(classifyToken('42')).toBe(SymbolClass.ObjectLiteral);
            });

            runner.it('random token → Unknown', () => {
                runner.expect(classifyToken('the')).toBe(SymbolClass.Unknown);
            });
        });

        runner.describe('FSM transitions', () => {

            runner.it('valid subject → ExpectPredicate', () => {
                runner.expect(fsmStep(FsmState.ExpectSubject, 'did:alice')).toBe(FsmState.ExpectPredicate);
            });

            runner.it('predicate in wrong slot → Rejected', () => {
                runner.expect(fsmStep(FsmState.ExpectSubject, 'foaf:knows')).toBe(FsmState.Rejected);
            });

            runner.it('literal in object slot → Complete', () => {
                runner.expect(fsmStep(FsmState.ExpectObject, '"hello"')).toBe(FsmState.Complete);
            });

            runner.it('full triple sequence reaches Complete', () => {
                const state = runSieve(['did:alice', 'foaf:knows', '"Bob"']);
                runner.expect(state).toBe(FsmState.Complete);
            });

            runner.it('garbled sequence reaches Rejected', () => {
                const state = runSieve(['foaf:knows', 'did:alice', '"Bob"']);
                runner.expect(state).toBe(FsmState.Rejected);
            });

            runner.it('early termination stays at intermediate state (incomplete triple)', () => {
                const state = runSieve(['did:alice', 'foaf:knows']);
                runner.expect(state).toBe(FsmState.ExpectObject);
            });
        });

        runner.describe('Logit masking', () => {

            const vocab = [
                'did:alice', 'foaf:knows', '"Bob"', 'the', 'is', '<http://ex.org/thing>'
            ];

            runner.it('in ExpectSubject state, only SubjectRef tokens are unmasked', () => {
                const mask = allowedMask(vocab, FsmState.ExpectSubject);
                const allowed = vocab.filter((_, i) => mask[i]);
                runner.expect(allowed.every(t => classifyToken(t) === SymbolClass.SubjectRef)).toBeTruthy();
            });

            runner.it('in ExpectPredicate state, only PredicateRef tokens are unmasked', () => {
                const mask = allowedMask(vocab, FsmState.ExpectPredicate);
                const allowed = vocab.filter((_, i) => mask[i]);
                runner.expect(allowed.every(t => classifyToken(t) === SymbolClass.PredicateRef)).toBeTruthy();
            });

            runner.it('mask eliminates non-grammar tokens — at least half are blocked in ExpectSubject', () => {
                const mask = allowedMask(vocab, FsmState.ExpectSubject);
                const blockedCount = mask.filter(v => !v).length;
                runner.expect(blockedCount).toBeGreaterThan(vocab.length / 2);
            });
        });
    });
}

export default register;
