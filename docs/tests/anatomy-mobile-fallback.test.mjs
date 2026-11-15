import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const anatomy = readFileSync(new URL('../playground/anatomy.js', import.meta.url), 'utf8');
const host = readFileSync(new URL('../playground/anatomy-host.js', import.meta.url), 'utf8');
const shell = readFileSync(new URL('../js/qualia-shell.js', import.meta.url), 'utf8');
const portal = readFileSync(
  new URL('../../crates/qualia-core-db/src/render/portal/mod.rs', import.meta.url),
  'utf8',
);
const webgl2 = readFileSync(
  new URL('../../crates/qualia-core-db/src/render/anatomy/webgl2.rs', import.meta.url),
  'utf8',
);

assert.match(anatomy, /Deprecated as of 0\.0\.41/);
assert.match(anatomy, /anatomy-host\.js/);
assert.match(anatomy, /throw new Error\("anatomy\.js is deprecated/);
assert.doesNotMatch(anatomy, /getBrowserCapabilityReceipt/);
assert.doesNotMatch(anatomy, /__anatomyBooted/);

const anatomyHtml = readFileSync(new URL('../playground/anatomy.html', import.meta.url), 'utf8');
assert.match(anatomyHtml, /anatomy-host\.js/);
assert.doesNotMatch(anatomyHtml, /src=["']anatomy\.js/);

assert.match(host, /loadQualiaPortal/);
assert.match(host, /allowWebGl2: true/);
assert.match(host, /load_hmc_mesh/);
assert.match(host, /human-welfare\.hmc/);
assert.match(host, /cache: "no-store"/);
assert.doesNotMatch(host, /cache: "force-cache"/);
assert.doesNotMatch(host, /if \(!navigator\.gpu\)/);

assert.match(readFileSync(new URL('../js/browser-capability.js', import.meta.url), 'utf8'), /WEBGPU_PROBE_BUDGET_MS/);
assert.match(readFileSync(new URL('../js/browser-capability.js', import.meta.url), 'utf8'), /adapter_timeout/);

assert.match(shell, /portal_init_webgl2/);
assert.match(shell, /requireBodyRenderer/);
assert.match(shell, /renderer: armedRenderer/);

assert.match(portal, /BodyRendererBackend::CpuCanvas/);
assert.match(portal, /BodyRendererBackend::WebGl2/);
assert.match(portal, /body_frames_presented > 0/);
assert.match(webgl2, /draw_elements_with_i32/);
assert.match(webgl2, /webgl2_context_lost/);

console.log('Anatomy mobile WebGL2 fallback contract tests passed.');
