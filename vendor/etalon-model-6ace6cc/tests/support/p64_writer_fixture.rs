//! Golden files were emitted by actual Qualia source at immutable commit
//! 07018bb8dbb123f105a91609532b9b564e54a2af before any writer adoption.
//! compiler.rs SHA256 edbd46780226040bcf5497017907a31275265be087c1468f786f57151e4f7aab
//! transcode.rs SHA256 098ebd16c34ae48905c59fff7390ff00fe11f56624bf13f5dd5cea8f87b6e3f6
//! Full compiler/transcode/GGUF/safetensor/quant/ternary modules were compiled by
//! path. ManifoldCoordinate10D and q_hash were extracted verbatim from actual
//! source; unrelated unused platform/Q4-AWQ/FP8 helpers panic if invoked.
//! The harness exercises GGUF F32 verbatim, safetensor F32 verbatim, and vision
//! F32 only. It does not qualify other numerical conversion profiles.
//!
//! Retained generator: Qualia target/inference-migration-baseline/
//! p64-writer-reference-20261103-01/generate.rs (SHA256
//! 404a7b3733b7bcb074f13b96bcf7dee2a04d53aa8de3e1387ef756b70133036c).
//! Run from the Qualia workspace after compiling with Rust 1.96.1 edition2021,
//! -Dwarnings and cached serde/serde_json/bytemuck/half/memmap2 extern crates.
//! The exact executable source and generated log are retained in that 32MiB
//! marker-owned directory. Output copies here are immutable test goldens.
//! Durable reproduction uses the adjacent legacy_p64_writer_generator.rs template
//! and regenerate_p64_writer_goldens.py (stdlib only; no Qualia Cargo dependency):
//! `python crates/etalon-model/tests/support/regenerate_p64_writer_goldens.py verify`
//! `python crates/etalon-model/tests/support/regenerate_p64_writer_goldens.py generate
//! --qualia-root PATH --dependencies PATH/target/debug/deps --output NEW_DIRECTORY`
//! The helper verifies the supplied source checkout against the immutable revision,
//! compiles with cached dependencies, verifies all nine SHA256 values, promotes
//! only matching files and cleans its unique temporary directory on every exit.
//!
//! GGUF fixture has two layers (second empty), reordered Q/K/global/unknown
//! source tensors and a real two-token tokenizer. Raw has Q/FFN/unknown tensors.
//! Vision preserves caller order and ends with page padding. Field extraction
//! below uses explicit LE offsets, not the candidate reader or writer.
use etalon_model::p64::build::{BuildInput, BuildProfile, TensorDescriptor};
use etalon_model::p64::P64TensorEntry;

pub struct Golden {
    pub name: &'static str,
    pub profile: BuildProfile,
    pub page: u16,
    pub bytes: &'static [u8],
    pub sha256: &'static str,
}
pub const GOLDENS: [Golden; 9] = [
    Golden {
        name: "gguf8",
        profile: BuildProfile::GgufLayerPacked,
        page: 8,
        bytes: include_bytes!("p64_writer_golden/gguf-page8.p64"),
        sha256: "7793c8397b60e0f3c3e50d90d5759cd5520129ee3ad9597522c7897ff1944453",
    },
    Golden {
        name: "gguf9",
        profile: BuildProfile::GgufLayerPacked,
        page: 9,
        bytes: include_bytes!("p64_writer_golden/gguf-page9.p64"),
        sha256: "3b9eb20b98badd44ef14c6e442c3ea9c983016f3da4d11667f0f4049613fb9b6",
    },
    Golden {
        name: "gguf12",
        profile: BuildProfile::GgufLayerPacked,
        page: 12,
        bytes: include_bytes!("p64_writer_golden/gguf-page12.p64"),
        sha256: "71f37813cf6a63076f849f70c5cd4d3af452f3ab1b07b60c2b5f5b121bea4741",
    },
    Golden {
        name: "raw8",
        profile: BuildProfile::Safetensor,
        page: 8,
        bytes: include_bytes!("p64_writer_golden/raw-page8.p64"),
        sha256: "94a939b08f341ebdfa0a11ccf6b6eccf6c8660a11587cb1cb5663129250457ec",
    },
    Golden {
        name: "raw9",
        profile: BuildProfile::Safetensor,
        page: 9,
        bytes: include_bytes!("p64_writer_golden/raw-page9.p64"),
        sha256: "e933e63a84663040d193cb0481a8ee2f5676ffa82c1795b6858e6272ed4ffd06",
    },
    Golden {
        name: "raw12",
        profile: BuildProfile::Safetensor,
        page: 12,
        bytes: include_bytes!("p64_writer_golden/raw-page12.p64"),
        sha256: "924ba2ee7b96efed62f9b847ae129e4d683a4c43751f6bd971d90fbf46cc2275",
    },
    Golden {
        name: "vision8",
        profile: BuildProfile::Vision,
        page: 8,
        bytes: include_bytes!("p64_writer_golden/vision-page8.p64"),
        sha256: "25d9ac5f002c9f27ed273e41336a84fc7531cc81371420864a8c9a0e4b431c73",
    },
    Golden {
        name: "vision9",
        profile: BuildProfile::Vision,
        page: 9,
        bytes: include_bytes!("p64_writer_golden/vision-page9.p64"),
        sha256: "0e1089491b33e949161b4772585957ad9e7202b7b99a2a078d1ce264228916be",
    },
    Golden {
        name: "vision12",
        profile: BuildProfile::Vision,
        page: 12,
        bytes: include_bytes!("p64_writer_golden/vision-page12.p64"),
        sha256: "ec8a0959384b981f276c88d346a4413a1cdc69735bd880635851a76e8181560c",
    },
];
pub fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap())
}
pub fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}
pub fn descriptors(g: &Golden) -> Vec<TensorDescriptor<'_>> {
    let b = g.bytes;
    let table = u32le(b, 12) as usize;
    let strings = u32le(b, 24) as usize;
    (0..u32le(b, 36) as usize)
        .map(|i| {
            let e = table + i * 64;
            let name = strings + u32le(b, e) as usize;
            let end = name + b[name..].iter().position(|v| *v == 0).unwrap();
            let blob = u32le(b, e + 32) as usize;
            let len = u32le(b, e + 36) as usize;
            TensorDescriptor {
                name: &b[name..end],
                role_id: u16le(b, e + 4),
                dtype: u16le(b, e + 6),
                manifold_idx: u32le(b, e + 8),
                rank: u32le(b, e + 12),
                dimensions: core::array::from_fn(|d| u32le(b, e + 16 + d * 4)),
                source_offset: u64le(b, e + 40),
                source_name_hash: u64le(b, e + 48),
                alt_dtype: u16le(b, e + 56),
                precision_views_mask: u16le(b, e + 58),
                alt_blob_offset: u32le(b, e + 60),
                blob: &b[blob..blob + len],
            }
        })
        .collect()
}
pub fn input<'a>(g: &'a Golden, tensors: &'a [TensorDescriptor<'a>]) -> BuildInput<'a> {
    let b = g.bytes;
    let manifold = u32le(b, 32) as usize;
    let tokenizer = u32le(b, 16) as usize;
    let checksums = u32le(b, 28) as usize;
    BuildInput {
        profile: g.profile,
        page_log2: g.page,
        flags: u16le(b, 6),
        header_reserved: b[44..64].try_into().unwrap(),
        hparams_bytes: b[64..128].try_into().unwrap(),
        manifold_bytes: &b[manifold..tokenizer],
        tokenizer_bytes: &b[tokenizer..checksums],
        tensors,
    }
}
pub const fn sentinel() -> P64TensorEntry {
    P64TensorEntry {
        name_offset: 0xaaaa_aaaa,
        role_id: 0xbbbb,
        dtype: 0xcccc,
        manifold_idx: 0xdddd_dddd,
        rank: 0xeeee_eeee,
        dimensions: [0x1212_1212; 4],
        blob_offset: 0x3434_3434,
        blob_size: 0x5656_5656,
        source_offset: 0x7878_7878_7878_7878,
        source_name_hash: 0x9090_9090_9090_9090,
        alt_dtype: 0xabab,
        precision_views_mask: 0xcdcd,
        alt_blob_offset: 0xefef_efef,
    }
}
pub fn entry_bytes(e: P64TensorEntry) -> [u8; 64] {
    let mut b = [0; 64];
    for (o, v) in [
        (0, e.name_offset),
        (8, e.manifold_idx),
        (12, e.rank),
        (32, e.blob_offset),
        (36, e.blob_size),
        (60, e.alt_blob_offset),
    ] {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    for (o, v) in [
        (4, e.role_id),
        (6, e.dtype),
        (56, e.alt_dtype),
        (58, e.precision_views_mask),
    ] {
        b[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }
    for (i, v) in e.dimensions.into_iter().enumerate() {
        b[16 + i * 4..20 + i * 4].copy_from_slice(&v.to_le_bytes());
    }
    b[40..48].copy_from_slice(&e.source_offset.to_le_bytes());
    b[48..56].copy_from_slice(&e.source_name_hash.to_le_bytes());
    b
}
pub fn crc(bytes: &[u8]) -> u32 {
    let mut v = u32::MAX;
    for byte in bytes {
        v ^= u32::from(*byte);
        for _ in 0..8 {
            v = (v >> 1) ^ if v & 1 != 0 { 0x82f6_3b78 } else { 0 };
        }
    }
    !v
}
