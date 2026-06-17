//! Binary tensor buffer export for Qualia portal GPU upload.
//!
//! Layout: header (32 B) + N × Tensor10D (40 B each), little-endian f32.

use bytemuck::{bytes_of, Pod, Zeroable};

use super::Tensor10D;

pub const TENSOR_BUFFER_MAGIC: u32 = 0x5134_322A; // "Q42*"
pub const TENSOR_BUFFER_VERSION: u16 = 1;
pub const TENSOR_STRIDE: usize = 40;
/// Byte offset to the first `Tensor10D` record in a baked buffer blob.
pub const TENSOR_HEADER_BYTES: usize = std::mem::size_of::<TensorBufferHeader>();

#[repr(C, align(4))]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct TensorBufferHeader {
    pub magic: u32,
    pub version: u16,
    pub _pad0: u16,
    pub node_count: u32,
    pub stride: u32,
    pub _reserved: [u32; 4],
}

impl TensorBufferHeader {
    pub fn new(node_count: u32) -> Self {
        Self {
            magic: TENSOR_BUFFER_MAGIC,
            version: TENSOR_BUFFER_VERSION,
            node_count,
            stride: TENSOR_STRIDE as u32,
            ..Default::default()
        }
    }

    pub fn total_bytes(node_count: usize) -> usize {
        std::mem::size_of::<TensorBufferHeader>() + node_count * TENSOR_STRIDE
    }
}

/// Parse header from a tensor buffer blob. Returns (header, header_byte_len).
#[inline]
pub fn parse_header(bytes: &[u8]) -> Result<(TensorBufferHeader, usize), &'static str> {
    let header_len = std::mem::size_of::<TensorBufferHeader>();
    if bytes.len() < header_len {
        return Err("buffer too small for header");
    }
    let header: TensorBufferHeader = bytemuck::pod_read_unaligned(&bytes[..header_len]);
    if header.magic != TENSOR_BUFFER_MAGIC {
        return Err("invalid tensor buffer magic");
    }
    if header.stride as usize != TENSOR_STRIDE {
        return Err("unsupported tensor stride");
    }
    Ok((header, header_len))
}

/// Node count encoded in a tensor buffer (zero-alloc).
#[inline]
pub fn tensor_node_count(bytes: &[u8]) -> Result<usize, &'static str> {
    let (header, _) = parse_header(bytes)?;
    Ok(header.node_count as usize)
}

/// Write epistemic `q` for one node (wavefunction collapse). Returns previous q.
#[inline]
pub fn write_tensor_q_at(bytes: &mut [u8], index: usize, q: f32) -> Result<f32, &'static str> {
    let (header, header_len) = parse_header(bytes)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err("tensor index out of range");
    }
    let offset = header_len + index * TENSOR_STRIDE;
    let end = offset + 4;
    if bytes.len() < end {
        return Err("buffer truncated");
    }
    let prev = f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    bytes[offset..offset + 4].copy_from_slice(&q.to_le_bytes());
    Ok(prev)
}

/// Read one `Tensor10D` by index from a buffer (zero-alloc).
#[inline]
pub fn read_tensor_at(bytes: &[u8], index: usize) -> Result<Tensor10D, &'static str> {
    let (header, header_len) = parse_header(bytes)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err("tensor index out of range");
    }
    let offset = header_len + index * TENSOR_STRIDE;
    let end = offset + TENSOR_STRIDE;
    if bytes.len() < end {
        return Err("buffer truncated");
    }
    Ok(bytemuck::pod_read_unaligned(&bytes[offset..end]))
}

/// Pack σ / α / q into linear RGBA for viewport shaders (cold path).
#[inline]
pub fn tensor_render_color(t: &Tensor10D) -> [f32; 4] {
    let rgb = crate::portal_spectral::sigma_to_linear_rgb(t.sigma);
    let alpha = (0.35 + t.alpha * 0.55).clamp(0.15, 1.0);
    [rgb[0], rgb[1], rgb[2], alpha]
}

/// Write header + tensor slice into caller buffer. Returns bytes written.
pub fn write_tensor_buffer(tensors: &[Tensor10D], out: &mut [u8]) -> Result<usize, &'static str> {
    let need = TensorBufferHeader::total_bytes(tensors.len());
    if out.len() < need {
        return Err("output buffer too small");
    }
    let header = TensorBufferHeader::new(tensors.len() as u32);
    let header_bytes = bytes_of(&header);
    out[..header_bytes.len()].copy_from_slice(header_bytes);
    let mut offset = header_bytes.len();
    for tensor in tensors {
        let tb = bytes_of(tensor);
        out[offset..offset + tb.len()].copy_from_slice(tb);
        offset += tb.len();
    }
    Ok(offset)
}

/// Export a bounded slice from the resident U1 substrate into a tensor buffer blob.
pub fn write_tensor_slice_from_resident(
    substrate: &crate::tensor::resident_substrate::ResidentTensorSubstrate,
    max_nodes: usize,
    out: &mut [u8],
) -> Result<usize, &'static str> {
    let count = substrate.node_count() as usize;
    if count == 0 {
        return Err("no resident tensors");
    }
    let export_n = count.min(max_nodes);
    let need = TensorBufferHeader::total_bytes(export_n);
    if out.len() < need {
        return Err("output buffer too small");
    }
    let mut offset = std::mem::size_of::<TensorBufferHeader>();
    let header = TensorBufferHeader::new(export_n as u32);
    out[..offset].copy_from_slice(bytes_of(&header));
    for i in 0..export_n {
        let tensor = substrate
            .tensor_at(i as u32)
            .ok_or("resident tensor missing")?;
        let tb = bytes_of(&tensor);
        out[offset..offset + tb.len()].copy_from_slice(tb);
        offset += tb.len();
    }
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_stable() {
        assert_eq!(std::mem::size_of::<TensorBufferHeader>(), 32);
        assert_eq!(std::mem::size_of::<Tensor10D>(), 40);
    }

    #[test]
    fn write_tensor_slice_from_resident_round_trip() {
        use crate::tensor::resident_substrate::ResidentTensorSubstrate;
        let sub = ResidentTensorSubstrate::new();
        let tensors = [
            Tensor10D::ground_truth(0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5),
            Tensor10D::new(0.5, 0.0, 0.0, 0.4, 0.5, 0.6, 0.0, 1.0, 0.0, 0.75),
        ];
        let need = TensorBufferHeader::total_bytes(2);
        let mut buf = vec![0u8; need];
        write_tensor_buffer(&tensors, &mut buf).unwrap();
        sub.load_from_tensor_buffer(&buf, 0).unwrap();

        let mut slice = vec![0u8; need];
        let n = write_tensor_slice_from_resident(&sub, 2, &mut slice).unwrap();
        assert_eq!(n, need);
        assert_eq!(tensor_node_count(&slice).unwrap(), 2);
        let t1 = read_tensor_at(&slice, 1).unwrap();
        assert!((t1.q - 0.5).abs() < 1e-5);
    }

    #[test]
    fn write_tensor_q_at_collapses_epistemic_field() {
        let tensors = [
            Tensor10D::ground_truth(0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5),
        ];
        let need = TensorBufferHeader::total_bytes(tensors.len());
        let mut buf = vec![0u8; need];
        write_tensor_buffer(&tensors, &mut buf).unwrap();
        let prev = write_tensor_q_at(&mut buf, 0, 0.0).unwrap();
        assert!((prev - 0.0).abs() < 1e-6);
        let t = read_tensor_at(&buf, 0).unwrap();
        assert!(t.q.abs() < 1e-6);
    }

    #[test]
    fn read_tensor_at_round_trip() {
        let tensors = [
            Tensor10D::ground_truth(0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5),
            Tensor10D::ground_truth(0.0, 0.0, 0.4, 0.5, 0.6, 0.0, 1.0, 0.0, 0.75),
        ];
        let need = TensorBufferHeader::total_bytes(tensors.len());
        let mut buf = vec![0u8; need];
        write_tensor_buffer(&tensors, &mut buf).unwrap();
        assert_eq!(tensor_node_count(&buf).unwrap(), 2);
        let t1 = read_tensor_at(&buf, 1).unwrap();
        assert!((t1.x - 0.4).abs() < 1e-5);
        assert!((t1.sigma - 0.75).abs() < 1e-5);
    }

    #[test]
    fn round_trip_write() {
        let tensors = [
            Tensor10D::ground_truth(0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5),
            Tensor10D::ground_truth(0.0, 0.0, 0.4, 0.5, 0.6, 0.0, 1.0, 0.0, 0.5),
        ];
        let need = TensorBufferHeader::total_bytes(tensors.len());
        let mut buf = vec![0u8; need];
        let n = write_tensor_buffer(&tensors, &mut buf).unwrap();
        assert_eq!(n, need);
        let magic = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(magic, TENSOR_BUFFER_MAGIC);
    }
}