/**
 * ICP sovereign vault — OPFS pairing cache, IndexedDB handles, standpoint promotion.
 */

import { PAIRING_STORAGE_KEY, parsePairingPayload, pairingToJson } from './qualia-icp-session.js';

export const STANDPOINT_SPECTATOR = 0;
export const STANDPOINT_EPHEMERAL = 1;
export const STANDPOINT_IDENTIFIER = 2;
export const STANDPOINT_VAULT = 3;

export const PAIRING_OPFS_PATH = 'pairing.v1.json';
export const VAULT_MANIFEST_PATH = 'vault-manifest.v1.json';
export const IDB_NAME = 'qualia-icp-v1';
export const IDB_STORE = 'meta';
export const IDB_KEY_VAULT_DIR = 'vault_dir_handle';

const ICP_DIR = 'icp';

function opfsSupported() {
    return typeof navigator !== 'undefined'
        && navigator.storage
        && typeof navigator.storage.getDirectory === 'function';
}

async function getIcpDir(create = true) {
    if (!opfsSupported()) return null;
    const root = await navigator.storage.getDirectory();
    return root.getDirectoryHandle(ICP_DIR, { create });
}

async function writeOpfsText(name, text) {
    const dir = await getIcpDir(true);
    if (!dir) throw new Error('opfs_unavailable');
    const fh = await dir.getFileHandle(name, { create: true });
    const writable = await fh.createWritable();
    await writable.write(text);
    await writable.close();
}

async function readOpfsText(name) {
    const dir = await getIcpDir(false);
    if (!dir) return null;
    try {
        const fh = await dir.getFileHandle(name);
        const file = await fh.getFile();
        return await file.text();
    } catch {
        return null;
    }
}

async function deleteOpfsFile(name) {
    const dir = await getIcpDir(false);
    if (!dir) return;
    try {
        await dir.removeEntry(name);
    } catch {
        // already gone
    }
}

function openIdb() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open(IDB_NAME, 1);
        req.onupgradeneeded = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains(IDB_STORE)) {
                db.createObjectStore(IDB_STORE);
            }
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

export function deviceDidFromHash(hex) {
    return `did:icp:device:${hex}`;
}

export function randomDeviceHash() {
    const buf = new Uint8Array(16);
    crypto.getRandomValues(buf);
    return Array.from(buf, (b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Persist pairing to OPFS (durable across session clears).
 * @param {object} payload
 */
export async function savePairingOpfs(payload) {
    if (!payload) return false;
    const record = {
        ...payload,
        saved_at: Math.floor(Date.now() / 1000),
    };
    await writeOpfsText(PAIRING_OPFS_PATH, JSON.stringify(record));
    return true;
}

/**
 * @returns {object|null}
 */
export async function loadPairingOpfs() {
    const raw = await readOpfsText(PAIRING_OPFS_PATH);
    if (!raw) return null;
    return parsePairingPayload(raw);
}

export async function clearPairingOpfs() {
    await deleteOpfsFile(PAIRING_OPFS_PATH);
}

/**
 * Merge OPFS pairing into session/local storage if newer or missing.
 */
export async function hydratePairingFromOpfs() {
    const opfs = await loadPairingOpfs();
    if (!opfs) return null;
    const mem = parsePairingPayload(
        sessionStorage.getItem(PAIRING_STORAGE_KEY)
        || localStorage.getItem(PAIRING_STORAGE_KEY),
    );
    if (!mem || (opfs.saved_at && (!mem.saved_at || opfs.saved_at >= mem.saved_at))) {
        const json = pairingToJson(opfs);
        try { sessionStorage.setItem(PAIRING_STORAGE_KEY, json); } catch { /* */ }
        try { localStorage.setItem(PAIRING_STORAGE_KEY, json); } catch { /* */ }
        return opfs;
    }
    return mem;
}

/**
 * @returns {object|null}
 */
export async function loadVaultManifest() {
    const raw = await readOpfsText(VAULT_MANIFEST_PATH);
    if (!raw) return null;
    try {
        return JSON.parse(raw);
    } catch {
        return null;
    }
}

async function saveVaultManifest(manifest) {
    await writeOpfsText(VAULT_MANIFEST_PATH, JSON.stringify(manifest));
}

/**
 * @param {object} [opts]
 * @param {object} [opts.portal] QualiaPortal
 * @param {object} [opts.wasmMod] qualia wasm module
 * @param {boolean} [opts.promoteVault] class 3 after class 2
 */
export async function initVault(opts = {}) {
    const { portal, wasmMod, promoteVault = false } = opts;
    const existing = await loadVaultManifest();
    const deviceHash = existing?.device_hash || randomDeviceHash();
    const identifierDid = deviceDidFromHash(deviceHash);

    let quota = null;
    if (wasmMod?.estimate_browser_storage) {
        try {
            quota = await wasmMod.estimate_browser_storage();
        } catch {
            // ignore
        }
    }

    const standpointClass = promoteVault || existing?.standpoint_class === STANDPOINT_VAULT
        ? STANDPOINT_VAULT
        : STANDPOINT_IDENTIFIER;

    const manifest = {
        v: 1,
        created_at: existing?.created_at || Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
        device_hash: deviceHash,
        identifier_did: identifierDid,
        standpoint_class: standpointClass,
        opfs: true,
        folder_linked: existing?.folder_linked || false,
        block_count: existing?.block_count || 0,
        quota_snapshot: quota,
    };

    await saveVaultManifest(manifest);

    if (portal?.set_standpoint) {
        portal.set_standpoint(standpointClass, 1.0, 0.5, 0.08, identifierDid);
    }

    return manifest;
}

/**
 * @returns {object}
 */
export async function getVaultStatus() {
    const manifest = await loadVaultManifest();
    const pairing = await loadPairingOpfs();
    let quota = null;
    if (opfsSupported()) {
        try {
            const est = await navigator.storage.estimate();
            quota = {
                quota: est.quota ?? 0,
                usage: est.usage ?? 0,
                available: (est.quota ?? 0) - (est.usage ?? 0),
            };
        } catch {
            // ignore
        }
    }
    return {
        ready: !!manifest,
        manifest,
        pairing_cached: !!pairing,
        quota,
        opfs_supported: opfsSupported(),
        folder_picker_supported: typeof window !== 'undefined' && 'showDirectoryPicker' in window,
    };
}

/**
 * Optional sovereign folder — persists handle in IndexedDB.
 */
export async function pickVaultFolder() {
    if (!('showDirectoryPicker' in window)) {
        throw new Error('directory_picker_unavailable');
    }
    const handle = await window.showDirectoryPicker({ mode: 'readwrite' });
    await idbSet(IDB_KEY_VAULT_DIR, handle);
    const manifest = await loadVaultManifest();
    if (manifest) {
        manifest.folder_linked = true;
        manifest.updated_at = Math.floor(Date.now() / 1000);
        await saveVaultManifest(manifest);
    }
    return handle.name;
}

/**
 * @returns {FileSystemDirectoryHandle|null}
 */
export async function loadVaultFolderHandle() {
    return idbGet(IDB_KEY_VAULT_DIR);
}

export async function clearVault() {
    await deleteOpfsFile(VAULT_MANIFEST_PATH);
    await idbDelete(IDB_KEY_VAULT_DIR);
}