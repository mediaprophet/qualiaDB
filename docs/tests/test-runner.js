// Minimal browser test framework — no dependencies, no CDN.
// API: describe / it / beforeAll / afterAll / expect

export class AssertionError extends Error {
    constructor(msg) {
        super(msg);
        this.name = 'AssertionError';
    }
}

function fmt(v) {
    if (typeof v === 'bigint') return v.toString() + 'n';
    if (v === null) return 'null';
    if (v === undefined) return 'undefined';
    if (typeof v === 'string') return JSON.stringify(v);
    if (typeof v === 'object') {
        try { return JSON.stringify(v); } catch (_) { return String(v); }
    }
    return String(v);
}

class Expectation {
    constructor(actual, negated = false) {
        this._actual = actual;
        this._negated = negated;
    }

    get not() { return new Expectation(this._actual, !this._negated); }

    _pass(cond, msg) {
        const ok = this._negated ? !cond : cond;
        if (!ok) throw new AssertionError(this._negated ? `Expected NOT: ${msg}` : msg);
    }

    toBe(expected) {
        this._pass(Object.is(this._actual, expected),
            `Expected ${fmt(expected)}, got ${fmt(this._actual)}`);
    }

    toEqual(expected) {
        const a = JSON.stringify(this._actual, replacer);
        const b = JSON.stringify(expected, replacer);
        this._pass(a === b, `Expected ${b}, got ${a}`);
    }

    toBeTruthy() { this._pass(!!this._actual, `Expected truthy, got ${fmt(this._actual)}`); }
    toBeFalsy()  { this._pass(!this._actual,  `Expected falsy, got ${fmt(this._actual)}`); }

    toBeNull()      { this._pass(this._actual === null,      `Expected null, got ${fmt(this._actual)}`); }
    toBeUndefined() { this._pass(this._actual === undefined, `Expected undefined, got ${fmt(this._actual)}`); }

    toBeGreaterThan(n)        { this._pass(this._actual >  n, `Expected ${fmt(this._actual)} > ${n}`); }
    toBeLessThan(n)           { this._pass(this._actual <  n, `Expected ${fmt(this._actual)} < ${n}`); }
    toBeGreaterThanOrEqual(n) { this._pass(this._actual >= n, `Expected ${fmt(this._actual)} >= ${n}`); }
    toBeLessThanOrEqual(n)    { this._pass(this._actual <= n, `Expected ${fmt(this._actual)} <= ${n}`); }

    toBeCloseTo(expected, digits = 5) {
        const diff = Math.abs(Number(this._actual) - Number(expected));
        this._pass(diff < Math.pow(10, -digits) * 10,
            `Expected ${fmt(this._actual)} close to ${expected} (±10^-${digits})`);
    }

    toContain(item) {
        if (typeof this._actual === 'string') {
            this._pass(this._actual.includes(item),
                `Expected string to contain ${fmt(item)}`);
        } else if (Array.isArray(this._actual)) {
            this._pass(this._actual.some(x => Object.is(x, item)),
                `Expected array to contain ${fmt(item)}`);
        } else {
            throw new AssertionError('toContain requires string or array');
        }
    }

    toHaveProperty(key) {
        this._pass(key in Object(this._actual), `Expected object to have property ${fmt(key)}`);
    }

    toBeInstanceOf(klass) {
        this._pass(this._actual instanceof klass, `Expected instance of ${klass.name}`);
    }

    toThrow(expectedMsg) {
        if (typeof this._actual !== 'function') throw new AssertionError('toThrow requires a function');
        let threw = false;
        try { this._actual(); } catch (_) { threw = true; }
        this._pass(threw, expectedMsg
            ? `Expected function to throw "${expectedMsg}"`
            : 'Expected function to throw');
    }
}

function replacer(_, v) {
    return typeof v === 'bigint' ? v.toString() + 'n' : v;
}

// ─── Suite registry ────────────────────────────────────────────────────────────

export class TestRunner {
    constructor() {
        this._suites = [];
        this._stack  = [];      // current describe stack
    }

    describe(name, fn) {
        const suite = { name, tests: [], _beforeAll: null, _afterAll: null };
        if (this._stack.length) {
            this._stack[this._stack.length - 1].tests.push({ _isSuite: true, suite });
        } else {
            this._suites.push(suite);
        }
        this._stack.push(suite);
        fn();
        this._stack.pop();
        return suite;
    }

    beforeAll(fn) {
        const s = this._stack[this._stack.length - 1];
        if (s) s._beforeAll = fn;
    }

    afterAll(fn) {
        const s = this._stack[this._stack.length - 1];
        if (s) s._afterAll = fn;
    }

    it(name, fn) {
        const s = this._stack[this._stack.length - 1];
        if (s) s.tests.push({ name, fn });
    }

    expect(actual) { return new Expectation(actual); }

    // Run all registered suites, emitting events to `emit`.
    // emit(event) where event = { type: 'suite-start'|'pass'|'fail'|'suite-end', ... }
    async run(emit) {
        let totPassed = 0, totFailed = 0;
        for (const suite of this._suites) {
            const r = await this._runSuite(suite, emit, []);
            totPassed += r.passed;
            totFailed += r.failed;
        }
        return { passed: totPassed, failed: totFailed };
    }

    async _runSuite(suite, emit, path) {
        const fullName = [...path, suite.name].join(' › ');
        emit({ type: 'suite-start', name: fullName, suite });

        let passed = 0, failed = 0;

        if (suite._beforeAll) {
            try { await suite._beforeAll(); }
            catch (e) {
                emit({ type: 'fail', name: `${fullName} [beforeAll]`, suite, error: e });
                emit({ type: 'suite-end', name: fullName, suite, passed, failed: failed + 1 });
                return { passed, failed: failed + 1 };
            }
        }

        for (const item of suite.tests) {
            if (item._isSuite) {
                const r = await this._runSuite(item.suite, emit, [...path, suite.name]);
                passed += r.passed; failed += r.failed;
                continue;
            }
            let err = null;
            const t0 = performance.now();
            try { await item.fn(); }
            catch (e) { err = e; }
            const ms = performance.now() - t0;
            if (err) {
                failed++;
                emit({ type: 'fail', name: item.name, suite, error: err, ms });
            } else {
                passed++;
                emit({ type: 'pass', name: item.name, suite, ms });
            }
        }

        if (suite._afterAll) {
            try { await suite._afterAll(); } catch (_) {}
        }

        emit({ type: 'suite-end', name: fullName, suite, passed, failed });
        return { passed, failed };
    }
}
