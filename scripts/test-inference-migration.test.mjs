import test from 'node:test';
import assert from 'node:assert/strict';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { runBounded, testCount, withScratch } from './test-inference-migration.mjs';

test('only complete executed Rust test results count', () => {
  assert.equal(testCount('test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'), 7);
  for (const result of ['0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', '7 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;', '7 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;']) {
    assert.throws(() => testCount(`test result: ok. ${result}`));
  }
  assert.throws(() => testCount('test result: FAILED. 7 passed; 1 failed;'));
  assert.throws(() => testCount('test result: ok. 999999999999999999999999 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'));
  assert.throws(() => testCount('test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'));
  assert.throws(() => testCount('test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\ntest result: FAILED. 1 passed; 1 failed;'));
});

test('bounded child reports failures and enforces output/deadline budgets', async () => {
  assert.match(await runBounded(process.execPath, ['-e', 'console.log("ok")']), /ok/);
  await assert.rejects(runBounded(process.execPath, ['-e', 'process.exit(2)']), /exited 2/);
  await assert.rejects(runBounded(process.execPath, ['-e', 'console.log("a".repeat(4096))'], { maxBytes: 128 }), /byte budget/);
  await assert.rejects(runBounded(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { timeoutMs: 100 }), /deadline/);
});

test('owned scratch is removed on success and failure', async () => {
  const parent = await fs.mkdtemp(path.join(os.tmpdir(), 'inference-runner-test-'));
  try {
    let owned;
    await withScratch(parent, async (directory) => { owned = directory; await fs.writeFile(path.join(directory, 'output'), 'test'); });
    await assert.rejects(fs.stat(owned), { code: 'ENOENT' });
    await assert.rejects(withScratch(parent, async (directory) => { owned = directory; throw new Error('test failure'); }), /test failure/);
    await assert.rejects(fs.stat(owned), { code: 'ENOENT' });
  } finally {
    await fs.rmdir(parent);
  }
});
