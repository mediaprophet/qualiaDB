use super::*;
use core::sync::atomic::{AtomicBool, Ordering};
use etalon_sense_sdr::{
    acquire_into, adapt_sdr_frame, AppliedTuning, IqByteOrder, OverrunReport, ReaderError,
    RfTuning, SampleRateRatio, SdrObservationMapping, SdrPayload, SdrReadReport, SdrReadRequest,
    SdrReader, TimestampAnchor,
};
struct ReplayReader;
const IQ: [u8; 16] = [
    3, 0, 4, 0, 0xff, 0xff, 0, 0x80, 0, 0, 0xff, 0x7f, 1, 0, 0xfe, 0xff,
];
impl SdrReader for ReplayReader {
    fn reader_id(&self) -> u64 {
        41
    }
    fn read_i16_iq(
        &mut self,
        request: SdrReadRequest,
        cancelled: &AtomicBool,
        output: &mut [u8],
    ) -> Result<SdrReadReport, ReaderError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(ReaderError::Cancelled);
        }
        if output.len() != IQ.len() {
            return Err(ReaderError::Io);
        }
        output.copy_from_slice(&IQ);
        SdrReadReport::new(
            output.len(),
            request.sample_count(),
            None,
            TimestampAnchor::Unknown,
            AppliedTuning::Reported(request.tuning()),
            OverrunReport::NoneReported,
        )
        .and_then(|r| r.with_stream_position(Some(8), Some(2), Some(9)))
        .map_err(|_| ReaderError::Io)
    }
}
#[test]
fn actual_sdr_provider_replay_connects_to_authorized_acquisition() {
    let tuning = RfTuning::new(
        915_000_000,
        SampleRateRatio::new(2_000_000, 1).unwrap(),
        Some(1_500_000),
    )
    .unwrap();
    let request = SdrReadRequest::new(41, tuning, 4, IqByteOrder::LittleEndianIq, 1000).unwrap();
    let mut bytes = [0u8; 16];
    let frame = acquire_into(
        &mut ReplayReader,
        request,
        &AtomicBool::new(false),
        &mut bytes,
    )
    .unwrap();
    let mut output = None;
    adapt_sdr_frame(
        frame,
        SdrObservationMapping {
            source_kind: SourceKind::Synthetic,
            observation_kind: ObservationKind::SyntheticFrame,
            quantity_unit: QuantityUnitIds::new(Some(301), Some(302)).unwrap(),
        },
        &mut output,
    )
    .unwrap();
    let provider = output.unwrap();
    let SdrPayload::InterleavedI16Iq(raw) = provider.payload else {
        panic!("expected raw IQ");
    };
    assert_eq!(raw, IQ);
    let layout =
        SampleLayout::new(1, [4, 0, 0, 0], [4, 0, 0, 0], SampleEncoding::ComplexI16).unwrap();
    let mut m = mapping();
    m.components = a::ComponentOrder::RealImag;
    // Retain reported tuning as carrier; no requested/applied equality is inferred.
    m.signal_axes.carrier = a::Carrier::Hertz(match provider.metadata.applied_tuning {
        AppliedTuning::Reported(t) => t.center_frequency_hz() as f64,
        _ => panic!("replay explicitly reports tuning"),
    });
    let acquired = admit_observation(
        &authority_for(raw, 0),
        7,
        17,
        binding(raw),
        AcquisitionInput {
            observation: provider.envelope,
            layout: &layout,
            original_bytes: raw,
            payload_offset: 0,
            max_original_bytes: 16,
            mapping: m,
        },
    )
    .unwrap();
    assert_eq!(*acquired.observation(), provider.envelope);
    assert_eq!(acquired.observation().sequence(), Some(9));
    assert_eq!(acquired.tensor().region().original().bytes(), IQ);
    assert_eq!(acquired.tensor().numeric([0, 0, 0, 0]).unwrap(), [3.0, 4.0]);
    assert_eq!(
        acquired.tensor().numeric([1, 0, 0, 0]).unwrap(),
        [-1.0, -32768.0]
    );
    assert_eq!(
        acquired.tensor().signal_axes().carrier,
        a::Carrier::Hertz(915_000_000.0)
    );
    assert_eq!(acquired.tensor().clock(), None);
    assert_eq!(provider.metadata.requested_tuning, tuning);
}
