//! MeshIR → 3MF (3D Manufacturing Format) export (cold path; pure Rust, no dependency).
//!
//! 3MF is an OPC package — a ZIP container holding three XML parts:
//!   1. `[Content_Types].xml` — declares the `.rels` and `.model` content types.
//!   2. `_rels/.rels`         — the package start-part relationship.
//!   3. `3D/3dmodel.model`    — the mesh (vertices + triangles) in the 3MF core
//!                              namespace, in `millimeter` units.
//!
//! Since the crate ships no ZIP dependency, this file hand-rolls a minimal
//! STORED (method 0, uncompressed) ZIP writer — local file headers, a central
//! directory, and an end-of-central-directory record — plus a bitwise CRC-32
//! (IEEE polynomial `0xEDB88320`). All multi-byte lengths/offsets are little-endian.
//!
//! This is a cold, caller-buffered path (`String`/`Vec` are fine here); the hot
//! edge path never touches it. Fails closed to `CvError` — never panics — on an
//! empty mesh, non-triangle index counts, out-of-range indices, or a too-small
//! output buffer.

use super::geometry_ir::MeshIR;
use crate::specialized_libs::computer_vision::cv::error::CvError;

// ZIP record signatures (little-endian on the wire).
const LOCAL_FILE_HEADER_SIG: u32 = 0x0403_4b50; // "PK\x03\x04"
const CENTRAL_DIR_HEADER_SIG: u32 = 0x0201_4b50; // "PK\x01\x02"
const EOCD_SIG: u32 = 0x0605_4b50; // "PK\x05\x06"

// STORED (uncompressed) with a classic "version needed to extract" of 2.0.
const ZIP_VERSION: u16 = 20;
const METHOD_STORED: u16 = 0;

// The three OPC part names, in package order.
const PART_CONTENT_TYPES: &str = "[Content_Types].xml";
const PART_RELS: &str = "_rels/.rels";
const PART_MODEL: &str = "3D/3dmodel.model";

/// Bitwise CRC-32 (IEEE 802.3 polynomial `0xEDB88320`, reflected).
///
/// Matches the ZIP/PKZIP and zlib CRC-32 used across OPC packages.
fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            // `mask` is 0xFFFFFFFF when the low bit is set, else 0.
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Write a valid minimal 3MF (OPC/ZIP) package describing `mesh` into `out`.
///
/// Returns the number of bytes written. The ZIP holds exactly three STORED
/// parts: `[Content_Types].xml`, `_rels/.rels`, and `3D/3dmodel.model`.
///
/// Fails closed to `CvError`:
/// - `EmptyInput` when the mesh has no vertices or no indices,
/// - `InvalidParameter` when the index count is not a multiple of 3 or any
///   index is out of range,
/// - `BufferTooSmall` when `out` cannot hold the whole package.
pub fn mesh_ir_to_3mf(mesh: &MeshIR, out: &mut [u8]) -> Result<usize, CvError> {
    let n_verts = mesh.positions.len();
    let n_indices = mesh.indices.len();
    if n_verts == 0 || n_indices == 0 {
        return Err(CvError::EmptyInput);
    }
    // Triangle list only.
    if n_indices % 3 != 0 {
        return Err(CvError::InvalidParameter);
    }
    // Every index must reference a real vertex.
    for &i in &mesh.indices {
        if i as usize >= n_verts {
            return Err(CvError::InvalidParameter);
        }
    }

    // --- Build the three part bodies as Strings/bytes (cold path). ---
    let content_types = build_content_types_xml();
    let rels = build_rels_xml();
    let model = build_model_xml(mesh);

    let parts: [(&str, &[u8]); 3] = [
        (PART_CONTENT_TYPES, content_types.as_bytes()),
        (PART_RELS, rels.as_bytes()),
        (PART_MODEL, model.as_bytes()),
    ];

    // Assemble the whole ZIP into a local Vec, then bounds-check and copy into
    // the caller buffer. This keeps offset arithmetic in one place and lets us
    // fail closed (BufferTooSmall) without a partial write.
    let mut zip: Vec<u8> = Vec::new();

    // Record what we need to emit the central directory after all local records.
    struct DirEntry {
        crc: u32,
        size: u32,
        local_header_offset: u32,
        name: &'static str,
    }
    let mut dir: Vec<DirEntry> = Vec::with_capacity(parts.len());

    // --- Local file headers + data (STORED). ---
    for (name, data) in parts.iter() {
        let crc = crc32_ieee(data);
        let size = data.len() as u32;
        let local_header_offset = zip.len() as u32;

        push_u32(&mut zip, LOCAL_FILE_HEADER_SIG);
        push_u16(&mut zip, ZIP_VERSION); // version needed to extract
        push_u16(&mut zip, 0); // general purpose bit flag
        push_u16(&mut zip, METHOD_STORED); // compression method
        push_u16(&mut zip, 0); // last mod file time
        push_u16(&mut zip, 0); // last mod file date
        push_u32(&mut zip, crc); // crc-32
        push_u32(&mut zip, size); // compressed size (== uncompressed for STORED)
        push_u32(&mut zip, size); // uncompressed size
        push_u16(&mut zip, name.len() as u16); // file name length
        push_u16(&mut zip, 0); // extra field length
        zip.extend_from_slice(name.as_bytes()); // file name
        zip.extend_from_slice(data); // file data (uncompressed)

        dir.push(DirEntry {
            crc,
            size,
            local_header_offset,
            name,
        });
    }

    // --- Central directory. ---
    let central_dir_offset = zip.len() as u32;
    for e in dir.iter() {
        push_u32(&mut zip, CENTRAL_DIR_HEADER_SIG);
        push_u16(&mut zip, ZIP_VERSION); // version made by
        push_u16(&mut zip, ZIP_VERSION); // version needed to extract
        push_u16(&mut zip, 0); // general purpose bit flag
        push_u16(&mut zip, METHOD_STORED); // compression method
        push_u16(&mut zip, 0); // last mod file time
        push_u16(&mut zip, 0); // last mod file date
        push_u32(&mut zip, e.crc); // crc-32
        push_u32(&mut zip, e.size); // compressed size
        push_u32(&mut zip, e.size); // uncompressed size
        push_u16(&mut zip, e.name.len() as u16); // file name length
        push_u16(&mut zip, 0); // extra field length
        push_u16(&mut zip, 0); // file comment length
        push_u16(&mut zip, 0); // disk number start
        push_u16(&mut zip, 0); // internal file attributes
        push_u32(&mut zip, 0); // external file attributes
        push_u32(&mut zip, e.local_header_offset); // offset of local header
        zip.extend_from_slice(e.name.as_bytes()); // file name
    }
    let central_dir_size = zip.len() as u32 - central_dir_offset;

    // --- End of central directory record. ---
    let entry_count = dir.len() as u16;
    push_u32(&mut zip, EOCD_SIG);
    push_u16(&mut zip, 0); // number of this disk
    push_u16(&mut zip, 0); // disk where central directory starts
    push_u16(&mut zip, entry_count); // records on this disk
    push_u16(&mut zip, entry_count); // total records
    push_u32(&mut zip, central_dir_size); // size of central directory
    push_u32(&mut zip, central_dir_offset); // offset of central directory
    push_u16(&mut zip, 0); // comment length

    // --- Bounds check, then copy into the caller buffer (fail closed). ---
    if out.len() < zip.len() {
        return Err(CvError::BufferTooSmall);
    }
    out[..zip.len()].copy_from_slice(&zip);
    Ok(zip.len())
}

/// Append a little-endian `u16`.
fn push_u16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// Append a little-endian `u32`.
fn push_u32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// The fixed `[Content_Types].xml` OPC part.
fn build_content_types_xml() -> String {
    String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>\
</Types>",
    )
}

/// The fixed `_rels/.rels` start-part relationship.
fn build_rels_xml() -> String {
    String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>\
</Relationships>",
    )
}

/// Build the `3D/3dmodel.model` mesh XML for `mesh` (validated by the caller).
fn build_model_xml(mesh: &MeshIR) -> String {
    let mut s = String::with_capacity(256 + mesh.positions.len() * 48 + mesh.indices.len() * 12);
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    s.push_str(
        "<model unit=\"millimeter\" xml:lang=\"en-US\" \
xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">",
    );
    s.push_str("<resources><object id=\"1\" type=\"model\"><mesh><vertices>");
    for p in &mesh.positions {
        s.push_str("<vertex x=\"");
        push_coord(&mut s, p[0]);
        s.push_str("\" y=\"");
        push_coord(&mut s, p[1]);
        s.push_str("\" z=\"");
        push_coord(&mut s, p[2]);
        s.push_str("\"/>");
    }
    s.push_str("</vertices><triangles>");
    for t in mesh.indices.chunks_exact(3) {
        s.push_str("<triangle v1=\"");
        s.push_str(&t[0].to_string());
        s.push_str("\" v2=\"");
        s.push_str(&t[1].to_string());
        s.push_str("\" v3=\"");
        s.push_str(&t[2].to_string());
        s.push_str("\"/>");
    }
    s.push_str(
        "</triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>",
    );
    s
}

/// Append a mesh coordinate as a decimal that round-trips the `f32` value.
fn push_coord(s: &mut String, v: f32) {
    if v.is_finite() {
        // Widen to f64 so the shortest decimal round-trips back to this f32.
        s.push_str(&format!("{}", v as f64));
    } else {
        // Callers validate geometry; keep the XML well-formed regardless.
        s.push('0');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::spatial::geometry_ir::MeshIR;

    fn triangle() -> MeshIR {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m
    }

    /// Locate a subslice `needle` inside `hay`; returns the start offset.
    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || needle.len() > hay.len() {
            return None;
        }
        (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
    }

    #[test]
    fn tmf_zip_signatures_and_parts_present() {
        let m = triangle();
        let mut buf = vec![0u8; 8192];
        let n = mesh_ir_to_3mf(&m, &mut buf).expect("export");
        let z = &buf[..n];

        // (a) Starts with the ZIP local-file-header signature PK\x03\x04.
        assert_eq!(&z[0..4], b"PK\x03\x04");
        assert_eq!(
            u32::from_le_bytes([z[0], z[1], z[2], z[3]]),
            LOCAL_FILE_HEADER_SIG
        );

        // (b) Contains the EOCD signature PK\x05\x06.
        assert!(find(z, b"PK\x05\x06").is_some());
        // ...and the central directory header signature PK\x01\x02.
        assert!(find(z, b"PK\x01\x02").is_some());

        // (c) All three OPC part names appear, plus the model XML tokens.
        assert!(find(z, b"[Content_Types].xml").is_some());
        assert!(find(z, b"_rels/.rels").is_some());
        assert!(find(z, b"3D/3dmodel.model").is_some());
        assert!(find(z, b"<vertices>").is_some());
        assert!(find(z, b"<triangle").is_some());
    }

    #[test]
    fn tmf_crc32_of_rels_part_matches_header() {
        // Hand-verified golden: CRC-32 (IEEE) of the exact `_rels/.rels` body,
        // independently computed with Python's zlib.crc32 = 0x47FEF7AB.
        const RELS_CRC_GOLDEN: u32 = 0x47FE_F7AB;

        // Sanity: our bitwise CRC agrees with the golden on the part body.
        let rels = build_rels_xml();
        assert_eq!(crc32_ieee(rels.as_bytes()), RELS_CRC_GOLDEN);

        let m = triangle();
        let mut buf = vec![0u8; 8192];
        let n = mesh_ir_to_3mf(&m, &mut buf).expect("export");
        let z = &buf[..n];

        // Find the `_rels/.rels` local file header and read the CRC field from it.
        // Local header layout: sig(4) verNeed(2) flag(2) method(2) time(2) date(2)
        //                      crc(4) compSize(4) uncompSize(4) nameLen(2) extraLen(2) name...
        // Scan every local header and match the one whose name is "_rels/.rels".
        let name = b"_rels/.rels";
        let mut found_crc = None;
        let mut i = 0usize;
        while i + 30 <= z.len() {
            if &z[i..i + 4] == b"PK\x03\x04" {
                let name_len = u16::from_le_bytes([z[i + 26], z[i + 27]]) as usize;
                let name_start = i + 30;
                if name_start + name_len <= z.len() && &z[name_start..name_start + name_len] == name
                {
                    found_crc = Some(u32::from_le_bytes([
                        z[i + 14],
                        z[i + 15],
                        z[i + 16],
                        z[i + 17],
                    ]));
                    break;
                }
            }
            i += 1;
        }
        assert_eq!(
            found_crc,
            Some(RELS_CRC_GOLDEN),
            "CRC-32 in the _rels/.rels local header must match the hand-verified golden"
        );
    }

    #[test]
    fn tmf_eocd_central_directory_is_consistent() {
        let m = triangle();
        let mut buf = vec![0u8; 8192];
        let n = mesh_ir_to_3mf(&m, &mut buf).expect("export");
        let z = &buf[..n];

        // EOCD is the final 22 bytes (no comment).
        let eocd = n - 22;
        assert_eq!(&z[eocd..eocd + 4], b"PK\x05\x06");
        let entry_count = u16::from_le_bytes([z[eocd + 10], z[eocd + 11]]);
        assert_eq!(entry_count, 3, "three parts in the central directory");
        let cd_size =
            u32::from_le_bytes([z[eocd + 12], z[eocd + 13], z[eocd + 14], z[eocd + 15]]) as usize;
        let cd_off =
            u32::from_le_bytes([z[eocd + 16], z[eocd + 17], z[eocd + 18], z[eocd + 19]]) as usize;
        // The central directory sits exactly before the EOCD.
        assert_eq!(cd_off + cd_size, eocd);
        assert_eq!(&z[cd_off..cd_off + 4], b"PK\x01\x02");
    }

    #[test]
    fn tmf_too_small_buffer_fails_closed() {
        let m = triangle();
        let mut tiny = vec![0u8; 8]; // far smaller than any valid package
        let r = mesh_ir_to_3mf(&m, &mut tiny);
        assert_eq!(r, Err(CvError::BufferTooSmall));
    }

    #[test]
    fn tmf_empty_mesh_fails_closed() {
        let m = MeshIR::empty();
        let mut buf = vec![0u8; 1024];
        let r = mesh_ir_to_3mf(&m, &mut buf);
        assert_eq!(r, Err(CvError::EmptyInput));
    }

    #[test]
    fn tmf_non_triangle_index_count_rejected() {
        let mut m = triangle();
        m.indices = vec![0, 1]; // not a multiple of 3
        let mut buf = vec![0u8; 1024];
        let r = mesh_ir_to_3mf(&m, &mut buf);
        assert_eq!(r, Err(CvError::InvalidParameter));
    }

    #[test]
    fn tmf_out_of_range_index_rejected() {
        let mut m = triangle();
        m.indices = vec![0, 1, 99]; // 99 >= 3 vertices
        let mut buf = vec![0u8; 1024];
        let r = mesh_ir_to_3mf(&m, &mut buf);
        assert_eq!(r, Err(CvError::InvalidParameter));
    }
}
