#![allow(dead_code)]
//! Hand-authored format oracle; no Etalon or Qualia serializers are used.
pub const SIZE: usize = 1600;
pub const NAMES: [&[u8]; 3] = [
    b"blk.0.attn_q.weight",
    b"blk.0.attn_k.weight",
    b"tensor.1122334455667788",
];
pub const BLOBS: [(usize, usize); 3] = [(1024, 8), (1280, 8), (1536, 4)];
pub fn u16_at(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
pub fn u32_at(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
pub fn u64_at(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
pub fn get_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
pub fn crc(bytes: &[u8]) -> u32 {
    let mut value = u32::MAX;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ if value & 1 != 0 { 0x82f6_3b78 } else { 0 };
        }
    }
    !value
}
pub fn seal_metadata(bytes: &mut [u8]) {
    let value = crc(&bytes[..768]);
    u32_at(bytes, 768, value);
}
pub fn seal(bytes: &mut [u8]) {
    seal_metadata(bytes);
    for i in 0..get_u32(bytes, 36) as usize {
        let entry = 192 + i * 64;
        let start = get_u32(bytes, entry + 32) as usize;
        let len = get_u32(bytes, entry + 36) as usize;
        let value = crc(&bytes[start..start + len]);
        u32_at(bytes, 772 + i * 4, value);
    }
}
pub fn canonical() -> [u8; SIZE] {
    let mut b = [0; SIZE];
    b[..4].copy_from_slice(b"p64\0");
    u16_at(&mut b, 4, 4);
    u16_at(&mut b, 6, 0x71);
    for (at, value) in [
        (8, 128),
        (12, 192),
        (16, 640),
        (20, 64),
        (24, 384),
        (28, 768),
        (32, 512),
        (36, 3),
        (40, 512),
    ] {
        u32_at(&mut b, at, value);
    }
    for (at, value) in [
        (64, 1),
        (68, 4),
        (72, 2),
        (76, 1),
        (80, 2),
        (84, 10000f32.to_bits()),
        (88, 1f32.to_bits()),
        (92, 2),
        (96, 2),
        (100, 128),
        (104, 1),
        (108, 30f32.to_bits()),
        (112, 7),
        (116, 3),
    ] {
        u32_at(&mut b, at, value);
    }
    u32_at(&mut b, 128, 0);
    u32_at(&mut b, 132, 1024);
    u32_at(&mut b, 136, 1288);
    u16_at(&mut b, 140, 2);
    u16_at(&mut b, 142, 5);
    let mut name_at = 385;
    for (i, name) in NAMES.into_iter().enumerate() {
        let e = 192 + i * 64;
        u32_at(&mut b, e, (name_at - 384) as u32);
        b[name_at..name_at + name.len()].copy_from_slice(name);
        name_at += name.len() + 1;
        u16_at(&mut b, e + 4, [2, 0, 0xfffe][i]);
        u16_at(&mut b, e + 6, [0, 0, 1][i]);
        u32_at(&mut b, e + 8, if i == 2 { 1 } else { 0 });
        u32_at(&mut b, e + 12, 1);
        for (d, value) in [2, 1, 1, 1].into_iter().enumerate() {
            u32_at(&mut b, e + 16 + d * 4, value);
        }
        u32_at(&mut b, e + 32, BLOBS[i].0 as u32);
        u32_at(&mut b, e + 36, BLOBS[i].1 as u32);
        u64_at(&mut b, e + 40, 0x1000 + i as u64 * 0x100);
        u64_at(&mut b, e + 48, 0x1122_3344_5566_7786 + i as u64);
        u16_at(&mut b, e + 56, 0xffff);
        u16_at(&mut b, e + 58, 1 << i);
        u32_at(&mut b, e + 60, 0xf000_0000 + i as u32);
    }
    for record in 0..2 {
        for component in 0..10 {
            u32_at(
                &mut b,
                512 + record * 64 + component * 4,
                (component as f32 / 8.0 + record as f32).to_bits(),
            );
        }
    }
    b[640..644].copy_from_slice(b"Q42T");
    u16_at(&mut b, 644, 2);
    u16_at(&mut b, 646, 0);
    u32_at(&mut b, 648, 0);
    u32_at(&mut b, 652, 1);
    b[656] = 1;
    u32_at(&mut b, 660, 0);
    u32_at(&mut b, 664, 2); // empty pre-tokenizer, two tokens
    u32_at(&mut b, 668, 1);
    b[672] = b'a';
    u32_at(&mut b, 673, 1);
    b[677] = b'b';
    u32_at(&mut b, 678, 0);
    b[682] = 1;
    u32_at(&mut b, 686, 1); // no merges, one stop token + eight slots
    for (i, value) in [1.0f32, -2.0, 3.0, 4.0].into_iter().enumerate() {
        u32_at(
            &mut b,
            if i < 2 {
                1024 + i * 4
            } else {
                1280 + (i - 2) * 4
            },
            value.to_bits(),
        );
    }
    b[1536..1540].copy_from_slice(&[0, 0x3c, 0, 0xc0]);
    seal(&mut b);
    b
}

pub struct Case {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub accepted_modes: u8,
}
pub fn cases() -> Vec<Case> {
    let base = canonical();
    let mut result = vec![Case {
        name: "canonical",
        bytes: base.to_vec(),
        accepted_modes: 7,
    }];
    let mut add = |name, bytes: [u8; SIZE], accepted_modes| {
        result.push(Case {
            name,
            bytes: bytes.to_vec(),
            accepted_modes,
        })
    };
    let mut b = base;
    b[1024] ^= 1;
    add("blob CRC", b, 6);
    let mut b = base;
    b[68] ^= 1;
    add("metadata CRC", b, 4);
    let mut b = base;
    b[768] ^= 1;
    add("metadata CRC word", b, 4);
    let mut b = base;
    b[772] ^= 1;
    add("tensor CRC word", b, 6);
    let mut b = base;
    b[0] = b'P';
    add("magic", b, 0);
    for (name, at, value, width) in [
        ("version3", 4, 3, 2),
        ("version5", 4, 5, 2),
        ("endian", 6, 0x70, 2),
        ("page0", 40, 0, 4),
        ("page255", 40, 255, 4),
        ("page384", 40, 384, 4),
        ("hparams overflow", 20, u32::MAX, 4),
        ("table overlap", 12, 64, 4),
        ("tensor count overflow", 36, u32::MAX, 4),
        ("nlayer overflow", 64, u32::MAX, 4),
        ("strings overlap", 24, 300, 4),
        ("strings beyond manifold", 24, 513, 4),
        ("manifold misalignment", 32, 513, 4),
        ("manifold overlap", 16, 576, 4),
        ("tokenizer reversed", 16, 800, 4),
        ("checksum truncated", 28, 1599, 4),
        ("rank zero", 204, 0, 4),
        ("rank five", 204, 5, 4),
        ("manifold index", 200, 2, 4),
        ("name out of bounds", 192, 128, 4),
        ("name arithmetic", 192, u32::MAX, 4),
        ("blob unaligned", 224, 1025, 4),
        ("blob metadata overlap", 224, 512, 4),
        ("blob length overflow", 228, u32::MAX, 4),
        ("blob overlap", 288, 1024, 4),
        ("layer page alignment", 6, 0x51, 2),
        ("layer boundary alignment", 264, 1, 4),
    ] {
        let mut b = base;
        if width == 2 {
            u16_at(&mut b, at, value as u16)
        } else {
            u32_at(&mut b, at, value)
        };
        seal_metadata(&mut b);
        add(name, b, 0);
    }
    let mut b = base;
    b[385..512].fill(0xff);
    seal_metadata(&mut b);
    add("unterminated name", b, 0);
    let mut b = base;
    u32_at(&mut b, 36, 0);
    seal_metadata(&mut b);
    add("zero tensors", b, 7);
    let mut b = base;
    u32_at(&mut b, 228, 0);
    seal(&mut b);
    add("zero blob", b, 7);
    let mut b = base;
    b[385] = 0xff;
    seal_metadata(&mut b);
    add("non utf8 name", b, 7);
    let mut b = base;
    u16_at(&mut b, 6, 0xfff1);
    u16_at(&mut b, 196, 0xeeee);
    u16_at(&mut b, 198, 0xdddd);
    u16_at(&mut b, 250, 0xffff);
    seal_metadata(&mut b);
    add("unknown metadata", b, 7);
    let mut b = base;
    b[44..64].fill(0xab);
    b[120..128].fill(0xcd);
    b[552..576].fill(0xef);
    u32_at(&mut b, 512, 0x7fc0_0123);
    u32_at(&mut b, 516, f32::INFINITY.to_bits());
    u32_at(&mut b, 520, (-0f32).to_bits());
    seal_metadata(&mut b);
    add("reserved and special bits", b, 7);
    for length in [0, 3, 63, 127, 767, 783, 1024, 1539] {
        result.push(Case {
            name: "truncated",
            bytes: base[..length].to_vec(),
            accepted_modes: 0,
        });
    }
    result
}
