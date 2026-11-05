//! Exact borrowed FTW projection views; all execution output is caller-owned.
use crate::inference::moe::ftw_loader::{FtwModelPackage, FtwTensorEntry};
use etalon_inference::ordinary::qwen35::{self, weights::TensorBytes};
use etalon_model::inference::TensorInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightError {
    Missing,
    Ambiguous,
    Shape,
    Encoding,
    Bounds,
    NonFinite,
}

pub struct DenseSource<'a>(pub &'a FtwModelPackage);
impl TensorBytes for DenseSource<'_> {
    fn tensor_bytes(&self, info: &TensorInfo) -> Result<&[u8], qwen35::Error> {
        let len = crate::ggml_quants::tensor_byte_len(info).ok_or(qwen35::Error::Provider)?;
        self.0
            .fetch_by_global_offset(info.byte_offset, len)
            .ok_or(qwen35::Error::Provider)
    }
}

#[derive(Clone, Copy, Debug)]
struct Bytes {
    offset: u64,
    len: usize,
}
impl Bytes {
    fn from_entry(t: &FtwTensorEntry) -> Result<Self, WeightError> {
        Ok(Self {
            offset: t.global_off,
            len: usize::try_from(t.nbytes).map_err(|_| WeightError::Bounds)?,
        })
    }
    fn get(self, pkg: &FtwModelPackage) -> Result<&[u8], WeightError> {
        pkg.fetch_by_global_offset(self.offset, self.len)
            .ok_or(WeightError::Bounds)
    }
}

pub fn entry<'a>(pkg: &'a FtwModelPackage, name: &str) -> Result<&'a FtwTensorEntry, WeightError> {
    let mut found = pkg.manifest.tensors.iter().filter(|t| t.name == name);
    let result = found.next().ok_or(WeightError::Missing)?;
    if found.next().is_some() {
        return Err(WeightError::Ambiguous);
    }
    Ok(result)
}

/// Dense NVFP4 W4A16 matrix: two E2M1 values per byte, one E4M3 scale
/// per sixteen columns, and an independent F16 multiplier per row.
#[derive(Clone, Copy, Debug)]
pub struct Nvfp4Projection {
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    packed: Bytes,
    scales: Bytes,
    globals: Bytes,
}
#[cfg(test)]
#[path = "weights_tests.rs"]
mod tests;
impl Nvfp4Projection {
    pub fn admit(
        pkg: &FtwModelPackage,
        base: &str,
        rows: usize,
        cols: usize,
    ) -> Result<Self, WeightError> {
        if rows == 0 || cols == 0 || cols % 16 != 0 || cols > 16384 {
            return Err(WeightError::Shape);
        }
        let p = entry(pkg, &format!("{base}.weight"))?;
        let s = entry(pkg, &format!("{base}.weight_scale"))?;
        let g = entry(pkg, &format!("{base}.weight_global"))?;
        if p.dtype != "uint8" || s.dtype != "float8_e4m3fn" || g.dtype != "float16" {
            return Err(WeightError::Encoding);
        }
        if p.shape != [rows as u64, (cols / 2) as u64]
            || s.shape != [rows as u64, (cols / 16) as u64]
            || g.shape != [rows as u64]
        {
            return Err(WeightError::Shape);
        }
        let value = Self {
            rows,
            cols,
            packed: Bytes::from_entry(p)?,
            scales: Bytes::from_entry(s)?,
            globals: Bytes::from_entry(g)?,
        };
        let expected = rows.checked_mul(cols).ok_or(WeightError::Bounds)?;
        if value.packed.len != expected / 2
            || value.scales.len != expected / 16
            || value.globals.len != rows.checked_mul(2).ok_or(WeightError::Bounds)?
        {
            return Err(WeightError::Bounds);
        }
        value.packed.get(pkg)?;
        let scales = value.scales.get(pkg)?;
        let globals = value.globals.get(pkg)?;
        // E4M3FN has no infinity encoding; only these signed NaN codes
        // are non-finite. Inspect the encoding without a numerical decode.
        if scales.iter().any(|&x| x & 0x7f == 0x7f) {
            return Err(WeightError::NonFinite);
        }
        for row in 0..rows {
            let global =
                half::f16::from_le_bytes([globals[2 * row], globals[2 * row + 1]]).to_f32();
            if !global.is_finite() || global <= 0.0 {
                return Err(WeightError::NonFinite);
            }
        }
        Ok(value)
    }
    pub fn row(
        &self,
        pkg: &FtwModelPackage,
        row: usize,
        out: &mut [f32],
    ) -> Result<(), WeightError> {
        if row >= self.rows || out.len() != self.cols {
            return Err(WeightError::Shape);
        }
        let p = self.packed.get(pkg)?;
        let s = self.scales.get(pkg)?;
        let g = self.globals.get(pkg)?;
        let global = half::f16::from_le_bytes([g[2 * row], g[2 * row + 1]]).to_f32();
        crate::inference::moe::nvfp4::dequantize_nvfp4_row(
            &p[row * self.cols / 2..(row + 1) * self.cols / 2],
            &s[row * self.cols / 16..(row + 1) * self.cols / 16],
            global,
            out,
        )
        .map_err(|_| WeightError::Bounds)?;
        if out.iter().any(|x| !x.is_finite()) {
            return Err(WeightError::NonFinite);
        }
        Ok(())
    }
    pub fn gemv(
        &self,
        pkg: &FtwModelPackage,
        input: &[f32],
        out: &mut [f32],
    ) -> Result<(), WeightError> {
        if input.len() != self.cols
            || out.len() != self.rows
            || input.iter().any(|x| !x.is_finite())
        {
            return Err(WeightError::Shape);
        }
        super::nvfp4_rows::gemv(
            input,
            self.packed.get(pkg)?,
            self.scales.get(pkg)?,
            self.globals.get(pkg)?,
            out,
        )
        .map_err(row_error)
    }
    pub fn argmax(&self, pkg: &FtwModelPackage, input: &[f32]) -> Result<(u32, f32), WeightError> {
        if input.len() != self.cols || input.iter().any(|x| !x.is_finite()) {
            return Err(WeightError::Shape);
        }
        super::nvfp4_rows::argmax(
            input,
            self.packed.get(pkg)?,
            self.scales.get(pkg)?,
            self.globals.get(pkg)?,
            self.rows,
        )
        .map_err(row_error)
    }
}
fn row_error(error: super::nvfp4_rows::RowError) -> WeightError {
    match error {
        super::nvfp4_rows::RowError::Shape => WeightError::Shape,
        super::nvfp4_rows::RowError::NonFinite => WeightError::NonFinite,
    }
}
