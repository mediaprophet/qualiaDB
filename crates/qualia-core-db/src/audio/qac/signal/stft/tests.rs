use super::*;
use crate::audio::qac::{
    metadata::{OriginBasis, WindowKind},
    signal::{
        admit,
        writer::{PcmSamples, Provenance, ProvenanceRole, SignalRelation},
        Error, Limits, Rate, Sample, Timing,
    },
};
use crate::q42::{field_envelope as env, field_semantics as sem};
use etalon_signal::audio::{fill_hann_window_into, stft_frame_into};

fn doc<'a>(samples: PcmSamples<'a>) -> StftDocument<'a> {
    StftDocument {
        samples,
        timing: Timing {
            scale: 2,
            epoch_seconds: -5,
            epoch_nanoseconds: 123,
            ticks: Rate::new(96_000, 1).unwrap(),
            samples: Rate::new(48_000_000, 1001).unwrap(),
            start_tick: -3,
        },
        stream_id: 7,
        channels: 2,
        channel_order: "mic-L,mic-R",
        fft_size: 4,
        hop: 2,
        pcm_purpose: "original-acquisition",
        stft_purpose: "hann-complex-stft",
        quantity: StftQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        pcm_provenance: Provenance {
            role: ProvenanceRole::Measured,
            algorithm: None,
            algorithm_version: None,
            instrument_uri: Some("urn:instrument:mic-array"),
            lineage: None,
        },
        stft_provenance: Provenance {
            role: ProvenanceRole::Derived,
            algorithm: Some("urn:etalon:signal:hann-stft-f32"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            lineage: Some("symmetric-hann-radix2-no-padding"),
        },
        original: None,
    }
}

fn encode(document: &StftDocument<'_>, options: StftWriteOptions) -> Result<Vec<u8>, Error> {
    let mut mono = [0.0f32; 64];
    let mut fft = [0.0f32; 128];
    let mut window = [0.0f32; 64];
    let mut spectra = [0u8; 8192];
    let mut metadata = [0u8; 8192];
    let mut payload = [0u8; 8192];
    let mut file = [0u8; 32768];
    let mut output = vec![0xa5u8; 32768];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let res = document.encode_into(
        StftWriteBuffers {
            mono: &mut mono,
            fft: &mut fft,
            window: &mut window,
            spectra: &mut spectra,
            metadata: &mut metadata,
            payload: &mut payload,
            file: &mut file,
        },
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        &mut output,
        options,
    );
    match res {
        Ok(n) => {
            assert!(output[n..].iter().all(|b| *b == 0xa5));
            Ok(output[..n].to_vec())
        }
        Err(err) => {
            assert!(output.iter().all(|b| *b == 0xa5));
            Err(err)
        }
    }
}

fn admit_entry<'a>(view: &env::StructuralView<'a>, index: u32) -> crate::audio::qac::signal::SignalView<'a> {
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    admit(
        view,
        index,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        sem::Limits::default(),
        Limits::default(),
    )
    .unwrap()
}

#[test]
fn produce_and_admit_hann_stft_preserves_pcm_and_exact_etalon_bins() {
    // 7 sample frames x 2 channels = 14 scalars; trailing frame 6 is preserved in PCM.
    let pcm = [
        f32::from_bits(0x80000000),
        0.25,
        0.5,
        -0.5,
        1.0,
        0.75,
        -1.0,
        -0.25,
        0.125,
        0.625,
        -0.375,
        0.875,
        0.3125,
        -0.9375,
    ];
    let d = doc(PcmSamples::F32(&pcm));
    let bytes = encode(&d, StftWriteOptions::default()).unwrap();
    let again = encode(&d, StftWriteOptions::default()).unwrap();
    assert_eq!(bytes, again);

    let view =
        env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
            .unwrap();
    assert_eq!(view.entry_count(), 2);

    let pcm_view = admit_entry(&view, d.pcm_entry_index());
    assert_eq!(pcm_view.mode(), 0x100);
    assert_eq!(pcm_view.records(), 7);
    for frame in 0..7usize {
        for ch in 0..2usize {
            let Sample::F32(got) = pcm_view.sample(frame as u64, ch as u32, 0).unwrap() else {
                panic!()
            };
            assert_eq!(got.to_bits(), pcm[frame * 2 + ch].to_bits());
        }
    }

    let stft_view = admit_entry(&view, d.stft_entry_index());
    assert_eq!(stft_view.mode(), 0x101);
    assert_eq!(stft_view.records(), 2);
    assert_eq!(stft_view.bins(), 4);
    assert_eq!(
        stft_view.timing().unwrap().samples,
        Rate::new(24_000_000, 1001).unwrap()
    );
    let tx = stft_view.transform().unwrap();
    assert_eq!(tx.window_kind(), WindowKind::HannSymmetricF32);
    assert_eq!(tx.origin_basis(), OriginBasis::SourceEntryFirstSample);
    assert_eq!(tx.origin_samples, 0);

    let mut ref_win = [0.0f32; 4];
    fill_hann_window_into(&mut ref_win).unwrap();
    let win_bytes = stft_view.window_coefficients_le().unwrap();
    assert_eq!(win_bytes.len(), 16);
    for (i, chunk) in win_bytes.chunks_exact(4).enumerate() {
        assert_eq!(chunk, &ref_win[i].to_le_bytes());
    }

    let mut frame_buf = [0.0f32; 4];
    let mut spec_buf = [0.0f32; 8];
    for frame in 0..2usize {
        let start = frame * 2;
        for ch in 0..2usize {
            for i in 0..4usize {
                frame_buf[i] = pcm[(start + i) * 2 + ch];
            }
            stft_frame_into(&frame_buf, &mut spec_buf).unwrap();
            for bin in 0..4usize {
                let Sample::ComplexF32 { real, imag } = stft_view
                    .sample(frame as u64, ch as u32, bin as u32)
                    .unwrap()
                else {
                    panic!()
                };
                assert_eq!(real.to_bits(), spec_buf[bin * 2].to_bits());
                assert_eq!(imag.to_bits(), spec_buf[bin * 2 + 1].to_bits());
            }
        }
    }
}

#[test]
fn three_entry_layout_with_opaque_original_and_pressure_units_roundtrips() {
    let orig = b"raw-microphone-capture\0\xff";
    let pcm = [1.0f32, -1.0, 0.5, -0.5, 0.25, -0.25, 0.125, -0.125];
    let mut d = doc(PcmSamples::F32(&pcm));
    d.channels = 1;
    d.channel_order = "mic-mono";
    d.hop = 4;
    d.quantity = StftQuantity::Pressure;
    d.relation = SignalRelation::PhysicalBaseUnits;
    d.original = Some(orig);

    let bytes = encode(&d, StftWriteOptions::default()).unwrap();
    let view =
        env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
            .unwrap();
    assert_eq!(view.entry_count(), 3);
    assert_eq!(admit_entry(&view, 1).as_bytes(), orig);
    let pcm_view = admit_entry(&view, d.pcm_entry_index());
    assert_eq!(pcm_view.quantity(), "pressure");
    assert_eq!(pcm_view.units(), "Pa");
    let stft_view = admit_entry(&view, d.stft_entry_index());
    assert_eq!(stft_view.quantity(), "complex_pressure_spectrum");
    assert_eq!(stft_view.units(), "Pa");
    assert_eq!(stft_view.records(), 2);

    let stft_prov = view.blocks().find(|b| b.id() == 6).unwrap();
    assert_eq!(
        sem::field(stft_prov, 2).unwrap().value(),
        blake3::hash(pcm_view.as_bytes()).as_bytes()
    );
}

#[test]
fn malformed_inputs_and_tight_buffers_preserve_sentinel_output() {
    let good_pcm = [0.25f32, -0.25, 0.5, -0.5, 0.75, -0.75, 1.0, -1.0];
    let good = doc(PcmSamples::F32(&good_pcm));

    let i16s = [1i16, 2, 3, 4];
    let mut bad_scalar = good;
    bad_scalar.samples = PcmSamples::I16(&i16s);
    assert_eq!(encode(&bad_scalar, StftWriteOptions::default()).unwrap_err(), Error::Scalar);

    // Non-finite sample in the untransformed tail still rejects before output mutation.
    let nan_tail = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, f32::NAN, 1.0];
    let mut bad_tail = good;
    bad_tail.samples = PcmSamples::F32(&nan_tail);
    assert_eq!(encode(&bad_tail, StftWriteOptions::default()).unwrap_err(), Error::Nonfinite);

    let mut bad_fft = good;
    bad_fft.fft_size = 3;
    assert_eq!(encode(&bad_fft, StftWriteOptions::default()).unwrap_err(), Error::Shape);

    let mut bad_hop = good;
    bad_hop.hop = 0;
    assert_eq!(encode(&bad_hop, StftWriteOptions::default()).unwrap_err(), Error::Shape);

    let short = [0.1f32, 0.2, 0.3, 0.4];
    let mut bad_short = good;
    bad_short.samples = PcmSamples::F32(&short);
    assert_eq!(encode(&bad_short, StftWriteOptions::default()).unwrap_err(), Error::Shape);

    let mut bad_unit = good;
    bad_unit.relation = SignalRelation::PhysicalBaseUnits;
    assert_eq!(encode(&bad_unit, StftWriteOptions::default()).unwrap_err(), Error::Unit);

    let mut bad_role = good;
    bad_role.stft_provenance.role = ProvenanceRole::Measured;
    assert_eq!(encode(&bad_role, StftWriteOptions::default()).unwrap_err(), Error::Profile);

    let mut bad_alg = good;
    bad_alg.stft_provenance.algorithm = None;
    assert_eq!(encode(&bad_alg, StftWriteOptions::default()).unwrap_err(), Error::Profile);

    // Check output sentinel preservation under tight caller buffer sizes.
    let mut mono = [0.0f32; 2];
    let mut fft = [0.0f32; 8];
    let mut window = [0.0f32; 4];
    let mut spectra = [0u8; 256];
    let mut metadata = [0u8; 2048];
    let mut payload = [0u8; 256];
    let mut file = [0u8; 8192];
    let mut out = [0xa5u8; 8192];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    assert!(good
        .encode_into(
            StftWriteBuffers {
                mono: &mut mono,
                fft: &mut fft,
                window: &mut window,
                spectra: &mut spectra,
                metadata: &mut metadata,
                payload: &mut payload,
                file: &mut file,
            },
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e,
            },
            &mut out,
            StftWriteOptions::default(),
        )
        .is_err());
    assert!(out.iter().all(|b| *b == 0xa5));
}

#[test]
fn large_prime_rate_divided_by_roundtrips_hann_stft_and_rejects_bad_window() {
    let pcm = [0.25f32, -0.5, 0.75, -0.25, 0.5, -0.75];
    let mut d = doc(PcmSamples::F32(&pcm));
    d.channels = 1;
    d.channel_order = "mic-mono";
    d.fft_size = 4;
    d.hop = 1_000_000_007;
    d.timing.samples = Rate::new(1_000_000_007 * 999_983, 999_999_999_989).unwrap();

    let bytes = encode(&d, StftWriteOptions::default()).unwrap();
    let view =
        env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
            .unwrap();
    let stft_view = admit_entry(&view, d.stft_entry_index());
    assert_eq!(stft_view.records(), 1);
    assert_eq!(
        stft_view.timing().unwrap().samples,
        Rate {
            numerator: 999_983,
            denominator: 999_999_999_989,
        }
    );
}
