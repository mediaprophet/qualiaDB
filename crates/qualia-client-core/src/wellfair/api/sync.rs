//! Sync-operation protocol + transport


use super::super::journal::JournalEntry;
use super::super::sync_outbox::{SyncOutbox, SyncOutboxEntry, SyncOutboxState};
use super::super::sync_transport::SyncTransport;
use super::super::sync_protocol::{AdmitOutcome, InboxRecord, SyncInbox, SyncOperation};
use ed25519_dalek::Signer;


use super::*;

impl WebizenHostApi {
    // --- Phase 5 sync-operation protocol (SyncService, Â§4.2 / Â§9.5 / Â§17) ---

    /// Build a signed outbound sync operation from a committed journal entry.
    /// Returns `None` for Classified/Sanctuary records â€” they never enter the ordinary sync
    /// lane (Â§5.2). The signature is a real ed25519 signature over the operation's bound payload.
    pub fn build_outbound_operation(
        &self,
        entry: &JournalEntry,
        lamport: u64,
    ) -> Option<SyncOperation> {
        if entry.sensitivity == "Classified" {
            return None;
        }
        let op = SyncOperation::new(
            uuid::Uuid::new_v4().to_string(),
            entry.id.clone(),
            entry.kind.clone(),
            self.author_did.clone(),
            entry.sensitivity.clone(),
            entry.summary.clone().unwrap_or_default(),
            lamport,
            entry.committed_unix,
        );
        let signature = self.signing_key.sign(&op.signing_payload());
        Some(op.with_signature(hex::encode(signature.to_bytes())))
    }

    /// Admit an inbound sync operation into the durable quarantined inbox. Idempotent: a
    /// replayed operation id is recorded as `Duplicate` and never applied twice.
    pub fn admit_sync_operation(&self, op: &SyncOperation) -> Result<AdmitOutcome, String> {
        let inbox = SyncInbox::open(&self.storage_root).map_err(|e| e.to_string())?;
        inbox
            .admit(op, Self::now_unix() as u32)
            .map_err(|e| e.to_string())
    }

    /// Validated operations currently held in the inbox, in Lamport order.
    pub fn validated_sync_operations(&self) -> Result<Vec<SyncOperation>, String> {
        SyncInbox::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .validated_operations()
            .map_err(|e| e.to_string())
    }

    pub fn list_sync_inbox(&self, limit: usize) -> Result<Vec<InboxRecord>, String> {
        SyncInbox::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .list_recent(limit)
            .map_err(|e| e.to_string())
    }

    // --- Sync transport orchestration (T3.1: drain outbox â†’ transport â†’ peer inbox) ---

    /// The next Lamport value to stamp on outbound operations: one past the greatest observed among
    /// the locally-validated inbox operations. Keeps outbound clocks causally ahead of what we've seen.
    fn next_sync_lamport(&self) -> Result<u64, String> {
        let max = self
            .validated_sync_operations()?
            .iter()
            .map(|o| o.lamport)
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }

    /// **Drain the outbox through a transport.** For each `Queued` outbox entry, build a signed
    /// [`SyncOperation`] from its committed journal entry and publish it; on success the entry is
    /// marked `Sent`. Classified/Sanctuary records never enter the ordinary lane â€” they are marked
    /// `Rejected` so they stop being retried. Returns the number of operations published.
    ///
    /// The transport is a dumb pipe; correctness (dedup, convergence) is enforced by the peer's
    /// fail-closed inbox on the other side.
    pub fn sync_push_via<T: SyncTransport>(
        &self,
        transport: &T,
        limit: usize,
    ) -> Result<usize, String> {
        let outbox = SyncOutbox::open(&self.storage_root).map_err(|e| e.to_string())?;
        let queued: Vec<SyncOutboxEntry> = outbox
            .list_all()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|e| e.state == SyncOutboxState::Queued)
            .take(limit)
            .collect();
        if queued.is_empty() {
            return Ok(0);
        }
        let journal = self.list_health_records(512)?;
        let mut lamport = self.next_sync_lamport()?;
        let mut ops = Vec::new();
        let mut sent_ids = Vec::new();
        for entry in &queued {
            let Some(journal_entry) = journal.iter().find(|j| j.id == entry.record_id) else {
                continue; // the record is outside the recent window; leave it queued for a later drain
            };
            match self.build_outbound_operation(journal_entry, lamport) {
                Some(op) => {
                    lamport += 1;
                    ops.push(op);
                    sent_ids.push(entry.operation_id.clone());
                }
                None => {
                    // Classified/Sanctuary â€” never syncs; stop retrying it.
                    let _ = outbox.update_state(&entry.operation_id, SyncOutboxState::Rejected);
                }
            }
        }
        if ops.is_empty() {
            return Ok(0);
        }
        transport.publish(&ops)?;
        for id in &sent_ids {
            let _ = outbox.update_state(id, SyncOutboxState::Sent);
        }
        Ok(ops.len())
    }

    /// **Pull from a transport and admit into the quarantined inbox.** Every op is validated
    /// fail-closed on admission (bad signature/hash/version/oversize/Classified â†’ `Rejected`;
    /// replays â†’ `Duplicate`), so a hostile peer can only cause rejections. Returns the admission
    /// tally.
    pub fn sync_pull_via<T: SyncTransport>(
        &self,
        transport: &T,
        since: u64,
    ) -> Result<SyncPullReport, String> {
        let ops = transport.pull(since)?;
        let mut report = SyncPullReport {
            pulled: ops.len(),
            validated: 0,
            duplicate: 0,
            rejected: 0,
        };
        for op in &ops {
            match self.admit_sync_operation(op)? {
                AdmitOutcome::Validated => report.validated += 1,
                AdmitOutcome::Duplicate => report.duplicate += 1,
                AdmitOutcome::Rejected(_) => report.rejected += 1,
            }
        }
        Ok(report)
    }

    /// One-shot sync against an HTTP relay (the production wire): drain the outbox to the relay,
    /// then pull + admit from it. Returns `(pushed, pull_report)`. Native-only (`reqwest`).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sync_with_http_relay(
        &self,
        base_url: &str,
        since: u64,
    ) -> Result<(usize, SyncPullReport), String> {
        let transport = super::super::sync_transport::HttpRelayTransport::new(base_url);
        let pushed = self.sync_push_via(&transport, 256)?;
        let report = self.sync_pull_via(&transport, since)?;
        Ok((pushed, report))
    }

    /// One-shot sync against a **libp2p** peer/relay (noise-encrypted request-response â€” the peer-to-peer
    /// wire): drain the outbox to the peer, then pull + admit from it. `peer_id` is the base58 peer id,
    /// `peer_addr` a libp2p multiaddr (e.g. `/ip4/1.2.3.4/tcp/4001`). Returns `(pushed, pull_report)`.
    /// Native-only (libp2p). Same dumb-pipe contract as [`Self::sync_with_http_relay`]: correctness is
    /// enforced by the fail-closed inbox, not the transport.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sync_with_libp2p_peer(
        &self,
        peer_id: &str,
        peer_addr: &str,
        since: u64,
    ) -> Result<(usize, SyncPullReport), String> {
        let transport = super::super::sync_transport::Libp2pSyncTransport::connect(peer_id, peer_addr)?;
        let pushed = self.sync_push_via(&transport, 256)?;
        let report = self.sync_pull_via(&transport, since)?;
        Ok((pushed, report))
    }

}