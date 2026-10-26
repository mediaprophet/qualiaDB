//! Hand-authored BPTC vectors from Khronos Data Format, BC7 bit allocation.
//! This writer uses the format table, never the decoder's tables or helpers.
use super::*;

struct Bits {
    value: u128,
    position: u32,
}
impl Bits {
    fn put(&mut self, value: u8, width: u8) {
        assert!((value as u128) < (1u128 << width));
        self.value |= (value as u128) << self.position;
        self.position += width as u32;
    }
}
// subsets, partition, rotation, selection, RGB, A, endpoint P, shared P, I0, I1
const FORMAT: [[u8; 10]; 8] = [
    [3, 4, 0, 0, 4, 0, 1, 0, 3, 0],
    [2, 6, 0, 0, 6, 0, 0, 1, 3, 0],
    [3, 6, 0, 0, 5, 0, 0, 0, 2, 0],
    [2, 6, 0, 0, 7, 0, 1, 0, 2, 0],
    [1, 0, 2, 1, 5, 6, 0, 0, 2, 3],
    [1, 0, 2, 0, 7, 8, 0, 0, 2, 2],
    [1, 0, 0, 0, 7, 7, 1, 0, 4, 0],
    [2, 6, 0, 0, 5, 5, 1, 0, 2, 0],
];

fn block(
    mode: usize,
    endpoints: &[[u8; 4]],
    pbits: &[u8],
    rotation: u8,
    selection: u8,
    index0: [u8; 16],
    index1: [u8; 16],
) -> [u8; 16] {
    let f = FORMAT[mode];
    let mut b = Bits {
        value: 0,
        position: 0,
    };
    b.put(0, mode as u8);
    b.put(1, 1);
    if f[1] > 0 {
        b.put(0, f[1]);
    } // partition zero, specified below
    if f[2] > 0 {
        b.put(rotation, f[2]);
    }
    if f[3] > 0 {
        b.put(selection, f[3]);
    }
    assert_eq!(endpoints.len(), f[0] as usize * 2);
    for channel in 0..3 {
        for e in endpoints {
            b.put(e[channel], f[4]);
        }
    }
    if f[5] > 0 {
        for e in endpoints {
            b.put(e[3], f[5]);
        }
    }
    for p in pbits {
        b.put(*p, 1);
    }
    // BPTC partition-zero fix-up positions: subset0=0, subset1=15
    // for two subsets; subset0=0, subset1=3, subset2=15 for three.
    for (i, v) in index0.into_iter().enumerate() {
        let anchor = i == 0 || (f[0] == 2 && i == 15) || (f[0] == 3 && (i == 3 || i == 15));
        b.put(v, f[8] - u8::from(anchor));
    }
    if f[9] > 0 {
        for (i, v) in index1.into_iter().enumerate() {
            b.put(v, f[9] - u8::from(i == 0));
        }
    }
    assert_eq!(b.position, 128, "mode {mode} bit allocation");
    b.value.to_le_bytes()
}
fn decode(block: [u8; 16]) -> [[u8; 4]; 16] {
    let mut pixels = [[17; 4]; 16];
    decode_bc7_block(&block, &mut pixels);
    pixels
}
fn constant(mode: usize, transparent: bool) -> [u8; 16] {
    let f = FORMAT[mode];
    let rgb = (1u16 << f[4]) - 1;
    let alpha = if transparent { 0 } else { (1u16 << f[5]) - 1 };
    let endpoint = if transparent {
        [rgb as u8, 0, rgb as u8, alpha as u8]
    } else {
        [rgb as u8; 4]
    };
    let mut endpoints = vec![endpoint; f[0] as usize * 2];
    if !transparent {
        for e in &mut endpoints {
            e[3] = alpha as u8;
        }
    }
    let pcount = if f[6] > 0 {
        f[0] * 2
    } else if f[7] > 0 {
        f[0]
    } else {
        0
    };
    let pbits = vec![u8::from(!transparent); pcount as usize];
    block(mode, &endpoints, &pbits, 0, 0, [0; 16], [0; 16])
}

#[test]
fn bc7_all_modes_decode_deterministically_without_fallback() {
    // Opaque white requires all endpoint precisions and P bits to expand to255.
    for mode in 0..8 {
        let encoded = constant(mode, false);
        assert_eq!(decode(encoded), [[255; 4]; 16], "opaque mode {mode}");
        assert_eq!(
            decode(encoded),
            decode(encoded),
            "deterministic mode {mode}"
        );
    }
    // Explicit-alpha modes legitimately encode transparent *colored* texels.
    // The former arbitrary-payload alpha>0 oracle rejected these valid blocks.
    for (mode, red) in [(4, 255), (5, 255), (6, 254), (7, 251)] {
        assert_eq!(
            decode(constant(mode, true)),
            [[red, 0, red, 0]; 16],
            "transparent mode {mode}"
        );
    }
    assert_eq!(decode([0; 16]), [[0; 4]; 16], "reserved zero prefix");
}

#[test]
fn bc7_partition_zero_and_pbits_preserve_exact_subset_colors() {
    // Khronos partition zero is independent of implementation partition tables.
    let two = [0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1];
    let three = [0, 0, 1, 1, 0, 0, 1, 1, 0, 2, 2, 1, 2, 2, 2, 2];
    for mode in [0usize, 1, 2, 3, 7] {
        let f = FORMAT[mode];
        let rgb = ((1u16 << f[4]) - 1) as u8;
        let alpha = if f[5] > 0 {
            ((1u16 << f[5]) - 1) as u8
        } else {
            0
        };
        let mut endpoints = vec![[0; 4]; f[0] as usize * 2];
        for subset in 0..f[0] as usize {
            for e in &mut endpoints[subset * 2..subset * 2 + 2] {
                e[subset] = rgb;
                e[3] = alpha;
            }
        }
        let count = if f[6] > 0 {
            f[0] * 2
        } else if f[7] > 0 {
            f[0]
        } else {
            0
        };
        let encoded = block(
            mode,
            &endpoints,
            &vec![0; count as usize],
            0,
            0,
            [0; 16],
            [0; 16],
        );
        let pixels = decode(encoded);
        // Zero P bits expand component maxima to these exact UNORM8 values.
        let max = match mode {
            0 => 247,
            1 => 253,
            2 => 255,
            3 => 254,
            7 => 251,
            _ => unreachable!(),
        };
        for i in 0..16 {
            let subset = if f[0] == 2 { two[i] } else { three[i] };
            let mut expected = [0, 0, 0, if mode == 7 { max } else { 255 }];
            expected[subset] = max;
            assert_eq!(pixels[i], expected, "mode {mode}, pixel {i}");
        }
    }
}

#[test]
fn bc7_dual_index_selection_and_component_rotation_are_exact() {
    for (mode, alpha_index, alpha_weight) in [(4usize, 3, 27u16), (5, 1, 21)] {
        let f = FORMAT[mode];
        let endpoints = [
            [0; 4],
            [
                ((1u16 << f[4]) - 1) as u8,
                ((1u16 << f[4]) - 1) as u8,
                ((1u16 << f[4]) - 1) as u8,
                ((1u16 << f[5]) - 1) as u8,
            ],
        ];
        for selection in 0..=u8::from(mode == 4) {
            for rotation in 0..4 {
                let encoded = block(
                    mode,
                    &endpoints,
                    &[],
                    rotation,
                    selection,
                    [1; 16],
                    [alpha_index; 16],
                );
                let c = ((255 * 21 + 32) >> 6) as u8;
                let a = ((255 * alpha_weight + 32) >> 6) as u8;
                let (c, a) = if selection == 0 { (c, a) } else { (a, c) };
                let mut expected = [c, c, c, a];
                if rotation > 0 {
                    expected.swap((rotation - 1) as usize, 3);
                }
                assert_eq!(
                    decode(encoded),
                    [expected; 16],
                    "mode {mode}, selection {selection}, rotation {rotation}"
                );
            }
        }
    }
}

#[test]
fn bc7_mode6_four_bit_interpolation_and_fixup_are_exact() {
    let endpoints = [[0, 0, 0, 0], [127, 127, 127, 127]];
    let indices = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let encoded = block(6, &endpoints, &[0, 1], 0, 0, indices, [0; 16]);
    let weights = [
        0u16, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64,
    ];
    let expected = weights.map(|w| [((255 * w + 32) >> 6) as u8; 4]);
    assert_eq!(decode(encoded), expected);
}

#[test]
fn etc2_eac_and_astc_4x4_void_extent_decode_correctly() {
    let mut astc_block = [0u8; 16];
    astc_block[0] = 0xFC;
    astc_block[1] = 0xFD;
    astc_block[8..10].copy_from_slice(&0x8000u16.to_le_bytes());
    astc_block[10..12].copy_from_slice(&0x4000u16.to_le_bytes());
    astc_block[12..14].copy_from_slice(&0xFF00u16.to_le_bytes());
    astc_block[14..16].copy_from_slice(&0xFF00u16.to_le_bytes());
    let mut out_astc = [[0u8; 4]; 16];
    decode_astc_4x4_block(&astc_block, &mut out_astc);
    assert_eq!(out_astc[0], [0x80, 0x40, 0xFF, 0xFF]);
    assert_eq!(out_astc[15], [0x80, 0x40, 0xFF, 0xFF]);

    let mut etc2_block = [0u8; 16];
    etc2_block[0] = 200;
    etc2_block[8] = 0xFF;
    etc2_block[9] = 0x88;
    etc2_block[10] = 0x44;
    let mut out_etc2 = [[0u8; 4]; 16];
    decode_etc2_eac_rgba_block(&etc2_block, &mut out_etc2);
    assert!(out_etc2[0][3] >= 185 && out_etc2[0][3] <= 215);
    assert!(out_etc2[0][0] > out_etc2[0][2]);
}
