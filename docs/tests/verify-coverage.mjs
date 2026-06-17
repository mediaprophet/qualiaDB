#!/usr/bin/env node
/**
 * Ensures every suite file under docs/tests/suites/ is registered in suite-registry.js
 * and that headless + browser runners share the same registry.
 */

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const suitesDir = join(here, 'suites');
const registryPath = join(here, 'suite-registry.js');

const suiteFiles = readdirSync(suitesDir)
    .filter(f => f.endsWith('.js'))
    .sort();

const registrySrc = readFileSync(registryPath, 'utf8');
const imported = [...registrySrc.matchAll(/from '\.\/suites\/([^']+)'/g)].map(m => m[1]).sort();

const missing = suiteFiles.filter(f => !imported.includes(f));
const orphanImports = imported.filter(f => !suiteFiles.includes(f));

if (missing.length || orphanImports.length) {
    console.error('docs/tests coverage mismatch:\n');
    if (missing.length) {
        console.error('  Suite files NOT registered in suite-registry.js:');
        for (const f of missing) console.error(`    - suites/${f}`);
    }
    if (orphanImports.length) {
        console.error('  Registry imports with no suite file:');
        for (const f of orphanImports) console.error(`    - suites/${f}`);
    }
    process.exit(1);
}

console.log(`OK: ${suiteFiles.length} suite modules registered in suite-registry.js`);