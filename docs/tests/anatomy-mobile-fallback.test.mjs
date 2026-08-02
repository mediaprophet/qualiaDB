import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const anatomy = readFileSync(new URL('../playground/anatomy.js', import.meta.url), 'utf8');
const shell = readFileSync(new URL('../js/qualia-shell.js', import.meta.url), 'utf8');
const portal = readFileSync(
  new URL('../../crates/qualia-core-db/src/render/portal/mod.rs', import.meta.url),
  'utf8',
);
const webgl2 = readFileSync(
  new URL('../../crates/qualia-core-db/src/render/anatomy/webgl2.rs', import.meta.url),
  'utf8',
);

assert.match(anatomy, /getBrowserCapabilityReceipt/);
assert.match(anatomy, /capabilityReceipt\.selection\.anatomy/);
assert.match(anatomy, /allowWebGl2: capabilityReceipt\.webgl2\.available/);
assert.match(anatomy, /body_render_receipt/);
assert.match(anatomy, /if \(receipt\?\.success\)/);
assert.doesNotMatch(anatomy, /if \(!navigator\.gpu\)/);

assert.match(shell, /portal_init_webgl2/);
assert.match(shell, /requireBodyRenderer/);
assert.match(shell, /renderer: armedRenderer/);

assert.match(portal, /anatomy_renderer_unsupported/);
assert.match(portal, /BodyRendererBackend::WebGl2/);
assert.match(portal, /body_frames_presented > 0/);
assert.match(webgl2, /draw_elements_with_i32/);
assert.match(webgl2, /webgl2_context_lost/);

console.log('Anatomy mobile WebGL2 fallback contract tests passed.');
