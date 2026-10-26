//! Deterministic W3C CBOR-LD (`0xD9 0x0501`) binary codec for Portal HUD documents.

use super::{validate_hud_items, HudItem};

pub const HUD_CBOR_LD_TAG_COMPRESSED: u16 = 0x0501;
pub const HUD_CBOR_LD_TAG_UNCOMPRESSED: u16 = 0x0500;
const TERM_CONTEXT: u64 = 1;
const TERM_UI_CONTEXT_V1: u64 = 10;
const TERM_UI_ITEMS: u64 = 24;

/// Encodes a slice of [`HudItem`] into deterministic compressed W3C CBOR-LD (`0xD9 0x0501`).
pub fn encode_hud_document_cbor_ld(items: &[HudItem]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16 + items.len() * 88);
    // Tag 0x0501 (1281 = compressed CBOR-LD)
    buf.extend_from_slice(&[0xD9, 0x05, 0x01]);
    // 2-entry envelope map: { 1 (@context): 10 (ui/v1), 24 (ui:items): [...] }
    write_type_len(&mut buf, 5, 2);
    write_type_len(&mut buf, 0, TERM_CONTEXT);
    write_type_len(&mut buf, 0, TERM_UI_CONTEXT_V1);
    write_type_len(&mut buf, 0, TERM_UI_ITEMS);
    write_type_len(&mut buf, 4, items.len() as u64);

    for item in items {
        // 12-entry map with ascending integer term keys 1..=12 (RFC 8949 §4.2 canonical)
        write_type_len(&mut buf, 5, 12);
        write_type_len(&mut buf, 0, 1);
        write_str(&mut buf, &item.id);
        write_type_len(&mut buf, 0, 2);
        write_str(&mut buf, &item.kind);
        write_type_len(&mut buf, 0, 3);
        write_str(&mut buf, &item.text);
        write_type_len(&mut buf, 0, 4);
        write_f64(&mut buf, item.x);
        write_type_len(&mut buf, 0, 5);
        write_f64(&mut buf, item.y);
        write_type_len(&mut buf, 0, 6);
        write_f64(&mut buf, item.z);
        write_type_len(&mut buf, 0, 7);
        write_f64(&mut buf, item.w);
        write_type_len(&mut buf, 0, 8);
        write_f64(&mut buf, item.h);
        write_type_len(&mut buf, 0, 9);
        write_f64(&mut buf, item.value);
        write_type_len(&mut buf, 0, 10);
        buf.push(if item.disabled { 0xF5 } else { 0xF4 });
        write_type_len(&mut buf, 0, 11);
        write_str(&mut buf, &item.icon);
        write_type_len(&mut buf, 0, 12);
        write_str(&mut buf, &item.color);
    }
    buf
}

/// Decodes and validates a binary W3C CBOR-LD (`0xD9 0x0501` / `0xD9 0x0500`) HUD document.
pub fn decode_hud_document_cbor_ld(bytes: &[u8]) -> Result<Vec<HudItem>, String> {
    if bytes.len() > 96_000 {
        return Err("HUD document too large".into());
    }
    let mut pos = 0usize;
    let (tag_major, tag_val) = read_type_len(bytes, &mut pos)?;
    if tag_major != 6
        || (tag_val != HUD_CBOR_LD_TAG_COMPRESSED as u64
            && tag_val != HUD_CBOR_LD_TAG_UNCOMPRESSED as u64)
    {
        return Err("invalid CBOR-LD tag for HUD document".into());
    }
    let (map_major, map_len) = read_type_len(bytes, &mut pos)?;
    if map_major != 5 || map_len < 2 {
        return Err("invalid CBOR-LD HUD envelope map".into());
    }

    let mut items_opt: Option<Vec<HudItem>> = None;
    for _ in 0..map_len {
        let (k_major, k_val) = read_type_len(bytes, &mut pos)?;
        if k_major != 0 {
            return Err("expected compressed integer key in HUD envelope".into());
        }
        match k_val {
            TERM_CONTEXT => {
                let (c_major, _c_val) = read_type_len(bytes, &mut pos)?;
                if c_major == 3 {
                    let _ = read_str_payload(bytes, &mut pos, _c_val as usize)?;
                } else if c_major != 0 {
                    return Err("invalid @context value in HUD envelope".into());
                }
            }
            TERM_UI_ITEMS => {
                let (arr_major, arr_len) = read_type_len(bytes, &mut pos)?;
                if arr_major != 4 {
                    return Err("expected array for ui:items".into());
                }
                if arr_len > 256 {
                    return Err("HUD has too many items".into());
                }
                let mut items = Vec::with_capacity(arr_len as usize);
                for _ in 0..arr_len {
                    items.push(decode_hud_item(bytes, &mut pos)?);
                }
                items_opt = Some(items);
            }
            _ => return Err("unknown key in HUD CBOR-LD envelope".into()),
        }
    }

    let items = items_opt.ok_or_else(|| "missing ui:items in HUD CBOR-LD document".to_string())?;
    validate_hud_items(&items).map_err(|e| e.to_string())?;
    Ok(items)
}

fn decode_hud_item(bytes: &[u8], pos: &mut usize) -> Result<HudItem, String> {
    let (m_major, m_len) = read_type_len(bytes, pos)?;
    if m_major != 5 {
        return Err("expected map for HudItem".into());
    }
    let mut item = HudItem {
        id: String::new(),
        kind: String::new(),
        text: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
        h: 0.0,
        value: 0.0,
        disabled: false,
        icon: String::new(),
        color: String::new(),
    };
    for _ in 0..m_len {
        let (k_major, k_id) = read_type_len(bytes, pos)?;
        if k_major != 0 {
            return Err("expected integer field key in HudItem".into());
        }
        match k_id {
            1 => item.id = read_str(bytes, pos)?,
            2 => item.kind = read_str(bytes, pos)?,
            3 => item.text = read_str(bytes, pos)?,
            4 => item.x = read_f64(bytes, pos)?,
            5 => item.y = read_f64(bytes, pos)?,
            6 => item.z = read_f64(bytes, pos)?,
            7 => item.w = read_f64(bytes, pos)?,
            8 => item.h = read_f64(bytes, pos)?,
            9 => item.value = read_f64(bytes, pos)?,
            10 => item.disabled = read_bool(bytes, pos)?,
            11 => item.icon = read_str(bytes, pos)?,
            12 => item.color = read_str(bytes, pos)?,
            _ => return Err("unknown field key in HudItem".into()),
        }
    }
    Ok(item)
}

fn write_type_len(buf: &mut Vec<u8>, major: u8, len: u64) {
    if len < 24 {
        buf.push((major << 5) | (len as u8));
    } else if len < 256 {
        buf.push((major << 5) | 24);
        buf.push(len as u8);
    } else if len < 65536 {
        buf.push((major << 5) | 25);
        buf.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        buf.push((major << 5) | 26);
        buf.extend_from_slice(&(len as u32).to_be_bytes());
    }
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    write_type_len(buf, 3, s.len() as u64);
    buf.extend_from_slice(s.as_bytes());
}

fn write_f64(buf: &mut Vec<u8>, v: f64) {
    buf.push(0xFB);
    buf.extend_from_slice(&v.to_bits().to_be_bytes());
}

fn read_type_len(bytes: &[u8], pos: &mut usize) -> Result<(u8, u64), String> {
    let b = *bytes.get(*pos).ok_or("unexpected EOF in HUD CBOR-LD")?;
    *pos += 1;
    let major = b >> 5;
    let ai = b & 0x1F;
    let val = match ai {
        0..=23 => ai as u64,
        24 => {
            let v = *bytes.get(*pos).ok_or("unexpected EOF")? as u64;
            *pos += 1;
            v
        }
        25 => {
            if *pos + 2 > bytes.len() {
                return Err("unexpected EOF".into());
            }
            let v = u16::from_be_bytes([bytes[*pos], bytes[*pos + 1]]) as u64;
            *pos += 2;
            v
        }
        26 => {
            if *pos + 4 > bytes.len() {
                return Err("unexpected EOF".into());
            }
            let v = u32::from_be_bytes([
                bytes[*pos],
                bytes[*pos + 1],
                bytes[*pos + 2],
                bytes[*pos + 3],
            ]) as u64;
            *pos += 4;
            v
        }
        _ => return Err("unsupported CBOR additional info".into()),
    };
    Ok((major, val))
}

fn read_str(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    let (major, len) = read_type_len(bytes, pos)?;
    if major != 3 {
        return Err("expected CBOR text string".into());
    }
    read_str_payload(bytes, pos, len as usize)
}

fn read_str_payload(bytes: &[u8], pos: &mut usize, len: usize) -> Result<String, String> {
    if *pos + len > bytes.len() {
        return Err("unexpected EOF in string".into());
    }
    let s = std::str::from_utf8(&bytes[*pos..*pos + len])
        .map_err(|_| "invalid UTF-8 in HUD string".to_string())?
        .to_string();
    *pos += len;
    Ok(s)
}

fn read_f64(bytes: &[u8], pos: &mut usize) -> Result<f64, String> {
    let b = *bytes.get(*pos).ok_or("unexpected EOF")?;
    if b != 0xFB || *pos + 9 > bytes.len() {
        return Err("expected CBOR f64".into());
    }
    *pos += 1;
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[*pos..*pos + 8]);
    *pos += 8;
    Ok(f64::from_bits(u64::from_be_bytes(arr)))
}

fn read_bool(bytes: &[u8], pos: &mut usize) -> Result<bool, String> {
    let b = *bytes.get(*pos).ok_or("unexpected EOF")?;
    *pos += 1;
    match b {
        0xF5 => Ok(true),
        0xF4 => Ok(false),
        _ => Err("expected CBOR bool".into()),
    }
}
