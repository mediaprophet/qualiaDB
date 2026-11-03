import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const marker = '.inference-contract-test-run';
const limit = 1024 * 1024;

export async function runBounded(command, args, { cwd, timeoutMs = 120_000, maxBytes = limit } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const chunks = [];
    let bytes = 0;
    let failure;
    const stop = (reason) => {
      if (failure) return;
      failure = new Error(reason);
      if (process.platform === 'win32' && child.pid) {
        const killer = spawn('taskkill', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' });
        killer.on('error', () => child.kill());
        killer.on('exit', (code) => { if (code !== 0) child.kill(); });
      } else child.kill('SIGKILL');
    };
    const collect = (data) => {
      if (failure) return;
      bytes += data.length;
      if (bytes > maxBytes) stop('Child output exceeded its byte budget');
      else chunks.push(data);
    };
    child.stdout.on('data', collect);
    child.stderr.on('data', collect);
    const timer = setTimeout(() => stop('Child process deadline exceeded'), timeoutMs);
    child.on('error', (error) => { clearTimeout(timer); reject(error); });
    child.on('close', (code) => {
      clearTimeout(timer);
      const output = Buffer.concat(chunks).toString('utf8');
      if (failure) reject(failure);
      else if (code !== 0) reject(new Error(`${command} exited ${code}\n${output}`));
      else resolve(output);
    });
  });
}

export function testCount(output) {
  const results = [...output.matchAll(/^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/gm)];
  const result = results[0];
  if ([...output.matchAll(/^test result:/gm)].length !== 1 || results.length !== 1 || !Number.isSafeInteger(Number(result[1])) || Number(result[1]) === 0 || result.slice(2).some((value) => Number(value) !== 0)) {
    throw new Error('Expected executed tests with no failures, skips or filters');
  }
  return Number(result[1]);
}

async function sourceIdentity(directory) {
  const files = [];
  async function walk(current) {
    for (const item of await fs.readdir(current, { withFileTypes: true })) {
      const name = path.join(current, item.name);
      if (item.isSymbolicLink()) throw new Error(`Symlink not admitted: ${name}`);
      if (item.isDirectory()) await walk(name);
      else if (item.name.endsWith('.rs') || item.name === 'Cargo.toml') {
        files.push(name);
        if (files.length > 256) throw new Error('Source packet exceeds 256 files');
      }
    }
  }
  await walk(directory);
  const hash = createHash('sha256');
  let bytes = 0;
  for (const file of files.sort()) {
    const stat = await fs.stat(file);
    bytes += stat.size;
    if (bytes > 8 * limit) throw new Error('Source packet exceeds 8 MiB');
    hash.update(path.relative(directory, file).replaceAll('\\', '/'));
    hash.update('\0');
    hash.update(await fs.readFile(file));
    hash.update('\0');
  }
  return hash.digest('hex');
}

export async function withScratch(parent, action) {
  await fs.mkdir(parent, { recursive: true });
  const resolvedParent = await fs.realpath(parent);
  const scratch = await fs.mkdtemp(path.join(resolvedParent, 'contracts-'));
  await fs.writeFile(path.join(scratch, marker), 'inference-contract-tests-v1');
  try { return await action(scratch); }
  finally {
    const resolved = await fs.realpath(scratch);
    if (path.dirname(resolved) !== resolvedParent || await fs.readFile(path.join(resolved, marker), 'utf8') !== 'inference-contract-tests-v1') {
      throw new Error('Scratch ownership check failed; cleanup refused');
    }
    await fs.rm(resolved, { recursive: true });
  }
}

export async function main(etalonPath) {
  if (!etalonPath) throw new Error('Usage: node scripts/test-inference-migration.mjs <Etalon checkout>');
  const etalon = await fs.realpath(etalonPath);
  const rustc = process.env.RUSTC || 'rustc';
  const report = { scope: 'source contract tests only; no GPU, full LLM or Webizen integration qualification', toolchain: (await runBounded(rustc, ['--version'])).trim(), tests: [] };
  await withScratch(path.join(root, 'target', 'inference-migration-tests'), async (scratch) => {
    for (const name of ['etalon-qualification', 'etalon-inference-contract']) {
      const directory = path.join(etalon, 'crates', name);
      const before = await sourceIdentity(directory);
      for (const optimized of [false, true]) {
        const crate = name.replaceAll('-', '_');
        const binary = path.join(scratch, `${crate}-${optimized ? 'release' : 'debug'}${process.platform === 'win32' ? '.exe' : ''}`);
        const args = ['--edition=2021', '-Dwarnings', '--test', path.join(directory, 'src', 'lib.rs'), '--crate-name', crate,
          '-C', `opt-level=${optimized ? 3 : 0}`, '-C', `debug-assertions=${optimized ? 'no' : 'yes'}`, '-o', binary];
        await runBounded(rustc, args, { cwd: etalon });
        const output = await runBounded(binary, [], { cwd: etalon });
        report.tests.push({ crate: name, profile: optimized ? 'optimized' : 'debug', passed: testCount(output), source_sha256: before });
        const integrationDir = path.join(directory, 'tests');
        const entries = await fs.readdir(integrationDir).catch((error) => { if (error.code === 'ENOENT') return []; throw error; });
        if (entries.some((entry) => entry.endsWith('.rs'))) {
          const rlib = path.join(scratch, `lib${crate}.rlib`);
          await runBounded(rustc, ['--edition=2021', '-Dwarnings', '--crate-type=rlib', path.join(directory, 'src', 'lib.rs'), '--crate-name', crate, '-C', `opt-level=${optimized ? 3 : 0}`, '-o', rlib], { cwd: etalon });
          for (const entry of entries.filter((entry) => entry.endsWith('.rs')).sort()) {
            await runBounded(rustc, ['--edition=2021', '-Dwarnings', '--test', path.join(integrationDir, entry), '--extern', `${crate}=${rlib}`, '-C', `opt-level=${optimized ? 3 : 0}`, '-C', `debug-assertions=${optimized ? 'no' : 'yes'}`, '-o', binary], { cwd: etalon });
            const independent = await runBounded(binary, [], { cwd: etalon });
            report.tests.push({ crate: name, suite: entry, profile: optimized ? 'optimized' : 'debug', passed: testCount(independent), source_sha256: before });
          }
        }
      }
      if (await sourceIdentity(directory) !== before) throw new Error('Source changed during tests; evidence invalidated');
    }
  });
  return report;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv[2]).then((report) => console.log(JSON.stringify(report, null, 2))).catch((error) => { console.error(error.message); process.exitCode = 1; });
}
