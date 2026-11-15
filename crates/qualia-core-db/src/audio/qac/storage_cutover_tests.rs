//! Current-format .qac v1 storage cutover and complete-entry C3 parity tests.
use super::*;
use crate::audio::qac::metadata::{OriginBasis, TransformKind, WindowKind};
use crate::audio::qac::signal::{
    admit as signal_admit,
    stft::{StftDocument, StftQuantity, StftWriteBuffers, StftWriteOptions},
    writer::{
        PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation,
        WriteBuffers, WriteOptions,
    },
    ExactTime, Limits as SignalLimits, Rate, Sample, Scalar, Timing,
};
use crate::q42::{field_envelope as env, field_semantics as sem};
use crate::sensing::{acquisition as acq, acquisition::storage as acq_storage, SemanticId};

const CANDIDATE_OPAQUE: &[u8] = include_bytes!("../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
const CANDIDATE_PCM_SIGNED_ZERO: &[u8] = include_bytes!("../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac");
const Q4AU_V1: &[u8] = include_bytes!("fixtures/q4au-v1-contract.qac");
const Q4AU_V2: &[u8] = include_bytes!("fixtures/q4au-v2-contract.qac");
const Q4AU_V3: &[u8] = include_bytes!("fixtures/q4au-v3-contract.qac");
const Q4AU_SPELLING: &[u8] = include_bytes!("fixtures/q4au-ascii-spelling.qac");
const EMF_CANDIDATE: &[u8] = include_bytes!("../../q42/field_envelope/fixtures/candidate/emf_candidate-opaque.emf");

fn pcm_doc<'a>(samples: PcmSamples<'a>, quantity: PcmQuantity, relation: SignalRelation, original: Option<&'a [u8]>) -> PcmDocument<'a> {
    PcmDocument {
        samples,
        timing: Timing {
            scale: 2, epoch_seconds: -100, epoch_nanoseconds: 500_000_000,
            ticks: Rate::new(96_000, 1).unwrap(), samples: Rate::new(48_000, 1).unwrap(), start_tick: 48_000,
        },
        stream_id: 7, channels: 1, channel_order: "mic-A", purpose: "original-acquisition",
        quantity, relation, units_per_stored_sample: 1.0, stored_offset: 0.0,
        provenance: Provenance {
            role: ProvenanceRole::Measured, algorithm: None, algorithm_version: None,
            instrument_uri: Some("urn:instrument:mic-7"), lineage: None,
        },
        original,
    }
}

fn encode_pcm(doc: &PcmDocument<'_>) -> Vec<u8> {
    let mut metadata = [0u8; 4096]; let mut payload = [0u8; 8192]; let mut file = [0u8; 16384]; let mut output = vec![0xa5u8; 16384];
    let mut m = [sem::NodeState::default(); 64]; let mut e = [sem::NodeState::default(); 32];
    let n = doc.encode_into(
        WriteBuffers { metadata: &mut metadata, payload: &mut payload, file: &mut file },
        &mut sem::Scratch { metadata: &mut m, entries: &mut e },
        &mut output, WriteOptions::default(),
    ).unwrap();
    output.truncate(n);
    output
}

fn stft_doc<'a>(samples: PcmSamples<'a>, quantity: StftQuantity, relation: SignalRelation) -> StftDocument<'a> {
    StftDocument {
        samples,
        timing: Timing {
            scale: 2, epoch_seconds: 0, epoch_nanoseconds: 0,
            ticks: Rate::new(48_000, 1).unwrap(), samples: Rate::new(48_000, 1).unwrap(), start_tick: 0,
        },
        stream_id: 7, channels: 2, channel_order: "mic-L,mic-R", fft_size: 4, hop: 2,
        pcm_purpose: "original-acquisition", stft_purpose: "hann-complex-stft", quantity, relation,
        pcm_provenance: Provenance {
            role: ProvenanceRole::Measured, algorithm: None, algorithm_version: None,
            instrument_uri: Some("urn:instrument:stereo-mic"), lineage: None,
        },
        stft_provenance: Provenance {
            role: ProvenanceRole::Derived, algorithm: Some("urn:etalon:signal:hann-stft-f32"),
            algorithm_version: Some("1"), instrument_uri: None, lineage: Some("symmetric-hann-radix2-no-padding"),
        },
        original: None,
    }
}

fn encode_stft(doc: &StftDocument<'_>) -> Vec<u8> {
    let mut mono = [0.0f32; 64]; let mut fft = [0.0f32; 128]; let mut window = [0.0f32; 64];
    let mut spectra = [0u8; 8192]; let mut metadata = [0u8; 8192]; let mut payload = [0u8; 8192]; let mut file = [0u8; 32768];
    let mut output = vec![0xa5u8; 32768];
    let mut m = [sem::NodeState::default(); 64]; let mut e = [sem::NodeState::default(); 32];
    let n = doc.encode_into(
        StftWriteBuffers {
            mono: &mut mono, fft: &mut fft, window: &mut window,
            spectra: &mut spectra, metadata: &mut metadata, payload: &mut payload, file: &mut file,
        },
        &mut sem::Scratch { metadata: &mut m, entries: &mut e },
        &mut output, StftWriteOptions::default(),
    ).unwrap();
    output.truncate(n);
    output
}

fn acq_id(n: u64) -> SemanticId { SemanticId::new(n).unwrap() }

fn acq_context() -> acq::Context {
    acq::Context {
        quantity: acq::Quantity { kind: acq_id(11), unit: acq_id(12) },
        stage: acq::SampleStage::RawCodes, sensor: acq_id(13), geometry: None, calibration: None, uncertainty: None,
    }
}

fn acq_axes() -> acq::SignalAxes {
    acq::SignalAxes { carrier: acq::Carrier::Unknown, baseband_hz: None, pulse_rate: None, complex_convention: None, phase_origin: None }
}

fn acq_layout(enc: acq::Encoding, n: u32) -> acq::Layout {
    acq::Layout {
        rank: 1, extents: [n, 0, 0, 0], strides: [enc.bytes(), 0, 0, 0], axes: [acq::Axis::Sample; 4],
        encoding: enc, byte_order: acq::ByteOrder::Little,
        components: if enc.complex() { acq::ComponentOrder::RealImag } else { acq::ComponentOrder::Scalar },
    }
}

fn encode_acq(input: acq_storage::Input<'_>) -> Vec<u8> {
    let mut meta = [0u8; 16384]; let mut staging = vec![0u8; 65536]; let mut output = vec![0xccu8; 65536];
    let mut m = [sem::NodeState::default(); 256]; let mut e = [sem::NodeState::default(); 128];
    let n = acq_storage::encode(
        input, &mut meta, &mut staging,
        &mut sem::Scratch { metadata: &mut m, entries: &mut e },
        &mut output, env::Limits::default(), sem::Limits::default(), acq_storage::Limits::default(),
    ).unwrap();
    output.truncate(n);
    output
}

#[test]
fn mode_0100_pcm_document_exact_clock_units_and_signed_zero_bit_exactness() {
    let native = [f32::from_bits(0x00000000), f32::from_bits(0x80000000), 0.25f32, -0.5f32, 1.0f32, -1.0f32];
    let doc = pcm_doc(PcmSamples::F32(&native), PcmQuantity::Pressure, SignalRelation::PhysicalBaseUnits, None);
    let bytes = encode_pcm(&doc);
    let view = QacStorageView::open(&bytes, QacStorageOptions::default()).unwrap();
    assert_eq!(view.structure().family(), env::Family::QacV1Candidate);
    assert_eq!(view.structure().entry_count(), 1);
    let chunk = view.structure().verify_chunk(1).unwrap();
    assert_eq!(chunk.len(), native.len() * 4);

    let mut m = [sem::NodeState::default(); 64]; let mut e = [sem::NodeState::default(); 32];
    let sig = signal_admit(view.structure(), 1, &mut sem::Scratch { metadata: &mut m, entries: &mut e }, sem::Limits::default(), SignalLimits::default()).unwrap();

    assert_eq!((sig.mode(), sig.quantity(), sig.units(), sig.channels(), sig.stream_id()), (0x0100, "pressure", "Pa", 1, 7));
    assert_eq!(sig.records(), native.len() as u64);
    assert_eq!(sig.timing(), Some(doc.timing));
    assert_eq!(sig.time_at(0).unwrap(), ExactTime { numerator: -99, denominator: 1 });

    let (Sample::F32(p_zero), Sample::F32(n_zero)) = (sig.sample(0, 0, 0).unwrap(), sig.sample(1, 0, 0).unwrap()) else { panic!() };
    let (Sample::F32(v_quarter), Sample::F32(v_half)) = (sig.sample(2, 0, 0).unwrap(), sig.sample(3, 0, 0).unwrap()) else { panic!() };
    assert_eq!((p_zero.to_bits(), n_zero.to_bits()), (0x00000000, 0x80000000));
    assert_ne!(p_zero.to_bits(), n_zero.to_bits());
    assert_eq!((v_quarter.to_bits(), v_half.to_bits()), (0x3e800000, 0xbf000000));

    let fix_view = QacStorageView::open(CANDIDATE_PCM_SIGNED_ZERO, QacStorageOptions::default()).unwrap();
    assert_eq!(fix_view.structure().family(), env::Family::QacV1Candidate);
    let fix_chunk = fix_view.structure().verify_chunk(1).unwrap();
    assert_eq!((&fix_chunk[..4], &fix_chunk[4..8]), (&[0x00, 0x00, 0x00, 0x00][..], &[0x00, 0x00, 0x00, 0x80][..]));

    let doc_dig = pcm_doc(PcmSamples::F32(&native), PcmQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure, None);
    let bytes_dig = encode_pcm(&doc_dig);
    let view_dig = QacStorageView::open(&bytes_dig, QacStorageOptions::default()).unwrap();
    let mut m_dig = [sem::NodeState::default(); 64]; let mut e_dig = [sem::NodeState::default(); 32];
    let sig_dig = signal_admit(view_dig.structure(), 1, &mut sem::Scratch { metadata: &mut m_dig, entries: &mut e_dig }, sem::Limits::default(), SignalLimits::default()).unwrap();
    assert_eq!((sig_dig.mode(), sig_dig.quantity(), sig_dig.units()), (0x0100, "digital_sample", "1"));
}

#[test]
fn mode_0101_hann_stft_with_retained_pcm_entry() {
    let pcm = [0.0f32, -0.0f32, 0.1f32, -0.1f32, 0.2f32, -0.2f32, 0.3f32, -0.3f32, 0.4f32, -0.4f32, 0.5f32, -0.5f32, 0.6f32, -0.6f32, 0.7f32, -0.7f32];
    let doc = stft_doc(PcmSamples::F32(&pcm), StftQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure);
    let bytes = encode_stft(&doc);
    let view = QacStorageView::open(&bytes, QacStorageOptions::default()).unwrap();
    assert_eq!(view.structure().family(), env::Family::QacV1Candidate);
    assert_eq!(view.structure().entry_count(), 2);

    let chunk1 = view.structure().verify_chunk(1).unwrap();
    let chunk2 = view.structure().verify_chunk(2).unwrap();
    assert_eq!(chunk1.len(), pcm.len() * 4);
    assert!(!chunk2.is_empty());

    let mut m = [sem::NodeState::default(); 64]; let mut e = [sem::NodeState::default(); 32];
    let pcm_sig = signal_admit(view.structure(), 1, &mut sem::Scratch { metadata: &mut m, entries: &mut e }, sem::Limits::default(), SignalLimits::default()).unwrap();
    assert_eq!((pcm_sig.mode(), pcm_sig.channels(), pcm_sig.records()), (0x0100, 2, 8));

    let stft_sig = signal_admit(view.structure(), 2, &mut sem::Scratch { metadata: &mut m, entries: &mut e }, sem::Limits::default(), SignalLimits::default()).unwrap();
    assert_eq!((stft_sig.mode(), stft_sig.channels(), stft_sig.bins(), stft_sig.records()), (0x0101, 2, 4, 3));
    assert_eq!((stft_sig.quantity(), stft_sig.units()), ("complex_digital_spectrum", "1"));

    let tx = stft_sig.transform().unwrap();
    assert_eq!((tx.kind, tx.window, tx.origin_basis, tx.fft_size, tx.hop), (TransformKind::Complex, WindowKind::HannSymmetricF32, OriginBasis::SourceEntryFirstSample, 4, 2));

    let Sample::ComplexF32 { real, imag } = stft_sig.sample(0, 0, 0).unwrap() else { panic!() };
    assert!(real.is_finite() && imag.is_finite());
}

#[test]
fn mode_0104_opaque_original_qac_and_acquisition_storage() {
    let opaque = b"OPAQUE_ORIGINAL_TEST_PAYLOAD_WAVE_OR_SENSING_RECORD_12345678";
    let native = [0.125f32; 4];
    let doc = pcm_doc(PcmSamples::F32(&native), PcmQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure, Some(opaque));
    let bytes = encode_pcm(&doc);
    let view = QacStorageView::open(&bytes, QacStorageOptions::default()).unwrap();
    assert_eq!(view.structure().entry_count(), 2);

    let mut m = [sem::NodeState::default(); 64]; let mut e = [sem::NodeState::default(); 32];
    let orig_sig = signal_admit(view.structure(), 1, &mut sem::Scratch { metadata: &mut m, entries: &mut e }, sem::Limits::default(), SignalLimits::default()).unwrap();
    assert_eq!((orig_sig.mode(), orig_sig.scalar(), orig_sig.channels(), orig_sig.records()), (0x0104, Scalar::U8, 1, opaque.len() as u64));
    assert_eq!(orig_sig.as_bytes(), opaque);
    let Sample::U8(b) = orig_sig.sample(0, 0, 0).unwrap() else { panic!() };
    assert_eq!(b, opaque[0]);

    let pcm_sig = signal_admit(view.structure(), 2, &mut sem::Scratch { metadata: &mut m, entries: &mut e }, sem::Limits::default(), SignalLimits::default()).unwrap();
    assert_eq!(pcm_sig.mode(), 0x0100);

    let acq_bytes = [0x51, 0x44, 0x01, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x34, 0x12, 0, 0, 0, 0, 0xf8, 0xff, 0xee, 0xaa];
    let original = acq::Original::admit(&acq_bytes, acq_bytes.len()).unwrap();
    let v = acq::TensorView::admit(original.region(3, 16).unwrap(), acq_layout(acq::Encoding::Shared(Scalar::F64), 2), acq_context(), None, acq_axes()).unwrap();
    let acq_enc = encode_acq(acq_storage::Input { family: env::Family::QacV1Candidate, view: acq_storage::View::Tensor(v), derivation: None });
    let acq_view = QacStorageView::open(&acq_enc, QacStorageOptions::default()).unwrap();
    assert_eq!(acq_view.structure().entry(1).unwrap().mode(), 0x0104);
    let mut originals = [None; 65]; let mut m_acq = [sem::NodeState::default(); 256]; let mut e_acq = [sem::NodeState::default(); 128];
    let acq_doc = acq_storage::admit(
        acq_view.structure(), 1, &mut originals,
        &mut sem::Scratch { metadata: &mut m_acq, entries: &mut e_acq }, sem::Limits::default(), acq_storage::Limits::default(),
    ).unwrap();
    assert_eq!(acq_doc.original(1).unwrap().bytes(), acq_bytes);
    let acq_storage::View::Tensor(t) = acq_doc.view else { panic!() };
    assert_eq!(t.element([0; 4]).unwrap(), &acq_bytes[3..11]);

    let cand_view = QacStorageView::open(CANDIDATE_OPAQUE, QacStorageOptions::default()).unwrap();
    assert_eq!(cand_view.structure().entry(1).unwrap().mode(), 0x0104);
    assert!(cand_view.structure().verify_chunk(1).is_ok());
}

#[test]
fn c3_parity_strict_refusal_of_truncated_and_trailing_garbage_slices() {
    let native = [1.0f32; 8];
    let doc = pcm_doc(PcmSamples::F32(&native), PcmQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure, None);
    let valid = encode_pcm(&doc);
    assert!(QacStorageView::open(&valid, QacStorageOptions::default()).is_ok());

    for len in [0, 1, 16, 64, 127, valid.len() - 32, valid.len() - 1] {
        assert!(QacStorageView::open(&valid[..len], QacStorageOptions::default()).is_err(), "refusal expected for {len}");
    }

    let mut with_garbage = valid.clone();
    with_garbage.extend_from_slice(b"EXTRA_TRAILING_GARBAGE_BYTES_CORRUPT");
    assert_eq!(QacStorageView::open(&with_garbage, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Header));
}

#[test]
fn c3_parity_strict_refusal_of_corrupted_header_and_directory_crc() {
    let native = [0.5f32; 4];
    let doc = pcm_doc(PcmSamples::F32(&native), PcmQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure, None);
    let valid = encode_pcm(&doc);

    let mut bad_header_crc = valid.clone(); bad_header_crc[124] ^= 0x5a;
    assert_eq!(QacStorageView::open(&bad_header_crc, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Crc));

    let mut bad_header_body = valid.clone(); bad_header_body[14] ^= 0x01;
    assert_eq!(QacStorageView::open(&bad_header_body, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Crc));

    let mut bad_dir_crc = valid.clone(); bad_dir_crc[128 + 124] ^= 0xa5;
    assert_eq!(QacStorageView::open(&bad_dir_crc, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Crc));

    let mut bad_dir_body = valid.clone(); bad_dir_body[128 + 4] ^= 0x01;
    assert_eq!(QacStorageView::open(&bad_dir_body, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Crc));
}

#[test]
fn c3_parity_strict_refusal_of_corrupted_chunk_blake3_payload_digest() {
    let native = [0.25f32, -0.25f32, 0.75f32, -0.75f32];
    let doc = pcm_doc(PcmSamples::F32(&native), PcmQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure, None);
    let valid = encode_pcm(&doc);
    let view = QacStorageView::open(&valid, QacStorageOptions::default()).unwrap();
    let offset = view.structure().entry(1).unwrap().payload_offset() as usize;

    let mut corrupted = valid.clone();
    corrupted[offset] ^= 0xff;

    let corrupt_view = QacStorageView::open(&corrupted, QacStorageOptions::default()).unwrap();
    assert_eq!(corrupt_view.structure().verify_chunk(1).unwrap_err(), env::Error::PayloadDigest);

    let mut m = [sem::NodeState::default(); 64]; let mut e = [sem::NodeState::default(); 32];
    let err = signal_admit(corrupt_view.structure(), 1, &mut sem::Scratch { metadata: &mut m, entries: &mut e }, sem::Limits::default(), SignalLimits::default()).unwrap_err();
    assert_eq!(err, crate::audio::qac::signal::Error::Envelope(env::Error::PayloadDigest));
}

#[test]
fn strict_rejection_of_legacy_and_emf_headers_no_implicit_fallback() {
    for fixture in [Q4AU_V1, Q4AU_V2, Q4AU_V3] {
        assert_eq!(QacStorageView::open(fixture, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));
    }
    assert_eq!(QacStorageView::open(Q4AU_SPELLING, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));
    assert_eq!(QacStorageView::open(EMF_CANDIDATE, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));

    let riff = b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00";
    assert_eq!(QacStorageView::open(riff, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));

    let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01";
    assert_eq!(QacStorageView::open(png, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));

    let ua4q_lit = [0x55, 0x41, 0x34, 0x51, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(QacStorageView::open(&ua4q_lit, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));

    let q4au_lit = [0x51, 0x34, 0x41, 0x55, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(QacStorageView::open(&q4au_lit, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));

    let emf_lit = [0x51, 0x34, 0x32, 0x45, 0x4d, 0x46, 0x36, 0x34, 4, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(QacStorageView::open(&emf_lit, QacStorageOptions::default()).unwrap_err(), QacStorageError::Envelope(env::Error::Family));
}
