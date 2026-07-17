//! Binary STL export from MeshIR (3D print).

use super::geometry_ir::MeshIR;
use crate::cv::error::CvError;

/// Write binary STL into `out`. Returns bytes written.
pub fn mesh_ir_to_stl_binary(mesh: &MeshIR, out: &mut [u8]) -> Result<usize, CvError> {
    let n_tri = mesh.indices.len() / 3;
    let need = 80 + 4 + n_tri * 50;
    if out.len() < need {
        return Err(CvError::BufferTooSmall);
    }
    out[..80].fill(0);
    // header note
    let hdr = b"Qualia MeshIR STL";
    out[..hdr.len()].copy_from_slice(hdr);
    out[80..84].copy_from_slice(&(n_tri as u32).to_le_bytes());
    let mut o = 84usize;
    for t in 0..n_tri {
        let i0 = mesh.indices[t * 3] as usize;
        let i1 = mesh.indices[t * 3 + 1] as usize;
        let i2 = mesh.indices[t * 3 + 2] as usize;
        if i0 >= mesh.positions.len() || i1 >= mesh.positions.len() || i2 >= mesh.positions.len() {
            return Err(CvError::InvalidParameter);
        }
        let p0 = mesh.positions[i0];
        let p1 = mesh.positions[i1];
        let p2 = mesh.positions[i2];
        // normal
        let ux = p1[0] - p0[0];
        let uy = p1[1] - p0[1];
        let uz = p1[2] - p0[2];
        let vx = p2[0] - p0[0];
        let vy = p2[1] - p0[1];
        let vz = p2[2] - p0[2];
        let mut nx = uy * vz - uz * vy;
        let mut ny = uz * vx - ux * vz;
        let mut nz = ux * vy - uy * vx;
        let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-12);
        nx /= len;
        ny /= len;
        nz /= len;
        for f in [nx, ny, nz, p0[0], p0[1], p0[2], p1[0], p1[1], p1[2], p2[0], p2[1], p2[2]] {
            out[o..o + 4].copy_from_slice(&f.to_le_bytes());
            o += 4;
        }
        out[o..o + 2].copy_from_slice(&0u16.to_le_bytes());
        o += 2;
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::geometry_ir::MeshIR;
    #[test]
    fn stl_header() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        m.indices = vec![0, 1, 2];
        let mut buf = vec![0u8; 200];
        let n = mesh_ir_to_stl_binary(&m, &mut buf).unwrap();
        assert!(n >= 134);
        assert_eq!(u32::from_le_bytes([buf[80], buf[81], buf[82], buf[83]]), 1);
    }
}
