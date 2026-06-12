//! Browser + Mobile WASM storage layer.
//!
//! Exposes two categories of WASM-bindgen functions:
//!
//! **Pure-Rust packing / validation** (no web API; usable in any WASM context):
//! - `pack_quins_into_superblock` — pack NQuin hashes into a 40 960-byte SuperBlock
//!   with correct ECC parity. The JS ingest worker should call this instead of
//!   reimplementing the layout; that prevents silent format drift.
//! - `verify_superblock_ecc` — validate every quin's ECC in a raw block.
//!
//! **Async OPFS storage** (requires `Window`/`Worker` context):
//! - `estimate_browser_storage` — StorageManager.estimate() wrapped for JS callers.
//! - `write_opfs_block` — write a SuperBlock to the OPFS vault by index.
//! - `read_opfs_block` — read a cached SuperBlock; returns `null` on cache-miss.
//!
//! ## Browser vs Mobile PWA
//!
//! Both targets use the same OPFS API (`navigator.storage.getDirectory()`).
//! The mobile PWA (`android_pwa_edge` feature on `qualia-mobile-harness`) adds:
//! - `profile_minimal_512` to keep the footprint under 512 MB RAM.
//! - ServiceWorker caching (JS layer) for offline dataset access.
//! - `FileSystemSyncAccessHandle` in a dedicated Web Worker for synchronous
//!   block I/O without blocking the main thread (iOS 15.2+ / Chrome 102+).
//!
//! `FileSystemSyncAccessHandle` is NOT used here — all writes go through the
//! async `createWritable()` path which works on both main thread and workers.

#![cfg(target_arch = "wasm32")]

use crate::{NQuin, BLOCK_MULTIPLIER_SIZE, QUINS_PER_BLOCK};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    window, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream,
};

/// Byte size of the QualiaSuperBlock header (matches the `#[repr(C)]` layout in lib.rs).
const HEADER_SIZE: usize = 160; // 8+8+8+4+4+128

// ─── SuperBlock packing ───────────────────────────────────────────────────────

/// Pack raw NQuin field bytes into a fully-structured SuperBlock with correct ECC parity.
///
/// `raw_quin_bytes` must be `N × 48` bytes where each 48-byte chunk contains the
/// five semantic `u64` fields (40 bytes) followed by 8 placeholder bytes (ignored —
/// ECC is computed here). `N` must not exceed `QUINS_PER_BLOCK` (850).
///
/// Returns exactly `BLOCK_MULTIPLIER_SIZE` (40 960) bytes, ready to write to OPFS.
/// This is the canonical packing path — **the JS ingest worker must call this**
/// instead of reimplementing the SuperBlock layout in JavaScript.
#[wasm_bindgen]
pub fn pack_quins_into_superblock(
    seq_id: u64,
    owner_did: u64,
    raw_quin_bytes: &[u8],
) -> Result<Uint8Array, JsValue> {
    if raw_quin_bytes.len() % 48 != 0 {
        return Err(JsValue::from_str("raw_quin_bytes length must be a multiple of 48"));
    }
    let n_quins = raw_quin_bytes.len() / 48;
    if n_quins > QUINS_PER_BLOCK {
        return Err(JsValue::from_str(&format!(
            "too many quins: {} > {} (QUINS_PER_BLOCK)",
            n_quins, QUINS_PER_BLOCK
        )));
    }

    let mut block = [0u8; BLOCK_MULTIPLIER_SIZE];

    // ── Header (little-endian, matches QualiaSuperBlock repr(C) layout) ──────
    block[0..8].copy_from_slice(&seq_id.to_le_bytes());   // block_sequence_id
    block[8..16].copy_from_slice(&owner_did.to_le_bytes()); // storage_owner_did
    block[16..24].copy_from_slice(&(n_quins as u64).to_le_bytes()); // active_quin_count
    // [24..28] validation_checksum  = 0
    // [28..32] hardware_profile_flags = 0
    // [32..160] layout_padding        = 0 (already zeroed)

    // ── Quin ledger — ECC parity computed per-quin ────────────────────────────
    let ledger = &mut block[HEADER_SIZE..];
    for i in 0..n_quins {
        let src = &raw_quin_bytes[i * 48..];
        let dst = &mut ledger[i * 48..];

        // Copy the 5 semantic fields (40 bytes)
        dst[..40].copy_from_slice(&src[..40]);

        // Compute XOR parity over the five u64 fields and write into bytes [40..48]
        let s = u64::from_le_bytes(src[0..8].try_into().unwrap());
        let p = u64::from_le_bytes(src[8..16].try_into().unwrap());
        let o = u64::from_le_bytes(src[16..24].try_into().unwrap());
        let c = u64::from_le_bytes(src[24..32].try_into().unwrap());
        let m = u64::from_le_bytes(src[32..40].try_into().unwrap());
        let parity = NQuin::calculate_parity(s, p, o, c, m);
        dst[40..48].copy_from_slice(&parity.to_le_bytes());
    }

    Ok(Uint8Array::from(block.as_ref()))
}

/// Validate ECC parity for every NQuin in a raw SuperBlock.
///
/// Returns JSON: `{"valid":bool,"total":N,"bad":[indices...]}`
/// A non-empty `bad` array indicates sector corruption.
#[wasm_bindgen]
pub fn verify_superblock_ecc(block_bytes: &[u8]) -> String {
    if block_bytes.len() < BLOCK_MULTIPLIER_SIZE {
        return r#"{"valid":false,"total":0,"bad":[],"error":"block too small"}"#.to_string();
    }

    let count_bytes: [u8; 8] = block_bytes[16..24].try_into().unwrap_or([0u8; 8]);
    let n_quins = (u64::from_le_bytes(count_bytes) as usize).min(QUINS_PER_BLOCK);

    let ledger = &block_bytes[HEADER_SIZE..];
    let mut bad: Vec<usize> = Vec::new();

    for i in 0..n_quins {
        let off = i * 48;
        let q = &ledger[off..off + 48];
        let s = u64::from_le_bytes(q[0..8].try_into().unwrap());
        let p = u64::from_le_bytes(q[8..16].try_into().unwrap());
        let o = u64::from_le_bytes(q[16..24].try_into().unwrap());
        let c = u64::from_le_bytes(q[24..32].try_into().unwrap());
        let m = u64::from_le_bytes(q[32..40].try_into().unwrap());
        let actual = u64::from_le_bytes(q[40..48].try_into().unwrap());
        if NQuin::calculate_parity(s, p, o, c, m) != actual {
            bad.push(i);
        }
    }

    let valid = bad.is_empty();
    let bad_str = bad.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    format!(r#"{{"valid":{valid},"total":{n_quins},"bad":[{bad_str}]}}"#)
}

// ─── Async OPFS storage ───────────────────────────────────────────────────────

/// Query the browser's storage quota and current OPFS usage (bytes).
///
/// Returns `{ quota: number, usage: number, available: number }`.
/// On mobile PWA the quota is typically 60 % of free disk space (Chrome) or
/// up to 1 GB on iOS Safari. Call this before a large ingest to check headroom.
#[wasm_bindgen]
pub async fn estimate_browser_storage() -> Result<JsValue, JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("no window context"))?;
    let sm = win.navigator().storage();
    let estimate_promise = sm
        .estimate()
        .map_err(|_| JsValue::from_str("StorageManager.estimate() unavailable — requires HTTPS or localhost"))?;
    let se: web_sys::StorageEstimate = JsFuture::from(estimate_promise).await?.dyn_into()?;

    let quota = js_sys::Reflect::get(&se, &JsValue::from_str("quota"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let usage = js_sys::Reflect::get(&se, &JsValue::from_str("usage"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from_str("quota"), &JsValue::from_f64(quota))?;
    js_sys::Reflect::set(&obj, &JsValue::from_str("usage"), &JsValue::from_f64(usage))?;
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("available"),
        &JsValue::from_f64(quota - usage),
    )?;
    Ok(obj.into())
}

/// Write a SuperBlock to the OPFS vault at `block_index`.
///
/// `block_bytes` must be exactly `BLOCK_MULTIPLIER_SIZE` (40 960) bytes — use
/// `pack_quins_into_superblock()` to produce correctly-structured blocks.
///
/// File name: `block_XXXXXXXX.qblk` (zero-padded 8-digit decimal index).
/// Compatible with the naming convention used by the JS VFS class.
#[wasm_bindgen]
pub async fn write_opfs_block(block_index: u32, block_bytes: &[u8]) -> Result<(), JsValue> {
    if block_bytes.len() != BLOCK_MULTIPLIER_SIZE {
        return Err(JsValue::from_str(&format!(
            "block must be exactly {} bytes, got {}",
            BLOCK_MULTIPLIER_SIZE,
            block_bytes.len()
        )));
    }

    let root = opfs_root().await?;
    let file_name = block_file_name(block_index);
    let fh = get_or_create_file_handle(&root, &file_name).await?;

    let writable: FileSystemWritableFileStream =
        JsFuture::from(fh.create_writable()).await?.dyn_into()?;

    // write_with_u8_array takes &mut [u8] — copy is unavoidable here
    let mut buf = block_bytes.to_vec();
    JsFuture::from(
        writable
            .write_with_u8_array(&mut buf)
            .map_err(|e| e)?,
    )
    .await?;
    JsFuture::from(writable.close()).await?;

    Ok(())
}

/// Read a cached SuperBlock from the OPFS vault.
///
/// Returns the raw 40 960 bytes as `Uint8Array`, or `null` if the block has not
/// been written yet (cache miss). Callers should fall back to an HTTP Range
/// request (see the JS `VFS` class) on cache miss.
#[wasm_bindgen]
pub async fn read_opfs_block(block_index: u32) -> Result<JsValue, JsValue> {
    let root = opfs_root().await?;
    let file_name = block_file_name(block_index);

    // Return null on cache miss without propagating the NotFoundError
    let fh: FileSystemFileHandle =
        match JsFuture::from(root.get_file_handle(&file_name)).await {
            Ok(v) => v.dyn_into()?,
            Err(_) => return Ok(JsValue::NULL),
        };

    let file: web_sys::File = JsFuture::from(fh.get_file()).await?.dyn_into()?;
    // array_buffer() is on Blob (File extends Blob); the cast lets us call it.
    let blob: web_sys::Blob = file.dyn_into()?;
    let ab = JsFuture::from(blob.array_buffer()).await?;
    Ok(Uint8Array::new(&ab).into())
}

/// Check whether a SuperBlock is cached in the OPFS vault.
/// Returns `true` if the `.qblk` file exists, `false` otherwise.
#[wasm_bindgen]
pub async fn is_opfs_block_cached(block_index: u32) -> Result<bool, JsValue> {
    let root = opfs_root().await?;
    let file_name = block_file_name(block_index);
    Ok(JsFuture::from(root.get_file_handle(&file_name))
        .await
        .is_ok())
}

// ─── Private helpers ──────────────────────────────────────────────────────────

async fn opfs_root() -> Result<FileSystemDirectoryHandle, JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("no window context"))?;
    JsFuture::from(win.navigator().storage().get_directory())
        .await?
        .dyn_into()
}

async fn get_or_create_file_handle(
    dir: &FileSystemDirectoryHandle,
    name: &str,
) -> Result<FileSystemFileHandle, JsValue> {
    let mut opts = FileSystemGetFileOptions::new();
    opts.create(true);
    JsFuture::from(dir.get_file_handle_with_options(name, &opts))
        .await?
        .dyn_into()
}

fn block_file_name(index: u32) -> String {
    format!("block_{:08}.qblk", index)
}
