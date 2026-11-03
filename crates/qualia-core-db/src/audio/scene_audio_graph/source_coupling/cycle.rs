//! Bounded cue placement into a caller-owned PCM cycle. Original recordings,
//! rights, source event identifiers and decoder receipts stay with the host.
#[derive(Clone, Copy)]
pub struct CueSpan<'a> {
    pub cue_key: u64,
    pub start_frame: usize,
    pub end_frame: usize,
    pub source_start_frame: usize,
    pub gain: f32,
    pub samples: &'a [f32],
}
pub struct CycleBudget { pub max_cues: usize, pub max_frames: usize, pub max_work: usize }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CycleReceipt { pub frames:usize, pub cues:usize, pub peak:f32, pub work:usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleError { Input, Budget, Numeric, Clipping, Cancelled }

/// Exact half-open cue onsets/ends with zero outside each authored interval.
/// Destination is unchanged on any failure; scratch is unpublished/disposable.
/// Feed the complete result to existing PcmClip.loop_frames; catalogue stays
/// fixed across cycles, so a natural wrap does not reset room histories.
pub fn compile_cycle_pcm(cues:&[CueSpan<'_>], scratch:&mut[f32], out:&mut[f32],
    budget:CycleBudget, cancelled:&dyn Fn()->bool) -> Result<CycleReceipt,CycleError> {
    if out.is_empty() || scratch.len()<out.len() || cues.len()>budget.max_cues
        || out.len()>budget.max_frames { return Err(CycleError::Budget); }
    let mut work=out.len().checked_mul(3)
        .and_then(|n| cues.len().checked_mul(8).and_then(|c| n.checked_add(c)))
        .ok_or(CycleError::Budget)?;
    if work>budget.max_work { return Err(CycleError::Budget); }
    for (i,c) in cues.iter().enumerate() {
        if cancelled() { return Err(CycleError::Cancelled); }
        if c.cue_key==0 || i>0 && cues[i-1].cue_key>=c.cue_key || c.start_frame>=c.end_frame
            || c.end_frame>out.len() || !(0.0..=1.0).contains(&c.gain) {
            return Err(CycleError::Input);
        }
        let n=c.end_frame-c.start_frame;
        let end=c.source_start_frame.checked_add(n).ok_or(CycleError::Input)?;
        let source=c.samples.get(c.source_start_frame..end).ok_or(CycleError::Input)?;
        work=work.checked_add(n.checked_mul(2).ok_or(CycleError::Budget)?).ok_or(CycleError::Budget)?;
        if work>budget.max_work { return Err(CycleError::Budget); }
        for (j, value) in source.iter().enumerate() {
            if j % 256 == 0 && cancelled() { return Err(CycleError::Cancelled); }
            if !(-1.0..=1.0).contains(value) { return Err(CycleError::Numeric); }
        }
    }
    if work>budget.max_work { return Err(CycleError::Budget); }
    scratch[..out.len()].fill(0.);
    for c in cues {
        for (i,sample) in c.samples[c.source_start_frame..c.source_start_frame+c.end_frame-c.start_frame].iter().enumerate() {
            if i%256==0 && cancelled() { return Err(CycleError::Cancelled); }
            scratch[c.start_frame+i]+=sample*c.gain;
        }
    }
    let mut peak=0f32;
    for (i,&sample) in scratch[..out.len()].iter().enumerate() {
        if i%256==0 && cancelled() { return Err(CycleError::Cancelled); }
        if !sample.is_finite() { return Err(CycleError::Numeric); }
        peak=peak.max(sample.abs());
    }
    if peak>1. { return Err(CycleError::Clipping); }
    if cancelled() { return Err(CycleError::Cancelled); }
    out.copy_from_slice(&scratch[..out.len()]);
    Ok(CycleReceipt { frames:out.len(),cues:cues.len(),peak,work })
}
