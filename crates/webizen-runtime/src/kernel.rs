use crate::clock::FixedStepClock;
use crate::diffusion::{DiffusionConfig, DiffusionField};
use crate::error::RuntimeError;
use crate::snapshot::{SharedFrameBuffer, SimulationSnapshot};
use crossbeam_channel::{Receiver, Sender, TrySendError};
use std::time::Duration;

pub trait ComputeBackend {
    fn step(&mut self, epoch: u64) -> Result<SimulationSnapshot, RuntimeError>;

    fn reconfigure(&mut self, _config: DiffusionConfig) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn shared_frames(&self) -> Option<SharedFrameBuffer> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct LedgerRecord {
    pub epoch: u64,
    pub dimensions: (u32, u32),
    pub state_hash: crate::snapshot::StateHash,
}

impl LedgerRecord {
    pub fn from_snapshot(snapshot: &SimulationSnapshot) -> Self {
        Self {
            epoch: snapshot.epoch,
            dimensions: snapshot.dimensions,
            state_hash: snapshot.state_hash,
        }
    }
}

pub trait LedgerSink: Send + Sync {
    fn record(&self, record: LedgerRecord);
}

#[derive(Debug, Clone)]
pub struct ChannelLedgerSink {
    sender: Sender<LedgerRecord>,
}

impl ChannelLedgerSink {
    pub fn bounded(capacity: usize) -> (Self, Receiver<LedgerRecord>) {
        let (sender, receiver) = crossbeam_channel::bounded(capacity);
        (Self { sender }, receiver)
    }
}

impl LedgerSink for ChannelLedgerSink {
    fn record(&self, record: LedgerRecord) {
        match self.sender.try_send(record) {
            Ok(_) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeCommand {
    Reconfigure(DiffusionConfig),
}

#[derive(Default)]
pub struct NullLedgerSink;

impl LedgerSink for NullLedgerSink {
    fn record(&self, _record: LedgerRecord) {}
}

pub struct SimulationKernel<B, L>
where
    B: ComputeBackend,
    L: LedgerSink,
{
    clock: FixedStepClock,
    field: DiffusionField,
    backend: B,
    ledger: L,
    latest_snapshot: Option<SimulationSnapshot>,
    command_rx: Receiver<RuntimeCommand>,
    snapshot_tx: Sender<SimulationSnapshot>,
}

impl<B, L> SimulationKernel<B, L>
where
    B: ComputeBackend,
    L: LedgerSink,
{
    pub fn new(
        timestep: Duration,
        config: DiffusionConfig,
        backend: B,
        ledger: L,
    ) -> (Self, Receiver<SimulationSnapshot>, Sender<RuntimeCommand>) {
        let (command_tx, command_rx) = crossbeam_channel::bounded(16);
        let (snapshot_tx, snapshot_rx) = crossbeam_channel::bounded(1);
        let kernel = Self {
            clock: FixedStepClock::new(timestep),
            field: DiffusionField::new((config.width, config.height)),
            backend,
            ledger,
            latest_snapshot: None,
            command_rx,
            snapshot_tx,
        };
        (kernel, snapshot_rx, command_tx)
    }

    pub fn advance_elapsed(&mut self, elapsed: Duration) -> Result<usize, RuntimeError> {
        self.drain_control_plane()?;
        let ticks = self.clock.push_elapsed(elapsed);
        for _ in 0..ticks {
            self.tick_once()?;
        }
        Ok(ticks)
    }

    pub fn latest_snapshot(&self) -> Option<&SimulationSnapshot> {
        self.latest_snapshot.as_ref()
    }

    pub fn field(&self) -> DiffusionField {
        self.field
    }

    pub fn shared_frames(&self) -> Option<SharedFrameBuffer> {
        self.backend.shared_frames()
    }

    fn tick_once(&mut self) -> Result<(), RuntimeError> {
        let next_epoch = self.field.epoch + 1;
        let snapshot = self.backend.step(next_epoch)?;
        self.field.epoch = snapshot.epoch;
        self.ledger.record(LedgerRecord::from_snapshot(&snapshot));
        let _ = self.snapshot_tx.try_send(snapshot.clone());
        self.latest_snapshot = Some(snapshot);
        Ok(())
    }

    fn drain_control_plane(&mut self) -> Result<(), RuntimeError> {
        while let Ok(command) = self.command_rx.try_recv() {
            match command {
                RuntimeCommand::Reconfigure(config) => {
                    self.backend.reconfigure(config)?;
                    self.field = DiffusionField::new((config.width, config.height));
                    self.latest_snapshot = None;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ComputeBackend, NullLedgerSink, SimulationKernel};
    use crate::diffusion::DiffusionConfig;
    use crate::snapshot::{FrameHandle, SimulationSnapshot};
    use crate::RuntimeError;
    use std::time::Duration;

    struct MockBackend;

    impl ComputeBackend for MockBackend {
        fn step(&mut self, epoch: u64) -> Result<SimulationSnapshot, RuntimeError> {
            Ok(SimulationSnapshot {
                epoch,
                dimensions: (4, 4),
                state_hash: [epoch as u8; 32],
                frame: FrameHandle::CpuRgbaSlot((epoch as u8) & 1),
            })
        }
    }

    #[test]
    fn kernel_only_steps_on_complete_ticks() {
        let (mut kernel, _, _) = SimulationKernel::new(
            Duration::from_millis(16),
            DiffusionConfig {
                width: 4,
                height: 4,
                diffusion_rate: 0.2,
            },
            MockBackend,
            NullLedgerSink::default(),
        );

        assert_eq!(kernel.advance_elapsed(Duration::from_millis(8)).unwrap(), 0);
        assert!(kernel.latest_snapshot().is_none());

        assert_eq!(
            kernel.advance_elapsed(Duration::from_millis(24)).unwrap(),
            2
        );
        assert_eq!(kernel.field().epoch, 2);
        assert_eq!(kernel.latest_snapshot().unwrap().epoch, 2);
    }
}
