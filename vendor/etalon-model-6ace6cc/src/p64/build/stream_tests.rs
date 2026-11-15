use super::*;
use crate::p64::integrity::{crc32c, Crc32c};
use crate::p64::{IntegrityMode, P64TensorEntry, P64View};

struct Source {
    calls: usize,
    overrun: bool,
}
impl PayloadSource for Source {
    type Error = ();
    fn requirements(&self, _: usize, _: &P64TensorEntry) -> Result<WorkspaceRequirements, ()> {
        Ok(WorkspaceRequirements {
            byte_count: 2,
            f32_count: 1,
        })
    }
    fn emit(
        &mut self,
        _: usize,
        _: &P64TensorEntry,
        workspace: PayloadWorkspace<'_>,
        sink: &mut dyn PayloadSink,
    ) -> Result<(), ()> {
        assert_eq!(workspace.bytes.len(), 2);
        assert_eq!(workspace.floats.len(), 1);
        self.calls += 1;
        if self.overrun {
            assert_eq!(sink.write_all(&[1; 5]), Err(PayloadWriteRefusal::Overrun));
            assert_eq!(
                sink.write_all(&[1]),
                Err(PayloadWriteRefusal::AlreadyFailed)
            );
            return Ok(());
        }
        sink.write_all(&[1, 2]).map_err(|_| ())?;
        sink.write_all(&[3, 4]).map_err(|_| ())
    }
}

struct Sink {
    bytes: [u8; 2048],
    used: usize,
}
impl ByteSink for Sink {
    type Error = ();
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.bytes[self.used..self.used + bytes.len()].copy_from_slice(bytes);
        self.used += bytes.len();
        Ok(())
    }
}

fn entry() -> P64TensorEntry {
    P64TensorEntry {
        name_offset: 0,
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [1, 0, 0, 0],
        blob_offset: 0,
        blob_size: 0,
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
    }
}

fn tensor() -> TensorLayout<'static> {
    TensorLayout {
        name: b"x",
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [1, 0, 0, 0],
        blob_len: 4,
        source_offset: 17,
        source_name_hash: 19,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
    }
}

#[test]
fn chunked_stream_retains_raw_ffn_schedule_and_source_identity() {
    let tensors = [tensor()];
    let mut hp = [0; 64];
    hp[..4].copy_from_slice(&1u32.to_le_bytes());
    let input = LayoutInput {
        profile: LayoutProfile::FfnLegacy,
        page_log2: 8,
        flags: 0x71,
        header_reserved: [0; 20],
        hparams_bytes: &hp,
        manifold_bytes: &[0; 128],
        tokenizer_bytes: &[8, 9],
        schedule_bytes: &[7; 64],
        tensors: &tensors,
    };
    let budget = BuildBudget {
        output_bytes: 2048,
        scratch_bytes: 256,
    };
    let mut entries = [entry()];
    let plan = plan_layout_into(input, &mut entries, budget).unwrap();
    let mut source = Source {
        calls: 0,
        overrun: false,
    };
    let mut sink = Sink {
        bytes: [0; 2048],
        used: 0,
    };
    let mut requirements = [WorkspaceRequirements::default()];
    let mut checksums = [0];
    let mut bytes = [0; 8];
    let mut floats = [0.; 2];
    let report = stream_into(
        &plan,
        &mut source,
        &mut sink,
        StreamScratch {
            requirements: &mut requirements,
            checksums: &mut checksums,
            bytes: &mut bytes,
            floats: &mut floats,
        },
        budget,
        &|| false,
    )
    .unwrap();
    assert_eq!(source.calls, 2);
    assert_eq!(report.bytes_written, plan.output_len());
    assert_eq!(&sink.bytes[128..192], &[7; 64]);
    let view = P64View::parse(&sink.bytes[..sink.used], IntegrityMode::Full).unwrap();
    assert_eq!(view.tensor(0).unwrap().blob(), &[1, 2, 3, 4]);
    assert_eq!(view.tensor(0).unwrap().entry().source_offset, 17);
}

#[test]
fn ignored_first_pass_overrun_is_latched_and_never_reaches_sink() {
    let tensors = [tensor()];
    let input = LayoutInput {
        profile: LayoutProfile::Standard(BuildProfile::Safetensor),
        page_log2: 8,
        flags: 3,
        header_reserved: [0; 20],
        hparams_bytes: &[0; 64],
        manifold_bytes: &[0; 64],
        tokenizer_bytes: &[],
        schedule_bytes: &[],
        tensors: &tensors,
    };
    let budget = BuildBudget {
        output_bytes: 2048,
        scratch_bytes: 256,
    };
    let mut entries = [entry()];
    let plan = plan_layout_into(input, &mut entries, budget).unwrap();
    let mut source = Source {
        calls: 0,
        overrun: true,
    };
    let mut sink = Sink {
        bytes: [99; 2048],
        used: 0,
    };
    let error = stream_into(
        &plan,
        &mut source,
        &mut sink,
        StreamScratch {
            requirements: &mut [WorkspaceRequirements::default()],
            checksums: &mut [0],
            bytes: &mut [0; 2],
            floats: &mut [0.; 1],
        },
        budget,
        &|| false,
    )
    .unwrap_err();
    assert_eq!(error.phase, StreamPhase::Checksum);
    assert_eq!(
        error.kind,
        StreamFailureKind::Length {
            expected: 4,
            actual: 5
        }
    );
    assert_eq!(sink.used, 0);
    assert_eq!(sink.bytes, [99; 2048]);
}

#[test]
fn incremental_crc_preserves_standard_vector_and_chunk_boundaries() {
    let data = b"123456789";
    for split in 0..=data.len() {
        let mut state = Crc32c::new();
        state.update(&data[..split]);
        state.update(&data[split..]);
        assert_eq!(state.finish(), crc32c(data));
        assert_eq!(state.finish(), 0xe306_9283);
    }
}
