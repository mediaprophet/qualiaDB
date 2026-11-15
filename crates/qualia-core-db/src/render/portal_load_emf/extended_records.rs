//! Zero-copy validation and typed parsing of frozen EMF v4 extended record layouts
//! (`0x0100..0x0107` / candidate `0..7` per `wire-freeze.md` §4).

use crate::container_10d::crc32c::{crc32c, crc32c_update};
use crate::q42::emf_profiles as profiles;
use crate::render::portal_load_emf::Error;

#[inline]
pub(in crate::render::portal_load_emf) fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
#[inline]
pub(in crate::render::portal_load_emf) fn u64_at(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
#[inline]
pub(in crate::render::portal_load_emf) fn i64_at(b: &[u8], i: usize) -> i64 {
    i64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
#[inline]
pub(in crate::render::portal_load_emf) fn f32_at(b: &[u8], i: usize) -> f32 {
    f32::from_bits(u32_at(b, i))
}
#[inline]
pub(in crate::render::portal_load_emf) fn f64_at(b: &[u8], i: usize) -> f64 {
    f64::from_bits(u64_at(b, i))
}
#[inline]
pub(in crate::render::portal_load_emf) fn triple(b: &[u8], i: usize) -> [f64; 3] {
    [f64_at(b, i), f64_at(b, i + 8), f64_at(b, i + 16)]
}

pub(in crate::render::portal_load_emf) fn finite_f64_range(b: &[u8], start: usize, end: usize) -> Result<(), Error> {
    for i in (start..end).step_by(8) {
        if !f64_at(b, i).is_finite() {
            return Err(Error::Profile(profiles::Error::NonFinite));
        }
    }
    Ok(())
}

pub(in crate::render::portal_load_emf) fn zeros(b: &[u8]) -> Result<(), Error> {
    if b.iter().any(|&v| v != 0) {
        Err(Error::Profile(profiles::Error::Reserved))
    } else {
        Ok(())
    }
}

fn unit_vector(v: [f64; 3]) -> Result<(), Error> {
    let norm = v[0].hypot(v[1]).hypot(v[2]);
    if !norm.is_finite() || (norm - 1.0).abs() > 1.0e-5 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode0Field {
    pub electric_v_per_m: [f64; 3],
    pub magnetic_tesla: [f64; 3],
    pub position_m: [f64; 3],
    pub tick: i64,
    pub carrier_hz: Option<f64>,
    pub phase_rad: Option<f64>,
    pub stored_poynting: Option<[f64; 3]>,
    pub confidence: Option<f32>,
}

pub(in crate::render::portal_load_emf) fn parse_mode0_field(bytes: &[u8], valid: bool) -> Result<Option<Mode0Field>, Error> {
    if bytes.len() != 128 {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    if !valid {
        zeros(&bytes[..72])?;
        zeros(&bytes[80..])?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 120)?;
    let flags = u32_at(bytes, 120);
    if flags & !15 != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    let carrier_hz = if flags & 1 != 0 {
        let f = f64_at(bytes, 80);
        if f <= 0.0 {
            return Err(Error::Profile(profiles::Error::Value));
        }
        Some(f)
    } else {
        zeros(&bytes[80..88])?;
        None
    };
    let phase_rad = if flags & 2 != 0 {
        Some(f64_at(bytes, 88))
    } else {
        zeros(&bytes[88..96])?;
        None
    };
    let stored_poynting = if flags & 4 != 0 {
        Some(triple(bytes, 96))
    } else {
        zeros(&bytes[96..120])?;
        None
    };
    let confidence = if flags & 8 != 0 {
        let c = f32_at(bytes, 124);
        if !c.is_finite() || !(0.0..=1.0).contains(&c) {
            return Err(Error::Profile(profiles::Error::Value));
        }
        Some(c)
    } else {
        zeros(&bytes[124..128])?;
        None
    };
    Ok(Some(Mode0Field {
        electric_v_per_m: triple(bytes, 0),
        magnetic_tesla: triple(bytes, 24),
        position_m: triple(bytes, 48),
        tick: i64_at(bytes, 72),
        carrier_hz,
        phase_rad,
        stored_poynting,
        confidence,
    }))
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode1Ray {
    pub position_m: [f64; 3],
    pub direction: [f64; 3],
    pub stokes: [f64; 4],
    pub carrier_hz: f64,
    pub path_m: f64,
    pub phase_rad: Option<f64>,
}

pub(in crate::render::portal_load_emf) fn parse_mode1_ray(bytes: &[u8], valid: bool) -> Result<Option<Mode1Ray>, Error> {
    if bytes.len() != 128 {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    if !valid {
        zeros(bytes)?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 120)?;
    let flags = u32_at(bytes, 124);
    if flags & !1 != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    let dir = triple(bytes, 24);
    unit_vector(dir)?;
    let s0 = f64_at(bytes, 48);
    let s1 = f64_at(bytes, 56);
    let s2 = f64_at(bytes, 64);
    let s3 = f64_at(bytes, 72);
    let pol = s1.hypot(s2).hypot(s3);
    if s0 < 0.0 || pol > s0 * (1.0 + 1.0e-6) + 1.0e-12 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    let carrier_hz = f64_at(bytes, 80);
    let bandwidth_hz = f64_at(bytes, 88);
    let path_m = f64_at(bytes, 96);
    let div = f64_at(bytes, 112);
    if carrier_hz <= 0.0 || bandwidth_hz < 0.0 || path_m < 0.0 || !(0.0..std::f64::consts::FRAC_PI_2).contains(&div) {
        return Err(Error::Profile(profiles::Error::Value));
    }
    let phase_rad = if flags & 1 != 0 {
        Some(f64_at(bytes, 104))
    } else {
        zeros(&bytes[104..112])?;
        None
    };
    Ok(Some(Mode1Ray {
        position_m: triple(bytes, 0),
        direction: dir,
        stokes: [s0, s1, s2, s3],
        carrier_hz,
        path_m,
        phase_rad,
    }))
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode2Yee {
    pub electric_v_per_m: [f64; 3],
    pub magnetic_h_a_per_m: [f64; 3],
    pub eps_r: f64,
    pub mu_r: f64,
}

pub(in crate::render::portal_load_emf) fn parse_mode2_yee(bytes: &[u8], valid: bool) -> Result<Option<Mode2Yee>, Error> {
    if bytes.len() != 128 {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    if !valid {
        zeros(bytes)?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 112)?;
    let eps_r = f64_at(bytes, 88);
    let mu_r = f64_at(bytes, 96);
    let sigma = f64_at(bytes, 104);
    if eps_r <= 0.0 || mu_r <= 0.0 || sigma < 0.0 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    Ok(Some(Mode2Yee {
        electric_v_per_m: triple(bytes, 0),
        magnetic_h_a_per_m: triple(bytes, 24),
        eps_r,
        mu_r,
    }))
}

pub(in crate::render::portal_load_emf) struct Mode3Spectrum<'a> {
    pub position_m: [f64; 3],
    pub tick: i64,
    pub values: &'a [u8],
    pub bins: u32,
}

impl<'a> Mode3Spectrum<'a> {
    #[inline]
    pub fn bins(&self) -> u32 {
        self.bins
    }
}

pub(in crate::render::portal_load_emf) fn parse_mode3_spectrum<'a>(
    bytes: &'a [u8],
    valid: bool,
    bins: u32,
    expected_axis_id: u32,
) -> Result<Option<Mode3Spectrum<'a>>, Error> {
    let expected_stride = 64usize + (((bins as usize) * 8 + 63) & !63);
    if !(1..=4096).contains(&bins) || bytes.len() != expected_stride {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    zeros(&bytes[52..64])?;
    zeros(&bytes[64 + (bins as usize) * 8..])?;
    if !valid {
        zeros(&bytes[..24])?;
        zeros(&bytes[32..])?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 24)?;
    finite_f64_range(bytes, 40, 48)?;
    if u32_at(bytes, 36) != expected_axis_id {
        return Err(Error::Profile(profiles::Error::Descriptor));
    }
    let flags = u32_at(bytes, 48);
    if flags & !1 != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    if flags & 1 != 0 {
        if f64_at(bytes, 40) <= 0.0 {
            return Err(Error::Profile(profiles::Error::Value));
        }
    } else {
        zeros(&bytes[40..48])?;
    }
    let vals = &bytes[64..64 + (bins as usize) * 8];
    for c in vals.chunks_exact(8) {
        let v = f64::from_le_bytes(c.try_into().unwrap());
        if !v.is_finite() || v < 0.0 {
            return Err(Error::Profile(profiles::Error::Value));
        }
    }
    let spec = Mode3Spectrum {
        position_m: triple(bytes, 0),
        tick: i64_at(bytes, 24),
        values: vals,
        bins,
    };
    let _ = spec.bins();
    Ok(Some(spec))
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode4Mueller {
    pub position_m: [f64; 3],
    pub frequency_hz: f64,
    pub m00: f64,
    pub m10: f64,
    pub m20: f64,
    pub m30: f64,
}

pub(in crate::render::portal_load_emf) fn parse_mode4_mueller(bytes: &[u8], valid: bool) -> Result<Option<Mode4Mueller>, Error> {
    if bytes.len() != 256 {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    zeros(&bytes[88..128])?;
    if !valid {
        zeros(bytes)?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 80)?;
    finite_f64_range(bytes, 128, 256)?;
    if u32_at(bytes, 84) != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    unit_vector(triple(bytes, 24))?;
    unit_vector(triple(bytes, 48))?;
    let freq = f64_at(bytes, 72);
    if freq <= 0.0 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    let m00 = f64_at(bytes, 128);
    let m10 = f64_at(bytes, 128 + 4 * 8);
    let m20 = f64_at(bytes, 128 + 8 * 8);
    let m30 = f64_at(bytes, 128 + 12 * 8);
    if m00 < 0.0 || m10.hypot(m20).hypot(m30) > m00 * (1.0 + 1.0e-6) + 1.0e-12 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    Ok(Some(Mode4Mueller {
        position_m: triple(bytes, 0),
        frequency_hz: freq,
        m00,
        m10,
        m20,
        m30,
    }))
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode5IqSummary {
    pub first_tick: i64,
    pub channel_index: u32,
    pub centre_hz: f64,
    pub rms_v: f64,
    pub mean_phase_rad: f64,
}

pub(in crate::render::portal_load_emf) fn parse_mode5_iq(bytes: &[u8], valid: bool) -> Result<Option<Mode5IqSummary>, Error> {
    if bytes.len() < 80 || !bytes.len().is_multiple_of(64) {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    let n = u32_at(bytes, 8) as usize;
    let expected = 64 + ((16 * n + 63) & !63);
    if !(1..=4096).contains(&n) || bytes.len() != expected {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    zeros(&bytes[52..60])?;
    zeros(&bytes[64 + 16 * n..])?;
    let crc = crc32c_update(0xFFFF_FFFF, &bytes[..60]);
    let crc = crc32c_update(crc, &[0u8; 4]);
    let computed = !crc32c_update(crc, &bytes[64..]);
    if computed != u32_at(bytes, 60) {
        return Err(Error::Profile(profiles::Error::Reserved));
    }
    if !valid {
        zeros(&bytes[16..60])?;
        zeros(&bytes[64..])?;
        return Ok(None);
    }
    let flags = u32_at(bytes, 48);
    if flags & !1 != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    finite_f64_range(bytes, 16, 40)?;
    let centre_hz = f64_at(bytes, 16);
    let lo_phase = if flags & 1 != 0 {
        f64_at(bytes, 24)
    } else {
        zeros(&bytes[24..32])?;
        0.0
    };
    let units_per_sample = f64_at(bytes, 32);
    if centre_hz < 0.0 || units_per_sample <= 0.0 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    let mut sum_sq = 0.0_f64;
    let mut sum_i = 0.0_f64;
    let mut sum_q = 0.0_f64;
    for pair in bytes[64..64 + 16 * n].chunks_exact(16) {
        let i_val = f64_at(pair, 0);
        let q_val = f64_at(pair, 8);
        if !i_val.is_finite() || !q_val.is_finite() {
            return Err(Error::Profile(profiles::Error::NonFinite));
        }
        sum_sq += i_val * i_val + q_val * q_val;
        sum_i += i_val;
        sum_q += q_val;
    }
    let rms_v = (sum_sq / n as f64).sqrt() * units_per_sample;
    let bb_phase = if sum_i == 0.0 && sum_q == 0.0 {
        0.0
    } else {
        sum_q.atan2(sum_i)
    };
    Ok(Some(Mode5IqSummary {
        first_tick: i64_at(bytes, 0),
        channel_index: u32_at(bytes, 12),
        centre_hz,
        rms_v,
        mean_phase_rad: lo_phase + bb_phase,
    }))
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode6Ionizing {
    pub position_m: [f64; 3],
    pub tick: i64,
    pub photon_energy_j: f64,
    pub photon_fluence_per_m2: f64,
    pub absorbed_dose_gy: f64,
}

pub(in crate::render::portal_load_emf) fn parse_mode6_ionizing(bytes: &[u8], valid: bool) -> Result<Option<Mode6Ionizing>, Error> {
    if bytes.len() != 128 {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    zeros(&bytes[112..124])?;
    if crc32c(&bytes[..124]) != u32_at(bytes, 124) {
        return Err(Error::Profile(profiles::Error::Reserved));
    }
    if !valid {
        zeros(&bytes[..24])?;
        zeros(&bytes[32..112])?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 24)?;
    finite_f64_range(bytes, 32, 96)?;
    let vflags = u32_at(bytes, 108);
    if vflags & !0xFF != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    for bit in 0..8 {
        let off = 32 + bit * 8;
        let v = f64_at(bytes, off);
        if (vflags & (1 << bit)) == 0 {
            zeros(&bytes[off..off + 8])?;
        } else if v < 0.0 {
            return Err(Error::Profile(profiles::Error::Value));
        }
    }
    Ok(Some(Mode6Ionizing {
        position_m: triple(bytes, 0),
        tick: i64_at(bytes, 24),
        photon_energy_j: f64_at(bytes, 32),
        photon_fluence_per_m2: f64_at(bytes, 56),
        absorbed_dose_gy: f64_at(bytes, 64),
    }))
}

#[derive(Clone, Copy, Debug)]
pub(in crate::render::portal_load_emf) struct Mode7BioDipole {
    pub position_m: [f64; 3],
    pub dipole_a_m: [f64; 3],
    pub potential_v: f64,
    pub tick: i64,
    pub carrier_hz: f64,
}

pub(in crate::render::portal_load_emf) fn parse_mode7_bio(bytes: &[u8], valid: bool) -> Result<Option<Mode7BioDipole>, Error> {
    if bytes.len() != 128 {
        return Err(Error::Profile(profiles::Error::Shape));
    }
    zeros(&bytes[100..124])?;
    if crc32c(&bytes[..124]) != u32_at(bytes, 124) {
        return Err(Error::Profile(profiles::Error::Reserved));
    }
    if !valid {
        zeros(&bytes[..56])?;
        zeros(&bytes[64..100])?;
        return Ok(None);
    }
    finite_f64_range(bytes, 0, 56)?;
    finite_f64_range(bytes, 64, 80)?;
    let flags = u32_at(bytes, 96);
    if flags & !7 != 0 {
        return Err(Error::Profile(profiles::Error::Flags));
    }
    let potential_v = if flags & 1 != 0 {
        f64_at(bytes, 48)
    } else {
        zeros(&bytes[48..56])?;
        0.0
    };
    let carrier_hz = if flags & 2 != 0 {
        let f = f64_at(bytes, 64);
        if f <= 0.0 {
            return Err(Error::Profile(profiles::Error::Value));
        }
        f
    } else {
        zeros(&bytes[64..72])?;
        0.0
    };
    if flags & 4 != 0 {
        if f64_at(bytes, 72) < 0.0 {
            return Err(Error::Profile(profiles::Error::Value));
        }
    } else {
        zeros(&bytes[72..80])?;
    }
    Ok(Some(Mode7BioDipole {
        position_m: triple(bytes, 0),
        dipole_a_m: triple(bytes, 24),
        potential_v,
        tick: i64_at(bytes, 56),
        carrier_hz,
    }))
}
