import assert from 'node:assert/strict';
import fs from 'node:fs';

const root = new URL('../..', import.meta.url);
const helper = fs.readFileSync(new URL('crates/qualia-core-db/src/gguf_bridge/browser/webgpu/output_top1.rs', root), 'utf8');
const forward = fs.readFileSync(new URL('crates/qualia-core-db/src/gguf_bridge/forward.rs', root), 'utf8');
const asyncOutput = fs.readFileSync(new URL('crates/qualia-core-db/src/gguf_bridge/prefill_async.rs', root), 'utf8');
const glue = fs.readFileSync(new URL('docs/playground/qualia_core_db.js', root), 'utf8');

assert.match(helper, /BrowserTop1Plan/);
assert.match(helper, /TOPK_BLOCK_SIZE/);
assert.match(helper, /encode_browser_top1_chunk/);
assert.match(helper, /read_browser_top1/);
assert.match(helper, /value == max_logit && token_id < best_token_id/);

assert.match(forward, /encode_browser_top1_chunk/);
assert.match(forward, /read_browser_top1/);
assert.doesNotMatch(forward, /still reads the complete vocabulary back for CPU argmax/);
assert.match(asyncOutput, /dispatch_output_argmax_batched_async[\s\S]*encode_browser_top1_chunk/);
assert.match(asyncOutput, /dispatch_output_argmax_batched_async[\s\S]*read_browser_top1/);
assert.match(glue, /export function getBrowserExecutionReceipt\(/);

// SmolLM2's 49,152-token vocabulary becomes 48 block winners:
// 48 × (f32 score + u32 token id) = 384 bytes, not 196,608 bytes.
const vocab = 49_152;
const compactBytes = Math.ceil(vocab / 1024) * 8;
assert.equal(compactBytes, 384);
assert.equal(vocab * 4, 196_608);

console.log('Browser WebGPU compact top-1 contract passed.');
