import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const releaseVersion = '0.0.34';
const root = path.resolve(import.meta.dirname, '..', '..');
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8');

const rootManifest = read('Cargo.toml');
const workspaceMembers = [...rootManifest.matchAll(/^\s*"([^"]+)",?\s*$/gm)]
  .map((match) => match[1])
  .filter((member) => member.startsWith('crates/'));

assert.equal(workspaceMembers.length, 25, 'expected all 25 workspace crates');

const workspacePackageNames = [];
for (const member of workspaceMembers) {
  const manifest = read(path.join(member, 'Cargo.toml'));
  const packageSection = manifest.match(/^\[package\]\s*$([\s\S]*?)(?=^\[|\Z)/m)?.[1] ?? '';
  const name = packageSection.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
  const version = packageSection.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  assert.ok(name, `${member} package name`);
  assert.equal(version, releaseVersion, `${member} package version`);
  workspacePackageNames.push(name);
}

const lockPackages = new Map(
  [...read('Cargo.lock').matchAll(/\[\[package\]\]\r?\nname = "([^"]+)"\r?\nversion = "([^"]+)"/g)]
    .map((match) => [match[1], match[2]]),
);
for (const name of workspacePackageNames) {
  assert.equal(lockPackages.get(name), releaseVersion, `Cargo.lock ${name} version`);
}

const versionedJson = [
  'crates/webizen-desktop/tauri.conf.json',
  'crates/webizen-desktop/static/portal/menu.json',
  'docs/data/knowledge-universe-manifest.json',
  'docs/menu.json',
  'docs/pkg/qualia/package.json',
  'docs/playground/package.json',
  'scripts/studio-gui-e2e/package.json',
];
for (const relativePath of versionedJson) {
  const json = JSON.parse(read(relativePath).replace(/^\uFEFF/, ''));
  assert.equal(json.version, releaseVersion, `${relativePath} version`);
}

const liveReleaseSurfaces = [
  '.github/workflows/benchmarks.yml',
  '.github/workflows/pages.yml',
  '.github/workflows/release-cli.yml',
  '.github/workflows/release-desktop.yml',
  '.github/workflows/release-p64-models.yml',
  '.github/workflows/release-wasm.yml',
  'crates/webizen-desktop/static/portal/js/qualia-wasm-runtime.js',
  'docs/api-explorer/index.html',
  'docs/api.html',
  'docs/benchmark.html',
  'docs/js/mobile-wasm-lab.js',
  'docs/js/qualia-wasm-runtime.js',
  'docs/online-llm-demo.html',
  'docs/playground/anatomy.js',
  'docs/tests/index.html',
];
for (const relativePath of liveReleaseSurfaces) {
  const text = read(relativePath);
  assert.ok(text.includes(releaseVersion), `${relativePath} must identify ${releaseVersion}`);
  assert.ok(!text.includes('0.0.29-moredev'), `${relativePath} must not identify a development branch`);
}

const apiExplorer = read('docs/api-explorer/index.html');
assert.match(apiExplorer, /href="\.\.\/css\/tailwind-built\.css"/,
  'API Explorer must load the utility styles used by the shared navigation');
assert.match(apiExplorer, /href="\.\.\/css\/site-nav\.css"/,
  'API Explorer must load shared navigation styles');
assert.match(apiExplorer, /menu-loader\.js/,
  'API Explorer must load the shared navigation renderer');

assert.match(read('.github/workflows/pages.yml'), /- "0\.0\.34"/);
assert.match(read('.github/workflows/release-p64-models.yml'), /- 0\.0\.34/);

const comparative = JSON.parse(read('docs/comparative_benchmark_results.json'));
assert.equal(
  comparative.execution_environment?.engine_version,
  releaseVersion,
  'synthetic-10k comparative JSON must stamp the current engine',
);
assert.equal(
  comparative.engines?.qualia_wasm?.engine_version,
  releaseVersion,
  'synthetic-10k Qualia WASM row must stamp the current engine',
);
const schemaorgComparative = JSON.parse(
  read('docs/comparative_benchmark_results.schemaorg-30-current-https.json'),
);
assert.equal(
  schemaorgComparative.execution_environment?.engine_version,
  releaseVersion,
  'Schema.org comparative JSON must stamp the current engine',
);

console.log(`Release version consistency passed for ${workspaceMembers.length} crates at ${releaseVersion}.`);
