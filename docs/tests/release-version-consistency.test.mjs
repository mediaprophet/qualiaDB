import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const releaseVersion = '0.0.28';
const previousVersion = ['0', '0', '27'].join('.');
const root = path.resolve(import.meta.dirname, '..', '..');

const rootManifest = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
const workspaceMembers = [...rootManifest.matchAll(/^\s*"([^"]+)",?\s*$/gm)]
  .map((match) => match[1])
  .filter((member) => member.startsWith('crates/'));

assert.equal(workspaceMembers.length, 20, 'expected all 20 workspace crates');

for (const member of workspaceMembers) {
  const manifestPath = path.join(root, member, 'Cargo.toml');
  const manifest = fs.readFileSync(manifestPath, 'utf8');
  const packageSection = manifest.match(/^\[package\]\s*$([\s\S]*?)(?=^\[|\Z)/m)?.[1] ?? '';
  const version = packageSection.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  assert.equal(version, releaseVersion, `${member} package version`);
}

const textExtensions = new Set([
  '.html', '.json', '.js', '.mjs', '.md', '.ps1', '.sh', '.toml', '.yaml', '.yml',
]);
const scanRoots = [
  'Cargo.toml',
  'Cargo.lock',
  '.github/workflows',
  'crates/webizen-desktop/static/portal',
  'docs',
];
const stale = [];

function scan(relativePath) {
  const absolutePath = path.join(root, relativePath);
  const stat = fs.statSync(absolutePath);
  if (stat.isDirectory()) {
    for (const entry of fs.readdirSync(absolutePath)) {
      scan(path.join(relativePath, entry));
    }
    return;
  }
  if (!textExtensions.has(path.extname(absolutePath))) return;
  const text = fs.readFileSync(absolutePath, 'utf8');
  if (text.includes(previousVersion)) stale.push(relativePath.replaceAll('\\', '/'));
  if (path.basename(absolutePath) === 'package.json') {
    const packageVersion = JSON.parse(text.replace(/^\uFEFF/, '')).version;
    if (packageVersion !== undefined) {
      assert.equal(packageVersion, releaseVersion, `${relativePath} package version`);
    }
  }
}

for (const scanRoot of scanRoots) scan(scanRoot);

assert.deepEqual(stale, [], `stale ${previousVersion} identifiers: ${stale.join(', ')}`);
assert.equal(
  JSON.parse(fs.readFileSync(path.join(root, 'docs/menu.json'), 'utf8')).version,
  releaseVersion,
);
assert.equal(
  JSON.parse(fs.readFileSync(path.join(root, 'docs/playground/package.json'), 'utf8')).version,
  releaseVersion,
);

console.log(`Release version consistency passed for ${workspaceMembers.length} crates at ${releaseVersion}.`);
