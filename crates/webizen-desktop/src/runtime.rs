use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use qualia_client_core::state::AppState;

use qualia_core_db::{q_hash, NQuin};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use webizen_runtime::{
    DiffusionConfig, FrameHandle, LedgerRecord, LedgerSink, RuntimeCommand, SharedFrameBuffer,
    SimulationKernel, WgpuDiffusionBackend,
};

const DEFAULT_TIMESTEP: Duration = Duration::from_millis(16);
const DEFAULT_LOOP_SLEEP: Duration = Duration::from_millis(1);
const DEFAULT_DIFFUSION_CONFIG: DiffusionConfig = DiffusionConfig {
    width: 128,
    height: 128,
    diffusion_rate: 0.18,
};
const LEDGER_CHANNEL_CAPACITY: usize = 1024;
const LEDGER_EVENT_NAME: &str = "diffusion-ledger-health";

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSnapshotRecord {
    pub epoch: u64,
    pub dimensions: (u32, u32),
    pub frame_slot: u8,
    pub state_hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeLedgerHealth {
    pub persisted_epoch: u64,
    pub dropped_events: u64,
    pub gap_events: u64,
    pub recovery_events: u64,
    pub write_failures: u64,
    pub last_gap_from_epoch: Option<u64>,
    pub last_gap_to_epoch: Option<u64>,
    pub degraded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LedgerHealthFingerprint {
    dropped_events: u64,
    gap_events: u64,
    recovery_events: u64,
    write_failures: u64,
    last_gap_from_epoch: u64,
    last_gap_to_epoch: u64,
}

enum RuntimeSignal {
    RefreshConfig,
}

#[derive(Default)]
struct LedgerMetrics {
    persisted_epoch: AtomicU64,
    dropped_events: AtomicU64,
    gap_events: AtomicU64,
    recovery_events: AtomicU64,
    write_failures: AtomicU64,
    last_gap_from_epoch: AtomicU64,
    last_gap_to_epoch: AtomicU64,
}

impl LedgerMetrics {
    fn snapshot(&self) -> RuntimeLedgerHealth {
        let last_gap_from_epoch = self.last_gap_from_epoch.load(Ordering::Relaxed);
        let last_gap_to_epoch = self.last_gap_to_epoch.load(Ordering::Relaxed);
        let dropped_events = self.dropped_events.load(Ordering::Relaxed);
        let gap_events = self.gap_events.load(Ordering::Relaxed);
        let recovery_events = self.recovery_events.load(Ordering::Relaxed);
        let write_failures = self.write_failures.load(Ordering::Relaxed);

        RuntimeLedgerHealth {
            persisted_epoch: self.persisted_epoch.load(Ordering::Relaxed),
            dropped_events,
            gap_events,
            recovery_events,
            write_failures,
            last_gap_from_epoch: (last_gap_from_epoch != 0).then_some(last_gap_from_epoch),
            last_gap_to_epoch: (last_gap_to_epoch != 0).then_some(last_gap_to_epoch),
            degraded: dropped_events > 0 || gap_events > 0 || write_failures > 0,
        }
    }

    fn fingerprint(&self) -> LedgerHealthFingerprint {
        LedgerHealthFingerprint {
            dropped_events: self.dropped_events.load(Ordering::Relaxed),
            gap_events: self.gap_events.load(Ordering::Relaxed),
            recovery_events: self.recovery_events.load(Ordering::Relaxed),
            write_failures: self.write_failures.load(Ordering::Relaxed),
            last_gap_from_epoch: self.last_gap_from_epoch.load(Ordering::Relaxed),
            last_gap_to_epoch: self.last_gap_to_epoch.load(Ordering::Relaxed),
        }
    }

    fn note_drop(&self) {
        self.dropped_events.fetch_add(1, Ordering::Relaxed);
    }

    fn note_gap(&self, previous_epoch: u64, next_epoch: u64) {
        self.gap_events.fetch_add(1, Ordering::Relaxed);
        self.recovery_events.fetch_add(1, Ordering::Relaxed);
        self.last_gap_from_epoch
            .store(previous_epoch, Ordering::Relaxed);
        self.last_gap_to_epoch.store(next_epoch, Ordering::Relaxed);
    }

    fn note_persisted(&self, epoch: u64) {
        self.persisted_epoch.store(epoch, Ordering::Relaxed);
    }

    fn note_write_failure(&self) {
        self.write_failures.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone)]
struct DesktopLedgerSink {
    sender: Sender<LedgerRecord>,
    metrics: Arc<LedgerMetrics>,
}

impl DesktopLedgerSink {
    fn bounded(capacity: usize, metrics: Arc<LedgerMetrics>) -> (Self, Receiver<LedgerRecord>) {
        let (sender, receiver) = bounded(capacity);
        (Self { sender, metrics }, receiver)
    }
}

impl LedgerSink for DesktopLedgerSink {
    fn record(&self, record: LedgerRecord) {
        match self.sender.try_send(record) {
            Ok(_) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.metrics.note_drop();
            }
        }
    }
}

pub struct RuntimeHandle {
    latest_snapshot: Arc<Mutex<Option<RuntimeSnapshotRecord>>>,
    pending_config: Arc<Mutex<Option<DiffusionConfig>>>,
    frames: SharedFrameBuffer,
    ledger_metrics: Arc<LedgerMetrics>,
    signal_tx: Sender<RuntimeSignal>,
}

impl RuntimeHandle {
    pub fn latest_snapshot(&self) -> Option<RuntimeSnapshotRecord> {
        self.latest_snapshot.lock().ok()?.clone()
    }

    pub fn queue_reconfigure(&self, config: DiffusionConfig) -> Result<(), String> {
        let mut pending = self
            .pending_config
            .lock()
            .map_err(|_| "runtime config lock poisoned".to_string())?;
        *pending = Some(config);
        drop(pending);

        match self.signal_tx.try_send(RuntimeSignal::RefreshConfig) {
            Ok(_) | Err(TrySendError::Full(_)) => Ok(()),
            Err(TrySendError::Disconnected(_)) => Err("runtime thread is offline".to_string()),
        }
    }

    pub fn frame_rgba(&self, slot: u8) -> Option<Vec<u8>> {
        self.frames.with_slot(slot, |bytes| bytes.to_vec())
    }

    pub fn ledger_health(&self) -> RuntimeLedgerHealth {
        self.ledger_metrics.snapshot()
    }
}

pub fn spawn_runtime(
    app_handle: AppHandle,
    app_state: Arc<AppState>,
) -> Result<RuntimeHandle, String> {
    let backend = pollster::block_on(WgpuDiffusionBackend::new(DEFAULT_DIFFUSION_CONFIG))
        .map_err(|err| err.to_string())?;
    let ledger_metrics = Arc::new(LedgerMetrics::default());
    let (ledger_sink, ledger_rx) =
        DesktopLedgerSink::bounded(LEDGER_CHANNEL_CAPACITY, ledger_metrics.clone());
    let (kernel, _snapshot_rx, kernel_control_tx) = SimulationKernel::new(
        DEFAULT_TIMESTEP,
        DEFAULT_DIFFUSION_CONFIG,
        backend,
        ledger_sink,
    );
    let frames = kernel
        .shared_frames()
        .ok_or_else(|| "runtime backend did not expose shared frame storage".to_string())?;
    let latest_snapshot = Arc::new(Mutex::new(None));
    let pending_config = Arc::new(Mutex::new(None));
    let (signal_tx, signal_rx) = bounded::<RuntimeSignal>(1);
    let ledger_storage_path = {
        let config = app_state
            .config
            .lock()
            .map_err(|_| "app state config lock poisoned".to_string())?;
        PathBuf::from(config.storage_path.clone())
    };

    let latest_snapshot_thread = latest_snapshot.clone();
    let pending_config_thread = pending_config.clone();
    let app_handle_thread = app_handle.clone();
    let ledger_metrics_thread = ledger_metrics.clone();

    spawn_persistence_worker(
        app_handle.clone(),
        ledger_storage_path,
        ledger_rx,
        ledger_metrics_thread.clone(),
    )
    .map_err(|err| err.to_string())?;

    thread::Builder::new()
        .name("webizen-runtime-kernel".to_string())
        .spawn(move || {
            let mut kernel = kernel;
            let mut last_frame = Instant::now();
            let mut last_emitted_epoch = 0u64;
            let mut last_ledger_fingerprint = ledger_metrics_thread.fingerprint();

            loop {
                while signal_rx.try_recv().is_ok() {}

                let next_config = pending_config_thread
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.take());
                if let Some(config) = next_config {
                    let _ = kernel_control_tx.send(RuntimeCommand::Reconfigure(config));
                }

                let now = Instant::now();
                let elapsed = now.saturating_duration_since(last_frame);
                last_frame = now;

                if kernel.advance_elapsed(elapsed).is_ok() {
                    if let Some(snapshot) = kernel.latest_snapshot() {
                        if snapshot.epoch != last_emitted_epoch {
                            last_emitted_epoch = snapshot.epoch;
                            let FrameHandle::CpuRgbaSlot(frame_slot) = snapshot.frame;
                            let record = RuntimeSnapshotRecord {
                                epoch: snapshot.epoch,
                                dimensions: snapshot.dimensions,
                                frame_slot,
                                state_hash: snapshot.state_hash,
                            };

                            if let Ok(mut latest) = latest_snapshot_thread.lock() {
                                *latest = Some(record.clone());
                            }

                            let _ = app_handle_thread.emit_all("diffusion-epoch-ready", record);
                        }
                    }
                }

                let next_fingerprint = ledger_metrics_thread.fingerprint();
                if next_fingerprint != last_ledger_fingerprint {
                    last_ledger_fingerprint = next_fingerprint;
                    let _ = app_handle_thread
                        .emit_all(LEDGER_EVENT_NAME, ledger_metrics_thread.snapshot());
                }

                thread::sleep(DEFAULT_LOOP_SLEEP);
            }
        })
        .map_err(|err| err.to_string())?;

    Ok(RuntimeHandle {
        latest_snapshot,
        pending_config,
        frames,
        ledger_metrics,
        signal_tx,
    })
}

fn spawn_persistence_worker(
    app_handle: AppHandle,
    storage_path: PathBuf,
    ledger_rx: Receiver<LedgerRecord>,
    metrics: Arc<LedgerMetrics>,
) -> std::io::Result<()> {
    thread::Builder::new()
        .name("webizen-runtime-ledger".to_string())
        .spawn(move || {
            let volume_path = diffusion_wal_path(&storage_path);
            if let Some(parent) = volume_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let mut appender =
                match qualia_core_db::q42_volume::StreamingVolumeAppender::new(&volume_path) {
                    Ok(a) => a,
                    Err(err) => {
                        metrics.note_write_failure();
                        let _ = emit_ledger_health(&app_handle, &metrics);
                        eprintln!(
                            "failed to init streaming appender for {}: {}",
                            volume_path.display(),
                            err
                        );
                        return;
                    }
                };

            use qualia_core_db::{NQuin, QUINS_PER_BLOCK};

            let mut block_buffer = Vec::with_capacity(QUINS_PER_BLOCK);
            let mut last_persisted_epoch = 0u64;

            let mut flush_block = |buf: &mut Vec<NQuin>, epoch: u64| {
                if buf.is_empty() {
                    return;
                }
                if let Err(e) = appender.append_block(epoch, buf) {
                    eprintln!("Failed to append block to Q42: {}", e);
                }
                buf.clear();
            };

            while let Ok(record) = ledger_rx.recv() {
                if last_persisted_epoch != 0 && record.epoch > last_persisted_epoch + 1 {
                    metrics.note_gap(last_persisted_epoch, record.epoch);

                    let mut marker = NQuin {
                        subject: q_hash("urn:webizen:runtime:diffusion"),
                        predicate: q_hash("q42:ledgerBaselineReset"),
                        object: record.epoch,
                        context: last_persisted_epoch,
                        metadata: gap_metadata(last_persisted_epoch, record.epoch),
                        parity: 0,
                    };
                    marker.recalculate_parity();
                    block_buffer.push(marker);
                    if block_buffer.len() == QUINS_PER_BLOCK {
                        flush_block(&mut block_buffer, record.epoch);
                    }

                    let _ = emit_ledger_health(&app_handle, &metrics);
                }

                let mut header_quin = NQuin {
                    subject: q_hash("urn:webizen:runtime:diffusion"),
                    predicate: q_hash("q42:simulationSnapshot"),
                    object: record.epoch,
                    context: pack_dimensions(record.dimensions),
                    metadata: 0,
                    parity: 0,
                };
                header_quin.set_lamport_clock(record.epoch as u32);
                header_quin.recalculate_parity();
                block_buffer.push(header_quin);
                if block_buffer.len() == QUINS_PER_BLOCK {
                    flush_block(&mut block_buffer, record.epoch);
                }

                let mut hash_lo = NQuin {
                    subject: q_hash("urn:webizen:runtime:diffusion"),
                    predicate: q_hash("q42:stateHashLo"),
                    object: hash_chunk(&record.state_hash, 0),
                    context: hash_chunk(&record.state_hash, 1),
                    metadata: hash_chunk_metadata(record.epoch, 0),
                    parity: 0,
                };
                hash_lo.recalculate_parity();
                block_buffer.push(hash_lo);
                if block_buffer.len() == QUINS_PER_BLOCK {
                    flush_block(&mut block_buffer, record.epoch);
                }

                let mut hash_hi = NQuin {
                    subject: q_hash("urn:webizen:runtime:diffusion"),
                    predicate: q_hash("q42:stateHashHi"),
                    object: hash_chunk(&record.state_hash, 2),
                    context: hash_chunk(&record.state_hash, 3),
                    metadata: hash_chunk_metadata(record.epoch, 1),
                    parity: 0,
                };
                hash_hi.recalculate_parity();
                block_buffer.push(hash_hi);
                if block_buffer.len() == QUINS_PER_BLOCK {
                    flush_block(&mut block_buffer, record.epoch);
                }

                last_persisted_epoch = record.epoch;
                metrics.note_persisted(record.epoch);
            }

            flush_block(&mut block_buffer, last_persisted_epoch);
        })
        .map(|_| ())
}

fn emit_ledger_health(app_handle: &AppHandle, metrics: &LedgerMetrics) -> Result<(), tauri::Error> {
    app_handle.emit_all(LEDGER_EVENT_NAME, metrics.snapshot())
}

fn diffusion_wal_path(storage_path: &Path) -> PathBuf {
    storage_path.join("runtime").join("diffusion-session.q42")
}

fn pack_dimensions(dimensions: (u32, u32)) -> u64 {
    ((dimensions.0 as u64) << 32) | dimensions.1 as u64
}

fn hash_chunk(hash: &[u8; 32], index: usize) -> u64 {
    let start = index * 8;
    let end = start + 8;
    u64::from_le_bytes(hash[start..end].try_into().expect("state hash chunk"))
}

fn hash_chunk_metadata(epoch: u64, chunk_index: u8) -> u64 {
    ((chunk_index as u64) << 32) | (epoch & 0xFFFF_FFFF)
}

fn gap_metadata(previous_epoch: u64, next_epoch: u64) -> u64 {
    let gap = next_epoch.saturating_sub(previous_epoch + 1);
    (gap.min(u32::MAX as u64) << 32) | (next_epoch & 0xFFFF_FFFF)
}
