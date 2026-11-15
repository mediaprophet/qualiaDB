import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { etalonDependencyIdentity, validateDependencyIdentity } from './device-harness-identity.mjs';

const gpu = 'a'.repeat(40);
const pure = 'b'.repeat(40);
const declaration = (name, revision) => `${name} = { git = "https://github.com/mediaprophet/etalon.git", rev = "${revision}" }`;
const locked = (name, revision, resolved = revision) => `[[package]]\nname = "${name}"\nversion = "0.0.2"\nsource = "git+https://github.com/mediaprophet/etalon.git?rev=${revision}#${resolved}"\n`;
const cargo = `[workspace.dependencies]\n${declaration('wgpu', gpu)}\n${declaration('etalon', gpu)}\n${declaration('etalon-model', pure)}\n`;
const lock = locked('wgpu', gpu) + locked('etalon', gpu) + locked('etalon-model', pure);

test('records separate reviewed GPU and pure-library pins without a fixed declaration count', () => {
  const identity = etalonDependencyIdentity(cargo, lock);
  assert.deepEqual(identity.declarations.map(({ revision }) => revision), [gpu, gpu, pure]);
  assert.equal(identity.packages.length, 3);
  assert.match(identity.cargo_toml_sha256, /^[a-f0-9]{64}$/);
  validateDependencyIdentity(structuredClone(identity), identity);
});

test('all current production declarations resolve in the checked-in lockfile', () => {
  const identity = etalonDependencyIdentity(
    readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8'),
    readFileSync(new URL('../Cargo.lock', import.meta.url), 'utf8'),
  );
  assert.ok(identity.declarations.length > 0);
  assert.ok(identity.packages.length > 0);
});

test('rejects short, missing and path-overridden declaration identities', () => {
  for (const invalid of [cargo.replace(gpu, 'main'), cargo.replace(`rev = "${gpu}"`, ''), cargo.replace(`rev = "${gpu}"`, `rev = "${gpu}", path = "../etalon"`)]) {
    assert.throws(() => etalonDependencyIdentity(invalid, lock), /full reviewed git revisions/);
  }
});

test('rejects missing production declarations', () => {
  assert.throws(() => etalonDependencyIdentity('', lock), /Missing production/);
});

test('rejects replacing a production declaration with a local path or registry source', () => {
  for (const replacement of ['etalon-model = { path = "../etalon" }', 'etalon-model = "0.0.2"']) {
    assert.throws(() => etalonDependencyIdentity(cargo.replace(declaration('etalon-model', pure), replacement), lock), /require reviewed git identities/);
  }
});

test('rejects missing or differently resolved locked packages', () => {
  assert.throws(() => etalonDependencyIdentity(cargo, locked('wgpu', gpu)), /does not resolve/);
  assert.throws(() => etalonDependencyIdentity(cargo, lock.replace(`#${pure}`, `#${gpu}`)), /Invalid or unresolved/);
});

test('rejects undeclared lockfile revisions', () => {
  assert.throws(() => etalonDependencyIdentity(cargo, lock + locked('etalon-extra', 'c'.repeat(40))), /Undeclared/);
});

test('source record requires complete current dependency identity', () => {
  const identity = etalonDependencyIdentity(cargo, lock);
  assert.throws(() => validateDependencyIdentity(undefined, identity), /Source record/);
  assert.throws(() => validateDependencyIdentity({ ...identity, declarations: [] }, identity), /Source record/);
  assert.throws(() => validateDependencyIdentity(identity, etalonDependencyIdentity(cargo + '\n', lock)), /Source record/);
  assert.throws(() => validateDependencyIdentity(identity, etalonDependencyIdentity(cargo, lock + '\n')), /Source record/);
});
