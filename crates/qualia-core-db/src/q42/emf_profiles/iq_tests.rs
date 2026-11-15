use super::super::field_envelope::{Family, Limits as StoreLimits, StructuralView};
use super::super::field_semantics::NodeState;
use super::*;

fn test_clock() -> FieldClock {
    FieldClock {
        scale: 0,
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks_per_second_num: 4000,
        ticks_per_second_den: 1,
    }
}

fn test_frame() -> FieldFrame {
    FieldFrame {
        origin_m: [0.0, 0.0, 0.0],
        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        grid: None,
    }
}

fn test_prov() -> FieldProvenance<'static> {
    FieldProvenance {
        role: 0,
        algorithm: Some("sdr-iq-capture"),
        algorithm_version: Some("1.0"),
        lineage: Some("test-e05"),
    }
}

fn test_cal(validity: Option<[i64; 2]>, unit: &'static str) -> CalibrationSpec<'static> {
    CalibrationSpec {
        instrument_identifier: "rf-rx-ch0",
        content_digest: [0x5a; 32],
        quantity_unit: unit,
        scale: 1.0,
        offset: 0.0,
        uncertainty: Some((0.05, "dB_1sigma")),
        validity_ticks: validity,
    }
}

#[test]
fn iq_stride_bit_exact_preservation_crc32c_and_padding_rejects() {
    assert_eq!(iq_record_stride(1).unwrap(), 128);
    assert_eq!(iq_record_stride(4).unwrap(), 128);
    assert_eq!(iq_record_stride(5).unwrap(), 192);
    assert_eq!(iq_record_stride(0), Err(Error::Shape));
    assert_eq!(iq_record_stride(MAX_IQ_SAMPLES + 1), Err(Error::Shape));

    // Include -0.0, MIN_POSITIVE, and irregular IEEE-754 patterns to verify zero mutation.
    let samples = [
        (-0.0, f64::MIN_POSITIVE),
        (1.234_567_890_123_456_7, -9.876_543_210_987_654),
        (0.0, 1.0e-200),
        (-3.141592653589793, 2.718281828459045),
        (42.0, -42.0),
    ];
    let hdr = IqHeader {
        first_sample_tick: 120,
        sample_count: 5,
        channel_index: 1,
        centre_hz: 2.4e9,
        lo_phase_rad: 0.5,
        lo_phase_known: true,
        units_per_sample: 0.25,
        calibration_metadata_id: 6,
        acquisition_metadata_id: 7,
        flags: FLAG_LO_PHASE_KNOWN,
        record_crc32c: 0,
    };
    let mut buf = [0u8; 192];
    let written = encode_iq_packet(&hdr, &samples, true, &mut buf).unwrap();
    assert_eq!(written, 192);

    let view = IqPacketView::parse(&buf, true).unwrap().unwrap();
    assert_eq!(view.raw_sample_bytes().len(), 80);
    assert_eq!(view.raw_sample(0).unwrap().0.to_bits(), (-0.0f64).to_bits());
    assert_eq!(view.calibrated_sample(4).unwrap(), (10.5, -10.5));

    let mut out_inter = [(0.0, 0.0); 5];
    view.extract_interleaved(false, &mut out_inter).unwrap();
    assert_eq!(out_inter[1], samples[1]);
    let mut out_i = [0.0; 5];
    let mut out_q = [0.0; 5];
    view.extract_split(true, &mut out_i, &mut out_q).unwrap();
    assert_eq!(out_i[4], 10.5);
    assert_eq!(out_q[4], -10.5);

    // Tamper CRC or sample byte -> Error::Reserved.
    let mut bad_crc = buf;
    bad_crc[70] ^= 0x01;
    assert_eq!(IqPacketView::parse(&bad_crc, true).unwrap_err(), Error::Reserved);

    // Dirty trailing alignment padding -> Error::Reserved even if CRC recomputed!
    let mut bad_pad = buf;
    bad_pad[191] = 1;
    let new_crc = iq_record_crc32c(&bad_pad).unwrap();
    bad_pad[60..64].copy_from_slice(&new_crc.to_le_bytes());
    assert_eq!(IqPacketView::parse(&bad_pad, true).unwrap_err(), Error::Reserved);

    // LO phase unknown with -0.0 in lo_phase_rad rejects Error::Reserved.
    let bad_lo_hdr = IqHeader {
        lo_phase_rad: -0.0,
        lo_phase_known: false,
        flags: 0,
        ..hdr
    };
    assert_eq!(
        encode_iq_packet(&bad_lo_hdr, &samples, true, &mut buf),
        Err(Error::Reserved)
    );

    // Invalid (absent) record check.
    let mut inv_buf = [0u8; 192];
    let inv_hdr = IqHeader { lo_phase_rad: 0.0, flags: 0, lo_phase_known: false, ..hdr };
    encode_iq_packet(&inv_hdr, &samples, false, &mut inv_buf).unwrap();
    assert!(IqPacketView::parse(&inv_buf, false).unwrap().is_none());
}

#[test]
fn iq_carrier_sign_reconstruction_dft_idft_hann_and_signed_bins_oracle() {
    // [1, i, -1, -i] at Fs=4000Hz encodes f_c - 1000Hz under I_plus_iQ_exp_minus_iwt.
    let iq = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
    let fs = 4000.0;
    let fc = 10_000.0;
    let mut rf_minus = [0.0; 4];
    reconstruct_rf_real_signal(&iq, fs, fc, 0.0, 0.0, HarmonicSign::MinusIwt, &mut rf_minus).unwrap();
    for (n, &val) in rf_minus.iter().enumerate() {
        let t = (n as f64) / fs;
        let expected = (2.0 * core::f64::consts::PI * (fc - 1000.0) * t).cos();
        assert!((val - expected).abs() < 1.0e-12);
    }

    // Conjugation switches to +iwt preserving identical physical RF signal.
    let mut conj = [(0.0, 0.0); 4];
    let mut rf_plus = [0.0; 4];
    convert_carrier_sign_conjugate(&iq, &mut conj).unwrap();
    reconstruct_rf_real_signal(&conj, fs, fc, 0.0, 0.0, HarmonicSign::PlusIwt, &mut rf_plus).unwrap();
    for i in 0..4 {
        assert!((rf_minus[i] - rf_plus[i]).abs() < 1.0e-12);
    }

    // Forward unscaled DFT + 1/N IDFT roundtrip; bin 1 holds all energy (4, 0).
    let mut bins = [(0.0, 0.0); 4];
    iq_dft_forward_unscaled(&iq, None, &mut bins).unwrap();
    assert!((bins[1].0 - 4.0).abs() < 1.0e-12 && bins[1].1.abs() < 1.0e-12);
    assert!(bins[0].0.abs() < 1.0e-12 && bins[2].0.abs() < 1.0e-12 && bins[3].0.abs() < 1.0e-12);

    let mut rec = [(0.0, 0.0); 4];
    iq_idft_scaled(&bins, &mut rec).unwrap();
    for i in 0..4 {
        assert!((rec[i].0 - iq[i].0).abs() < 1.0e-12);
        assert!((rec[i].1 - iq[i].1).abs() < 1.0e-12);
    }

    // Even N=4 and Odd N=5 physical frequency ordering without mutating raw FFT order.
    let mut pf4 = [0.0; 4];
    let mut pb4 = [(0.0, 0.0); 4];
    fft_bins_to_ascending_physical(&bins, fs, fc, HarmonicSign::MinusIwt, &mut pf4, &mut pb4).unwrap();
    assert_eq!(pf4, [9000.0, 10_000.0, 11_000.0, 12_000.0]);
    assert!((pb4[0].0 - 4.0).abs() < 1.0e-12);

    let bins5 = [(1.0, 0.0), (2.0, 0.0), (3.0, 0.0), (4.0, 0.0), (5.0, 0.0)];
    let mut pf5 = [0.0; 5];
    let mut pb5 = [(0.0, 0.0); 5];
    fft_bins_to_ascending_physical(&bins5, 5000.0, 10_000.0, HarmonicSign::PlusIwt, &mut pf5, &mut pb5)
        .unwrap();
    assert_eq!(pf5, [8000.0, 9000.0, 10_000.0, 11_000.0, 12_000.0]);

    // Non-positive mapped RF frequency must fail closed.
    assert_eq!(
        fft_bins_to_ascending_physical(&bins, 4000.0, 1000.0, HarmonicSign::PlusIwt, &mut pf4, &mut pb4),
        Err(Error::Value)
    );

    // Hann weights & complex Nyquist aliasing check.
    assert!((window_weight(WindowKind::Hann, 0, 4) - 0.0).abs() < 1.0e-15);
    assert!((window_weight(WindowKind::Hann, 2, 4) - 1.0).abs() < 1.0e-15);
    assert_eq!(check_complex_nyquist_band(4000.0, 4500.0, false), Err(Error::Value));
    check_complex_nyquist_band(4000.0, 4500.0, true).unwrap();
}

#[test]
fn iq_0x0105_and_candidate_5_envelope_roundtrip_and_calibration_tick_bounds() {
    let samples = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
    let stride = iq_record_stride(4).unwrap();
    let mut payload = vec![0u8; stride];
    let hdr = IqHeader {
        first_sample_tick: 100,
        sample_count: 4,
        channel_index: 0,
        centre_hz: 2.45e9,
        lo_phase_rad: 0.0,
        lo_phase_known: false,
        units_per_sample: 1.0,
        calibration_metadata_id: 6,
        acquisition_metadata_id: 7,
        flags: 0,
        record_crc32c: 0,
    };
    encode_iq_packet(&hdr, &samples, true, &mut payload).unwrap();

    let spec = ExtendedModeSpec::Iq {
        acquisition: AcquisitionSpec {
            sample_rate_num: 4000,
            sample_rate_den: 1,
            signal_kind: "complex_stft",
            unit: "V",
            complex_convention: IQ_CONVENTION,
            window: Some("hann"),
            fft_size: Some(4),
            hop: Some(2),
            fft_normalization: Some(STFT_FFT_NORMALIZATION),
            carrier_hz: Some(2.45e9),
            channel_count: Some(2),
        },
        calibration: test_cal(Some([100, 105]), "V"),
        samples_per_packet: 4,
    };

    for mode_id in [0x0105u16, 5u16] {
        let mut mb = [0u8; 4096];
        let mut st = [0u8; 8192];
        let mut out = [0u8; 8192];
        let mut md = [NodeState::default(); 32];
        let mut ed = [NodeState::default(); 8];
        let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
        let written = write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            mode_id,
            Some(4),
            100,
            1,
            None,
            1,
            &payload,
            &spec,
            &mut mb,
            &mut st,
            &mut out,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap();

        let view = StructuralView::open(written, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 65536).unwrap();
        let chunk = reader.chunk(1).unwrap();
        let pkt = chunk.iq(0).unwrap().unwrap();
        assert_eq!(pkt.header.centre_hz, 2.45e9);
        assert_eq!(pkt.raw_sample(1).unwrap(), (0.0, 1.0));
        assert!(chunk.spectrum(0).is_err());
    }

    // Half-open [100, 105): tick 105 must be rejected during admission!
    let mut out_bad = [0u8; 8192];
    let mut bad_payload = vec![0u8; stride];
    let bad_hdr = IqHeader { first_sample_tick: 105, ..hdr };
    encode_iq_packet(&bad_hdr, &samples, true, &mut bad_payload).unwrap();
    let mut mb = [0u8; 4096];
    let mut st = [0u8; 8192];
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    assert_eq!(
        write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            0x0105,
            Some(4),
            105,
            1,
            None,
            1,
            &bad_payload,
            &spec,
            &mut mb,
            &mut st,
            &mut out_bad,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap_err(),
        Error::Time
    );

    // Channel index out of bounds rejects Error::Index.
    let oob_hdr = IqHeader { channel_index: 2, ..hdr };
    encode_iq_packet(&oob_hdr, &samples, true, &mut bad_payload).unwrap();
    assert_eq!(
        write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            0x0105,
            Some(4),
            100,
            1,
            None,
            1,
            &bad_payload,
            &spec,
            &mut mb,
            &mut st,
            &mut out_bad,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap_err(),
        Error::Index
    );
}
