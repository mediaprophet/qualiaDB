import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';

const docsRoot = path.resolve(import.meta.dirname, '..');
const read = (relativePath) => fs.readFileSync(path.join(docsRoot, relativePath), 'utf8');

const css = read('css/release-showcase.css');
assert.match(css, /\[data-theme="release-038"\]/, '038 theme tokens must exist');
assert.match(css, /\.q-prose/, 'markdown prose styles must exist');
assert.match(css, /\.hex-stack/, 'mobile honeycomb fallback must exist');

const qdnf = read('qdnf.html');
assert.match(qdnf, /data-theme="release-038"/);
assert.match(qdnf, /0\.0\.38/);
assert.match(qdnf, /human \(NaturalAgent\)/);
assert.match(qdnf, /Handle revoke ≠ who-erase|handle revoke ≠ who-erase/i);
assert.match(qdnf, /connection manager/);
assert.match(qdnf, /did:qi/);
assert.match(qdnf, /CSCP-08/);
assert.match(qdnf, /CSCP-12/);
assert.doesNotMatch(qdnf, /Ask · Keep · Talk/);
assert.doesNotMatch(
  qdnf,
  /href="(?!read\.html\?doc=)[^"]+\.md"/,
  'qdnf.html must not dump readers into raw markdown',
);
assert.match(qdnf, /read\.html\?doc=manuals\/standards\/qualia-decentralized-network-fabric\/README\.md/);
assert.match(qdnf, /read\.html\?doc=manuals\/standards\/did-qi-method\.md/);
assert.match(qdnf, /read\.html\?doc=standards\/ietf\/CSCP-08-LOCAL-CHORES\.md/);
assert.match(qdnf, /read\.html\?doc=work-in-progress\/CONTINUITY_GATE_HANDLE_REVOKE_WIP\.md/);
assert.doesNotMatch(qdnf, /public_relay_dialed\(\)\s*=\s*true/);
assert.doesNotMatch(qdnf, /MASQUE evidence flags stay <strong>true/);

const reader = read('read.html');
assert.match(reader, /data-theme="release-038"/);
assert.match(reader, /markdown-doc\.js/);
assert.match(reader, /marked@12/);

const layout = read('_layouts/qualia.html');
assert.match(layout, /data-theme="release-038"/);
assert.match(layout, /release-showcase\.css/);
assert.match(layout, /markdown-doc\.js/);
assert.match(layout, /data-mode="rewrite"/);

const relatedDocs = [
  'manuals/standards/qualia-decentralized-network-fabric/README.md',
  'manuals/standards/qualia-decentralized-network-fabric/peer-runtime.md',
  'manuals/standards/qualia-decentralized-network-fabric/network-cell.md',
  'manuals/standards/did-qi-method.md',
  'manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/README.md',
  'standards/ietf/CSCP-08-LOCAL-CHORES.md',
  'work-in-progress/CONTINUITY_GATE_HANDLE_REVOKE_WIP.md',
];
for (const relativePath of relatedDocs) {
  assert.ok(fs.existsSync(path.join(docsRoot, relativePath)), relativePath);
}

const scriptSource = read('js/markdown-doc.js');
const sandbox = {
  window: {
    location: { pathname: '/qdnf.html' },
  },
  document: {
    querySelector: () => null,
    querySelectorAll: () => [],
    currentScript: { dataset: { mode: 'rewrite' } },
    readyState: 'complete',
    addEventListener() {},
    getElementById: () => null,
  },
  URLSearchParams,
};
vm.runInNewContext(scriptSource, sandbox, { filename: 'markdown-doc.js' });
const api = sandbox.window.QualiaMarkdownDoc;
assert.ok(api, 'QualiaMarkdownDoc exports');
assert.equal(
  api.normalizeDocPath('manuals/standards/did-qi-method.md'),
  'manuals/standards/did-qi-method.md',
);
assert.equal(api.normalizeDocPath('../secret.md'), null);
assert.equal(api.normalizeDocPath('https://example.com/x.md'), null);
assert.equal(api.normalizeDocPath('crates/qualia-core-db/README.md'), null);
const resolved = api.resolveRelativeDoc(
  'manuals/standards/qualia-decentralized-network-fabric/README.md',
  './peer-runtime.md',
);
assert.equal(resolved.path, 'manuals/standards/qualia-decentralized-network-fabric/peer-runtime.md');
assert.equal(resolved.hash, '');
assert.equal(
  api.readerHref('standards/ietf/CSCP-08-LOCAL-CHORES.md', ''),
  'read.html?doc=standards%2Fietf%2FCSCP-08-LOCAL-CHORES.md',
);

console.log('qdnf Pages chrome checks passed.');
