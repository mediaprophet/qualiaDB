use super::metadata::{emit_metadata, zeros};
use super::*;
use crate::p64::integrity::Crc32c;

/// Two-pass forward-only streaming. Requirements and aggregate budget are checked
/// before emission. Scratch can change on failure; external writes cannot roll back.
/// Fixed stack metadata/padding blocks are bounded independently of tensor count.
/// CRC replay agreement detects changes under the format contract, not authentication.
pub fn stream_into<P: PayloadSource, W: ByteSink>(
    plan: &LayoutPlan<'_, '_>,
    source: &mut P,
    sink: &mut W,
    scratch: StreamScratch<'_>,
    budget: BuildBudget,
    cancel: &dyn Fn() -> bool,
) -> Result<StreamReport, StreamFailure<P::Error, W::Error>> {
    let count = plan.entries.len();
    let preflight = |kind| StreamFailure {
        phase: StreamPhase::Preflight,
        tensor_index: None,
        confirmed_bytes: 0,
        kind,
    };
    if cancel() {
        return Err(preflight(StreamFailureKind::Cancelled));
    }
    if scratch.requirements.len() < count || scratch.checksums.len() < count {
        return Err(preflight(StreamFailureKind::ScratchCapacity));
    }
    let scratch_bytes = count
        .checked_mul(64)
        .and_then(|n| {
            count
                .checked_mul(core::mem::size_of::<WorkspaceRequirements>())
                .and_then(|m| n.checked_add(m))
        })
        .and_then(|n| count.checked_mul(4).and_then(|m| n.checked_add(m)))
        .and_then(|n| n.checked_add(scratch.bytes.len()))
        .and_then(|n| {
            scratch
                .floats
                .len()
                .checked_mul(4)
                .and_then(|m| n.checked_add(m))
        })
        .ok_or_else(|| preflight(StreamFailureKind::ArithmeticOverflow))?;
    if scratch_bytes > budget.scratch_bytes {
        return Err(preflight(StreamFailureKind::Build(
            BuildError::ScratchBudget {
                required_bytes: scratch_bytes,
                available_bytes: budget.scratch_bytes,
            },
        )));
    }
    if plan.output_len > budget.output_bytes {
        return Err(preflight(StreamFailureKind::Build(
            BuildError::OutputBudget {
                required_bytes: plan.output_len,
                available_bytes: budget.output_bytes,
            },
        )));
    }
    for (index, entry) in plan.entries.iter().enumerate() {
        if cancel() {
            return Err(preflight(StreamFailureKind::Cancelled));
        }
        let requirements = source
            .requirements(index, entry)
            .map_err(|error| StreamFailure {
                phase: StreamPhase::Preflight,
                tensor_index: Some(index),
                confirmed_bytes: 0,
                kind: StreamFailureKind::Source(error),
            })?;
        scratch.requirements[index] = requirements;
        if requirements.byte_count > scratch.bytes.len()
            || requirements.f32_count > scratch.floats.len()
        {
            return Err(preflight(StreamFailureKind::ScratchCapacity));
        }
    }
    let mut output = Output {
        sink,
        cancel,
        confirmed: 0,
        limit: plan.output_len,
    };
    for (index, entry) in plan.entries.iter().enumerate() {
        let requirements = scratch.requirements[index];
        scratch.checksums[index] = payload(
            source,
            index,
            entry,
            PayloadWorkspace {
                bytes: &mut scratch.bytes[..requirements.byte_count],
                floats: &mut scratch.floats[..requirements.f32_count],
            },
            &mut output,
            true,
        )?;
    }
    let mut metadata_crc = Crc32c::new();
    emit_metadata(
        plan,
        |chunk| -> Result<(), StreamFailure<P::Error, W::Error>> {
            if cancel() {
                return Err(StreamFailure {
                    phase: StreamPhase::Metadata,
                    tensor_index: None,
                    confirmed_bytes: 0,
                    kind: StreamFailureKind::Cancelled,
                });
            }
            metadata_crc.update(chunk);
            Ok(())
        },
    )?;
    emit_metadata(plan, |chunk| {
        output.write(StreamPhase::Metadata, None, chunk)
    })?;
    output.write(
        StreamPhase::Metadata,
        None,
        &metadata_crc.finish().to_le_bytes(),
    )?;
    for checksum in &scratch.checksums[..count] {
        output.write(StreamPhase::Metadata, None, &checksum.to_le_bytes())?;
    }
    for (index, entry) in plan.entries.iter().enumerate() {
        let padding = entry.blob_offset as usize - output.confirmed;
        zeros(padding, &mut |chunk| {
            output.write::<P::Error>(StreamPhase::Padding, Some(index), chunk)
        })?;
        let requirements = scratch.requirements[index];
        let checksum = payload(
            source,
            index,
            entry,
            PayloadWorkspace {
                bytes: &mut scratch.bytes[..requirements.byte_count],
                floats: &mut scratch.floats[..requirements.f32_count],
            },
            &mut output,
            false,
        )?;
        if checksum != scratch.checksums[index] {
            return Err(StreamFailure {
                phase: StreamPhase::Payload,
                tensor_index: Some(index),
                confirmed_bytes: output.confirmed,
                kind: StreamFailureKind::ReplayChecksum {
                    expected: scratch.checksums[index],
                    actual: checksum,
                },
            });
        }
    }
    let trailing = plan.output_len - output.confirmed;
    zeros(trailing, &mut |chunk| {
        output.write::<P::Error>(StreamPhase::Padding, None, chunk)
    })?;
    if cancel() {
        return Err(StreamFailure {
            phase: StreamPhase::Padding,
            tensor_index: None,
            confirmed_bytes: output.confirmed,
            kind: StreamFailureKind::Cancelled,
        });
    }
    Ok(StreamReport {
        bytes_written: output.confirmed,
        tensors_written: count,
        scratch_bytes,
    })
}

struct Output<'a, W: ByteSink> {
    sink: &'a mut W,
    cancel: &'a dyn Fn() -> bool,
    confirmed: usize,
    limit: usize,
}

impl<W: ByteSink> Output<'_, W> {
    fn write<S>(
        &mut self,
        phase: StreamPhase,
        index: Option<usize>,
        chunk: &[u8],
    ) -> Result<(), StreamFailure<S, W::Error>> {
        let fail = |kind| StreamFailure {
            phase,
            tensor_index: index,
            confirmed_bytes: self.confirmed,
            kind,
        };
        if (self.cancel)() {
            return Err(fail(StreamFailureKind::Cancelled));
        }
        let next = self
            .confirmed
            .checked_add(chunk.len())
            .ok_or_else(|| fail(StreamFailureKind::ArithmeticOverflow))?;
        if next > self.limit {
            return Err(fail(StreamFailureKind::Length {
                expected: self.limit as u64,
                actual: next as u64,
            }));
        }
        self.sink
            .write_all(chunk)
            .map_err(|error| fail(StreamFailureKind::Sink(error)))?;
        self.confirmed = next;
        Ok(())
    }
}

struct Bounded<'a, 'b, S, W: ByteSink> {
    output: &'a mut Output<'b, W>,
    expected: u64,
    actual: u64,
    crc: Crc32c,
    dry: bool,
    phase: StreamPhase,
    index: usize,
    failure: Option<StreamFailure<S, W::Error>>,
}

impl<S, W: ByteSink> PayloadSink for Bounded<'_, '_, S, W> {
    fn write_all(&mut self, chunk: &[u8]) -> Result<(), PayloadWriteRefusal> {
        if self.failure.is_some() {
            return Err(PayloadWriteRefusal::AlreadyFailed);
        }
        let mut next = self.actual;
        let refusal = if (self.output.cancel)() {
            Some((StreamFailureKind::Cancelled, PayloadWriteRefusal::Cancelled))
        } else if let Some(total) = self.actual.checked_add(chunk.len() as u64) {
            next = total;
            if total > self.expected {
                Some((
                    StreamFailureKind::Length {
                        expected: self.expected,
                        actual: total,
                    },
                    PayloadWriteRefusal::Overrun,
                ))
            } else {
                None
            }
        } else {
            Some((
                StreamFailureKind::ArithmeticOverflow,
                PayloadWriteRefusal::Overrun,
            ))
        };
        if let Some((kind, refusal)) = refusal {
            self.failure = Some(StreamFailure {
                phase: self.phase,
                tensor_index: Some(self.index),
                confirmed_bytes: self.output.confirmed,
                kind,
            });
            return Err(refusal);
        }
        if !self.dry {
            if let Err(error) = self.output.write(self.phase, Some(self.index), chunk) {
                let refusal = if matches!(error.kind, StreamFailureKind::Cancelled) {
                    PayloadWriteRefusal::Cancelled
                } else {
                    PayloadWriteRefusal::SinkFailed
                };
                self.failure = Some(error);
                return Err(refusal);
            }
        }
        self.actual = next;
        self.crc.update(chunk);
        Ok(())
    }
}

fn payload<P: PayloadSource, W: ByteSink>(
    source: &mut P,
    index: usize,
    entry: &crate::p64::P64TensorEntry,
    workspace: PayloadWorkspace<'_>,
    output: &mut Output<'_, W>,
    dry: bool,
) -> Result<u32, StreamFailure<P::Error, W::Error>> {
    let phase = if dry {
        StreamPhase::Checksum
    } else {
        StreamPhase::Payload
    };
    if (output.cancel)() {
        return Err(StreamFailure {
            phase,
            tensor_index: Some(index),
            confirmed_bytes: output.confirmed,
            kind: StreamFailureKind::Cancelled,
        });
    }
    let mut bounded = Bounded {
        output,
        expected: entry.blob_size as u64,
        actual: 0,
        crc: Crc32c::new(),
        dry,
        phase,
        index,
        failure: None,
    };
    let result = source.emit(index, entry, workspace, &mut bounded);
    if let Some(failure) = bounded.failure {
        return Err(failure);
    }
    result.map_err(|error| StreamFailure {
        phase,
        tensor_index: Some(index),
        confirmed_bytes: bounded.output.confirmed,
        kind: StreamFailureKind::Source(error),
    })?;
    if bounded.actual != bounded.expected {
        return Err(StreamFailure {
            phase,
            tensor_index: Some(index),
            confirmed_bytes: bounded.output.confirmed,
            kind: StreamFailureKind::Length {
                expected: bounded.expected,
                actual: bounded.actual,
            },
        });
    }
    Ok(bounded.crc.finish())
}
