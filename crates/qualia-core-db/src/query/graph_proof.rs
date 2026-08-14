//! Bounded-memory proof of the encoded graph represented by an N-Triples source
//! and a Q42 volume.
//!
//! The `qualia-cli ingest semantic` pipeline stores the default graph as hashed
//! `(subject, predicate, object, context)` records.  This module compares those
//! records exactly as a *set*, using sorted fixed-width disk runs rather than an
//! in-memory graph or a lossy aggregate checksum.
//! Blank-node graphs require lexical Q42 terms for canonical isomorphism.

use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::mini_parser::hash_token;
use crate::q42_volume::{Q42Volume, QUIN_SIZE, SUPERBLOCK_HEADER, SUPERBLOCK_SIZE};

const RECORD_BYTES: u64 = 32;
const READ_BUFFER_BYTES: usize = 32 * 1024;
const MERGE_FAN_IN: usize = 16;

/// Default RAM reserved for the sort buffer.  The verifier uses bounded I/O
/// buffers in addition to this allocation.
pub const DEFAULT_GRAPH_PROOF_MEMORY_BYTES: usize = 32 * 1024 * 1024;
/// Default maximum temporary on-disk footprint.  The verifier fails closed
/// rather than exhausting an arbitrary temp volume.
pub const DEFAULT_GRAPH_PROOF_TEMP_BYTES: u64 = 24 * 1024 * 1024 * 1024;

/// Resource limits for [`prove_cli_ntriples_q42_equivalence`].
#[derive(Clone, Copy, Debug)]
pub struct GraphProofOptions {
    /// Upper bound for the in-memory record sort buffer, in bytes.
    pub memory_limit_bytes: usize,
    /// Upper bound for live temporary run files, in bytes.
    pub temporary_byte_budget: u64,
}

impl Default for GraphProofOptions {
    fn default() -> Self {
        Self {
            memory_limit_bytes: DEFAULT_GRAPH_PROOF_MEMORY_BYTES,
            temporary_byte_budget: DEFAULT_GRAPH_PROOF_TEMP_BYTES,
        }
    }
}

/// The level of RDF claim that can be made from an encoded Q42 comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum RdfIsomorphismStatus {
    /// The source contains no blank-node labels, so exact encoded-set equality
    /// is a ground-graph equivalence proof in the ingestion representation.
    GroundGraphProven,
    /// The sets match only when original blank-node labels are treated as
    /// identities.  Canonical blank-node isomorphism needs lexical Q42 terms.
    BlankNodeCanonicalizationRequired,
    /// The encoded graph sets differ, so no isomorphism claim is possible.
    Different,
}

/// Result of an exact, external-sort comparison.
#[derive(Clone, Debug, serde::Serialize)]
pub struct GraphProofReport {
    /// Number of N-Triples accepted by the CLI-compatible normaliser.
    pub source_records: u64,
    /// Number of Q42 Quins examined.
    pub q42_records: u64,
    /// Number of unique encoded quads in the source.
    pub source_unique_records: u64,
    /// Number of unique encoded quads in the Q42 volume.
    pub q42_unique_records: u64,
    /// Source records that do not occur in the Q42 set.
    pub missing_from_q42: u64,
    /// Q42 records that do not occur in the source set.
    pub unexpected_in_q42: u64,
    /// A representative missing encoded quad, if any.
    pub first_missing: Option<[u64; 4]>,
    /// A representative unexpected encoded quad, if any.
    pub first_unexpected: Option<[u64; 4]>,
    /// Lines skipped because they are blank, comments, or not accepted by the
    /// same three-token N-Triples compatibility parser used by CLI ingest.
    pub source_skipped_lines: u64,
    pub source_contains_blank_nodes: bool,
    pub rdf_isomorphism: RdfIsomorphismStatus,
}

impl GraphProofReport {
    /// Exact equality of the two encoded RDF default-graph sets.
    pub fn encoded_sets_match(&self) -> bool {
        self.missing_from_q42 == 0 && self.unexpected_in_q42 == 0
    }
}

/// Compare an N-Triples input to a Q42 volume without retaining either graph
/// in memory.
///
/// Source normalisation intentionally mirrors `qualia-cli ingest semantic`:
/// it takes the first three ASCII-whitespace-delimited tokens and applies
/// [`hash_token`] to each.  This proves the bytes that that ingest mode can
/// encode, rather than pretending the current hash-only volume can recover
/// lexical RDF values that it never stored.
pub fn prove_cli_ntriples_q42_equivalence(
    source_path: &Path,
    q42_path: &Path,
    options: GraphProofOptions,
) -> io::Result<GraphProofReport> {
    let records_per_chunk = records_per_chunk(options.memory_limit_bytes)?;
    let workspace = TempDir::new()?;
    let mut budget = TempBudget::new(options.temporary_byte_budget);

    let mut source_spool = DiskSpool::new(workspace.path(), "source", records_per_chunk);
    let (source_skipped_lines, source_contains_blank_nodes) =
        stream_source_records(source_path, &mut source_spool, &mut budget)?;
    let source_records = source_spool.record_count;
    let source_runs = source_spool.finish(&mut budget)?;
    let source_run = merge_to_one(source_runs, workspace.path(), "source", &mut budget)?;

    let mut q42_spool = DiskSpool::new(workspace.path(), "q42", records_per_chunk);
    stream_q42_records(q42_path, &mut q42_spool, &mut budget)?;
    let q42_records = q42_spool.record_count;
    let q42_runs = q42_spool.finish(&mut budget)?;
    let q42_run = merge_to_one(q42_runs, workspace.path(), "q42", &mut budget)?;

    let comparison = compare_unique_sets(&source_run.path, &q42_run.path)?;
    let encoded_sets_match = comparison.missing == 0 && comparison.unexpected == 0;
    let rdf_isomorphism = if !encoded_sets_match {
        RdfIsomorphismStatus::Different
    } else if source_contains_blank_nodes {
        RdfIsomorphismStatus::BlankNodeCanonicalizationRequired
    } else {
        RdfIsomorphismStatus::GroundGraphProven
    };

    Ok(GraphProofReport {
        source_records,
        q42_records,
        source_unique_records: comparison.left_unique,
        q42_unique_records: comparison.right_unique,
        missing_from_q42: comparison.missing,
        unexpected_in_q42: comparison.unexpected,
        first_missing: comparison.first_missing.map(QuadRecord::as_array),
        first_unexpected: comparison.first_unexpected.map(QuadRecord::as_array),
        source_skipped_lines,
        source_contains_blank_nodes,
        rdf_isomorphism,
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadRecord {
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
}

impl QuadRecord {
    fn as_array(self) -> [u64; 4] {
        [self.subject, self.predicate, self.object, self.context]
    }
}

impl Ord for QuadRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.object, self.subject, self.predicate, self.context).cmp(&(
            other.object,
            other.subject,
            other.predicate,
            other.context,
        ))
    }
}

impl PartialOrd for QuadRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct DiskRun {
    path: PathBuf,
    bytes: u64,
}

struct TempBudget {
    maximum: u64,
    current: u64,
}

impl TempBudget {
    fn new(maximum: u64) -> Self {
        Self {
            maximum,
            current: 0,
        }
    }

    fn reserve(&mut self, bytes: u64) -> io::Result<()> {
        let next = self.current.checked_add(bytes).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "graph-proof temporary budget overflow",
            )
        })?;
        if next > self.maximum {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "graph-proof temporary budget exceeded: need {next} bytes, limit is {} bytes",
                    self.maximum
                ),
            ));
        }
        self.current = next;
        Ok(())
    }

    fn release(&mut self, bytes: u64) {
        self.current = self.current.saturating_sub(bytes);
    }
}

struct DiskSpool {
    temp_dir: PathBuf,
    prefix: &'static str,
    records: Vec<QuadRecord>,
    runs: Vec<DiskRun>,
    run_index: usize,
    record_count: u64,
}

impl DiskSpool {
    fn new(temp_dir: &Path, prefix: &'static str, records_per_chunk: usize) -> Self {
        Self {
            temp_dir: temp_dir.to_path_buf(),
            prefix,
            records: Vec::with_capacity(records_per_chunk),
            runs: Vec::new(),
            run_index: 0,
            record_count: 0,
        }
    }

    fn push(&mut self, record: QuadRecord, budget: &mut TempBudget) -> io::Result<()> {
        self.records.push(record);
        self.record_count += 1;
        if self.records.len() == self.records.capacity() {
            self.flush(budget)?;
        }
        Ok(())
    }

    fn flush(&mut self, budget: &mut TempBudget) -> io::Result<()> {
        if self.records.is_empty() {
            return Ok(());
        }
        self.records.sort_unstable();
        let bytes = self.records.len() as u64 * RECORD_BYTES;
        budget.reserve(bytes)?;
        let path = self
            .temp_dir
            .join(format!("{}-run-{:08}.bin", self.prefix, self.run_index));
        self.run_index += 1;
        let write_result = write_records(&path, &self.records);
        if let Err(error) = write_result {
            budget.release(bytes);
            return Err(error);
        }
        self.records.clear();
        self.runs.push(DiskRun { path, bytes });
        Ok(())
    }

    fn finish(&mut self, budget: &mut TempBudget) -> io::Result<Vec<DiskRun>> {
        self.flush(budget)?;
        Ok(std::mem::take(&mut self.runs))
    }
}

fn records_per_chunk(memory_limit_bytes: usize) -> io::Result<usize> {
    let records = memory_limit_bytes / RECORD_BYTES as usize;
    if records == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "graph-proof memory limit must hold at least one 32-byte record",
        ));
    }
    Ok(records)
}

fn stream_source_records(
    source_path: &Path,
    spool: &mut DiskSpool,
    budget: &mut TempBudget,
) -> io::Result<(u64, bool)> {
    let source = File::open(source_path)?;
    let mut reader = BufReader::with_capacity(READ_BUFFER_BYTES, source);
    let mut buffer = Vec::with_capacity(READ_BUFFER_BYTES);
    let mut skipped = 0u64;
    let mut has_blank_nodes = false;

    loop {
        buffer.clear();
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        let line = std::str::from_utf8(&buffer).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "N-Triples source is not UTF-8")
        })?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            skipped += 1;
            continue;
        }
        let mut tokens = line.split_ascii_whitespace();
        let (Some(subject), Some(predicate), Some(object)) =
            (tokens.next(), tokens.next(), tokens.next())
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "source contains a non-comment line without an RDF triple",
            ));
        };
        has_blank_nodes |=
            subject.starts_with("_:") || predicate.starts_with("_:") || object.starts_with("_:");
        spool.push(
            QuadRecord {
                subject: hash_token(subject),
                predicate: hash_token(predicate),
                object: hash_token(object),
                context: 0,
            },
            budget,
        )?;
    }
    Ok((skipped, has_blank_nodes))
}

fn stream_q42_records(
    q42_path: &Path,
    spool: &mut DiskSpool,
    budget: &mut TempBudget,
) -> io::Result<()> {
    let volume = Q42Volume::open(q42_path)?;
    if volume.volume_manifest()?.is_some() {
        let set = crate::q42_volume::Q42VolumeSet::open_root(q42_path)?;
        for segment in set.segments() {
            stream_q42_volume_records(segment, spool, budget)?;
        }
        return Ok(());
    }
    stream_q42_volume_records(&volume, spool, budget)
}

fn stream_q42_volume_records(
    volume: &Q42Volume,
    spool: &mut DiskSpool,
    budget: &mut TempBudget,
) -> io::Result<()> {
    let mut buffer = [0u8; SUPERBLOCK_SIZE];
    for block_index in 0..volume.block_count() as usize {
        volume.read_superblock_into(block_index, &mut buffer)?;
        let quin_count = u64::from_le_bytes(buffer[16..24].try_into().unwrap()) as usize;
        if quin_count > crate::QUINS_PER_BLOCK {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 superblock declares too many Quins",
            ));
        }
        let mut offset = SUPERBLOCK_HEADER;
        for _ in 0..quin_count {
            let quin: crate::NQuin =
                bytemuck::pod_read_unaligned(&buffer[offset..offset + QUIN_SIZE]);
            if !quin.verify_ecc_parity() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Q42 parity mismatch in block {block_index}"),
                ));
            }
            spool.push(
                QuadRecord {
                    subject: quin.subject,
                    predicate: quin.predicate,
                    object: quin.object,
                    context: quin.context,
                },
                budget,
            )?;
            offset += QUIN_SIZE;
        }
    }
    Ok(())
}

fn write_records(path: &Path, records: &[QuadRecord]) -> io::Result<()> {
    let mut writer = BufWriter::with_capacity(READ_BUFFER_BYTES, File::create(path)?);
    for record in records {
        writer.write_all(bytemuck::bytes_of(record))?;
    }
    writer.flush()
}

fn merge_to_one(
    mut runs: Vec<DiskRun>,
    temp_dir: &Path,
    prefix: &str,
    budget: &mut TempBudget,
) -> io::Result<DiskRun> {
    if runs.is_empty() {
        let path = temp_dir.join(format!("{prefix}-empty.bin"));
        File::create(&path)?;
        return Ok(DiskRun { path, bytes: 0 });
    }
    let mut round = 0usize;
    while runs.len() > 1 {
        let mut next = Vec::with_capacity(runs.len().div_ceil(MERGE_FAN_IN));
        let mut group_index = 0usize;
        while !runs.is_empty() {
            let group_len = runs.len().min(MERGE_FAN_IN);
            let group: Vec<DiskRun> = runs.drain(..group_len).collect();
            let bytes: u64 = group.iter().map(|run| run.bytes).sum();
            budget.reserve(bytes)?;
            let output = temp_dir.join(format!("{prefix}-merge-{round:04}-{group_index:08}.bin"));
            group_index += 1;
            if let Err(error) = merge_group(&group, &output) {
                budget.release(bytes);
                return Err(error);
            }
            for run in group {
                std::fs::remove_file(&run.path)?;
                budget.release(run.bytes);
            }
            next.push(DiskRun {
                path: output,
                bytes,
            });
        }
        runs = next;
        round += 1;
    }
    Ok(runs.pop().expect("non-empty run list"))
}

fn merge_group(group: &[DiskRun], output: &Path) -> io::Result<()> {
    let mut readers = Vec::with_capacity(group.len());
    for run in group {
        readers.push(RecordReader::open(&run.path)?);
    }
    let mut writer = BufWriter::with_capacity(READ_BUFFER_BYTES, File::create(output)?);
    loop {
        let mut minimum: Option<(usize, QuadRecord)> = None;
        for (index, reader) in readers.iter().enumerate() {
            if let Some(record) = reader.peek {
                if minimum.is_none_or(|(_, current)| record < current) {
                    minimum = Some((index, record));
                }
            }
        }
        let Some((index, record)) = minimum else {
            break;
        };
        writer.write_all(bytemuck::bytes_of(&record))?;
        readers[index].advance()?;
    }
    writer.flush()
}

struct RecordReader {
    reader: BufReader<File>,
    peek: Option<QuadRecord>,
}

impl RecordReader {
    fn open(path: &Path) -> io::Result<Self> {
        let mut reader = Self {
            reader: BufReader::with_capacity(READ_BUFFER_BYTES, File::open(path)?),
            peek: None,
        };
        reader.advance()?;
        Ok(reader)
    }

    fn advance(&mut self) -> io::Result<()> {
        let mut bytes = [0u8; RECORD_BYTES as usize];
        match self.reader.read_exact(&mut bytes) {
            Ok(()) => {
                self.peek = Some(bytemuck::pod_read_unaligned(&bytes));
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                self.peek = None;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn next_distinct(&mut self) -> io::Result<Option<QuadRecord>> {
        let Some(record) = self.peek else {
            return Ok(None);
        };
        while self.peek == Some(record) {
            self.advance()?;
        }
        Ok(Some(record))
    }
}

#[derive(Default)]
struct SetComparison {
    left_unique: u64,
    right_unique: u64,
    missing: u64,
    unexpected: u64,
    first_missing: Option<QuadRecord>,
    first_unexpected: Option<QuadRecord>,
}

fn compare_unique_sets(left: &Path, right: &Path) -> io::Result<SetComparison> {
    let mut left_reader = RecordReader::open(left)?;
    let mut right_reader = RecordReader::open(right)?;
    let mut left_record = left_reader.next_distinct()?;
    let mut right_record = right_reader.next_distinct()?;
    let mut result = SetComparison::default();

    loop {
        match (left_record, right_record) {
            (Some(left), Some(right)) => match left.cmp(&right) {
                Ordering::Equal => {
                    result.left_unique += 1;
                    result.right_unique += 1;
                    left_record = left_reader.next_distinct()?;
                    right_record = right_reader.next_distinct()?;
                }
                Ordering::Less => {
                    result.left_unique += 1;
                    result.missing += 1;
                    result.first_missing.get_or_insert(left);
                    left_record = left_reader.next_distinct()?;
                }
                Ordering::Greater => {
                    result.right_unique += 1;
                    result.unexpected += 1;
                    result.first_unexpected.get_or_insert(right);
                    right_record = right_reader.next_distinct()?;
                }
            },
            (Some(left), None) => {
                result.left_unique += 1;
                result.missing += 1;
                result.first_missing.get_or_insert(left);
                left_record = left_reader.next_distinct()?;
            }
            (None, Some(right)) => {
                result.right_unique += 1;
                result.unexpected += 1;
                result.first_unexpected.get_or_insert(right);
                right_record = right_reader.next_distinct()?;
            }
            (None, None) => return Ok(result),
        }
    }
}
