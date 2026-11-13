//! Borrowed complete geometry using the canonical mesh codec.
//!
//! Admission checks every triangle before exposing any geometry. The view owns
//! no allocation and is independent of optional rendering/scientific sidecars.
use super::{
    dequantize, parse_mesh_header, MeshMiniHeader, MeshSectionError as E, FLAG_U16_INDICES,
    MESH_MINI_HEADER_SIZE,
};

pub struct MeshSectionView<'a> {
    header: MeshMiniHeader,
    vertices: &'a [u8],
    indices: &'a [u8],
    index_width: usize,
}

impl<'a> MeshSectionView<'a> {
    /// Validates a complete, exact-length section, including all index bounds.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, E> {
        let (header, total) = parse_mesh_header(bytes)?;
        if total != bytes.len() {
            return Err(E::TrailingBytes {
                expected: total,
                got: bytes.len(),
            });
        }
        let end = MESH_MINI_HEADER_SIZE + header.vertex_count as usize * 6;
        let index_width = if header.flags & FLAG_U16_INDICES != 0 {
            2
        } else {
            4
        };
        let view = Self {
            header,
            vertices: &bytes[MESH_MINI_HEADER_SIZE..end],
            indices: &bytes[end..],
            index_width,
        };
        for raw in view.indices.chunks_exact(index_width) {
            let index = decode_index(raw);
            if index >= header.vertex_count {
                return Err(E::IndexOutOfBounds {
                    index,
                    vertices: header.vertex_count,
                });
            }
        }
        Ok(view)
    }

    pub const fn header(&self) -> &MeshMiniHeader {
        &self.header
    }

    pub fn vertex(&self, index: usize) -> Option<[f32; 3]> {
        let start = index.checked_mul(6)?;
        let raw = self.vertices.get(start..start.checked_add(6)?)?;
        let mut out = [0.; 3];
        for (axis, dst) in out.iter_mut().enumerate() {
            let q = u16::from_le_bytes([raw[axis * 2], raw[axis * 2 + 1]]);
            *dst = dequantize(q, self.header.min[axis], self.header.max[axis]);
        }
        Some(out)
    }

    pub fn triangle(&self, index: usize) -> Option<[u32; 3]> {
        let stride = self.index_width * 3;
        let start = index.checked_mul(stride)?;
        let raw = self.indices.get(start..start.checked_add(stride)?)?;
        let mut out = [0; 3];
        for (dst, bytes) in out.iter_mut().zip(raw.chunks_exact(self.index_width)) {
            *dst = decode_index(bytes);
        }
        Some(out)
    }
}

fn decode_index(raw: &[u8]) -> u32 {
    if raw.len() == 2 {
        u16::from_le_bytes([raw[0], raw[1]]) as u32
    } else {
        u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])
    }
}

#[cfg(test)]
mod tests;
