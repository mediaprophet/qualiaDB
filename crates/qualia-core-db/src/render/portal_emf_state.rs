//! One accepted current field and exact immutable source, independent of GPU life.
use crate::q42::{
    emf_container::{EmfContainerReader, EmfReadLimits},
    field_spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S,
};
use std::sync::Arc;

/// Active local-to-parent rigid pose (`rotation_xyzw` unit Hamilton quaternion)
/// associated with the selected EMF coordinate frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmfFramePose {
    pub origin_m: [f64; 3],
    pub rotation_xyzw: [f64; 4],
}

impl Default for EmfFramePose {
    fn default() -> Self {
        Self::identity()
    }
}

impl EmfFramePose {
    pub const fn identity() -> Self {
        Self {
            origin_m: [0.0; 3],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn rotation_matrix(self) -> [[f64; 3]; 3] {
        let [x, y, z, w] = self.rotation_xyzw;
        let (xx, yy, zz) = (x * x, y * y, z * z);
        let (xy, xz, yz) = (x * y, x * z, y * z);
        let (wx, wy, wz) = (w * x, w * y, w * z);
        [
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy)],
            [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx)],
            [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy)],
        ]
    }

    pub fn inverse_transform_vector(self, parent: [f64; 3]) -> [f64; 3] {
        let r = self.rotation_matrix();
        [
            r[0][0] * parent[0] + r[1][0] * parent[1] + r[2][0] * parent[2],
            r[0][1] * parent[0] + r[1][1] * parent[1] + r[2][1] * parent[2],
            r[0][2] * parent[0] + r[1][2] * parent[1] + r[2][2] * parent[2],
        ]
    }

    pub fn inverse_transform_point(self, parent: [f64; 3]) -> [f64; 3] {
        let delta = [
            parent[0] - self.origin_m[0],
            parent[1] - self.origin_m[1],
            parent[2] - self.origin_m[2],
        ];
        self.inverse_transform_vector(delta)
    }
}

/// Pure-Rust FIPS 180-4 SHA-256 digest used for source-bound EMF identity.
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let compress = |state: &mut [u32; 8], block: &[u8; 64]| {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (s, v) in state.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *s = s.wrapping_add(v);
        }
    };
    let mut chunks = data.chunks_exact(64);
    for chunk in &mut chunks {
        compress(&mut h, chunk.try_into().unwrap());
    }
    let rem = chunks.remainder();
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let bit_len = (data.len() as u64).wrapping_mul(8);
    if rem.len() < 56 {
        tail[56..64].copy_from_slice(&bit_len.to_be_bytes());
        compress(&mut h, tail[..64].try_into().unwrap());
    } else {
        tail[120..128].copy_from_slice(&bit_len.to_be_bytes());
        compress(&mut h, tail[..64].try_into().unwrap());
        compress(&mut h, tail[64..128].try_into().unwrap());
    }
    let mut out = [0u8; 32];
    for (i, word) in h.into_iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub const MAX_DISPLAY_CELLS: usize = 1_048_576;
/// Vacuum wave impedance Z0 = mu_0 * c in Ohms.
pub const VACUUM_IMPEDANCE_OHM: f64 =
    4.0e-7 * std::f64::consts::PI * VACUUM_SPEED_OF_LIGHT_M_PER_S;
const MU_0_F64: f64 = 4.0e-7 * std::f64::consts::PI;

/// Plain CPU display derivative, not a scientific record or a GPU ABI.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplayEmfCell {
    pub magnitude_v_per_m: f64,
    pub valid: bool,
}

/// Explicit scientific-to-display optical/polarimetric/spectral derivative per cell.
/// Static or unpolarized fields carry `spectral_sigma == 0.0` and zero `stokes_q/u/v`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplayOpticalCell {
    pub stokes_i: f64,
    pub stokes_q: f64,
    pub stokes_u: f64,
    pub stokes_v: f64,
    pub spectral_sigma: f64,
    pub carrier_hz: f64,
    pub phase_rad: f64,
    pub coherence: f64,
    pub valid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySelection {
    pub descriptor_id: u32,
    pub frame_id: u32,
    pub clock_id: u32,
    pub stream_id: u32,
}

pub struct LoadedEmf {
    pub(super) nx: u32,
    pub(super) ny: u32,
    pub(super) nz: u32,
    pub(super) nt: u32,
    pub(super) cell_count: u32,
    pub(super) bounds: [f32; 6],
    pub(super) frame_pose: EmfFramePose,
    pub(super) cells: Vec<DisplayEmfCell>,
    pub(super) optical_cells: Vec<DisplayOpticalCell>,
    pub(super) display_mode: u16,
    pub(super) source: Arc<[u8]>,
    pub(super) selection: DisplaySelection,
    pub(super) ticks: Vec<i64>,
    pub(super) temporal: bool,
}

impl LoadedEmf {
    pub fn dimensions(&self) -> [u32; 4] {
        [self.nx, self.ny, self.nz, self.nt]
    }
    pub fn cell_count(&self) -> u32 {
        self.cell_count
    }
    /// Interleaved local cell-footprint bounds: xmin,xmax,ymin,ymax,zmin,zmax.
    pub fn local_bounds(&self) -> [f32; 6] {
        self.bounds
    }
    /// Selected frame pose (`origin_m` and `rotation_xyzw`) mapping local cell-footprint
    /// coordinates to the selected frame's physical metre coordinates.
    pub fn frame_pose(&self) -> EmfFramePose {
        self.frame_pose
    }
    /// SHA-256 digest of the immutable source bytes.
    pub fn source_sha256(&self) -> [u8; 32] {
        sha256_bytes(&self.source)
    }
    pub fn display_cells(&self) -> &[DisplayEmfCell] {
        &self.cells
    }
    pub fn optical_cells(&self) -> &[DisplayOpticalCell] {
        &self.optical_cells
    }
    pub fn display_mode(&self) -> u16 {
        self.display_mode
    }
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    pub fn selection(&self) -> DisplaySelection {
        self.selection
    }
    pub fn time_ticks(&self) -> &[i64] {
        &self.ticks
    }
    pub fn is_temporal(&self) -> bool {
        self.temporal
    }
    /// Decode a standalone verified `Q42EMF64` container (v2/v3) into `LoadedEmf`.
    pub fn from_emf64(bytes: &[u8]) -> Result<Self, String> {
        decode_emf64_display(bytes)
    }
}

/// Compute physical Poynting irradiance W/m^2 from stored Poynting, E x B / mu_0, or |E|^2 / Z0.
pub fn poynting_irradiance_w_per_m2(
    e_v_per_m: [f64; 3],
    b_tesla: Option<[f64; 3]>,
    stored_poynting: Option<[f64; 3]>,
) -> f64 {
    if let Some([sx, sy, sz]) = stored_poynting {
        return sx.hypot(sy).hypot(sz);
    }
    if let Some([bx, by, bz]) = b_tesla {
        if bx != 0.0 || by != 0.0 || bz != 0.0 {
            let cx = e_v_per_m[1] * bz - e_v_per_m[2] * by;
            let cy = e_v_per_m[2] * bx - e_v_per_m[0] * bz;
            let cz = e_v_per_m[0] * by - e_v_per_m[1] * bx;
            return cx.hypot(cy).hypot(cz) / MU_0_F64;
        }
    }
    let e_mag = e_v_per_m[0].hypot(e_v_per_m[1]).hypot(e_v_per_m[2]);
    let scaled = e_mag / VACUUM_IMPEDANCE_OHM.sqrt();
    scaled * scaled
}

/// Derive equivalent plane-wave electric-field magnitude V/m from irradiance I (W/m^2).
pub fn e_magnitude_from_poynting(irradiance_w_per_m2: f64) -> f64 {
    if irradiance_w_per_m2.is_nan() || irradiance_w_per_m2 <= 0.0 {
        0.0
    } else if irradiance_w_per_m2.is_infinite() {
        f64::INFINITY
    } else {
        irradiance_w_per_m2.sqrt() * VACUUM_IMPEDANCE_OHM.sqrt()
    }
}

/// Map a physical carrier frequency (Hz) to full-spectrum 6-band display `sigma in [0.0, 6.0)`.
/// Returns `0.0` when static or absent (`carrier_hz <= 0.0`).
pub fn carrier_hz_to_spectral_sigma(carrier_hz: f64) -> f64 {
    if !carrier_hz.is_finite() || carrier_hz <= 0.0 {
        return 0.0;
    }
    const F_RF_MAX: f64 = 3.0e11;
    const F_VIS_LOW: f64 = VACUUM_SPEED_OF_LIGHT_M_PER_S / 700.0e-9;
    const F_VIS_HIGH: f64 = VACUUM_SPEED_OF_LIGHT_M_PER_S / 400.0e-9;
    const F_UV_MAX: f64 = 3.0e16;
    const F_XRAY_MAX: f64 = 3.0e19;
    if carrier_hz >= F_VIS_LOW && carrier_hz <= F_VIS_HIGH {
        let lambda_nm = (VACUUM_SPEED_OF_LIGHT_M_PER_S * 1.0e9) / carrier_hz;
        ((lambda_nm - 400.0) / 300.0).clamp(0.0, 0.999_999)
    } else if carrier_hz < F_RF_MAX {
        let t = (carrier_hz.max(1.0e-3).log10() + 3.0) / (F_RF_MAX.log10() + 3.0);
        1.0 + t.clamp(0.0, 0.999_999)
    } else if carrier_hz < F_VIS_LOW {
        let lo = F_RF_MAX.log10();
        let t = (carrier_hz.log10() - lo) / (F_VIS_LOW.log10() - lo);
        2.0 + t.clamp(0.0, 0.999_999)
    } else if carrier_hz < F_UV_MAX {
        let lo = F_VIS_HIGH.log10();
        let t = (carrier_hz.log10() - lo) / (F_UV_MAX.log10() - lo);
        3.0 + t.clamp(0.0, 0.999_999)
    } else if carrier_hz < F_XRAY_MAX {
        let lo = F_UV_MAX.log10();
        let t = (carrier_hz.log10() - lo) / (F_XRAY_MAX.log10() - lo);
        4.0 + t.clamp(0.0, 0.999_999)
    } else {
        let t = (carrier_hz.log10() - F_XRAY_MAX.log10()) / 12.0;
        5.0 + t.clamp(0.0, 0.999_999)
    }
}

pub fn decode_emf64_display(bytes: &[u8]) -> Result<LoadedEmf, String> {
    let reader = EmfContainerReader::parse_with_limits(bytes, EmfReadLimits::conservative())
        .map_err(|e| format!("emf64: {e}"))?;
    let version = reader.header().version;
    let dense = reader.dense_volume().map_err(|e| format!("emf64: {e}"))?;
    let total = usize::try_from(dense.nx)
        .ok()
        .and_then(|x| x.checked_mul(dense.ny as usize))
        .and_then(|xy| xy.checked_mul(dense.nz as usize))
        .and_then(|xyz| xyz.checked_mul(dense.nt as usize))
        .ok_or_else(|| "emf64: dimension overflow".to_string())?;
    if total == 0 || total > MAX_DISPLAY_CELLS || total != dense.cells.len() {
        return Err("emf64: invalid dense display dimensions".into());
    }
    let bounds = dense.bounds_meters();
    if bounds.iter().any(|v| !v.is_finite()) || (0..3).any(|i| bounds[i * 2] >= bounds[i * 2 + 1]) {
        return Err("emf64: invalid display bounds".into());
    }
    let mut cells = Vec::new();
    let mut optical_cells = Vec::new();
    cells.try_reserve_exact(total).map_err(|_| "emf64: alloc".to_string())?;
    optical_cells.try_reserve_exact(total).map_err(|_| "emf64: alloc".to_string())?;
    for rec in &dense.cells {
        let finite_ok = rec.e_field.iter().all(|v| v.is_finite())
            && rec.b_field.iter().all(|v| v.is_finite())
            && rec.poynting_flux.is_finite() && rec.poynting_flux >= 0.0
            && rec.wavelength_nm.is_finite() && rec.wavelength_nm >= 0.0
            && rec.coherence_phase.is_finite()
            && rec.stokes_q.is_finite() && rec.stokes_u.is_finite() && rec.stokes_v.is_finite()
            && rec.spectral_sigma.is_finite()
            && rec.confidence_alpha.is_finite() && (0.0..=1.0).contains(&rec.confidence_alpha);
        if !finite_ok {
            return Err("emf64: non-finite or out-of-range cell record".into());
        }
        if rec.confidence_alpha == 0.0 {
            cells.push(DisplayEmfCell::default());
            optical_cells.push(DisplayOpticalCell::default());
            continue;
        }
        let e = [rec.e_field[0] as f64, rec.e_field[1] as f64, rec.e_field[2] as f64];
        let b = [rec.b_field[0] as f64, rec.b_field[1] as f64, rec.b_field[2] as f64];
        let e_mag = e[0].hypot(e[1]).hypot(e[2]);
        let flux = if rec.poynting_flux > 0.0 {
            rec.poynting_flux as f64
        } else {
            poynting_irradiance_w_per_m2(e, Some(b), None)
        };
        let carrier_hz = if rec.wavelength_nm > 0.0 {
            (VACUUM_SPEED_OF_LIGHT_M_PER_S * 1.0e9) / (rec.wavelength_nm as f64)
        } else {
            0.0
        };
        let sigma = if rec.spectral_sigma > 0.0 {
            (rec.spectral_sigma as f64).clamp(0.0, 5.999_999)
        } else {
            carrier_hz_to_spectral_sigma(carrier_hz)
        };
        let (rq, ru, rv) = (rec.stokes_q as f64, rec.stokes_u as f64, rec.stokes_v as f64);
        let pol_norm = rq.hypot(ru).hypot(rv);
        let (stokes_q, stokes_u, stokes_v) = if pol_norm > flux && pol_norm <= 1.0 + 1.0e-5 && flux > 0.0 {
            (rq * flux, ru * flux, rv * flux)
        } else if pol_norm <= flux * (1.0 + 1.0e-5) + 1.0e-12 {
            (rq, ru, rv)
        } else if pol_norm == 0.0 {
            (0.0, 0.0, 0.0)
        } else {
            return Err("emf64: Stokes polarization exceeds total irradiance I".into());
        };
        cells.push(DisplayEmfCell {
            magnitude_v_per_m: if e_mag > 0.0 { e_mag } else { e_magnitude_from_poynting(flux) },
            valid: true,
        });
        optical_cells.push(DisplayOpticalCell {
            stokes_i: flux,
            stokes_q,
            stokes_u,
            stokes_v,
            spectral_sigma: sigma,
            carrier_hz,
            phase_rad: rec.coherence_phase as f64,
            coherence: rec.confidence_alpha as f64,
            valid: true,
        });
    }
    let ticks: Vec<i64> = (0..dense.nt as i64).collect();
    let mut source = Vec::new();
    source.try_reserve_exact(bytes.len()).map_err(|_| "emf64: source alloc".to_string())?;
    source.extend_from_slice(bytes);
    Ok(LoadedEmf {
        nx: dense.nx,
        ny: dense.ny,
        nz: dense.nz,
        nt: dense.nt,
        cell_count: total as u32,
        bounds,
        frame_pose: EmfFramePose::identity(),
        cells,
        optical_cells,
        display_mode: version,
        source: Arc::from(source.into_boxed_slice()),
        selection: DisplaySelection { descriptor_id: 0, frame_id: 0, clock_id: 0, stream_id: 0 },
        ticks,
        temporal: dense.nt > 1,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmfBackend {
    Pending,
    WebGpu,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum EmfDisplayStatus {
    #[default]
    Empty = 0,
    Pending = 1,
    Uploaded = 2,
    Unavailable = 3,
}

impl EmfBackend {
    pub fn accepted_status(self) -> EmfDisplayStatus {
        match self {
            Self::Pending => EmfDisplayStatus::Pending,
            Self::WebGpu => EmfDisplayStatus::Uploaded,
            Self::Unavailable => EmfDisplayStatus::Unavailable,
        }
    }
}

impl EmfDisplayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Pending => "pending",
            Self::Uploaded => "uploaded",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Default)]
pub struct RetainedEmf {
    field: Option<LoadedEmf>,
    status: EmfDisplayStatus,
}

impl RetainedEmf {
    pub fn bounds(&self) -> Option<[f32; 6]> {
        self.field.as_ref().map(|f| f.bounds)
    }
    pub fn status(&self) -> EmfDisplayStatus {
        self.status
    }
    pub fn accepted(&self) -> Option<&LoadedEmf> {
        self.field.as_ref()
    }
    /// Recheck bounded derivative shape at the ownership boundary. Scientific source
    /// was fully admitted before construction; upload refusal never swaps old state.
    pub fn accept(
        &mut self,
        field: LoadedEmf,
        backend: EmfBackend,
        upload: impl FnOnce(&LoadedEmf) -> Result<(), String>,
    ) -> Result<(), String> {
        let count = usize::try_from(field.nx)
            .ok()
            .and_then(|n| n.checked_mul(usize::try_from(field.ny).ok()?))
            .and_then(|n| n.checked_mul(usize::try_from(field.nz).ok()?))
            .and_then(|n| n.checked_mul(usize::try_from(field.nt).ok()?));
        let shape_ok = matches!(
            count,
            Some(n) if n > 0 && n <= MAX_DISPLAY_CELLS
                && n == field.cells.len()
                && n == field.optical_cells.len()
                && n == field.cell_count as usize
        );
        let cells_invalid = field.cells.iter().zip(field.optical_cells.iter()).any(|(c, o)| {
            if c.valid != o.valid {
                return true;
            }
            if c.valid {
                let pol = o.stokes_q.hypot(o.stokes_u).hypot(o.stokes_v);
                c.magnitude_v_per_m.is_nan()
                    || c.magnitude_v_per_m < 0.0
                    || o.stokes_i.is_nan()
                    || o.stokes_i < 0.0
                    || !o.stokes_q.is_finite()
                    || !o.stokes_u.is_finite()
                    || !o.stokes_v.is_finite()
                    || pol > o.stokes_i * (1.0 + 1.0e-6) + 1.0e-12
                    || !o.spectral_sigma.is_finite()
                    || !(0.0..=6.0).contains(&o.spectral_sigma)
                    || !o.carrier_hz.is_finite()
                    || o.carrier_hz < 0.0
                    || !o.phase_rad.is_finite()
                    || !o.coherence.is_finite()
                    || !(0.0..=1.0).contains(&o.coherence)
            } else {
                c.magnitude_v_per_m.to_bits() != 0
                    || o.stokes_i.to_bits() != 0
                    || o.stokes_q.to_bits() != 0
                    || o.stokes_u.to_bits() != 0
                    || o.stokes_v.to_bits() != 0
                    || o.spectral_sigma.to_bits() != 0
                    || o.carrier_hz.to_bits() != 0
                    || o.phase_rad.to_bits() != 0
                    || o.coherence.to_bits() != 0
            }
        });
        if !shape_ok
            || field.source.is_empty()
            || field.ticks.len() != field.nt as usize
            || field.ticks.windows(2).any(|p| p[0] >= p[1])
            || field.bounds.iter().any(|v| !v.is_finite())
            || (0..3).any(|i| field.bounds[i * 2] >= field.bounds[i * 2 + 1])
            || cells_invalid
        {
            return Err("emf: invalid retained current display projection".into());
        }
        if backend == EmfBackend::WebGpu {
            upload(&field)?;
        }
        self.field = Some(field);
        self.status = backend.accepted_status();
        Ok(())
    }
    pub fn invalidate(&mut self, pending: bool) {
        if self.field.is_some() {
            self.status = if pending {
                EmfDisplayStatus::Pending
            } else {
                EmfDisplayStatus::Unavailable
            }
        }
    }
    pub fn clear(&mut self) {
        self.field = None;
        self.status = EmfDisplayStatus::Empty;
    }
    pub fn replay(
        &mut self,
        upload: impl FnOnce(&LoadedEmf) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(field) = &self.field {
            if let Err(e) = upload(field) {
                self.status = EmfDisplayStatus::Unavailable;
                return Err(e);
            }
            self.status = EmfDisplayStatus::Uploaded;
        }
        Ok(())
    }
}

/// Convert paired scalar and optical/polarimetric display cells into GPU `EmfFieldCell`.
#[cfg(feature = "gpu-runtime")]
pub fn display_optical_to_gpu_cell(
    cell: &DisplayEmfCell,
    opt: &DisplayOpticalCell,
) -> crate::render::gpu::EmfFieldCell {
    if !cell.valid || !opt.valid {
        return crate::render::gpu::EmfFieldCell::default();
    }
    let amplitude = -(-cell.magnitude_v_per_m).exp_m1() as f32;
    let attention_depth = if opt.stokes_i.is_infinite() {
        1.0
    } else {
        (-(-opt.stokes_i).exp_m1() as f32).clamp(0.0, 1.0)
    };
    let frequency = if opt.carrier_hz.is_finite() && opt.carrier_hz > 0.0 {
        opt.carrier_hz.min(f32::MAX as f64) as f32
    } else {
        0.0
    };
    let spectral_sigma = opt.spectral_sigma.clamp(0.0, 6.0) as f32;
    let chromatic = opt.spectral_sigma > 0.0 || opt.carrier_hz > 0.0 || opt.phase_rad != 0.0;
    let scale = if chromatic { 1.0 } else { 0.0 };
    let sigma_phase = ((opt.spectral_sigma.fract() * std::f64::consts::TAU) as f32).max(0.0);
    let phase = if opt.phase_rad != 0.0 {
        opt.phase_rad.rem_euclid(std::f64::consts::TAU) as f32
    } else if opt.spectral_sigma > 0.0 {
        sigma_phase
    } else {
        0.0
    };
    let (mut sq, mut su, mut sv) = if opt.stokes_i.is_finite() && opt.stokes_i > 0.0 && amplitude > 0.0 {
        let s = f64::from(amplitude) / opt.stokes_i;
        let clamped = |v: f64| ((v / opt.stokes_i).clamp(-1.0, 1.0) * f64::from(amplitude)) as f32;
        let _ = s;
        (clamped(opt.stokes_q), clamped(opt.stokes_u), clamped(opt.stokes_v))
    } else {
        (0.0, 0.0, 0.0)
    };
    let pol_mag = (sq * sq + su * su + sv * sv).sqrt();
    if pol_mag > amplitude && pol_mag > 0.0 {
        let s = amplitude / pol_mag;
        sq *= s;
        su *= s;
        sv *= s;
    }
    let dop = if amplitude > 0.0 { (pol_mag / amplitude).clamp(0.0, 1.0) } else { 0.0 };
    crate::render::gpu::EmfFieldCell {
        amplitude,
        phase,
        frequency,
        scale,
        attention_depth,
        epistemic_weight: 1.0,
        topological_spin: 0.0,
        temporal_decay: 0.0,
        entropy_bias: 0.0,
        spatial_phase: sigma_phase,
        recurrence_frequency: spectral_sigma,
        density_threshold: opt.coherence.clamp(0.0, 1.0) as f32,
        manifold_curvature: if chromatic { dop * 0.5 } else { 0.0 },
        stokes_q: sq,
        stokes_u: su,
        stokes_v: sv,
    }
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub(crate) fn upload(
    gpu: &mut crate::render::gpu::PortalGpu,
    field: &LoadedEmf,
) -> Result<(), String> {
    gpu.emf_upload_loaded(field)
}
