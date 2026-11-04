import { createHash } from 'node:crypto';

const repository = 'https://github.com/mediaprophet/etalon.git';
const fullRevision = /^[0-9a-f]{40}$/;
const digest = (text) => createHash('sha256').update(text).digest('hex');

// Cargo.toml is the reviewed declaration authority; the standalone accelerator
// harness revision is independent of these incrementally migrated dependencies.
export function etalonDependencyIdentity(cargo, lock) {
  const declarations = [];
  let section = '';
  for (const raw of cargo.split(/\r?\n/)) {
    const line = raw.replace(/\s+#.*$/, '').trim();
    if (line.startsWith('[')) section = line;
    if (line.startsWith('#')) continue;
    if (/^\[(?:workspace\.dependencies|patch\.[^\]]+)\]$/.test(section)
      && /^(?:etalon(?:-[\w-]+)?|wgpu|naga)\s*=/.test(line)
      && !line.includes(repository)) {
      throw new Error('Production Etalon dependencies require reviewed git identities; local overrides require --etalon-path');
    }
    if (!line.includes(repository)) continue;
    const name = /^([\w-]+)\s*=\s*\{/.exec(line)?.[1];
    const revision = /\brev\s*=\s*"([^"]+)"/.exec(line)?.[1];
    if (!name || !fullRevision.test(revision ?? '') || /\bpath\s*=/.test(line)) {
      throw new Error('Etalon dependencies must declare full reviewed git revisions without path overrides');
    }
    const packageName = /\bpackage\s*=\s*"([^"]+)"/.exec(line)?.[1] ?? name;
    declarations.push({ section, name, package: packageName, revision });
  }
  if (!declarations.some(({ name }) => name === 'wgpu') || !declarations.some(({ name }) => name === 'etalon')) {
    throw new Error('Missing production Etalon dependency declarations; local overrides require --etalon-path');
  }
  const packages = [];
  for (const block of lock.split(/^\[\[package\]\]\s*$/m).slice(1)) {
    const source = /^source = "([^"]+)"/m.exec(block)?.[1];
    if (!source?.includes(repository)) continue;
    const name = /^name = "([^"]+)"/m.exec(block)?.[1];
    const version = /^version = "([^"]+)"/m.exec(block)?.[1];
    const match = /^git\+https:\/\/github\.com\/mediaprophet\/etalon\.git\?rev=([0-9a-f]{40})#([0-9a-f]{40})$/.exec(source);
    if (!name || !version || !match || match[1] !== match[2]) throw new Error('Invalid or unresolved Etalon lockfile identity');
    packages.push({ name, version, revision: match[1] });
  }
  for (const declaration of declarations) {
    if (!packages.some(({ name, revision }) => name === declaration.package && revision === declaration.revision)) {
      throw new Error(`Etalon lockfile does not resolve ${declaration.name} at its declared revision`);
    }
  }
  const declaredRevisions = new Set(declarations.map(({ revision }) => revision));
  if (packages.some(({ revision }) => !declaredRevisions.has(revision))) throw new Error('Undeclared Etalon lockfile revision');
  return { cargo_toml_sha256: digest(cargo), cargo_lock_sha256: digest(lock), declarations, packages };
}

export function validateDependencyIdentity(record, current) {
  if (JSON.stringify(record) !== JSON.stringify(current)) {
    throw new Error('Source record does not match current Etalon dependency declarations and lockfile');
  }
}
