//! Exact-order FP8 projection over admitted immutable bytes. No allocation or
//! activation quantization; scalar norm reads retain the canonical oracle.
use etalon_inference::ordinary::qwen35::{
    weights::{CpuByteHost, TensorBytes},
    Error, Host,
};
use etalon_model::inference::{qwen_roles::Projection, TensorInfo, WeightEncoding};

pub struct DenseHost<'a, S: TensorBytes + ?Sized> {
    pub source: &'a S,
}

const fn table() -> [f32; 256] {
    let mut result = [0.0; 256];
    let mut code = 0;
    while code < 256 {
        let magnitude = code & 127;
        let exponent = magnitude >> 3;
        let mantissa = magnitude & 7;
        let absolute = if magnitude == 127 {
            f32::NAN
        } else if exponent == 0 {
            (mantissa as f32) * (1.0 / 512.0)
        } else {
            f32::from_bits(((exponent as u32) + 120) << 23 | (mantissa as u32) << 20)
        };
        result[code] = if code & 128 != 0 { -absolute } else { absolute };
        code += 1;
    }
    result
}
const FP8: [f32; 256] = table();

struct Scales<'a> {
    bytes: &'a [u8],
    encoding: WeightEncoding,
}
impl Scales<'_> {
    fn at(&self, row: usize) -> f32 {
        match self.encoding {
            WeightEncoding::IeeeF32 => {
                let i = row * 4;
                f32::from_le_bytes([
                    self.bytes[i],
                    self.bytes[i + 1],
                    self.bytes[i + 2],
                    self.bytes[i + 3],
                ])
            }
            WeightEncoding::IeeeF16 => {
                let i = row * 2;
                half::f16::from_le_bytes([self.bytes[i], self.bytes[i + 1]]).to_f32()
            }
            WeightEncoding::BrainF16 => {
                let i = row * 2;
                f32::from_bits(
                    (u16::from_le_bytes([self.bytes[i], self.bytes[i + 1]]) as u32) << 16,
                )
            }
            _ => unreachable!("validated scalar scale encoding"),
        }
    }
}
struct View<'a> {
    weights: &'a [u8],
    scales: Scales<'a>,
    cols: usize,
    rows: usize,
}
impl<S: TensorBytes + ?Sized> DenseHost<'_, S> {
    fn fp8(&self, p: &Projection) -> Result<View<'_>, Error> {
        if p.weight.n_dims != 2 {
            return Err(Error::Shape);
        }
        let cols = usize::try_from(p.weight.dims[0]).map_err(|_| Error::Overflow)?;
        let rows = usize::try_from(p.weight.dims[1]).map_err(|_| Error::Overflow)?;
        if cols == 0 || rows == 0 {
            return Err(Error::Shape);
        }
        let count = rows.checked_mul(cols).ok_or(Error::Overflow)?;
        let weights = self.source.tensor_bytes(&p.weight)?;
        if weights.len() != count {
            return Err(Error::Provider);
        }
        let scale = p.row_scale.as_ref().ok_or(Error::UnsupportedProjection)?;
        if scale.n_dims != 1 || scale.dims[0] != p.weight.dims[1] {
            return Err(Error::UnsupportedProjection);
        }
        let width = match scale.encoding() {
            WeightEncoding::IeeeF32 => 4,
            WeightEncoding::IeeeF16 | WeightEncoding::BrainF16 => 2,
            _ => return Err(Error::UnsupportedProjection),
        };
        let bytes = self.source.tensor_bytes(scale)?;
        if bytes.len() != rows.checked_mul(width).ok_or(Error::Overflow)? {
            return Err(Error::Provider);
        }
        let scales = Scales {
            bytes,
            encoding: scale.encoding(),
        };
        // Match canonical admission: finite raw weights and positive finite
        // scales. Scaled-weight/product overflow belongs to execution.
        for row in 0..rows {
            let scale = scales.at(row);
            if !scale.is_finite() || scale <= 0.0 {
                return Err(Error::Provider);
            }
        }
        if weights.iter().any(|code| code & 127 == 127) {
            return Err(Error::Provider);
        }
        Ok(View {
            weights,
            scales,
            cols,
            rows,
        })
    }
}
impl<S: TensorBytes + ?Sized> Host for DenseHost<'_, S> {
    fn admits(&self, p: &Projection) -> bool {
        if p.weight.encoding() == WeightEncoding::Float8E4M3 {
            self.fp8(p).is_ok()
        } else {
            CpuByteHost {
                source: self.source,
            }
            .admits(p)
        }
    }
    fn gemm(&self, p: &Projection, input: &[f32], out: &mut [f32]) -> Result<(), Error> {
        if p.weight.encoding() != WeightEncoding::Float8E4M3 {
            return CpuByteHost {
                source: self.source,
            }
            .gemm(p, input, out);
        }
        let v = self.fp8(p)?;
        if input.len() != v.cols || out.len() != v.rows {
            return Err(Error::Shape);
        }
        if input.iter().any(|x| !x.is_finite()) {
            return Err(Error::Provider);
        }
        for ((row, target), weights) in out
            .iter_mut()
            .enumerate()
            .zip(v.weights.chunks_exact(v.cols))
        {
            let scale = v.scales.at(row);
            let mut sum = 0.0f32;
            for (&code, &x) in weights.iter().zip(input) {
                // Preserve the oracle's two multiplications and left fold.
                let w = FP8[code as usize] * scale;
                sum += w * x;
            }
            // An overflowing IEEE sum cannot become finite again. A final
            // check rejects exactly the same exceptional accumulation paths.
            if !sum.is_finite() {
                return Err(Error::Provider);
            }
            *target = sum;
        }
        Ok(())
    }
    fn read(&self, tensor: &TensorInfo, out: &mut [f32]) -> Result<(), Error> {
        CpuByteHost {
            source: self.source,
        }
        .read(tensor, out)
    }
}

#[cfg(test)]
#[path = "dense_host_tests.rs"]
mod tests;
