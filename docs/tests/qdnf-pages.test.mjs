import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const docsRoot = path.resolve(import.meta.dirname, '..');
const read = (relativePath) => fs.readFileSync(path.join(docsRoot, relativePath), 'utf8');

const css = read('css/release-showcase.css');
assert.match(css, /\[data-theme="release-037"\]/);
assert.match(css, /\[data-theme="release-038"\]/);
assert.match(css, /--q-bg:\s*#05070c/);
assert.match(css, /--q-ink:\s*#e8eef7/);
assert.match(css, /--q-muted:\s*rgba\(232, 238, 247, 0\.62\)/);
assert.match(css, /--q-cyan:\s*#22d3ee/);
assert.match(css, /--q-emerald:\s*#34d399/);
assert.match(css, /--q-violet:\s*#a78bfa/);
assert.match(css, /"Space Grotesk"/);
assert.match(css, /"Inter"/);
assert.match(css, /"JetBrains Mono"/);
assert.doesNotMatch(css, /Fraunces|Source Sans 3|IBM Plex Mono/);
assert.match(css, /q-soft-rise/);
assert.match(css, /translateY\(10px\)/);
assert.match(css, /380ms ease-out/);
assert.match(css, /prefers-reduced-motion:\s*reduce/);
assert.match(css, /q-soft-rise-opacity/);
assert.match(css, /140ms ease-out/);
assert.match(css, /\.hex:hover/);

const qdnf = read('qdnf.html');
assert.match(qdnf, /data-theme="release-038"/);
assert.match(qdnf, /0\.0\.38/);
assert.match(qdnf, /Space\+Grotesk/);
assert.match(qdnf, /family=Inter/);
assert.match(qdnf, /JetBrains\+Mono/);
assert.match(qdnf, /human \(NaturalAgent\)/);
assert.match(qdnf, /connection manager/);
assert.match(qdnf, /did:qi/);
assert.match(qdnf, /CSCP-08/);
assert.match(qdnf, /CSCP-12/);
assert.match(qdnf, /Handle revoke ≠ who-erase/);
assert.match(qdnf, /q-soft-rise/);
assert.match(qdnf, /q-pill-primary/);
assert.match(qdnf, /id="continuity"/);
assert.match(qdnf, /id="cscp-08"/);
assert.match(qdnf, /Handle ≠ human/);
assert.match(qdnf, /q-continuity-strip/);
assert.match(qdnf, /js\/qdnf-map\.js/);
assert.match(qdnf, /id="hex-detail"/);
assert.match(qdnf, /data-hex="qlink"/);
assert.match(qdnf, /data-hex="dni"/);
assert.match(qdnf, /data-hex="qroute"/);
assert.match(qdnf, /data-hex="qresolve"/);
assert.match(qdnf, /data-hex="qpolicy"/);
assert.match(qdnf, /data-hex="qsession-qsync"/);
assert.match(qdnf, /data-plain=/);
assert.match(qdnf, /data-tech=/);
assert.match(qdnf, /aria-controls="hex-detail"/);
assert.match(qdnf, /aria-expanded="false"/);
assert.match(css, /q-continuity-strip/);
assert.match(css, /\.hex-detail/);
assert.match(css, /\.hex:focus-visible/);
const mapJs = read('js/qdnf-map.js');
assert.match(mapJs, /aria-expanded/);
assert.match(mapJs, /Escape/);
assert.doesNotMatch(mapJs, /react|vue|jquery/i);
assert.doesNotMatch(qdnf, /Ask · Keep · Talk/);
assert.doesNotMatch(qdnf, /href="[^"]+\.md"/);
assert.doesNotMatch(qdnf, /read\.html\?doc=/);
assert.doesNotMatch(qdnf, /border-rose-500/);
assert.match(qdnf, /href="manuals\/standards\/qualia-decentralized-network-fabric\/"/);
assert.match(qdnf, /href="qdnf-peer-runtime.html"/);
assert.match(qdnf, /href="qdnf-network-cells.html"/);
assert.match(qdnf, /href="qdnf-did-qi.html"/);
assert.match(qdnf, /href="#cscp-08"/);
assert.match(qdnf, /href="#continuity"/);
assert.doesNotMatch(qdnf, /public_relay_dialed\(\)\s*=\s*true/);
assert.doesNotMatch(qdnf, /MASQUE evidence flags stay <strong>true/);

for (const stub of ['qdnf-peer-runtime.html', 'qdnf-network-cells.html', 'qdnf-did-qi.html']) {
  const html = read(stub);
  assert.match(html, /data-theme="release-038"/);
  assert.match(html, /Handle ≠ human/);
  assert.doesNotMatch(html, /href="[^"]+\.md"/);
}

const menuLoader = read('js/menu-loader.js');
assert.match(menuLoader, /rawName === 'Webizen'/);

console.log('qdnf Pages chrome checks passed.');
