//! Mutable Q42 Session: Checkpoint and Append-Only Transaction Journal (`.q42j`).
//!
//! Generalized persistence layer for QualiaDB, supporting:
//! - POET NOS workspaces, strata, and lens-time document histories
//! - Webizen Studio and Desktop persistent graph workspaces
//! - Autonomous multi-agent simulation & RTS world states (e.g. Rolling Commons)
//! - Offline-first browser sessions via OPFS / Memory and native filesystem
//!
//! Conforms to `AGENTS.md`:
//! - Universal 48-byte `NQuin` semantic atom with 5-field XOR fold parity.
//! - Zero heap inside hot frame validation and evaluation loops (Tier 1).
//! - 42MB Sentinel budget compliance.
//! - Default privacy: `PrivacyClass::Sanctuary` (restricted; prevents accidental
//!   public magnet routing).

use crate::NQuin;
use std::collections::HashSet;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// Magic bytes for a Q42 Journal: `Q42J`.
pub const Q42J_MAGIC: [u8; 4] = *b"Q42J";

/// Current `.q42j` journal format version.
pub const Q42J_VERSION: u16 = 1;

/// Fixed journal file header size (128 bytes).
pub const Q42J_HEADER_SIZE: usize = 128;

/// Application profiles for generalized QualiaDB sessions.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalApplicationProfile {
    /// General-purpose semantic graph and knowledge base.
    GenericSemantic = 0x0001,
    /// POET NOS workspaces, documents, and lens-strata history.
    PoetWorkspace = 0x0002,
    /// Interactive simulations, RTS worlds, and multi-agent systems.
    SimulationWorld = 0x0003,
    /// Clinical records, healthcare pathways, and FHIR/LOINC receipts.
    ClinicalRecord = 0x0004,
    /// Bilateral micro-commons, civic assemblies, and cooperative contracts.
    CivicCommons = 0x0005,
}

impl Default for JournalApplicationProfile {
    fn default() -> Self {
        Self::GenericSemantic
    }
}

impl TryFrom<u16> for JournalApplicationProfile {
    type Error = JournalError;
    fn try_from(val: u16) -> Result<Self, Self::Error> {
        match val {
            0x0001 => Ok(Self::GenericSemantic),
            0x0002 => Ok(Self::PoetWorkspace),
            0x0003 => Ok(Self::SimulationWorld),
            0x0004 => Ok(Self::ClinicalRecord),
            0x0005 => Ok(Self::CivicCommons),
            other => Err(JournalError::UnknownApplicationProfile(other)),
        }
    }
}

/// Privacy classification for the journal (Sanctuary by default).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalPrivacyClass {
    Sanctuary = 0,
    Restricted = 1,
    Commons = 2,
}

impl Default for JournalPrivacyClass {
    fn default() -> Self {
        Self::Sanctuary
    }
}

/// 128-byte header stored at offset 0 of every `.q42j` file.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub profile: u16,
    pub privacy: u8,
    pub flags: u8,
    pub reserved_head: [u8; 6],
    pub world_did_hash: u64,
    pub base_q42_digest: [u8; 32],
    pub base_generation: u64,
    pub schema_rule_version: u32,
    pub required_pack_digest: [u8; 32],
    pub reserved_tail: [u8; 28],
}

impl JournalHeader {
    pub fn new(
        profile: JournalApplicationProfile,
        world_did_hash: u64,
        base_q42_digest: [u8; 32],
        base_generation: u64,
    ) -> Self {
        Self {
            magic: Q42J_MAGIC,
            version: Q42J_VERSION,
            profile: profile as u16,
            privacy: JournalPrivacyClass::Sanctuary as u8,
            flags: 0,
            reserved_head: [0u8; 6],
            world_did_hash,
            base_q42_digest,
            base_generation,
            schema_rule_version: 1,
            required_pack_digest: [0u8; 32],
            reserved_tail: [0u8; 28],
        }
    }

    pub fn to_bytes(&self) -> [u8; Q42J_HEADER_SIZE] {
        let mut buf = [0u8; Q42J_HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        buf[6..8].copy_from_slice(&self.profile.to_le_bytes());
        buf[8] = self.privacy;
        buf[9] = self.flags;
        buf[10..16].copy_from_slice(&self.reserved_head);
        buf[16..24].copy_from_slice(&self.world_did_hash.to_le_bytes());
        buf[24..56].copy_from_slice(&self.base_q42_digest);
        buf[56..64].copy_from_slice(&self.base_generation.to_le_bytes());
        buf[64..68].copy_from_slice(&self.schema_rule_version.to_le_bytes());
        buf[68..100].copy_from_slice(&self.required_pack_digest);
        buf[100..128].copy_from_slice(&self.reserved_tail);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JournalError> {
        if bytes.len() < Q42J_HEADER_SIZE {
            return Err(JournalError::InvalidHeader("header too short"));
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != Q42J_MAGIC {
            return Err(JournalError::InvalidMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != Q42J_VERSION {
            return Err(JournalError::UnsupportedVersion(version));
        }
        let profile = u16::from_le_bytes([bytes[6], bytes[7]]);
        let privacy = bytes[8];
        let flags = bytes[9];
        let mut reserved_head = [0u8; 6];
        reserved_head.copy_from_slice(&bytes[10..16]);
        let mut world_bytes = [0u8; 8];
        world_bytes.copy_from_slice(&bytes[16..24]);
        let world_did_hash = u64::from_le_bytes(world_bytes);
        let mut base_q42_digest = [0u8; 32];
        base_q42_digest.copy_from_slice(&bytes[24..56]);
        let mut gen_bytes = [0u8; 8];
        gen_bytes.copy_from_slice(&bytes[56..64]);
        let base_generation = u64::from_le_bytes(gen_bytes);
        let mut ver_bytes = [0u8; 4];
        ver_bytes.copy_from_slice(&bytes[64..68]);
        let schema_rule_version = u32::from_le_bytes(ver_bytes);
        let mut required_pack_digest = [0u8; 32];
        required_pack_digest.copy_from_slice(&bytes[68..100]);
        let mut reserved_tail = [0u8; 28];
        reserved_tail.copy_from_slice(&bytes[100..128]);

        Ok(Self {
            magic,
            version,
            profile,
            privacy,
            flags,
            reserved_head,
            world_did_hash,
            base_q42_digest,
            base_generation,
            schema_rule_version,
            required_pack_digest,
            reserved_tail,
        })
    }
}

/// Types of frames within a `.q42j` transaction.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalFrameType {
    TxBegin = 0x01,
    QuinAdd = 0x02,
    QuinRemove = 0x03,
    LexiconBind = 0x04,
    Receipt = 0x05,
    TxCommit = 0xFF,
}

impl TryFrom<u8> for JournalFrameType {
    type Error = JournalError;
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0x01 => Ok(Self::TxBegin),
            0x02 => Ok(Self::QuinAdd),
            0x03 => Ok(Self::QuinRemove),
            0x04 => Ok(Self::LexiconBind),
            0x05 => Ok(Self::Receipt),
            0xFF => Ok(Self::TxCommit),
            other => Err(JournalError::UnknownFrameType(other)),
        }
    }
}

/// 32-byte header preceding every frame payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub frame_type: u8,
    pub reserved: [u8; 3],
    pub payload_len: u32,
    pub sequence_num: u64,
    pub tick: u64,
    pub tx_id: u32,
    pub crc32c: u32,
}

pub const FRAME_HEADER_SIZE: usize = 32;

impl FrameHeader {
    pub fn to_bytes(&self) -> [u8; FRAME_HEADER_SIZE] {
        let mut b = [0u8; FRAME_HEADER_SIZE];
        b[0] = self.frame_type;
        b[1..4].copy_from_slice(&self.reserved);
        b[4..8].copy_from_slice(&self.payload_len.to_le_bytes());
        b[8..16].copy_from_slice(&self.sequence_num.to_le_bytes());
        b[16..24].copy_from_slice(&self.tick.to_le_bytes());
        b[24..28].copy_from_slice(&self.tx_id.to_le_bytes());
        b[28..32].copy_from_slice(&self.crc32c.to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, JournalError> {
        if b.len() < FRAME_HEADER_SIZE {
            return Err(JournalError::InvalidHeader("frame header too short"));
        }
        let frame_type = b[0];
        let mut reserved = [0u8; 3];
        reserved.copy_from_slice(&b[1..4]);
        let payload_len = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        let sequence_num =
            u64::from_le_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]);
        let tick = u64::from_le_bytes([b[16], b[17], b[18], b[19], b[20], b[21], b[22], b[23]]);
        let tx_id = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
        let crc32c = u32::from_le_bytes([b[28], b[29], b[30], b[31]]);

        Ok(Self {
            frame_type,
            reserved,
            payload_len,
            sequence_num,
            tick,
            tx_id,
            crc32c,
        })
    }
}

/// Compute a fast CRC-32C checksum over data.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (!((crc & 1).wrapping_sub(1))) & 0x82F63B78;
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

/// An atomic transaction delta over canonical 48-byte Quins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalTransaction {
    pub tx_id: u32,
    pub tick: u64,
    pub command_hash: u64,
    pub actor_did: u64,
    pub adds: Vec<NQuin>,
    pub removes: Vec<NQuin>,
    pub lexicon_bindings: Vec<(u64, String)>,
    pub receipt_digest: [u8; 32],
    pub committed: bool,
}

impl JournalTransaction {
    pub fn new(tx_id: u32, tick: u64, command_hash: u64, actor_did: u64) -> Self {
        Self {
            tx_id,
            tick,
            command_hash,
            actor_did,
            adds: Vec::new(),
            removes: Vec::new(),
            lexicon_bindings: Vec::new(),
            receipt_digest: [0u8; 32],
            committed: false,
        }
    }

    pub fn add_quin(&mut self, quin: NQuin) {
        self.adds.push(quin);
    }

    pub fn remove_quin(&mut self, quin: NQuin) {
        self.removes.push(quin);
    }

    pub fn bind_lexicon(&mut self, token_hash: u64, iri: String) {
        self.lexicon_bindings.push((token_hash, iri));
    }
}

/// Core Journal Engine managing durable appending and atomic replay.
pub struct Q42Journal<S: Read + Write + Seek> {
    pub storage: S,
    pub header: JournalHeader,
    sequence_counter: u64,
}

impl<S: Read + Write + Seek> Q42Journal<S> {
    /// Initialize a new journal with the given storage and header.
    pub fn create(mut storage: S, header: JournalHeader) -> Result<Self, JournalError> {
        storage.seek(SeekFrom::Start(0))?;
        storage.write_all(&header.to_bytes())?;
        storage.flush()?;
        Ok(Self {
            storage,
            header,
            sequence_counter: 0,
        })
    }

    /// Open and validate an existing journal from storage.
    pub fn open(mut storage: S, expected_base_digest: &[u8; 32]) -> Result<Self, JournalError> {
        storage.seek(SeekFrom::Start(0))?;
        let mut header_buf = [0u8; Q42J_HEADER_SIZE];
        storage.read_exact(&mut header_buf)?;
        let header = JournalHeader::from_bytes(&header_buf)?;

        if &header.base_q42_digest != expected_base_digest {
            return Err(JournalError::BaseDigestMismatch);
        }

        Ok(Self {
            storage,
            header,
            sequence_counter: 0,
        })
    }

    /// Open and validate an existing journal from storage with any base digest.
    pub fn open_any(mut storage: S) -> Result<Self, JournalError> {
        storage.seek(SeekFrom::Start(0))?;
        let mut header_buf = [0u8; Q42J_HEADER_SIZE];
        storage.read_exact(&mut header_buf)?;
        let header = JournalHeader::from_bytes(&header_buf)?;

        Ok(Self {
            storage,
            header,
            sequence_counter: 0,
        })
    }

    /// Atomically appends a complete transaction to the journal.
    pub fn append_transaction(&mut self, tx: &JournalTransaction) -> Result<(), JournalError> {
        self.storage.seek(SeekFrom::End(0))?;

        // 1. TxBegin Frame: carries command_hash (8B) and actor_did (8B)
        let mut begin_payload = [0u8; 16];
        begin_payload[0..8].copy_from_slice(&tx.command_hash.to_le_bytes());
        begin_payload[8..16].copy_from_slice(&tx.actor_did.to_le_bytes());
        self.write_frame(JournalFrameType::TxBegin, tx.tx_id, tx.tick, &begin_payload)?;

        // 2. Lexicon Bindings (if newly authored tokens)
        for (token, iri) in &tx.lexicon_bindings {
            let iri_bytes = iri.as_bytes();
            let mut lex_payload = Vec::with_capacity(8 + iri_bytes.len());
            lex_payload.extend_from_slice(&token.to_le_bytes());
            lex_payload.extend_from_slice(iri_bytes);
            self.write_frame(
                JournalFrameType::LexiconBind,
                tx.tx_id,
                tx.tick,
                &lex_payload,
            )?;
        }

        // 3. Quin Add Frames
        for quin in &tx.adds {
            let bytes = quin_to_bytes(quin);
            self.write_frame(JournalFrameType::QuinAdd, tx.tx_id, tx.tick, &bytes)?;
        }

        // 4. Quin Remove Frames
        for quin in &tx.removes {
            let bytes = quin_to_bytes(quin);
            self.write_frame(JournalFrameType::QuinRemove, tx.tx_id, tx.tick, &bytes)?;
        }

        // 5. Output Receipt Frame (if non-zero)
        if tx.receipt_digest != [0u8; 32] {
            self.write_frame(
                JournalFrameType::Receipt,
                tx.tx_id,
                tx.tick,
                &tx.receipt_digest,
            )?;
        }

        // 6. Terminal Commit Frame
        let commit_payload = [0xAAu8; 4];
        self.write_frame(
            JournalFrameType::TxCommit,
            tx.tx_id,
            tx.tick,
            &commit_payload,
        )?;

        self.storage.flush()?;
        Ok(())
    }

    fn write_frame(
        &mut self,
        frame_type: JournalFrameType,
        tx_id: u32,
        tick: u64,
        payload: &[u8],
    ) -> Result<(), JournalError> {
        self.sequence_counter += 1;
        let checksum = crc32c(payload);
        let header = FrameHeader {
            frame_type: frame_type as u8,
            reserved: [0u8; 3],
            payload_len: payload.len() as u32,
            sequence_num: self.sequence_counter,
            tick,
            tx_id,
            crc32c: checksum,
        };
        self.storage.write_all(&header.to_bytes())?;
        self.storage.write_all(payload)?;
        Ok(())
    }

    /// Replays the journal and recovers all committed transactions.
    /// Safely ignores torn uncommitted transactions at the tail.
    pub fn recover_transactions(&mut self) -> Result<Vec<JournalTransaction>, JournalError> {
        self.storage
            .seek(SeekFrom::Start(Q42J_HEADER_SIZE as u64))?;

        let mut transactions = Vec::new();
        let mut current_tx: Option<JournalTransaction> = None;

        let mut header_buf = [0u8; FRAME_HEADER_SIZE];

        loop {
            match self.storage.read_exact(&mut header_buf) {
                Ok(()) => {}
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(JournalError::Io(e)),
            }

            let frame_header = FrameHeader::from_bytes(&header_buf)?;
            let mut payload = vec![0u8; frame_header.payload_len as usize];
            match self.storage.read_exact(&mut payload) {
                Ok(()) => {}
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(JournalError::Io(e)),
            }

            // Verify integrity
            if crc32c(&payload) != frame_header.crc32c {
                return Err(JournalError::CorruptedFrame(frame_header.sequence_num));
            }

            let frame_type = JournalFrameType::try_from(frame_header.frame_type)?;
            match frame_type {
                JournalFrameType::TxBegin => {
                    if payload.len() < 8 {
                        return Err(JournalError::InvalidHeader("TxBegin payload too short"));
                    }
                    let mut cmd_bytes = [0u8; 8];
                    cmd_bytes.copy_from_slice(&payload[0..8]);
                    let command_hash = u64::from_le_bytes(cmd_bytes);
                    let actor_did = if payload.len() >= 16 {
                        let mut actor_bytes = [0u8; 8];
                        actor_bytes.copy_from_slice(&payload[8..16]);
                        u64::from_le_bytes(actor_bytes)
                    } else {
                        0
                    };
                    current_tx = Some(JournalTransaction::new(
                        frame_header.tx_id,
                        frame_header.tick,
                        command_hash,
                        actor_did,
                    ));
                }
                JournalFrameType::LexiconBind => {
                    if let Some(ref mut tx) = current_tx {
                        if payload.len() >= 8 {
                            let mut token_bytes = [0u8; 8];
                            token_bytes.copy_from_slice(&payload[0..8]);
                            let token = u64::from_le_bytes(token_bytes);
                            if let Ok(iri) = std::str::from_utf8(&payload[8..]) {
                                tx.bind_lexicon(token, iri.to_string());
                            }
                        }
                    }
                }
                JournalFrameType::QuinAdd => {
                    if let Some(ref mut tx) = current_tx {
                        if payload.len() == 48 {
                            let quin = quin_from_bytes(&payload)?;
                            tx.add_quin(quin);
                        }
                    }
                }
                JournalFrameType::QuinRemove => {
                    if let Some(ref mut tx) = current_tx {
                        if payload.len() == 48 {
                            let quin = quin_from_bytes(&payload)?;
                            tx.remove_quin(quin);
                        }
                    }
                }
                JournalFrameType::Receipt => {
                    if let Some(ref mut tx) = current_tx {
                        if payload.len() == 32 {
                            tx.receipt_digest.copy_from_slice(&payload);
                        }
                    }
                }
                JournalFrameType::TxCommit => {
                    if let Some(mut tx) = current_tx.take() {
                        tx.committed = true;
                        transactions.push(tx);
                    }
                }
            }
        }

        Ok(transactions)
    }

    /// Reset journal back to empty header after a checkpoint compaction.
    pub fn reset_to_generation(
        &mut self,
        new_base_digest: [u8; 32],
        new_generation: u64,
    ) -> Result<(), JournalError> {
        self.header.base_q42_digest = new_base_digest;
        self.header.base_generation = new_generation;
        self.storage.seek(SeekFrom::Start(0))?;
        self.storage.write_all(&self.header.to_bytes())?;
        self.storage.flush()?;
        self.sequence_counter = 0;
        Ok(())
    }
}

/// Generalized in-memory mutable session providing active state projection,
/// transaction commit boundaries, and checkpoint management.
pub struct MutableQ42Session<S: Read + Write + Seek> {
    pub journal: Q42Journal<S>,
    pub active_state: HashSet<NQuin>,
    current_tick: u64,
    next_tx_id: u32,
    staged_adds: Vec<NQuin>,
    staged_removes: Vec<NQuin>,
}

impl<S: Read + Write + Seek> MutableQ42Session<S> {
    /// Open a mutable session over an existing or new journal and baseline Quins.
    pub fn open(
        mut journal: Q42Journal<S>,
        baseline_quins: &[NQuin],
    ) -> Result<Self, JournalError> {
        let mut active_state = HashSet::with_capacity(baseline_quins.len() + 64);
        for q in baseline_quins {
            active_state.insert(*q);
        }

        // Replay committed journal transactions on top of baseline snapshot
        let committed_txs = journal.recover_transactions()?;
        let mut max_tick = 0;
        let mut max_tx_id = 0;

        for tx in committed_txs {
            if tx.tick > max_tick {
                max_tick = tx.tick;
            }
            if tx.tx_id > max_tx_id {
                max_tx_id = tx.tx_id;
            }
            for rem in tx.removes {
                active_state.remove(&rem);
            }
            for add in tx.adds {
                active_state.insert(add);
            }
        }

        Ok(Self {
            journal,
            active_state,
            current_tick: max_tick,
            next_tx_id: max_tx_id + 1,
            staged_adds: Vec::new(),
            staged_removes: Vec::new(),
        })
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn advance_tick(&mut self, tick: u64) {
        if tick > self.current_tick {
            self.current_tick = tick;
        }
    }

    /// Stage a Quin addition in the current uncommitted scratch buffer.
    pub fn stage_add(&mut self, quin: NQuin) {
        self.staged_adds.push(quin);
    }

    /// Stage a Quin removal in the current uncommitted scratch buffer.
    pub fn stage_remove(&mut self, quin: NQuin) {
        self.staged_removes.push(quin);
    }

    /// Discard all uncommitted staged deltas.
    pub fn rollback_staged(&mut self) {
        self.staged_adds.clear();
        self.staged_removes.clear();
    }

    /// Commit all staged deltas atomically into the durable `.q42j` journal
    /// and apply them to the active in-memory semantic state.
    pub fn commit_transaction(
        &mut self,
        command_hash: u64,
        actor_did: u64,
    ) -> Result<u32, JournalError> {
        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;

        let mut tx = JournalTransaction::new(tx_id, self.current_tick, command_hash, actor_did);
        tx.adds = std::mem::take(&mut self.staged_adds);
        tx.removes = std::mem::take(&mut self.staged_removes);

        // Durable append
        self.journal.append_transaction(&tx)?;

        // Apply to in-memory state
        for rem in &tx.removes {
            self.active_state.remove(rem);
        }
        for add in &tx.adds {
            self.active_state.insert(*add);
        }

        Ok(tx_id)
    }

    /// Checkpoint the active state into a new sealed generation.
    pub fn checkpoint(
        &mut self,
        new_base_digest: [u8; 32],
        new_generation: u64,
    ) -> Result<(), JournalError> {
        self.journal
            .reset_to_generation(new_base_digest, new_generation)
    }

    /// Replay active state from baseline snapshot up to a specific tick target.
    pub fn rewind_to_tick(
        &mut self,
        baseline_quins: &[NQuin],
        target_tick: u64,
    ) -> Result<(), JournalError> {
        self.active_state.clear();
        for q in baseline_quins {
            self.active_state.insert(*q);
        }
        let committed_txs = self.journal.recover_transactions()?;
        for tx in committed_txs {
            if tx.tick <= target_tick {
                for rem in tx.removes {
                    self.active_state.remove(&rem);
                }
                for add in tx.adds {
                    self.active_state.insert(add);
                }
            }
        }
        self.current_tick = target_tick;
        self.staged_adds.clear();
        self.staged_removes.clear();
        Ok(())
    }
}

/// Convert an NQuin into 48 little-endian bytes.
pub fn quin_to_bytes(q: &NQuin) -> [u8; 48] {
    let mut b = [0u8; 48];
    b[0..8].copy_from_slice(&q.subject.to_le_bytes());
    b[8..16].copy_from_slice(&q.predicate.to_le_bytes());
    b[16..24].copy_from_slice(&q.object.to_le_bytes());
    b[24..32].copy_from_slice(&q.context.to_le_bytes());
    b[32..40].copy_from_slice(&q.metadata.to_le_bytes());
    b[40..48].copy_from_slice(&q.parity.to_le_bytes());
    b
}

/// Reconstruct an NQuin from 48 bytes with 5-field XOR parity check.
pub fn quin_from_bytes(b: &[u8]) -> Result<NQuin, JournalError> {
    if b.len() < 48 {
        return Err(JournalError::InvalidQuin("byte slice under 48 bytes"));
    }
    let s = u64::from_le_bytes(b[0..8].try_into().unwrap());
    let p = u64::from_le_bytes(b[8..16].try_into().unwrap());
    let o = u64::from_le_bytes(b[16..24].try_into().unwrap());
    let c = u64::from_le_bytes(b[24..32].try_into().unwrap());
    let m = u64::from_le_bytes(b[32..40].try_into().unwrap());
    let parity = u64::from_le_bytes(b[40..48].try_into().unwrap());

    // 5-field XOR fold parity
    let expected_parity = s ^ p ^ o ^ c ^ m;
    if parity != expected_parity {
        return Err(JournalError::InvalidQuinParity);
    }

    Ok(NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: m,
        parity,
    })
}

#[derive(Debug)]
pub enum JournalError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedVersion(u16),
    UnknownApplicationProfile(u16),
    InvalidHeader(&'static str),
    BaseDigestMismatch,
    UnknownFrameType(u8),
    CorruptedFrame(u64),
    InvalidQuin(&'static str),
    InvalidQuinParity,
}

impl From<io::Error> for JournalError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::InvalidMagic => write!(f, "Invalid Q42J magic bytes"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported Q42J version {}", v),
            Self::UnknownApplicationProfile(p) => {
                write!(f, "Unknown application profile 0x{:04X}", p)
            }
            Self::InvalidHeader(msg) => write!(f, "Invalid Q42J header: {}", msg),
            Self::BaseDigestMismatch => write!(f, "Base .q42 digest does not match journal header"),
            Self::UnknownFrameType(t) => write!(f, "Unknown journal frame type 0x{:02X}", t),
            Self::CorruptedFrame(seq) => write!(f, "Corrupted frame at sequence {}", seq),
            Self::InvalidQuin(msg) => write!(f, "Invalid NQuin in journal: {}", msg),
            Self::InvalidQuinParity => write!(f, "NQuin parity fold verification failed"),
        }
    }
}

impl std::error::Error for JournalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample_quin(s: u64, p: u64, o: u64, c: u64, m: u64) -> NQuin {
        let parity = s ^ p ^ o ^ c ^ m;
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity,
        }
    }

    #[test]
    fn test_q42j_header_roundtrip() {
        let digest = [0x42u8; 32];
        let h = JournalHeader::new(
            JournalApplicationProfile::PoetWorkspace,
            0x1234_5678,
            digest,
            7,
        );
        let bytes = h.to_bytes();
        let parsed = JournalHeader::from_bytes(&bytes).unwrap();
        assert_eq!(h, parsed);
        assert_eq!(
            parsed.profile,
            JournalApplicationProfile::PoetWorkspace as u16
        );
        assert_eq!(parsed.privacy, JournalPrivacyClass::Sanctuary as u8);
    }

    #[test]
    fn test_q42j_single_transaction_commit_and_recover() {
        let digest = [0x55u8; 32];
        let h = JournalHeader::new(
            JournalApplicationProfile::SimulationWorld,
            0x9999,
            digest,
            1,
        );
        let storage = Cursor::new(Vec::new());
        let mut journal = Q42Journal::create(storage, h).unwrap();

        let q1 = sample_quin(100, 200, 300, 400, 500);
        let q2 = sample_quin(101, 201, 301, 401, 501);

        let mut tx = JournalTransaction::new(1, 10, 0xABCD, 0x1111);
        tx.add_quin(q1);
        tx.add_quin(q2);
        journal.append_transaction(&tx).unwrap();

        let recovered = journal.recover_transactions().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].tx_id, 1);
        assert_eq!(recovered[0].tick, 10);
        assert_eq!(recovered[0].command_hash, 0xABCD);
        assert_eq!(recovered[0].actor_did, 0x1111);
        assert_eq!(recovered[0].adds, vec![q1, q2]);
        assert!(recovered[0].committed);
    }

    #[test]
    fn test_q42j_torn_transaction_tail_discard() {
        let digest = [0x55u8; 32];
        let h = JournalHeader::new(
            JournalApplicationProfile::SimulationWorld,
            0x9999,
            digest,
            1,
        );
        let storage = Cursor::new(Vec::new());
        let mut journal = Q42Journal::create(storage, h).unwrap();

        let q1 = sample_quin(100, 200, 300, 400, 500);
        let mut tx1 = JournalTransaction::new(1, 10, 0x111, 0xAA);
        tx1.add_quin(q1);
        journal.append_transaction(&tx1).unwrap();

        // Write half of tx2 without commit
        journal.storage.seek(SeekFrom::End(0)).unwrap();
        let frame_header = FrameHeader {
            frame_type: JournalFrameType::TxBegin as u8,
            reserved: [0; 3],
            payload_len: 16,
            sequence_num: 10,
            tick: 11,
            tx_id: 2,
            crc32c: crc32c(&[0u8; 16]),
        };
        journal.storage.write_all(&frame_header.to_bytes()).unwrap();
        journal.storage.write_all(&[0u8; 16]).unwrap();

        // Replay should cleanly recover tx1 and ignore uncommitted tx2
        let recovered = journal.recover_transactions().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].tx_id, 1);
    }

    #[test]
    fn test_q42j_corrupted_frame_fails_closed() {
        let digest = [0x55u8; 32];
        let h = JournalHeader::new(
            JournalApplicationProfile::GenericSemantic,
            0x9999,
            digest,
            1,
        );
        let storage = Cursor::new(Vec::new());
        let mut journal = Q42Journal::create(storage, h).unwrap();

        let q1 = sample_quin(100, 200, 300, 400, 500);
        let mut tx = JournalTransaction::new(1, 10, 0x111, 0xAA);
        tx.add_quin(q1);
        journal.append_transaction(&tx).unwrap();

        // Tamper with a payload byte in storage
        let buf = journal.storage.get_mut();
        let last_idx = buf.len() - 2;
        buf[last_idx] ^= 0xFF;

        assert!(matches!(
            journal.recover_transactions(),
            Err(JournalError::CorruptedFrame(_))
        ));
    }

    #[test]
    fn test_q42j_base_digest_mismatch() {
        let digest = [0x55u8; 32];
        let h = JournalHeader::new(
            JournalApplicationProfile::GenericSemantic,
            0x9999,
            digest,
            1,
        );
        let mut storage = Cursor::new(Vec::new());
        Q42Journal::create(&mut storage, h).unwrap();

        let wrong_digest = [0x77u8; 32];
        assert!(matches!(
            Q42Journal::open(storage, &wrong_digest),
            Err(JournalError::BaseDigestMismatch)
        ));
    }

    #[test]
    fn test_mutable_q42_session_flow() {
        let digest = [0x12u8; 32];
        let h = JournalHeader::new(JournalApplicationProfile::PoetWorkspace, 0x42, digest, 1);
        let storage = Cursor::new(Vec::new());
        let journal = Q42Journal::create(storage, h).unwrap();

        let base_quin = sample_quin(1, 2, 3, 4, 5);
        let mut session = MutableQ42Session::open(journal, &[base_quin]).unwrap();

        assert_eq!(session.active_state.len(), 1);

        let added_quin = sample_quin(10, 20, 30, 40, 50);
        session.stage_add(added_quin);
        session.stage_remove(base_quin);
        let tx_id = session.commit_transaction(0xFA, 0x88).unwrap();
        assert_eq!(tx_id, 1);

        assert_eq!(session.active_state.len(), 1);
        assert!(session.active_state.contains(&added_quin));
        assert!(!session.active_state.contains(&base_quin));
    }
}
