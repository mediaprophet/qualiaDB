#!/usr/bin/env node
/**
 * Device-storage policy fixtures against wasm-webcivics (+ pure JS adapter smoke).
 * Usage: node docs/tests/webcivics-device-storage.test.mjs
 */
import { createHash } from 'node:crypto';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(here, '..', 'pkg', 'webcivics');
const wasmPath = join(pkgDir, 'qualia_webcivics_bg.wasm');
const jsPath = join(pkgDir, 'qualia.js');

function fail(msg) {
  console.error(`FAIL: ${msg}`);
  process.exitCode = 1;
}
function ok(msg) {
  console.log(`ok  ${msg}`);
}

async function main() {
  if (!existsSync(wasmPath)) {
    fail(`missing ${wasmPath}`);
    return;
  }
  const bytes = readFileSync(wasmPath);
  const mod = await import(pathToFileURL(jsPath).href);
  if (typeof mod.initSync === 'function') mod.initSync({ module: bytes });
  else await mod.default(bytes);

  if (typeof mod.device_storage_policy_wasm !== 'function') {
    fail('device_storage_policy_wasm missing — rebuild wasm-webcivics');
    return;
  }

  const policy = mod.device_storage_policy_wasm();
  if (policy.primary?.opfsDirectory !== 'webcivics') {
    fail(`opfs dir ${policy.primary?.opfsDirectory}`);
  } else ok('policy.primary.opfsDirectory=webcivics');
  if (policy.backup?.suggestedSubdir !== 'webcivics-backups') {
    fail(`subdir ${policy.backup?.suggestedSubdir}`);
  } else ok('policy.backup.suggestedSubdir=webcivics-backups');
  if (!policy.recovery?.primaryWipeDoesNotEraseBackup) {
    fail('recovery flag missing');
  } else ok('backup survives primary wipe');

  const plan = mod.plan_device_storage_wasm({
    quotaBytes: 50_000_000,
    usageBytes: 1_000,
    backupFolderLinked: false,
    lastBackupUnix: null,
    nowUnix: Math.floor(Date.now() / 1000),
    staleAfterSecs: 60,
  });
  if (!plan.recommendLinkBackupFolder) fail('expected recommendLinkBackupFolder');
  else ok('plan recommends linking backup folder');

  const hash = createHash('sha256').update('payload').digest('hex');
  const good = mod.verify_backup_manifest_wasm(
    {
      schema: 'webcivics.vault-backup.v1',
      createdUnix: 1,
      contentSha256: hash,
      byteLength: 7,
      primaryKind: 'opfs',
      notes: null,
    },
    hash,
  );
  if (!good.ok) fail(`verify good: ${JSON.stringify(good)}`);
  else ok('verify_backup_manifest accepts match');

  const bad = mod.verify_backup_manifest_wasm(
    {
      schema: 'webcivics.vault-backup.v1',
      createdUnix: 1,
      contentSha256: hash,
      byteLength: 7,
      primaryKind: 'opfs',
      notes: null,
    },
    '0'.repeat(64),
  );
  if (bad.ok) fail('verify should reject mismatch');
  else ok('verify rejects hash mismatch');

  const caps = mod.list_capabilities_wasm?.() ?? [];
  const list = Array.isArray(caps) ? caps : caps.capabilities ?? [];
  for (const need of ['device-storage-policy', 'opfs-primary-vault', 'backup-folder-link']) {
    if (!list.includes(need)) fail(`cap missing ${need}`);
    else ok(`capability ${need}`);
  }

  // Adapter module loads in Node (no DOM APIs exercised).
  const adapter = await import(pathToFileURL(join(here, '..', 'js', 'webcivics-device-storage.js')).href);
  if (adapter.OPFS_DIR !== 'webcivics') fail('adapter OPFS_DIR');
  else ok('host adapter OPFS_DIR');
  if (adapter.BACKUP_SUBDIR !== 'webcivics-backups') fail('adapter BACKUP_SUBDIR');
  else ok('host adapter BACKUP_SUBDIR');

  if (process.exitCode) console.error('webcivics device-storage fixtures FAILED');
  else console.log('webcivics device-storage fixtures PASSED');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
