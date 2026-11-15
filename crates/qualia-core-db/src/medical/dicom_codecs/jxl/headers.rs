//! ISO/IEC 18181-1 JPEG-XL `SizeHeader`, `ImageMetadata`, `TransformData`, and `FrameHeader` + TOC parser.
use super::container::{
    default_opsin_inverse_matrix, JxlEncoding, JxlError, JxlFrameHeader, JxlFrameType,
    JxlImageHeader, JxlLimits, JxlTransferFunction,
};
use super::entropy::{decode_prefix_or_ans_blob, JxlBitReader};

pub fn parse_image_header(br: &mut JxlBitReader<'_>, limits: JxlLimits) -> Result<JxlImageHeader, JxlError> {
    let b0 = br.read_bits(8)? as u8;
    let b1 = br.read_bits(8)? as u8;
    if [b0, b1] != [0xFF, 0x0A] {
        return Err(JxlError::CorruptStream("expected FF0A JPEG-XL signature"));
    }
    let small = br.read_bool()?;
    let height = if small {
        (br.read_bits(5)? as u32 + 1) * 8
    } else {
        br.read_u32_dist(&[(1, 9), (1, 13), (1, 18), (1, 30)])?
    };
    let ratio = br.read_bits(3)?;
    let width = match ratio {
        0 => {
            if small {
                (br.read_bits(5)? as u32 + 1) * 8
            } else {
                br.read_u32_dist(&[(1, 9), (1, 13), (1, 18), (1, 30)])?
            }
        }
        1 => height,
        2 => (height as u64 * 12 / 10) as u32,
        3 => (height as u64 * 4 / 3) as u32,
        4 => (height as u64 * 3 / 2) as u32,
        5 => (height as u64 * 16 / 9) as u32,
        6 => (height as u64 * 5 / 4) as u32,
        7 => height.saturating_mul(2),
        _ => return Err(JxlError::CorruptStream("invalid JXL ratio")),
    };
    if width == 0 || height == 0 || width > limits.max_width as u32 || height > limits.max_height as u32 {
        return Err(JxlError::DimensionLimit);
    }

    let mut bits_per_sample = 8u8;
    let mut floating_point = false;
    let mut xyb_encoded = true;
    let mut is_gray = false;
    let mut num_extra_channels = 0u16;
    let mut has_alpha = false;
    let mut transfer = JxlTransferFunction::Srgb;
    let mut opsin_inverse_matrix = default_opsin_inverse_matrix();
    let mut opsin_bias = [0.003_793_073_255_275_449_3_f64; 3];
    let mut want_icc = false;

    let all_default = br.read_bool()?;
    if !all_default {
        let extra_fields = br.read_bool()?;
        if extra_fields {
            let _orientation_m1 = br.read_bits(3)?;
            if br.read_bool()? {
                let is_s = br.read_bool()?;
                if is_s { br.read_bits(5)?; } else { br.read_u32_dist(&[(1, 9), (1, 13), (1, 18), (1, 30)])?; }
                let ir = br.read_bits(3)?;
                if ir == 0 {
                    if is_s { br.read_bits(5)?; } else { br.read_u32_dist(&[(1, 9), (1, 13), (1, 18), (1, 30)])?; }
                }
            }
            if br.read_bool()? {
                let div8 = br.read_bool()?;
                if div8 { br.read_bits(5)?; } else { br.read_u32_dist(&[(1, 6), (1, 8), (1, 10), (1, 12)])?; }
                let pr = br.read_bits(3)?;
                if pr == 0 {
                    if div8 { br.read_bits(5)?; } else { br.read_u32_dist(&[(1, 6), (1, 8), (1, 10), (1, 12)])?; }
                }
            }
            if br.read_bool()? {
                br.read_u32_dist(&[(100, 0), (1000, 0), (1, 10), (1, 30)])?;
                br.read_u32_dist(&[(1, 0), (1001, 0), (1, 8), (1, 10)])?;
                br.read_u32_dist(&[(0, 0), (0, 3), (0, 16), (0, 32)])?;
                br.read_bool()?;
            }
        }
        floating_point = br.read_bool()?;
        let bps = if !floating_point {
            br.read_u32_dist(&[(8, 0), (10, 0), (12, 0), (1, 6)])?
        } else {
            let b = br.read_u32_dist(&[(32, 0), (16, 0), (24, 0), (1, 6)])?;
            let _exp = 1 + br.read_bits(4)?;
            b
        };
        if bps == 0 || bps > 32 {
            return Err(JxlError::BitPrecisionMismatch);
        }
        bits_per_sample = bps as u8;
        let _modular_16bit_suff = br.read_bool()?;
        let nec = br.read_u32_dist(&[(0, 0), (1, 0), (2, 4), (1, 12)])? as usize;
        if nec > limits.max_extra_channels {
            return Err(JxlError::DimensionLimit);
        }
        num_extra_channels = nec as u16;
        for _ in 0..nec {
            let ec_default = br.read_bool()?;
            let ec_type = if ec_default { 0 } else { br.read_enum()? };
            if ec_type == 0 {
                has_alpha = true;
            }
            if !ec_default {
                if br.read_bool()? {
                    let fp = br.read_bool()?;
                    br.read_u32_dist(if fp { &[(32, 0), (16, 0), (24, 0), (1, 6)] } else { &[(8, 0), (10, 0), (12, 0), (1, 6)] })?;
                    if fp { br.read_bits(4)?; }
                }
                br.read_u32_dist(&[(0, 0), (3, 0), (4, 0), (1, 3)])?;
                let name_len = br.read_u32_dist(&[(0, 0), (0, 4), (16, 5), (48, 10)])?;
                for _ in 0..name_len { br.read_bits(8)?; }
                if ec_type == 0 { br.read_bool()?; }
                if ec_type == 1 { for _ in 0..4 { br.read_f16()?; } }
                if ec_type == 6 { br.read_u32_dist(&[(1, 0), (0, 2), (3, 3), (19, 10)])?; }
            }
        }
        xyb_encoded = br.read_bool()?;
        let col_default = br.read_bool()?;
        if !col_default {
            want_icc = br.read_bool()?;
            let cspace = br.read_enum()?;
            is_gray = cspace == 1;
            if !want_icc {
                if cspace != 3 {
                    let wp = br.read_enum()?;
                    if wp == 2 {
                        br.read_u32_dist(&[(0, 19), (524288, 19), (1048576, 20), (2097152, 21)])?;
                        br.read_u32_dist(&[(0, 19), (524288, 19), (1048576, 20), (2097152, 21)])?;
                    }
                    if cspace != 1 {
                        let prim = br.read_enum()?;
                        if prim == 2 {
                            for _ in 0..6 {
                                br.read_u32_dist(&[(0, 19), (524288, 19), (1048576, 20), (2097152, 21)])?;
                            }
                        }
                    }
                }
                if br.read_bool()? {
                    transfer = JxlTransferFunction::Gamma(br.read_bits(24)? as u32);
                } else {
                    transfer = match br.read_enum()? {
                        8 => JxlTransferFunction::Linear,
                        17 => JxlTransferFunction::Pq,
                        18 => JxlTransferFunction::Hlg,
                        _ => JxlTransferFunction::Srgb,
                    };
                }
                let _intent = br.read_enum()?;
            }
        }
        if extra_fields && !br.read_bool()? {
            br.read_f16()?; br.read_f16()?; br.read_bool()?; br.read_f16()?;
        }
        if br.read_u64()? != 0 {
            return Err(JxlError::Unsupported("ImageMetadata extensions unsupported"));
        }
    }

    let td_default = br.read_bool()?;
    if !td_default {
        if !br.read_bool()? {
            for row in &mut opsin_inverse_matrix {
                for v in row {
                    *v = br.read_f16()? as f64;
                }
            }
            for b in &mut opsin_bias {
                *b = br.read_f16()? as f64;
            }
            for _ in 0..4 { br.read_f16()?; }
        }
        let cw_mask = br.read_bits(3)?;
        if cw_mask & 1 != 0 { for _ in 0..15 { br.read_f16()?; } }
        if cw_mask & 2 != 0 { for _ in 0..55 { br.read_f16()?; } }
        if cw_mask & 4 != 0 { for _ in 0..210 { br.read_f16()?; } }
        if !br.read_bool()? {
            br.read_f16()?; br.read_f16()?;
        }
        if br.read_u64()? != 0 {
            return Err(JxlError::Unsupported("TransformData extensions unsupported"));
        }
    }

    if want_icc {
        let icc_size = br.read_u64()? as usize;
        if icc_size > limits.max_compressed_bytes {
            return Err(JxlError::ByteLimit);
        }
        let _icc = decode_prefix_or_ans_blob(br, icc_size, 41, limits.max_compressed_bytes)?;
    }

    Ok(JxlImageHeader {
        width, height, bits_per_sample, floating_point, xyb_encoded, is_gray,
        num_extra_channels, has_alpha, transfer, opsin_inverse_matrix, opsin_bias,
        cfl_default_factors: true,
    })
}

pub fn parse_frame_header(br: &mut JxlBitReader<'_>, img: &JxlImageHeader) -> Result<JxlFrameHeader, JxlError> {
    br.skip_pad_to_byte()?;
    let all_default = br.read_bool()?;
    let mut fh = JxlFrameHeader {
        frame_type: JxlFrameType::Regular,
        encoding: JxlEncoding::VarDct,
        flags: 0,
        do_ycbcr: false,
        group_size_shift: 1,
        num_passes: 1,
        is_last: true,
        gab: true,
        epf_iters: 2,
        toc_lengths: Vec::new(),
        x_qm_scale: 3,
        b_qm_scale: 2,
    };
    if !all_default {
        fh.frame_type = match br.read_bits(2)? {
            0 => JxlFrameType::Regular,
            1 => JxlFrameType::LfFrame,
            2 => JxlFrameType::ReferenceOnly,
            _ => JxlFrameType::SkipProgressive,
        };
        fh.encoding = if br.read_bits(1)? == 1 { JxlEncoding::Modular } else { JxlEncoding::VarDct };
        fh.flags = br.read_u64()?;
        if !img.xyb_encoded {
            fh.do_ycbcr = br.read_bool()?;
        }
        if (fh.flags & (1 << 5)) == 0 {
            if fh.do_ycbcr {
                for _ in 0..3 { br.read_bits(2)?; }
            }
            let _upsampling = br.read_u32_dist(&[(1, 0), (2, 0), (4, 0), (8, 0)])?;
            for _ in 0..img.num_extra_channels {
                br.read_u32_dist(&[(1, 0), (2, 0), (4, 0), (8, 0)])?;
            }
        }
        if fh.encoding == JxlEncoding::Modular {
            fh.group_size_shift = br.read_bits(2)? as u8;
        } else if img.xyb_encoded {
            fh.x_qm_scale = br.read_bits(3)? as u8;
            fh.b_qm_scale = br.read_bits(3)? as u8;
        }
        if fh.frame_type != JxlFrameType::ReferenceOnly {
            fh.num_passes = br.read_u32_dist(&[(1, 0), (2, 0), (3, 0), (4, 3)])?;
            if fh.num_passes != 1 {
                let num_ds = br.read_u32_dist(&[(0, 0), (1, 0), (2, 0), (3, 1)])?;
                for _ in 0..num_ds { br.read_bits(2)?; }
                for _ in 0..num_ds { br.read_u32_dist(&[(0, 0), (1, 0), (2, 0), (0, 3)])?; }
                for _ in 0..fh.num_passes { br.read_bits(2)?; }
            }
        }
        if fh.frame_type == JxlFrameType::LfFrame {
            let _lf_level = 1 + br.read_bits(2)?;
        } else if br.read_bool()? {
            for _ in 0..2 {
                if br.read_bool()? { br.read_bits(32)?; } else { br.read_u32_dist(&[(0, 8), (256, 11), (2304, 14), (18688, 30)])?; }
            }
            for _ in 0..2 { br.read_u32_dist(&[(0, 8), (256, 11), (2304, 14), (18688, 30)])?; }
        }
        if matches!(fh.frame_type, JxlFrameType::Regular | JxlFrameType::SkipProgressive) {
            let mode = br.read_bits(2)?;
            if img.num_extra_channels > 0 && mode > 0 {
                br.read_bool()?;
                if mode == 1 || mode == 2 {
                    br.read_u32_dist(&[(0, 0), (1, 0), (2, 0), (3, 3)])?;
                }
            }
            for _ in 0..img.num_extra_channels {
                let m2 = br.read_bits(2)?;
                if m2 > 0 { br.read_bool()?; br.read_u32_dist(&[(0, 0), (1, 0), (2, 0), (3, 3)])?; }
            }
            fh.is_last = br.read_bool()?;
        }
        let name_len = br.read_u32_dist(&[(0, 0), (0, 4), (16, 5), (48, 10)])?;
        for _ in 0..name_len { br.read_bits(8)?; }
        if !br.read_bool()? {
            fh.gab = br.read_bool()?;
            if fh.gab && br.read_bool()? {
                for _ in 0..6 { br.read_f16()?; }
            }
            fh.epf_iters = br.read_bits(2)? as u8;
            if fh.epf_iters > 0 {
                if fh.encoding == JxlEncoding::VarDct && br.read_bool()? {
                    for _ in 0..8 { br.read_f16()?; }
                }
                if br.read_bool()? { for _ in 0..3 { br.read_f16()?; } }
                if br.read_bool()? { br.read_f16()?; }
                if !br.read_bool()? {
                    br.read_f16()?; br.read_f16()?; br.read_f16()?; br.read_f16()?;
                }
            }
            if br.read_u64()? != 0 {
                return Err(JxlError::Unsupported("RestorationFilter extensions unsupported"));
            }
        }
        if br.read_u64()? != 0 {
            return Err(JxlError::Unsupported("FrameHeader extensions unsupported"));
        }
    }

    let group_dim = 128u32 << fh.group_size_shift;
    let gx = img.width.div_ceil(group_dim);
    let gy = img.height.div_ceil(group_dim);
    let num_groups = gx * gy;
    let lfx = gx.div_ceil(8);
    let lfy = gy.div_ceil(8);
    let num_entries = if num_groups == 1 && fh.num_passes == 1 {
        1usize
    } else {
        1 + (lfx * lfy) as usize + 1 + (num_groups as usize * fh.num_passes as usize)
    };
    if br.read_bool()? {
        return Err(JxlError::Unsupported("permuted JXL TOC sections unsupported"));
    }
    br.skip_pad_to_byte()?;
    fh.toc_lengths.reserve_exact(num_entries);
    for _ in 0..num_entries {
        fh.toc_lengths.push(br.read_u32_dist(&[(0, 10), (1024, 14), (17408, 22), (4211712, 30)])? as usize);
    }
    br.skip_pad_to_byte()?;
    Ok(fh)
}
