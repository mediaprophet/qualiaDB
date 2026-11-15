use super::legacy;
use etalon_model::p64::build::*;
use etalon_model::p64::P64TensorEntry;
use std::cell::Cell;

// Actual immutable Qualia 07018bb8 compiler output, including a real FFN ternary
// transform and the historical zero schedule. Regenerate with the adjacent
// regenerate_p64_stream_golden.py helper; its SHA256 check is independent of Rust.
// This does not qualify Q4/AWQ numerical transforms.
pub const FFN: legacy::Golden = legacy::Golden {
    name: "ffn-ternary9",
    profile: BuildProfile::GgufLayerPacked,
    page: 9,
    bytes: include_bytes!("p64_stream_golden/ffn-ternary-page9.p64"),
    sha256: "b4646e8180c7fca6943fb94cae9464574a60d2badda6a97d8c42d35642b71ac8",
};
pub fn unlimited() -> BuildBudget {
    BuildBudget {
        output_bytes: usize::MAX,
        scratch_bytes: usize::MAX,
    }
}
pub fn sized<'a>(tensors: &[TensorDescriptor<'a>]) -> Vec<TensorLayout<'a>> {
    tensors
        .iter()
        .map(|t| TensorLayout {
            name: t.name,
            role_id: t.role_id,
            dtype: t.dtype,
            manifold_idx: t.manifold_idx,
            rank: t.rank,
            dimensions: t.dimensions,
            source_offset: t.source_offset,
            source_name_hash: t.source_name_hash,
            alt_dtype: t.alt_dtype,
            precision_views_mask: t.precision_views_mask,
            alt_blob_offset: t.alt_blob_offset,
            blob_len: t.blob.len() as u64,
        })
        .collect()
}
pub fn input<'a>(
    g: &'a legacy::Golden,
    tensors: &'a [TensorLayout<'a>],
    ffn: bool,
) -> LayoutInput<'a> {
    let b = g.bytes;
    let manifold = legacy::u32le(b, 32) as usize;
    let tokenizer = legacy::u32le(b, 16) as usize;
    let checksum = legacy::u32le(b, 28) as usize;
    LayoutInput {
        profile: if ffn {
            LayoutProfile::FfnLegacy
        } else {
            LayoutProfile::Standard(g.profile)
        },
        page_log2: g.page,
        flags: legacy::u16le(b, 6),
        header_reserved: b[44..64].try_into().unwrap(),
        hparams_bytes: b[64..128].try_into().unwrap(),
        manifold_bytes: &b[manifold..tokenizer],
        tokenizer_bytes: &b[tokenizer..checksum],
        schedule_bytes: if ffn {
            &b[128..legacy::u32le(b, 12) as usize]
        } else {
            &[]
        },
        tensors,
    }
}

#[derive(Clone, Copy)]
pub enum Fault {
    None,
    Short(usize),
    Long(usize),
    ChangedReplay,
    Source(usize),
    IgnoreOverrun(usize),
    IgnoreRefusal(usize),
    Panic(usize),
}
pub struct Source<'a> {
    pub blobs: &'a [&'a [u8]],
    pub requirements_calls: [Cell<usize>; 8],
    pub emits: [usize; 8],
    pub fault: Fault,
    pub requirement_error: Option<usize>,
    pub oversized: bool,
    pub cancel_after_chunk: Option<&'a Cell<bool>>,
}
impl<'a> Source<'a> {
    pub fn new(blobs: &'a [&'a [u8]]) -> Self {
        Self {
            blobs,
            requirements_calls: core::array::from_fn(|_| Cell::new(0)),
            emits: [0; 8],
            fault: Fault::None,
            requirement_error: None,
            oversized: false,
            cancel_after_chunk: None,
        }
    }
}
impl PayloadSource for Source<'_> {
    type Error = u8;
    fn requirements(&self, index: usize, _: &P64TensorEntry) -> Result<WorkspaceRequirements, u8> {
        self.requirements_calls[index].set(self.requirements_calls[index].get() + 1);
        if self.requirement_error == Some(index) {
            return Err(31);
        }
        Ok(WorkspaceRequirements {
            byte_count: if self.oversized { usize::MAX } else { 4 },
            f32_count: 2,
        })
    }
    fn emit(
        &mut self,
        index: usize,
        _: &P64TensorEntry,
        workspace: PayloadWorkspace<'_>,
        output: &mut dyn PayloadSink,
    ) -> Result<(), u8> {
        self.emits[index] += 1;
        let pass = self.emits[index];
        assert_eq!(
            self.requirements_calls[index].get(),
            1,
            "requirements must be frozen once"
        );
        assert_eq!(workspace.bytes.len(), 4);
        assert_eq!(workspace.floats.len(), 2);
        let mut data = self.blobs[index];
        if index == 0 && matches!(self.fault,Fault::Short(p) if p==pass) {
            data = &data[..data.len().saturating_sub(1)];
        }
        if index == 0 && matches!(self.fault, Fault::ChangedReplay) && pass == 2 && !data.is_empty()
        {
            let altered = [data[0] ^ 1];
            output.write_all(&altered).map_err(|_| 90)?;
            data = &data[1..];
        }
        for chunk in data.chunks(3) {
            let result = output.write_all(chunk);
            if let Some(cancel) = self.cancel_after_chunk {
                cancel.set(true);
            }
            if index == 0
                && matches!(self.fault,Fault::IgnoreOverrun(p)|Fault::IgnoreRefusal(p) if p==pass)
            {
                let _ = result;
            } else {
                result.map_err(|_| 90)?;
            }
            if index == 0 && matches!(self.fault,Fault::Source(p) if p==pass) {
                return Err(42);
            }
            if index == 0 && matches!(self.fault,Fault::Panic(p) if p==pass) {
                panic!("injected payload unwind");
            }
        }
        if index == 0 && matches!(self.fault,Fault::Long(p)|Fault::IgnoreOverrun(p) if p==pass) {
            let result = output.write_all(&[9]);
            if matches!(self.fault, Fault::IgnoreOverrun(_)) {
                assert!(result.is_err());
                assert!(output.write_all(&[]).is_err(), "refusal must latch");
                return Ok(());
            }
            result.map_err(|_| 90)?;
        }
        Ok(())
    }
}

pub struct Sink<'a> {
    pub bytes: &'a mut [u8],
    pub actual: usize,
    pub confirmed: usize,
    pub calls: usize,
    pub fail_at: Option<usize>,
    pub cancel_after: Option<(&'a Cell<bool>, usize)>,
}
impl<'a> Sink<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self {
            bytes,
            actual: 0,
            confirmed: 0,
            calls: 0,
            fail_at: None,
            cancel_after: None,
        }
    }
}
impl ByteSink for Sink<'_> {
    type Error = u16;
    fn write_all(&mut self, chunk: &[u8]) -> Result<(), u16> {
        self.calls += 1;
        if self.fail_at.is_some_and(|offset| self.confirmed >= offset) {
            // Real partial side effect on a failed sink call; it is not confirmed.
            if !chunk.is_empty() {
                self.bytes[self.actual] = chunk[0];
                self.actual += 1;
            }
            return Err(77);
        }
        self.bytes[self.actual..self.actual + chunk.len()].copy_from_slice(chunk);
        self.actual += chunk.len();
        self.confirmed += chunk.len();
        if let Some((cancel, after)) = self.cancel_after {
            if self.confirmed >= after {
                cancel.set(true);
            }
        }
        Ok(())
    }
}
pub fn run<P: PayloadSource, W: ByteSink>(
    plan: &LayoutPlan<'_, '_>,
    source: &mut P,
    sink: &mut W,
    budget: BuildBudget,
    cancel: &dyn Fn() -> bool,
) -> Result<StreamReport, StreamFailure<P::Error, W::Error>> {
    let mut requirements = [WorkspaceRequirements::default(); 8];
    let mut checksums = [0; 8];
    let mut bytes = [0; 32];
    let mut floats = [0.; 16];
    stream_into(
        plan,
        source,
        sink,
        StreamScratch {
            requirements: &mut requirements,
            checksums: &mut checksums,
            bytes: &mut bytes,
            floats: &mut floats,
        },
        budget,
        cancel,
    )
}
