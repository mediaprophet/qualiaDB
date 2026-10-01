/**
 * Web Civics device storage — OPFS primary vault + remembered backup folder.
 *
 * Primary data lives under OPFS `webcivics/` (survives reloads; cleared with site data).
 * A user-picked directory (File System Access API) or host-provided path holds
 * dated backups so OPFS wipe is recoverable. IndexedDB stores the directory handle.
 *
 * Works in Chromium / Android WebView PWAs. On hosts without `showDirectoryPicker`
 * (many iOS PWAs), `exportBackupDownload()` uses a blob download / share sheet.
 *
 * Pair with WASM: `device_storage_policy_wasm`, `plan_device_storage_wasm`,
 * `verify_backup_manifest_wasm` from the wasm-webcivics package.
 */

export const OPFS_DIR = 'webcivics';
export const VAULT_MANIFEST = 'vault-manifest.v1.json';
export const BACKUP_SUBDIR = 'webcivics-backups';
export const IDB_NAME = 'webcivics-device-storage-v1';
export const IDB_STORE = 'handles';
export const IDB_KEY_BACKUP = 'backup_dir_handle';
export const IDB_KEY_HOST_PATH = 'backup_host_path';

function opfsSupported() {
  return typeof navigator !== 'undefined'
    && navigator.storage
    && typeof navigator.storage.getDirectory === 'function';
}

export function folderPickerSupported() {
  return typeof window !== 'undefined' && typeof window.showDirectoryPicker === 'function';
}

async function getVaultDir(create = true) {
  if (!opfsSupported()) return null;
  const root = await navigator.storage.getDirectory();
  return root.getDirectoryHandle(OPFS_DIR, { create });
}

async function writeOpfsText(name, text) {
  const dir = await getVaultDir(true);
  if (!dir) throw new Error('opfs_unavailable');
  const fh = await dir.getFileHandle(name, { create: true });
  const w = await fh.createWritable();
  await w.write(text);
  await w.close();
}

async function readOpfsText(name) {
  const dir = await getVaultDir(false);
  if (!dir) return null;
  try {
    const fh = await dir.getFileHandle(name);
    return await (await fh.getFile()).text();
  } catch {
    return null;
  }
}

function openIdb() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(IDB_STORE)) db.createObjectStore(IDB_STORE);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function idbSet(key, value) {
  const db = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readwrite');
    tx.objectStore(IDB_STORE).put(value, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function idbGet(key) {
  const db = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readonly');
    const req = tx.objectStore(IDB_STORE).get(key);
    req.onsuccess = () => resolve(req.result ?? null);
    req.onerror = () => reject(req.error);
  });
}

async function idbDelete(key) {
  const db = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readwrite');
    tx.objectStore(IDB_STORE).delete(key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function sha256Hex(bytes) {
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Request durable storage (Chrome / Android: reduces eviction risk).
 */
export async function requestPersist() {
  if (!navigator?.storage?.persist) return { persisted: false, reason: 'unsupported' };
  const persisted = await navigator.storage.persist();
  return { persisted, reason: persisted ? 'granted' : 'denied' };
}

export async function estimateQuota() {
  if (!navigator?.storage?.estimate) {
    return { quota: 0, usage: 0, available: 0 };
  }
  const est = await navigator.storage.estimate();
  const quota = est.quota ?? 0;
  const usage = est.usage ?? 0;
  return { quota, usage, available: quota - usage };
}

/**
 * Save / load vault manifest (what packages are resident in OPFS).
 */
export async function saveVaultManifest(manifest) {
  const body = {
    schema: 'webcivics.vault-manifest.v1',
    updatedAt: Math.floor(Date.now() / 1000),
    ...manifest,
  };
  await writeOpfsText(VAULT_MANIFEST, JSON.stringify(body, null, 2));
  return body;
}

export async function loadVaultManifest() {
  const text = await readOpfsText(VAULT_MANIFEST);
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

/**
 * Write an arbitrary named artifact into the OPFS vault (JSON-LD, Turtle, package bytes).
 */
export async function putVaultFile(name, data) {
  const dir = await getVaultDir(true);
  if (!dir) throw new Error('opfs_unavailable');
  const fh = await dir.getFileHandle(name, { create: true });
  const w = await fh.createWritable();
  if (typeof data === 'string') await w.write(data);
  else await w.write(data);
  await w.close();
}

export async function getVaultFile(name) {
  const dir = await getVaultDir(false);
  if (!dir) return null;
  try {
    const fh = await dir.getFileHandle(name);
    return await fh.getFile();
  } catch {
    return null;
  }
}

export async function listVaultFiles() {
  const dir = await getVaultDir(false);
  if (!dir) return [];
  const names = [];
  for await (const [name] of dir.entries()) names.push(name);
  return names;
}

/**
 * Prompt the user to choose a backup folder and remember the handle.
 * @returns {{ name: string, linked: true }}
 */
export async function pickBackupFolder() {
  if (!folderPickerSupported()) {
    throw new Error('directory_picker_unavailable');
  }
  const handle = await window.showDirectoryPicker({
    mode: 'readwrite',
    id: 'webcivics-backup',
    startIn: 'documents',
  });
  // Ensure suggested subdir exists
  let backupRoot = handle;
  try {
    backupRoot = await handle.getDirectoryHandle(BACKUP_SUBDIR, { create: true });
  } catch {
    backupRoot = handle;
  }
  await idbSet(IDB_KEY_BACKUP, backupRoot);
  const manifest = (await loadVaultManifest()) || {};
  manifest.backupFolderLinked = true;
  manifest.backupFolderName = backupRoot.name || BACKUP_SUBDIR;
  manifest.updatedAt = Math.floor(Date.now() / 1000);
  await saveVaultManifest(manifest);
  return { name: manifest.backupFolderName, linked: true };
}

/** Native / Flutter hosts may set an absolute path string instead of a picker handle. */
export async function setHostBackupPath(path) {
  if (!path || typeof path !== 'string') throw new Error('invalid_host_path');
  await idbSet(IDB_KEY_HOST_PATH, path);
  const manifest = (await loadVaultManifest()) || {};
  manifest.backupFolderLinked = true;
  manifest.backupHostPath = path;
  manifest.updatedAt = Math.floor(Date.now() / 1000);
  await saveVaultManifest(manifest);
  return { path, linked: true };
}

export async function loadBackupFolderHandle() {
  return idbGet(IDB_KEY_BACKUP);
}

export async function loadHostBackupPath() {
  return idbGet(IDB_KEY_HOST_PATH);
}

export async function isBackupFolderLinked() {
  const h = await loadBackupFolderHandle();
  if (h) return true;
  const p = await loadHostBackupPath();
  return typeof p === 'string' && p.length > 0;
}

/**
 * Export current vault files as a single JSON backup package into the linked folder.
 * Falls back to returning the blob for download when no folder is linked.
 */
export async function exportBackup({ label } = {}) {
  const names = await listVaultFiles();
  const files = {};
  for (const name of names) {
    const file = await getVaultFile(name);
    if (!file) continue;
    files[name] = await file.text();
  }
  const createdUnix = Math.floor(Date.now() / 1000);
  const payload = {
    schema: 'webcivics.vault-backup.v1',
    createdUnix,
    label: label || `backup-${createdUnix}`,
    files,
  };
  const bytes = new TextEncoder().encode(JSON.stringify(payload));
  const contentSha256 = await sha256Hex(bytes);
  const manifest = {
    schema: 'webcivics.vault-backup.v1',
    createdUnix,
    contentSha256,
    byteLength: bytes.byteLength,
    primaryKind: 'opfs',
    notes: label || null,
  };
  const packageJson = JSON.stringify({ manifest, payload }, null, 2);
  const packageBytes = new TextEncoder().encode(packageJson);

  const handle = await loadBackupFolderHandle();
  if (handle) {
    const fname = `webcivics-backup-${createdUnix}.json`;
    const fh = await handle.getFileHandle(fname, { create: true });
    const w = await fh.createWritable();
    await w.write(packageBytes);
    await w.close();
    const vm = (await loadVaultManifest()) || {};
    vm.lastBackupUnix = createdUnix;
    vm.lastBackupFile = fname;
    await saveVaultManifest(vm);
    return { written: fname, manifest, via: 'directory' };
  }

  return {
    written: null,
    manifest,
    via: 'blob',
    blob: new Blob([packageBytes], { type: 'application/vnd.web-civics.vault-backup+json' }),
    fileName: `webcivics-backup-${createdUnix}.json`,
  };
}

/** Trigger a browser download when the directory picker is unavailable. */
export async function exportBackupDownload(opts) {
  const result = await exportBackup(opts);
  if (result.via === 'directory') return result;
  const url = URL.createObjectURL(result.blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = result.fileName;
  a.click();
  URL.revokeObjectURL(url);
  return { ...result, via: 'download' };
}

/**
 * Restore vault files from a backup package object or File/Blob.
 * Optionally verify with WASM `verify_backup_manifest_wasm`.
 */
export async function restoreBackup(source, { verifyWasm } = {}) {
  let text;
  if (typeof source === 'string') text = source;
  else if (source instanceof Blob) text = await source.text();
  else if (source && typeof source === 'object' && source.manifest && source.payload) {
    text = JSON.stringify(source);
  } else throw new Error('unsupported_backup_source');

  const pkg = JSON.parse(text);
  const { manifest, payload } = pkg;
  if (!manifest || !payload?.files) throw new Error('malformed_backup');

  const payloadBytes = new TextEncoder().encode(JSON.stringify(payload));
  const hash = await sha256Hex(payloadBytes);
  if (verifyWasm) {
    const v = verifyWasm(manifest, hash);
    if (v && v.ok === false) {
      throw new Error(`backup_verify_failed:${(v.errors || []).join(',')}`);
    }
  } else if (hash.toLowerCase() !== String(manifest.contentSha256 || '').toLowerCase()) {
    throw new Error('backup_verify_failed:content_hash_mismatch');
  }

  for (const [name, content] of Object.entries(payload.files)) {
    await putVaultFile(name, content);
  }
  const vm = (await loadVaultManifest()) || {};
  vm.restoredFromBackupUnix = Math.floor(Date.now() / 1000);
  vm.restoredContentSha256 = hash;
  await saveVaultManifest(vm);
  return { restored: Object.keys(payload.files).length, contentSha256: hash };
}

/**
 * Status snapshot for UI / Civics adapters.
 * Pass optional `planWasm` = plan_device_storage_wasm from the package.
 */
export async function deviceStorageStatus({ planWasm } = {}) {
  const quota = await estimateQuota();
  const manifest = await loadVaultManifest();
  const linked = await isBackupFolderLinked();
  const files = await listVaultFiles();
  const base = {
    opfsSupported: opfsSupported(),
    folderPickerSupported: folderPickerSupported(),
    backupFolderLinked: linked,
    hostBackupPath: await loadHostBackupPath(),
    vaultFileCount: files.length,
    vaultFiles: files,
    manifest,
    quota,
  };
  if (typeof planWasm === 'function') {
    base.plan = planWasm({
      quotaBytes: quota.quota,
      usageBytes: quota.usage,
      backupFolderLinked: linked,
      lastBackupUnix: manifest?.lastBackupUnix ?? null,
      nowUnix: Math.floor(Date.now() / 1000),
      staleAfterSecs: 7 * 24 * 3600,
    });
  }
  return base;
}

export async function clearVault(unlinkBackup = false) {
  const dir = await getVaultDir(false);
  if (dir) {
    for await (const [name] of dir.entries()) {
      await dir.removeEntry(name);
    }
  }
  if (unlinkBackup) {
    await idbDelete(IDB_KEY_BACKUP);
    await idbDelete(IDB_KEY_HOST_PATH);
  }
}
