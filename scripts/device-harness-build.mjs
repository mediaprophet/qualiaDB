import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, stat } from 'node:fs/promises';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const option = (name) => {
  const index = args.indexOf(name);
  if (index < 0) return undefined;
  if (!args[index + 1] || args[index + 1].startsWith('--')) throw new Error(`${name} needs a path`);
  return resolve(args[index + 1]);
};
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const git = (...values) => execFileSync('git', ['-c', 'core.safecrlf=false', ...values], { cwd: root, maxBuffer: 32 * 1024 * 1024 });
const revision = git('rev-parse', 'HEAD').toString().trim();
const etalonRevision = (await readFile(resolve(root, 'tools/accelerator-harness/etalon-revision.txt'), 'utf8')).trim();
if (!/^[0-9a-f]{40}$/.test(revision) || !/^[0-9a-f]{40}$/.test(etalonRevision)) {
  throw new Error('Both source revisions must be full reviewed commit hashes');
}

const capturePath = option('--capture-source');
if (capturePath) {
  const status = git('status', '--porcelain=v1', '--untracked-files=all').toString();
  const etalonPath = option('--etalon-path');
  if (!etalonPath) {
    const cargo = await readFile(resolve(root, 'Cargo.toml'), 'utf8');
    const declarations = cargo.split('\n').filter((line) => line.includes('git = "https://github.com/mediaprophet/etalon.git"'));
    if (declarations.length !== 11 || declarations.some((line) => !line.includes(`rev = "${etalonRevision}"`))) {
      throw new Error('Production Etalon declarations must all pin the manifest revision; local overrides require --etalon-path');
    }
  }
  let localEtalon = {};
  if (etalonPath) {
    const etalonGit = (...values) => execFileSync('git', ['-c', 'core.safecrlf=false', ...values], { cwd: etalonPath, maxBuffer: 32 * 1024 * 1024 });
    if (etalonGit('rev-parse', 'HEAD').toString().trim() !== etalonRevision) throw new Error('Local Etalon HEAD does not match the manifest revision');
    const etalonStatus = etalonGit('status', '--porcelain=v1', '--untracked-files=all').toString();
    localEtalon = {
      etalon_dirty: etalonStatus.length > 0,
      etalon_tracked_diff_sha256: sha256(etalonGit('diff', '--binary', 'HEAD', '--')),
      etalon_untracked_file_count: etalonStatus.split('\n').filter((line) => line.startsWith('??')).length,
    };
  }
  const dirty = status.length > 0 || localEtalon.etalon_dirty === true;
  if (dirty && !args.includes('--allow-dirty')) {
    throw new Error('Source checkout is dirty; explicitly use --allow-dirty for a local development build');
  }
  const record = {
    schema_version: 1,
    qualia_revision: revision,
    etalon_revision: etalonRevision,
    captured_at_utc: new Date().toISOString(),
    dirty,
    attribution: dirty ? 'development checkout; commit alone does not identify these sources' : 'clean commit checkout before asset generation',
    tracked_diff_sha256: sha256(git('diff', '--binary', 'HEAD', '--')),
    untracked_file_count: status.split('\n').filter((line) => line.startsWith('??')).length,
    ...localEtalon,
  };
  await mkdir(dirname(capturePath), { recursive: true });
  await writeFile(capturePath, JSON.stringify(record, null, 2) + '\n');
  console.log(`Captured ${record.dirty ? 'development' : 'clean'} source identity: ${capturePath}`);
} else {
  const sourcePath = option('--source-record');
  if (!sourcePath) throw new Error('Capture source identity before building, then pass --source-record');
  if ((await stat(sourcePath)).size > 64 * 1024) throw new Error('Source record exceeds budget');
  const source = JSON.parse(await readFile(sourcePath, 'utf8'));
  if (source.schema_version !== 1 || source.qualia_revision !== revision || source.etalon_revision !== etalonRevision || typeof source.dirty !== 'boolean') {
    throw new Error('Source record does not match this checkout and reviewed Etalon revision');
  }
  const output = option('--output') ?? resolve(root, 'docs/playground/device-harness-build.json');
  const assetDirectory = option('--asset-directory') ?? resolve(root, 'docs/pkg/qualia');
  if (!assetDirectory.startsWith(resolve(root, 'docs') + sep)) throw new Error('Harness assets must be within public docs');
  const assets = {};
  for (const [name, file] of [
    ['portal_wasm', 'qualia_bg.wasm'],
    ['portal_js', 'qualia.js'],
  ]) {
    const path = resolve(assetDirectory, file);
    const info = await stat(path);
    if (info.size === 0 || info.size > 32 * 1024 * 1024) throw new Error(`${file} exceeds the asset budget or is empty`);
    const bytes = await readFile(path);
    assets[name] = { url: relative(dirname(output), path).split(sep).join('/'), sha256: sha256(bytes), bytes: bytes.length };
  }
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, JSON.stringify({
    ...source,
    generated_at_utc: new Date().toISOString(),
    scope: 'QualiaDB/VibeScript CPU and Etalon Rust browser frontend; browser implements WebGPU; native HAL and NPU execution are separate',
    assets,
  }, null, 2) + '\n');
  console.log(`Wrote browser harness asset identity: ${output}`);
}
