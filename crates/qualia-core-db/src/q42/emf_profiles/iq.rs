//! Packet E05: Calibrated RF I/Q baseband and STFT packets (mode 0x0105 / calibrated_iq_stft_v1)
//! with bit-exact sample preservation, per-record CRC-32C, carrier reconstruction & DFT/STFT.

use super::schemas::{i64_at, SignalKind, WindowKind};
use super::yee::HarmonicSign;
use super::*;
use crate::container_10d::crc32c::crc32c_update;
use core::f64::consts::PI;

pub const IQ_PROFILE: &str = "calibrated_iq_stft_v1";
pub const IQ_MODEL_URI: &str = "urn:qualia:emf:calibrated_iq_stft_v1";
pub const IQ_CONVENTION: &str = "I_plus_iQ_exp_minus_iwt";
pub const STFT_FFT_NORMALIZATION: &str = "forward-unscaled-inverse-1/N";
pub const MAX_IQ_SAMPLES: u32 = 4096;
pub const FLAG_LO_PHASE_KNOWN: u32 = 1 << 0;

#[inline]
pub fn iq_record_stride(sample_count: u32) -> Result<usize, Error> {
    if !(1..=MAX_IQ_SAMPLES).contains(&sample_count) {
        return Err(Error::Shape);
    }
    let payload_bytes = (sample_count as usize)
        .checked_mul(16)
        .ok_or(Error::Overflow)?;
    let aligned = (payload_bytes + 63) & !63;
    64usize.checked_add(aligned).ok_or(Error::Overflow)
}

pub fn iq_record_crc32c(record_bytes: &[u8]) -> Result<u32, Error> {
    if record_bytes.len() < 80 {
        return Err(Error::Shape);
    }
    let crc = crc32c_update(0xFFFF_FFFF, &record_bytes[..60]);
    let crc = crc32c_update(crc, &[0u8; 4]);
    Ok(!crc32c_update(crc, &record_bytes[64..]))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IqHeader {
    pub first_sample_tick: i64,
    pub sample_count: u32,
    pub channel_index: u32,
    pub centre_hz: f64,
    pub lo_phase_rad: f64,
    pub lo_phase_known: bool,
    pub units_per_sample: f64,
    pub calibration_metadata_id: u32,
    pub acquisition_metadata_id: u32,
    pub flags: u32,
    pub record_crc32c: u32,
}

/// Non-mutating zero-copy view over an admitted mode 0x0105 record preserving original bits.
#[derive(Clone, Copy, Debug)]
pub struct IqPacketView<'a> {
    pub header: IqHeader,
    raw_iq_bytes: &'a [u8],
}

impl<'a> IqPacketView<'a> {
    pub fn parse(bytes: &'a [u8], valid: bool) -> Result<Option<Self>, Error> {
        if bytes.len() < 80 || !bytes.len().is_multiple_of(64) {
            return Err(Error::Shape);
        }
        let n = u32_at(bytes, 8);
        let expected_stride = iq_record_stride(n)?;
        if bytes.len() != expected_stride {
            return Err(Error::Shape);
        }
        zeros(&bytes[52..60])?;
        let data_end = 64 + (n as usize) * 16;
        zeros(&bytes[data_end..])?;
        let stored_crc = u32_at(bytes, 60);
        if iq_record_crc32c(bytes)? != stored_crc {
            return Err(Error::Reserved);
        }
        if !valid {
            zeros(&bytes[16..60])?;
            zeros(&bytes[64..])?;
            return Ok(None);
        }
        let flags = u32_at(bytes, 48);
        if flags & !FLAG_LO_PHASE_KNOWN != 0 {
            return Err(Error::Flags);
        }
        finite(bytes, 16, 40)?;
        let centre_hz = f64_at(bytes, 16);
        let lo_phase_known = flags & FLAG_LO_PHASE_KNOWN != 0;
        let lo_phase_rad = if lo_phase_known {
            f64_at(bytes, 24)
        } else {
            zeros(&bytes[24..32])?;
            0.0
        };
        let units_per_sample = f64_at(bytes, 32);
        if centre_hz < 0.0 || units_per_sample <= 0.0 {
            return Err(Error::Value);
        }
        let raw_iq_bytes = &bytes[64..data_end];
        for pair in raw_iq_bytes.chunks_exact(16) {
            if !f64_at(pair, 0).is_finite() || !f64_at(pair, 8).is_finite() {
                return Err(Error::NonFinite);
            }
        }
        Ok(Some(Self {
            header: IqHeader {
                first_sample_tick: i64_at(bytes, 0),
                sample_count: n,
                channel_index: u32_at(bytes, 12),
                centre_hz,
                lo_phase_rad,
                lo_phase_known,
                units_per_sample,
                calibration_metadata_id: u32_at(bytes, 40),
                acquisition_metadata_id: u32_at(bytes, 44),
                flags,
                record_crc32c: stored_crc,
            },
            raw_iq_bytes,
        }))
    }

    /// Returns exact unmodified IEEE-754 little-endian `(I, Q)` sample payload bytes.
    pub fn raw_sample_bytes(&self) -> &'a [u8] {
        self.raw_iq_bytes
    }

    pub fn raw_sample(&self, index: u32) -> Result<(f64, f64), Error> {
        if index >= self.header.sample_count {
            return Err(Error::Index);
        }
        let at = index as usize * 16;
        Ok((f64_at(self.raw_iq_bytes, at), f64_at(self.raw_iq_bytes, at + 8)))
    }

    pub fn calibrated_sample(&self, index: u32) -> Result<(f64, f64), Error> {
        let (i, q) = self.raw_sample(index)?;
        let s = self.header.units_per_sample;
        Ok((i * s, q * s))
    }

    pub fn extract_interleaved(&self, apply_scale: bool, out: &mut [(f64, f64)]) -> Result<usize, Error> {
        let n = self.header.sample_count as usize;
        if out.len() < n {
            return Err(Error::Shape);
        }
        let s = if apply_scale { self.header.units_per_sample } else { 1.0 };
        for (k, chunk) in self.raw_iq_bytes.chunks_exact(16).enumerate() {
            out[k] = (f64_at(chunk, 0) * s, f64_at(chunk, 8) * s);
        }
        Ok(n)
    }

    pub fn extract_split(
        &self,
        apply_scale: bool,
        out_i: &mut [f64],
        out_q: &mut [f64],
    ) -> Result<usize, Error> {
        let n = self.header.sample_count as usize;
        if out_i.len() < n || out_q.len() < n {
            return Err(Error::Shape);
        }
        let s = if apply_scale { self.header.units_per_sample } else { 1.0 };
        for (k, chunk) in self.raw_iq_bytes.chunks_exact(16).enumerate() {
            out_i[k] = f64_at(chunk, 0) * s;
            out_q[k] = f64_at(chunk, 8) * s;
        }
        Ok(n)
    }

    /// Verifies record metadata binding against Acquisition (136) and Calibration (107) blocks,
    /// checking channel bounds, carrier consistency, tick validity `[from, until)`, and sample scale.
    pub fn validate_against_blocks(
        &self,
        ext: Block<'_>,
        acq: Block<'_>,
        cal: Block<'_>,
        raw_count_mode: bool,
    ) -> Result<(), Error> {
        if acq.schema_type() != 136 || cal.schema_type() != 107 {
            return Err(Error::Shared(semantics::Error::ReferenceSchema));
        }
        if self.header.acquisition_metadata_id != acq.id()
            || self.header.calibration_metadata_id != cal.id()
        {
            return Err(Error::Descriptor);
        }
        if scalar32(acq, 6)? != cal.id() {
            return Err(Error::Descriptor);
        }
        if let Some(ext_n) = field(ext, 18) {
            if u32_at(ext_n.value(), 0) != self.header.sample_count {
                return Err(Error::Shape);
            }
        }
        if let Some(ch_count) = field(acq, 13) {
            if self.header.channel_index >= u32_at(ch_count.value(), 0) {
                return Err(Error::Index);
            }
        }
        if let Some(c_hz) = field(acq, 12) {
            let expected_fc = f64_at(c_hz.value(), 0);
            if (expected_fc - self.header.centre_hz).abs() > 1.0e-6 * expected_fc.max(1.0) {
                return Err(Error::Value);
            }
        }
        let kind = SignalKind::parse(text(required(acq, 4)?)?)?;
        if kind == SignalKind::ComplexStft {
            let fft_size = u32_at(required(acq, 9)?.value(), 0);
            if self.header.sample_count != fft_size {
                return Err(Error::Shape);
            }
        }
        if !raw_count_mode && text(required(acq, 5)?)? != "counts" {
            // Pre-calibrated packets carry units_per_sample == 1.0 with calibration as lineage.
            if (self.header.units_per_sample - 1.0).abs() > 1.0e-12 {
                return Err(Error::Value);
            }
        }
        if let Some(vt) = field(cal, 8) {
            let from = i64_at(vt.value(), 0);
            let until = i64_at(vt.value(), 8);
            let t = self.header.first_sample_tick;
            if t < from || t >= until {
                return Err(Error::Time);
            }
        }
        Ok(())
    }
}

pub fn encode_iq_packet(
    header: &IqHeader,
    iq_samples: &[(f64, f64)],
    valid: bool,
    out: &mut [u8],
) -> Result<usize, Error> {
    let n = iq_samples.len();
    let count_u32 = u32::try_from(n).map_err(|_| Error::Shape)?;
    if count_u32 != header.sample_count {
        return Err(Error::Shape);
    }
    let stride = iq_record_stride(count_u32)?;
    if out.len() < stride {
        return Err(Error::Shape);
    }
    let buf = &mut out[..stride];
    buf.fill(0);
    buf[0..8].copy_from_slice(&header.first_sample_tick.to_le_bytes());
    buf[8..12].copy_from_slice(&count_u32.to_le_bytes());
    buf[12..16].copy_from_slice(&header.channel_index.to_le_bytes());
    if valid {
        if !header.centre_hz.is_finite()
            || !header.lo_phase_rad.is_finite()
            || !header.units_per_sample.is_finite()
        {
            return Err(Error::NonFinite);
        }
        if header.centre_hz < 0.0 || header.units_per_sample <= 0.0 {
            return Err(Error::Value);
        }
        if header.flags & !FLAG_LO_PHASE_KNOWN != 0 {
            return Err(Error::Flags);
        }
        let lo_phase = if header.flags & FLAG_LO_PHASE_KNOWN != 0 {
            header.lo_phase_rad
        } else {
            if header.lo_phase_rad.to_bits() != 0 {
                return Err(Error::Reserved);
            }
            0.0
        };
        for &(i_v, q_v) in iq_samples {
            if !i_v.is_finite() || !q_v.is_finite() {
                return Err(Error::NonFinite);
            }
        }
        buf[16..24].copy_from_slice(&header.centre_hz.to_le_bytes());
        buf[24..32].copy_from_slice(&lo_phase.to_le_bytes());
        buf[32..40].copy_from_slice(&header.units_per_sample.to_le_bytes());
        buf[40..44].copy_from_slice(&header.calibration_metadata_id.to_le_bytes());
        buf[44..48].copy_from_slice(&header.acquisition_metadata_id.to_le_bytes());
        buf[48..52].copy_from_slice(&header.flags.to_le_bytes());
        for (idx, &(i_v, q_v)) in iq_samples.iter().enumerate() {
            let at = 64 + idx * 16;
            buf[at..at + 8].copy_from_slice(&i_v.to_le_bytes());
            buf[at + 8..at + 16].copy_from_slice(&q_v.to_le_bytes());
        }
    }
    let crc = iq_record_crc32c(buf)?;
    buf[60..64].copy_from_slice(&crc.to_le_bytes());
    Ok(stride)
}

/// Evaluates `s(t_n) = Re(z_n * exp(i * s * (2pi * f_c * (t_n - t_0) + phi_0)))`.
/// Under canonical `HarmonicSign::MinusIwt`, `s = -1`: `I cos(theta) + Q sin(theta)`.
pub fn reconstruct_rf_real_signal(
    iq: &[(f64, f64)],
    sample_rate_hz: f64,
    carrier_hz: f64,
    lo_phase_rad: f64,
    t0_s: f64,
    sign: HarmonicSign,
    out_real: &mut [f64],
) -> Result<(), Error> {
    if !sample_rate_hz.is_finite()
        || !carrier_hz.is_finite()
        || !lo_phase_rad.is_finite()
        || !t0_s.is_finite()
    {
        return Err(Error::NonFinite);
    }
    if sample_rate_hz <= 0.0 || carrier_hz < 0.0 {
        return Err(Error::Value);
    }
    if out_real.len() < iq.len() {
        return Err(Error::Shape);
    }
    let s = sign.sign_factor();
    for (n, &(i_v, q_v)) in iq.iter().enumerate() {
        if !i_v.is_finite() || !q_v.is_finite() {
            return Err(Error::NonFinite);
        }
        let dt = (n as f64) / sample_rate_hz - t0_s;
        let phase = s * (2.0 * PI * carrier_hz * dt + lo_phase_rad);
        let (sin_p, cos_p) = phase.sin_cos();
        out_real[n] = i_v * cos_p - q_v * sin_p;
    }
    Ok(())
}

/// Explicitly converts complex baseband between opposite RF carrier-sign conventions (`z -> conj(z)`).
pub fn convert_carrier_sign_conjugate(src: &[(f64, f64)], dst: &mut [(f64, f64)]) -> Result<(), Error> {
    if dst.len() < src.len() {
        return Err(Error::Shape);
    }
    for (idx, &(i_v, q_v)) in src.iter().enumerate() {
        if !i_v.is_finite() || !q_v.is_finite() {
            return Err(Error::NonFinite);
        }
        dst[idx] = (i_v, -q_v);
    }
    Ok(())
}

#[inline]
pub fn window_weight(kind: WindowKind, sample_idx: usize, length: usize) -> f64 {
    match kind {
        WindowKind::Rectangular => 1.0,
        WindowKind::Hann => {
            if length == 0 {
                0.0
            } else {
                0.5 * (1.0 - (2.0 * PI * (sample_idx as f64) / (length as f64)).cos())
            }
        }
    }
}

/// Forward DFT preserving raw signed FFT bin ordering (`forward-unscaled-inverse-1/N`).
/// Does NOT sort bins or falsely attach schema 132 to signed baseband offsets.
pub fn iq_dft_forward_unscaled(
    samples: &[(f64, f64)],
    window: Option<WindowKind>,
    out_bins: &mut [(f64, f64)],
) -> Result<(), Error> {
    let n = samples.len();
    if n == 0 || n > MAX_IQ_SAMPLES as usize || out_bins.len() < n {
        return Err(Error::Shape);
    }
    for &(i_v, q_v) in samples {
        if !i_v.is_finite() || !q_v.is_finite() {
            return Err(Error::NonFinite);
        }
    }
    let nf = n as f64;
    for (k, bin) in out_bins[..n].iter_mut().enumerate() {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (m, &(i_v, q_v)) in samples.iter().enumerate() {
            let w = window.map_or(1.0, |wk| window_weight(wk, m, n));
            let angle = -2.0 * PI * ((k * m) as f64) / nf;
            let (s, c) = angle.sin_cos();
            let xr = i_v * w;
            let xi = q_v * w;
            re += xr * c - xi * s;
            im += xr * s + xi * c;
        }
        *bin = (re, im);
    }
    Ok(())
}

/// Inverse DFT with exact `1/N` normalization matching `forward-unscaled-inverse-1/N`.
pub fn iq_idft_scaled(bins: &[(f64, f64)], out_samples: &mut [(f64, f64)]) -> Result<(), Error> {
    let n = bins.len();
    if n == 0 || n > MAX_IQ_SAMPLES as usize || out_samples.len() < n {
        return Err(Error::Shape);
    }
    for &(re, im) in bins {
        if !re.is_finite() || !im.is_finite() {
            return Err(Error::NonFinite);
        }
    }
    let nf = n as f64;
    for (m, sample) in out_samples[..n].iter_mut().enumerate() {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (k, &(br, bi)) in bins.iter().enumerate() {
            let angle = 2.0 * PI * ((k * m) as f64) / nf;
            let (s, c) = angle.sin_cos();
            re += br * c - bi * s;
            im += br * s + bi * c;
        }
        *sample = (re / nf, im / nf);
    }
    Ok(())
}

/// Signed FFT bin baseband frequency offset in Hz without reordering stored bins.
pub fn signed_baseband_offset_hz(
    bin_index: usize,
    fft_size: usize,
    sample_rate_hz: f64,
) -> Result<f64, Error> {
    if !sample_rate_hz.is_finite() {
        return Err(Error::NonFinite);
    }
    if fft_size == 0 || bin_index >= fft_size || sample_rate_hz <= 0.0 {
        return Err(Error::Shape);
    }
    let half = fft_size / 2;
    let signed_k = if fft_size % 2 == 0 {
        if bin_index < half {
            bin_index as isize
        } else {
            bin_index as isize - fft_size as isize
        }
    } else if bin_index <= half {
        bin_index as isize
    } else {
        bin_index as isize - fft_size as isize
    };
    Ok((signed_k as f64) * (sample_rate_hz / fft_size as f64))
}

/// Explicit permutation from native FFT order to strictly ascending physical frequencies (> 0 Hz).
/// Fails closed if any mapped frequency is `<= 0 Hz` or if sign convention is not declared.
pub fn fft_bins_to_ascending_physical(
    bins: &[(f64, f64)],
    sample_rate_hz: f64,
    centre_hz: f64,
    sign: HarmonicSign,
    out_freqs_hz: &mut [f64],
    out_bins: &mut [(f64, f64)],
) -> Result<(), Error> {
    let n = bins.len();
    if n == 0 || out_freqs_hz.len() < n || out_bins.len() < n {
        return Err(Error::Shape);
    }
    if !centre_hz.is_finite() || !sample_rate_hz.is_finite() {
        return Err(Error::NonFinite);
    }
    if centre_hz <= 0.0 || sample_rate_hz <= 0.0 {
        return Err(Error::Value);
    }
    let s = sign.sign_factor();
    let neg_start = n.div_ceil(2);
    let pos_top = (n - 1) / 2;
    // Traverse signed bins in ascending physical frequency order explicitly.
    for out_idx in 0..n {
        let src_bin = match sign {
            HarmonicSign::PlusIwt => (neg_start + out_idx) % n,
            HarmonicSign::MinusIwt => (pos_top + n - out_idx) % n,
        };
        let off = signed_baseband_offset_hz(src_bin, n, sample_rate_hz)?;
        let f_phys = centre_hz + s * off;
        if f_phys <= 0.0 {
            return Err(Error::Value);
        }
        if out_idx > 0 && f_phys <= out_freqs_hz[out_idx - 1] {
            return Err(Error::Value);
        }
        out_freqs_hz[out_idx] = f_phys;
        out_bins[out_idx] = bins[src_bin];
    }
    Ok(())
}

/// Rejects base complex-baseband signals violating Nyquist bound `|bw| <= Fs`
/// unless explicitly admitted under a declared bandpass-sampling profile.
pub fn check_complex_nyquist_band(
    sample_rate_hz: f64,
    occupied_bandwidth_hz: f64,
    bandpass_admitted: bool,
) -> Result<(), Error> {
    if !sample_rate_hz.is_finite() || !occupied_bandwidth_hz.is_finite() {
        return Err(Error::NonFinite);
    }
    if sample_rate_hz <= 0.0 || occupied_bandwidth_hz < 0.0 {
        return Err(Error::Value);
    }
    if !bandpass_admitted && occupied_bandwidth_hz > sample_rate_hz {
        return Err(Error::Value);
    }
    Ok(())
}
