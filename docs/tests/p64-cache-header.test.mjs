import assert from 'node:assert/strict';
import { isP64Header } from '../js/opfs-model-cache.js';

const p64v3 = Uint8Array.from([0x70, 0x36, 0x34, 0x00, 0x03, 0x00, 0x00, 0x00]);

assert.equal(isP64Header(p64v3, 3), true);
assert.equal(isP64Header(p64v3), true);
assert.equal(isP64Header(Uint8Array.from([0x70, 0x36, 0x34, 0x00])), true);
assert.equal(isP64Header(Uint8Array.from([0x70, 0x36, 0x34, 0x00]), 3), false);
assert.equal(isP64Header(p64v3, 2), false);
assert.equal(isP64Header(Uint8Array.from([0x50, 0x36, 0x34, 0x00, 3, 0]), 3), false);
assert.equal(isP64Header(Uint8Array.from([0x50, 0x36, 0x34]), 3), false);
assert.equal(isP64Header(Uint8Array.from([0x51, 0x34, 0x32, 0x57, 3, 0]), 3), false);
assert.equal(isP64Header(Uint8Array.from([0x70, 0x36, 0x34]), 3), false);

console.log('P64 cache header tests passed.');
