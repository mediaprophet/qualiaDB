//! Catalog of host `Family.method` invoke ids.
//!
//! Vibe is the language; Poet (and other hosts) supply capabilities.
//! Dotted calls such as `Animation.orbit_spin(t)` resolve against this
//! catalog at compile time and lower to `Host::capability_invoke`.
//! Unknown `Family.method` names fail closed.

mod cosmic;
mod cosmic_ext;
mod ids;
mod local;
mod logic;
mod specialized;
mod stdlib;
mod suggest;

pub use ids::{ALL_INVOKE_IDS, INVOKE_ID_COUNT};
pub use local::{invoke_local, payload_from_args, sample_to_value};
pub use suggest::did_you_mean;

use crate::animation::presets::list_all_presets;
use crate::animation::AnimationFamily;
use crate::value::Value;

/// `true` when known.
pub fn is_known(path: &str) -> bool {
    canonical_id(path).is_some()
}

/// Resolve a dotted path to a canonical invoke id.
///
/// `Animation.evaluate_preset` is already canonical.
/// `Animation.orbit_spin` is a preset alias of `Animation.evaluate_preset`.
/// `Statistics.ols` aliases `Statistics.linear_regression`.
/// `Econ.welfare` aliases `Econ.utilitarian_welfare`.
pub fn canonical_id(path: &str) -> Option<&'static str> {
    if path == "Statistics.ols" {
        return Some("Statistics.linear_regression");
    }
    if path == "Econ.welfare" {
        return Some("Econ.utilitarian_welfare");
    }
    if let Some(id) = lookup_static(path) {
        return Some(id);
    }
    if animation_preset(path).is_some() {
        return Some("Animation.evaluate_preset");
    }
    None
}

/// `Family` grant: `using Animation;` covers every `Animation.*` catalog id
/// and every animation preset alias.
fn utf8_eq_ignore_case(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Family of a builtin.
pub fn family_of(path: &str) -> Option<&str> {
    let dot = path.find('.')?;
    Some(&path[..dot])
}

/// `true` when the grant covers the capability.
pub fn granted_covers(granted: &[&str], path: &str) -> bool {
    if let Some(id) = canonical_id(path) {
        if granted.iter().any(|g| *g == id || *g == path) {
            return true;
        }
        if let Some(fam) = family_of(id) {
            if granted.iter().any(|g| utf8_eq_ignore_case(g, fam)) {
                return true;
            }
        }
        if let Some(fam) = family_of(path) {
            if granted.iter().any(|g| utf8_eq_ignore_case(g, fam)) {
                return true;
            }
        }
        return false;
    }
    if looks_like_catalog_path(path) {
        return false;
    }
    true
}

/// Known catalog family + dotted method, e.g. `HID.poll`.
/// User enums such as `Shape.Circle` are **not** catalog paths.
pub fn looks_like_catalog_path(path: &str) -> bool {
    if animation_preset(path).is_some() {
        return true;
    }
    let Some(fam) = family_of(path) else {
        return false;
    };
    let prefix = format!("{fam}.");
    ALL_INVOKE_IDS.iter().any(|id| id.starts_with(&prefix))
}

/// Animation preset.
pub fn animation_preset(path: &str) -> Option<(&'static str, &'static str)> {
    let name = path.strip_prefix("Animation.")?;
    for info in list_all_presets() {
        if info.preset == name {
            return Some((info.family, info.preset));
        }
    }
    None
}

/// When `Animation.orbit_spin(t)` lowers to `Animation.evaluate_preset`,
/// fill `family`/`preset` on the payload so every Host sees the alias.
/// Named fields already present win.
pub fn apply_preset_alias(path: &str, payload: &mut Value) {
    let Some((family, preset)) = animation_preset(path) else {
        return;
    };
    let Value::Record(map) = payload else {
        return;
    };
    map.entry("family".into())
        .or_insert_with(|| Value::String(family.into()));
    map.entry("preset".into())
        .or_insert_with(|| Value::String(preset.into()));
}

/// Parses family-from-preset.
pub fn parse_family_from_preset(family: &str) -> Option<AnimationFamily> {
    AnimationFamily::from_name(family)
}

fn lookup_static(path: &str) -> Option<&'static str> {
    ALL_INVOKE_IDS.iter().copied().find(|id| *id == path)
}

/// Expand `using Animation;` into the family grant id.
pub fn family_grant(family: &str) -> String {
    family.to_string()
}

/// Methods for a family.
pub fn methods_for_family(family: &str) -> Vec<&'static str> {
    let prefix = format!("{family}.");
    ALL_INVOKE_IDS
        .iter()
        .copied()
        .filter(|id| id.starts_with(&prefix))
        .collect()
}

/// Unique catalog family names, in catalog order.
pub fn families() -> Vec<&'static str> {
    let mut out = Vec::new();
    for id in ALL_INVOKE_IDS {
        if let Some(fam) = family_of(id) {
            if !out.contains(&fam) {
                out.push(fam);
            }
        }
    }
    out
}

/// One-line hover text for a catalog path or identifier.
pub fn describe(path: &str) -> String {
    if let Some((family, preset)) = animation_preset(path) {
        return format!(
            "Animation preset `{preset}` (family {family}) → `Animation.evaluate_preset`"
        );
    }
    if let Some(id) = canonical_id(path) {
        if id == "GraphAuthoring.process" || id == "N3Logic.evaluate" {
            return format!(
                "catalog `{id}` — graph host invoke (fail-closed; lease with `using {}`). Natural persons are rdfs:Class + SHACL/ShEx; owl:Thing is forbidden for persons.",
                family_of(id).unwrap_or("graph")
            );
        }
        if let Some(fam) = family_of(id) {
            return format!(
                "catalog `{id}` — {fam} host invoke (fail-closed; lease with `using {fam};`)"
            );
        }
        return format!("catalog `{id}` — host invoke (fail-closed)");
    }
    if looks_like_catalog_path(path) {
        return match did_you_mean(path) {
            Some(hint) => format!("unknown catalog path `{path}`; did you mean `{hint}`?"),
            None => format!("unknown catalog path `{path}`"),
        };
    }
    format!("ident `{path}`")
}

/// Pure-Rust binary container admission, envelope builder, and digest verifier
/// for `Q42EMF64` v4 and `qac\0\0\0\0\0` v1 (Packet UP-02).
///
/// Streaming remains a QDNF service of QBDL-profile HMC and never lives here;
/// methods strictly admit/verify already-admitted binary byte buffers by entry
/// identity/index and never sniff `.hmc`, stream ZIPs, or accept JSON bodies.
pub(crate) mod envelope_admission {
    use crate::error::{DiagCode, Diagnostic};
    use crate::span::Span;
    use crate::value::Value;

    pub(crate) const HEADER_BYTES: usize = 128;
    pub(crate) const DIRECTORY_BYTES: usize = 128;
    pub(crate) const INDEX_DOMAIN: &[u8] = b"QUALIA-FIELD-INDEX-v1\0";
    pub(crate) const MAX_ENTRIES: usize = 4096;
    pub(crate) const MAX_METADATA_BYTES: usize = 4 << 20;
    pub(crate) const MAX_CONTAINER_BYTES: usize = 64 << 20;

    const FORBIDDEN_SOURCE_KEYS: &[&str] = &[
        "filename",
        "file",
        "file_name",
        "path",
        "filepath",
        "file_path",
        "hmc",
        "hmc_path",
        "bundle",
        "bundle_path",
        "zip",
        "url",
        "uri",
    ];

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum EnvelopeFamily {
        EmfV4,
        QacV1,
    }

    impl EnvelopeFamily {
        pub(crate) const fn magic(self) -> &'static [u8; 8] {
            match self {
                Self::EmfV4 => b"Q42EMF64",
                Self::QacV1 => b"qac\0\0\0\0\0",
            }
        }

        pub(crate) const fn version(self) -> u16 {
            match self {
                Self::EmfV4 => 4,
                Self::QacV1 => 1,
            }
        }

        pub(crate) const fn label(self) -> &'static str {
            match self {
                Self::EmfV4 => "Q42EMF64",
                Self::QacV1 => "qac\\0\\0\\0\\0\\0",
            }
        }

        pub(crate) const fn mode_allowed(self, mode: u16) -> bool {
            match self {
                Self::EmfV4 => matches!(mode, 0x0100..=0x0107 | 0 | 1 | 3),
                Self::QacV1 => matches!(mode, 0x0100..=0x0105),
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) struct AdmittedEntry<'a> {
        pub(crate) mode: u16,
        pub(crate) revision: u16,
        pub(crate) flags: u32,
        pub(crate) stream_id: u32,
        pub(crate) frame_id: u32,
        pub(crate) start_tick: i64,
        pub(crate) time_count: u64,
        pub(crate) spatial_key: u64,
        pub(crate) payload_offset: u64,
        pub(crate) stored_bytes: u64,
        pub(crate) decoded_bytes: u64,
        pub(crate) record_count: u64,
        pub(crate) stride: u32,
        pub(crate) descriptor_id: u32,
        pub(crate) digest: &'a [u8],
        pub(crate) payload: &'a [u8],
    }

    #[derive(Debug)]
    pub(crate) struct AdmittedEnvelope<'a> {
        pub(crate) family: EnvelopeFamily,
        pub(crate) version: u16,
        pub(crate) flags: u32,
        pub(crate) total_bytes: u64,
        pub(crate) metadata: &'a [u8],
        pub(crate) index_digest: &'a [u8],
        pub(crate) entries: Vec<AdmittedEntry<'a>>,
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) struct BuildEntry<'a> {
        pub(crate) mode: u16,
        pub(crate) revision: u16,
        pub(crate) flags: u32,
        pub(crate) stream_id: u32,
        pub(crate) frame_id: u32,
        pub(crate) start_tick: i64,
        pub(crate) time_count: u64,
        pub(crate) spatial_key: u64,
        pub(crate) record_count: u64,
        pub(crate) stride: u32,
        pub(crate) descriptor_id: u32,
        pub(crate) payload: &'a [u8],
    }

    #[inline]
    fn u16le(b: &[u8], o: usize) -> u16 {
        u16::from_le_bytes([b[o], b[o + 1]])
    }

    #[inline]
    fn u32le(b: &[u8], o: usize) -> u32 {
        u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
    }

    #[inline]
    fn u64le(b: &[u8], o: usize) -> u64 {
        u64::from_le_bytes([
            b[o],
            b[o + 1],
            b[o + 2],
            b[o + 3],
            b[o + 4],
            b[o + 5],
            b[o + 6],
            b[o + 7],
        ])
    }

    #[inline]
    fn put16(b: &mut [u8], o: usize, v: u16) {
        b[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }

    #[inline]
    fn put32(b: &mut [u8], o: usize, v: u32) {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    #[inline]
    fn put64(b: &mut [u8], o: usize, v: u64) {
        b[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// CRC-32C (Castagnoli, reflected polynomial 0x82F63B78).
    pub(crate) fn crc32c(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0x82F6_3B78 & 0u32.wrapping_sub(crc & 1));
            }
        }
        !crc
    }

    fn block_crc32c(block: &[u8]) -> u32 {
        let mut copy = [0u8; 128];
        copy.copy_from_slice(&block[..128]);
        copy[124..128].fill(0);
        crc32c(&copy)
    }

    // Standard single-mode BLAKE3 pure-Rust reference hasher matching `blake3::hash`.
    const BLAKE3_IV: [u32; 8] = [
        0x6A09_E667,
        0xBB67_AE85,
        0x3C6E_F372,
        0xA54F_F53A,
        0x510E_527F,
        0x9B05_688C,
        0x1F83_D9AB,
        0x5BE0_CD19,
    ];
    const MSG_PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
    const FLAG_CHUNK_START: u32 = 1 << 0;
    const FLAG_CHUNK_END: u32 = 1 << 1;
    const FLAG_PARENT: u32 = 1 << 2;
    const FLAG_ROOT: u32 = 1 << 3;
    const BLAKE3_BLOCK_LEN: usize = 64;
    const BLAKE3_CHUNK_LEN: usize = 1024;

    #[inline]
    fn blake3_g(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
        s[a] = s[a].wrapping_add(s[b]).wrapping_add(mx);
        s[d] = (s[d] ^ s[a]).rotate_right(16);
        s[c] = s[c].wrapping_add(s[d]);
        s[b] = (s[b] ^ s[c]).rotate_right(12);
        s[a] = s[a].wrapping_add(s[b]).wrapping_add(my);
        s[d] = (s[d] ^ s[a]).rotate_right(8);
        s[c] = s[c].wrapping_add(s[d]);
        s[b] = (s[b] ^ s[c]).rotate_right(7);
    }

    fn blake3_round(s: &mut [u32; 16], m: &[u32; 16]) {
        blake3_g(s, 0, 4, 8, 12, m[0], m[1]);
        blake3_g(s, 1, 5, 9, 13, m[2], m[3]);
        blake3_g(s, 2, 6, 10, 14, m[4], m[5]);
        blake3_g(s, 3, 7, 11, 15, m[6], m[7]);
        blake3_g(s, 0, 5, 10, 15, m[8], m[9]);
        blake3_g(s, 1, 6, 11, 12, m[10], m[11]);
        blake3_g(s, 2, 7, 8, 13, m[12], m[13]);
        blake3_g(s, 3, 4, 9, 14, m[14], m[15]);
    }

    fn blake3_compress(
        cv: &[u32; 8],
        block_words: &[u32; 16],
        counter: u64,
        block_len: u32,
        flags: u32,
    ) -> [u32; 16] {
        let mut s = [
            cv[0],
            cv[1],
            cv[2],
            cv[3],
            cv[4],
            cv[5],
            cv[6],
            cv[7],
            BLAKE3_IV[0],
            BLAKE3_IV[1],
            BLAKE3_IV[2],
            BLAKE3_IV[3],
            counter as u32,
            (counter >> 32) as u32,
            block_len,
            flags,
        ];
        let mut m = *block_words;
        for round in 0..7 {
            blake3_round(&mut s, &m);
            if round < 6 {
                let mut next = [0u32; 16];
                for i in 0..16 {
                    next[i] = m[MSG_PERMUTATION[i]];
                }
                m = next;
            }
        }
        for i in 0..8 {
            s[i] ^= s[i + 8];
            s[i + 8] ^= cv[i];
        }
        s
    }

    fn words_from_block(block: &[u8; BLAKE3_BLOCK_LEN]) -> [u32; 16] {
        let mut words = [0u32; 16];
        for (i, w) in words.iter_mut().enumerate() {
            let o = i * 4;
            *w = u32::from_le_bytes([block[o], block[o + 1], block[o + 2], block[o + 3]]);
        }
        words
    }

    #[derive(Clone, Copy)]
    struct Blake3Output {
        input_cv: [u32; 8],
        block_words: [u32; 16],
        counter: u64,
        block_len: u32,
        flags: u32,
    }

    impl Blake3Output {
        fn chaining_value(&self) -> [u32; 8] {
            let out = blake3_compress(
                &self.input_cv,
                &self.block_words,
                self.counter,
                self.block_len,
                self.flags,
            );
            let mut cv = [0u32; 8];
            cv.copy_from_slice(&out[..8]);
            cv
        }

        fn root_bytes(&self) -> [u8; 32] {
            let out = blake3_compress(
                &self.input_cv,
                &self.block_words,
                0,
                self.block_len,
                self.flags | FLAG_ROOT,
            );
            let mut bytes = [0u8; 32];
            for (i, &w) in out[..8].iter().enumerate() {
                bytes[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
            }
            bytes
        }
    }

    struct Blake3ChunkState {
        cv: [u32; 8],
        chunk_counter: u64,
        block: [u8; BLAKE3_BLOCK_LEN],
        block_len: u8,
        blocks_compressed: u8,
    }

    impl Blake3ChunkState {
        fn new(chunk_counter: u64) -> Self {
            Self {
                cv: BLAKE3_IV,
                chunk_counter,
                block: [0u8; BLAKE3_BLOCK_LEN],
                block_len: 0,
                blocks_compressed: 0,
            }
        }

        fn len(&self) -> usize {
            BLAKE3_BLOCK_LEN * usize::from(self.blocks_compressed) + usize::from(self.block_len)
        }

        fn start_flag(&self) -> u32 {
            if self.blocks_compressed == 0 {
                FLAG_CHUNK_START
            } else {
                0
            }
        }

        fn update(&mut self, mut input: &[u8]) {
            while !input.is_empty() {
                if usize::from(self.block_len) == BLAKE3_BLOCK_LEN {
                    let words = words_from_block(&self.block);
                    let flags = self.start_flag();
                    let out = blake3_compress(
                        &self.cv,
                        &words,
                        self.chunk_counter,
                        BLAKE3_BLOCK_LEN as u32,
                        flags,
                    );
                    self.cv.copy_from_slice(&out[..8]);
                    self.blocks_compressed += 1;
                    self.block = [0u8; BLAKE3_BLOCK_LEN];
                    self.block_len = 0;
                }
                let want = BLAKE3_BLOCK_LEN - usize::from(self.block_len);
                let take = want.min(input.len());
                self.block[usize::from(self.block_len)..usize::from(self.block_len) + take]
                    .copy_from_slice(&input[..take]);
                self.block_len += take as u8;
                input = &input[take..];
            }
        }

        fn output(&self) -> Blake3Output {
            Blake3Output {
                input_cv: self.cv,
                block_words: words_from_block(&self.block),
                counter: self.chunk_counter,
                block_len: u32::from(self.block_len),
                flags: self.start_flag() | FLAG_CHUNK_END,
            }
        }
    }

    fn parent_output(left: [u32; 8], right: [u32; 8]) -> Blake3Output {
        let mut block_words = [0u32; 16];
        block_words[..8].copy_from_slice(&left);
        block_words[8..].copy_from_slice(&right);
        Blake3Output {
            input_cv: BLAKE3_IV,
            block_words,
            counter: 0,
            block_len: BLAKE3_BLOCK_LEN as u32,
            flags: FLAG_PARENT,
        }
    }

    pub(crate) struct Blake3Hasher {
        chunk_state: Blake3ChunkState,
        cv_stack: Vec<[u32; 8]>,
    }

    impl Blake3Hasher {
        pub(crate) fn new() -> Self {
            Self {
                chunk_state: Blake3ChunkState::new(0),
                cv_stack: Vec::new(),
            }
        }

        fn add_chunk_cv(&mut self, mut new_cv: [u32; 8], mut total_chunks: u64) {
            while total_chunks & 1 == 0 {
                let left = self.cv_stack.pop().expect("cv_stack non-empty");
                new_cv = parent_output(left, new_cv).chaining_value();
                total_chunks >>= 1;
            }
            self.cv_stack.push(new_cv);
        }

        pub(crate) fn update(&mut self, mut input: &[u8]) {
            while !input.is_empty() {
                if self.chunk_state.len() == BLAKE3_CHUNK_LEN {
                    let chunk_cv = self.chunk_state.output().chaining_value();
                    let total_chunks = self.chunk_state.chunk_counter + 1;
                    self.add_chunk_cv(chunk_cv, total_chunks);
                    self.chunk_state = Blake3ChunkState::new(total_chunks);
                }
                let want = BLAKE3_CHUNK_LEN - self.chunk_state.len();
                let take = want.min(input.len());
                self.chunk_state.update(&input[..take]);
                input = &input[take..];
            }
        }

        pub(crate) fn finalize(&self) -> [u8; 32] {
            let mut output = self.chunk_state.output();
            let mut idx = self.cv_stack.len();
            while idx > 0 {
                idx -= 1;
                output = parent_output(self.cv_stack[idx], output.chaining_value());
            }
            output.root_bytes()
        }
    }

    pub(crate) fn blake3_hash(input: &[u8]) -> [u8; 32] {
        let mut h = Blake3Hasher::new();
        h.update(input);
        h.finalize()
    }

    fn compute_index_digest(header_128: &[u8], directory: &[u8], metadata: &[u8]) -> [u8; 32] {
        let mut root_header = [0u8; HEADER_BYTES];
        root_header.copy_from_slice(&header_128[..HEADER_BYTES]);
        root_header[56..88].fill(0);
        root_header[124..128].fill(0);
        let mut h = Blake3Hasher::new();
        h.update(INDEX_DOMAIN);
        h.update(&root_header);
        h.update(directory);
        h.update(metadata);
        h.finalize()
    }

    fn align_up(n: u64, a: u64) -> Result<u64, String> {
        n.checked_add(a - 1)
            .map(|v| v & !(a - 1))
            .ok_or_else(|| "integer overflow during envelope alignment".to_string())
    }

    fn validate_padding_zero(bytes: &[u8], start: u64, len: u64) -> Result<(), String> {
        let s = usize::try_from(start).map_err(|_| "padding start overflow".to_string())?;
        let l = usize::try_from(len).map_err(|_| "padding len overflow".to_string())?;
        let slice = bytes
            .get(s..s.saturating_add(l))
            .ok_or_else(|| "container truncated inside alignment padding".to_string())?;
        if slice.iter().any(|&b| b != 0) {
            return Err("non-zero bytes inside container alignment padding".to_string());
        }
        Ok(())
    }

    fn tlv_item(bytes: &[u8]) -> Result<(&[u8], &[u8], &[u8]), String> {
        let h = bytes
            .get(..16)
            .ok_or_else(|| "truncated metadata TLV item header".to_string())?;
        let n = usize::try_from(u32le(h, 8)).map_err(|_| "metadata TLV length".to_string())?;
        let padded = (n + 7) & !7;
        let val = bytes
            .get(16..16 + n)
            .ok_or_else(|| "truncated metadata TLV item payload".to_string())?;
        let pad = bytes
            .get(16 + n..16 + padded)
            .ok_or_else(|| "truncated metadata TLV item padding".to_string())?;
        if pad.iter().any(|&b| b != 0) {
            return Err("non-zero padding inside metadata TLV".to_string());
        }
        Ok((h, val, &bytes[16 + padded..]))
    }

    pub(crate) fn validate_metadata_tlv(mut bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > MAX_METADATA_BYTES {
            return Err("metadata exceeds MAX_METADATA_BYTES".to_string());
        }
        let mut prev_block_id = 0u32;
        while !bytes.is_empty() {
            let (bh, mut body, tail) = tlv_item(bytes)?;
            bytes = tail;
            let block_id = u32le(bh, 0);
            if block_id <= prev_block_id || (u32le(bh, 12) & !1) != 0 {
                return Err("invalid or unordered metadata block".to_string());
            }
            prev_block_id = block_id;
            let mut prev_key = 0u16;
            while !body.is_empty() {
                let (fh, _val, ftail) = tlv_item(body)?;
                body = ftail;
                let key = u16le(fh, 0);
                if key <= prev_key || (fh[3] & !1) != 0 || u32le(fh, 12) != 0 {
                    return Err("invalid or unordered metadata field".to_string());
                }
                prev_key = key;
            }
        }
        Ok(())
    }

    pub(crate) fn reject_forbidden_bytes_signature(bytes: &[u8]) -> Result<(), String> {
        if bytes.len() >= 4 && (&bytes[..4] == b"PK\x03\x04" || &bytes[..4] == b"PK\x05\x06") {
            return Err("ZIP/HMC container streaming is forbidden inside Vibe catalog".to_string());
        }
        if bytes.len() >= 3 && &bytes[..3] == b"HMC" {
            return Err(".hmc sniffing is forbidden inside Vibe catalog".to_string());
        }
        if bytes.len() >= 4 && &bytes[..4] == b"QBDL" {
            return Err("QBDL stream headers must be admitted by QDNF, not Vibe catalog".to_string());
        }
        let trimmed = bytes.iter().copied().find(|b| !b.is_ascii_whitespace());
        if matches!(trimmed, Some(b'{') | Some(b'[')) {
            return Err("JSON is forbidden as native binary container body".to_string());
        }
        Ok(())
    }

    pub(crate) fn reject_forbidden_source_keys(
        args: &Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Value::Record(rec) = args else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "catalog invocation requires a Record argument with already-admitted binary bytes",
            ));
        };
        for key in FORBIDDEN_SOURCE_KEYS {
            if rec.contains_key(*key) {
                return Err(Diagnostic::new(
                    DiagCode::E300,
                    span,
                    format!(
                        "forbidden source/path key `{key}`: Vibe catalog operates only on already-admitted binary containers by identity/bytes, never filenames or .hmc sniffing"
                    ),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn decode_byte_list(val: &Value, field: &str, span: Span) -> Result<Vec<u8>, Diagnostic> {
        match val {
            Value::List(items) => {
                if items.is_empty() {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("binary `{field}` must not be empty"),
                    ));
                }
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::U64(n) if *n <= 255 => out.push(*n as u8),
                        Value::I64(n) if (0..=255).contains(n) => out.push(*n as u8),
                        _ => {
                            return Err(Diagnostic::new(
                                DiagCode::E100,
                                span,
                                format!("binary `{field}` elements must be u8 integers 0..=255"),
                            ))
                        }
                    }
                }
                if let Err(reason) = reject_forbidden_bytes_signature(&out) {
                    return Err(Diagnostic::new(DiagCode::E300, span, reason));
                }
                Ok(out)
            }
            Value::String(_) => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!(
                    "`{field}` must be binary bytes (List<u8>), never JSON or string path"
                ),
            )),
            _ => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("`{field}` must be binary bytes (List<u8>)"),
            )),
        }
    }

    pub(crate) fn extract_binary_bytes(
        args: &Value,
        keys: &[&str],
        span: Span,
    ) -> Result<Vec<u8>, Diagnostic> {
        reject_forbidden_source_keys(args, span)?;
        let Value::Record(rec) = args else {
            unreachable!();
        };
        for key in keys {
            if let Some(val) = rec.get(*key) {
                return decode_byte_list(val, key, span);
            }
        }
        Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("missing binary payload field (expected one of: {})", keys.join(", ")),
        ))
    }

    pub(crate) fn bytes_to_vibe_list(bytes: &[u8]) -> Value {
        Value::List(bytes.iter().map(|&b| Value::U64(u64::from(b))).collect())
    }

    pub(crate) fn admit_envelope<'a>(
        bytes: &'a [u8],
        expected_family: EnvelopeFamily,
    ) -> Result<AdmittedEnvelope<'a>, String> {
        reject_forbidden_bytes_signature(bytes)?;
        if bytes.len() < HEADER_BYTES {
            return Err("container shorter than 128-byte header".to_string());
        }
        if bytes.len() > MAX_CONTAINER_BYTES {
            return Err("container exceeds MAX_CONTAINER_BYTES".to_string());
        }
        let h = &bytes[..HEADER_BYTES];
        if &h[..8] != expected_family.magic() {
            return Err(format!(
                "container family magic mismatch (expected {})",
                expected_family.label()
            ));
        }
        let version = u16le(h, 8);
        if version != expected_family.version() {
            return Err(format!(
                "unsupported {} version {version} (expected {})",
                expected_family.label(),
                expected_family.version()
            ));
        }
        let stored_hdr_crc = u32le(h, 124);
        let got_hdr_crc = block_crc32c(h);
        if stored_hdr_crc != got_hdr_crc {
            return Err(format!(
                "header CRC-32C mismatch: stored {stored_hdr_crc:#010x}, computed {got_hdr_crc:#010x}"
            ));
        }
        let flags = u32le(h, 12);
        let total_bytes = u64le(h, 16);
        let dir_offset = u64le(h, 24);
        let entry_count = u32le(h, 32);
        let entry_bytes = u16le(h, 36);
        let index_ver = u16le(h, 38);
        if u16le(h, 10) != HEADER_BYTES as u16
            || (flags & !1) != 0
            || total_bytes != bytes.len() as u64
            || dir_offset != HEADER_BYTES as u64
            || entry_bytes != DIRECTORY_BYTES as u16
            || index_ver != 1
            || h[88..124].iter().any(|&b| b != 0)
        {
            return Err("invalid envelope header fields or non-zero reserved bytes".to_string());
        }
        if entry_count == 0 || entry_count as usize > MAX_ENTRIES {
            return Err(format!(
                "container entry_count must be in 1..={MAX_ENTRIES}, got {entry_count}"
            ));
        }
        let dir_len = u64::from(entry_count)
            .checked_mul(DIRECTORY_BYTES as u64)
            .ok_or_else(|| "directory byte length overflow".to_string())?;
        let meta_start = (HEADER_BYTES as u64)
            .checked_add(dir_len)
            .ok_or_else(|| "metadata offset overflow".to_string())?;
        let meta_len = u64le(h, 48);
        if u64le(h, 40) != meta_start || meta_len > MAX_METADATA_BYTES as u64 {
            return Err("invalid metadata offset or metadata limit exceeded".to_string());
        }
        let dir_end = usize::try_from(meta_start).map_err(|_| "dir_end overflow".to_string())?;
        let directory = bytes
            .get(HEADER_BYTES..dir_end)
            .ok_or_else(|| "truncated envelope directory".to_string())?;
        let meta_end_u64 = meta_start
            .checked_add(meta_len)
            .ok_or_else(|| "metadata end overflow".to_string())?;
        let meta_end =
            usize::try_from(meta_end_u64).map_err(|_| "metadata end overflow".to_string())?;
        let metadata = bytes
            .get(dir_end..meta_end)
            .ok_or_else(|| "truncated envelope metadata".to_string())?;
        validate_metadata_tlv(metadata)?;

        let alignment = if (flags & 1) != 0 { 4096u64 } else { 64u64 };
        let mut next = align_up(meta_end_u64, alignment)?;
        validate_padding_zero(bytes, meta_end_u64, next - meta_end_u64)?;

        let expected_index = compute_index_digest(h, directory, metadata);
        if expected_index != h[56..88] {
            return Err("envelope BLAKE3 index commitment mismatch".to_string());
        }

        let mut prior_key: Option<(u32, i64, u64, u16, u32)> = None;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for (idx, row) in directory.chunks_exact(DIRECTORY_BYTES).enumerate() {
            let stored_row_crc = u32le(row, 124);
            let got_row_crc = block_crc32c(row);
            if stored_row_crc != got_row_crc {
                return Err(format!("directory row {idx} CRC-32C mismatch"));
            }
            if row[78..80].iter().any(|&b| b != 0) || row[116..124].iter().any(|&b| b != 0) {
                return Err(format!("directory row {idx} has non-zero reserved bytes"));
            }
            let mode = u16le(row, 0);
            if !expected_family.mode_allowed(mode) {
                return Err(format!(
                    "unsupported mode {mode:#06x} in {} container (row {idx})",
                    expected_family.label()
                ));
            }
            let revision = u16le(row, 2);
            if revision == 0 {
                return Err(format!("directory row {idx} has zero revision"));
            }
            let e_flags = u32le(row, 4);
            let stream_id = u32le(row, 8);
            let frame_id = u32le(row, 12);
            let start_tick = u64le(row, 16) as i64;
            let time_count = u64le(row, 24);
            let spatial_key = u64le(row, 32);
            let payload_offset = u64le(row, 40);
            let stored_bytes = u64le(row, 48);
            let decoded_bytes = u64le(row, 56);
            let record_count = u64le(row, 64);
            let stride = u32le(row, 72);
            let codec = u16le(row, 76);
            let descriptor_id = u32le(row, 112);

            if codec != 0 {
                return Err(format!("unsupported compression codec {codec} at row {idx}"));
            }
            if (e_flags & !7) != 0 || descriptor_id == 0 || stride == 0 || record_count == 0 {
                return Err(format!("invalid directory invariants at row {idx}"));
            }
            if (e_flags & 2 == 0 && (start_tick != 0 || time_count != 0))
                || (e_flags & 2 != 0 && (time_count == 0 || stream_id == 0))
                || (e_flags & 4 == 0 && spatial_key != 0)
                || (e_flags & 4 != 0 && frame_id == 0)
            {
                return Err(format!("inconsistent time/spatial flags at directory row {idx}"));
            }
            let key = (stream_id, start_tick, spatial_key, mode, descriptor_id);
            if prior_key.is_some_and(|p| p >= key) {
                return Err(format!("non-ascending directory order at row {idx}"));
            }
            prior_key = Some(key);

            let expected_size = record_count
                .checked_mul(u64::from(stride))
                .ok_or_else(|| format!("size overflow at row {idx}"))?;
            if expected_size != stored_bytes
                || expected_size != decoded_bytes
                || payload_offset != next
            {
                return Err(format!("extent/offset mismatch at directory row {idx}"));
            }
            let p_start = usize::try_from(payload_offset)
                .map_err(|_| "payload_offset overflow".to_string())?;
            let p_len =
                usize::try_from(stored_bytes).map_err(|_| "stored_bytes overflow".to_string())?;
            let payload = bytes
                .get(p_start..p_start.saturating_add(p_len))
                .ok_or_else(|| format!("truncated payload at row {idx}"))?;
            let actual_digest = blake3_hash(payload);
            if actual_digest != row[80..112] {
                return Err(format!("payload BLAKE3 digest mismatch at row {idx}"));
            }
            let end = next
                .checked_add(stored_bytes)
                .ok_or_else(|| "payload end overflow".to_string())?;
            next = align_up(end, alignment)?;
            validate_padding_zero(bytes, end, next - end)?;

            entries.push(AdmittedEntry {
                mode,
                revision,
                flags: e_flags,
                stream_id,
                frame_id,
                start_tick,
                time_count,
                spatial_key,
                payload_offset,
                stored_bytes,
                decoded_bytes,
                record_count,
                stride,
                descriptor_id,
                digest: &row[80..112],
                payload,
            });
        }
        if next != bytes.len() as u64 {
            return Err("trailing bytes after final aligned container payload".to_string());
        }

        Ok(AdmittedEnvelope {
            family: expected_family,
            version,
            flags,
            total_bytes,
            metadata,
            index_digest: &h[56..88],
            entries,
        })
    }

    pub(crate) fn select_admitted_entry<'a, 'b>(
        env: &'b AdmittedEnvelope<'a>,
        args: &Value,
        span: Span,
    ) -> Result<(usize, &'b AdmittedEntry<'a>), Diagnostic> {
        let Value::Record(rec) = args else {
            return Err(Diagnostic::new(DiagCode::E100, span, "expected Record args"));
        };
        let by_idx = match rec.get("entry_index").or_else(|| rec.get("slice_index")) {
            None => None,
            Some(Value::U64(u)) => Some(
                usize::try_from(*u)
                    .map_err(|_| Diagnostic::new(DiagCode::E100, span, "entry_index overflow"))?,
            ),
            Some(Value::I64(i)) if *i >= 0 => Some(*i as usize),
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "entry_index must be a non-negative integer",
                ))
            }
        };
        let by_id = match rec.get("entry_id") {
            None => None,
            Some(Value::U64(u)) if *u > 0 && *u <= u64::from(u32::MAX) => Some(*u as u32),
            Some(Value::I64(i)) if *i > 0 && *i <= i64::from(u32::MAX) => Some(*i as u32),
            Some(Value::String(s)) => {
                let parsed = s.trim().parse::<u32>().ok().filter(|&id| id > 0);
                Some(parsed.ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E300,
                        span,
                        "entry_id must be a positive numeric entry descriptor_id, never a filename",
                    )
                })?)
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "entry_id must be a positive descriptor_id",
                ))
            }
        };

        let resolved_idx = match (by_idx, by_id) {
            (Some(idx), Some(id)) => {
                let entry = env.entries.get(idx).ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("entry_index {idx} out of bounds (count={})", env.entries.len()),
                    )
                })?;
                if entry.descriptor_id != id {
                    return Err(Diagnostic::new(
                        DiagCode::E300,
                        span,
                        format!(
                            "entry_index {idx} descriptor_id {} conflicts with requested entry_id {id}",
                            entry.descriptor_id
                        ),
                    ));
                }
                idx
            }
            (Some(idx), None) => {
                if idx >= env.entries.len() {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("entry_index {idx} out of bounds (count={})", env.entries.len()),
                    ));
                }
                idx
            }
            (None, Some(id)) => env
                .entries
                .iter()
                .position(|e| e.descriptor_id == id)
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E300,
                        span,
                        format!("admitted container has no entry with descriptor_id {id}"),
                    )
                })?,
            (None, None) => 0,
        };
        Ok((resolved_idx, &env.entries[resolved_idx]))
    }

    pub(crate) fn build_envelope(
        family: EnvelopeFamily,
        alignment_4096: bool,
        metadata: &[u8],
        entries: &[BuildEntry<'_>],
    ) -> Result<Vec<u8>, String> {
        if entries.is_empty() || entries.len() > MAX_ENTRIES {
            return Err(format!("entries count must be 1..={MAX_ENTRIES}"));
        }
        validate_metadata_tlv(metadata)?;
        let flags: u32 = if alignment_4096 { 1 } else { 0 };
        let alignment = if alignment_4096 { 4096u64 } else { 64u64 };
        let meta_start = (HEADER_BYTES + entries.len() * DIRECTORY_BYTES) as u64;
        let meta_end = meta_start
            .checked_add(metadata.len() as u64)
            .ok_or_else(|| "metadata overflow".to_string())?;
        let mut next = align_up(meta_end, alignment)?;

        let mut prior_key: Option<(u32, i64, u64, u16, u32)> = None;
        for (i, e) in entries.iter().enumerate() {
            if !family.mode_allowed(e.mode) {
                return Err(format!(
                    "mode {:#06x} is not admitted for {}",
                    e.mode,
                    family.label()
                ));
            }
            if e.revision == 0 || e.descriptor_id == 0 || e.stride == 0 || e.record_count == 0 {
                return Err(format!("entry {i} requires non-zero revision, descriptor_id, stride, and record_count"));
            }
            if (e.flags & !7) != 0
                || (e.flags & 2 == 0 && (e.start_tick != 0 || e.time_count != 0))
                || (e.flags & 2 != 0 && (e.time_count == 0 || e.stream_id == 0))
                || (e.flags & 4 == 0 && e.spatial_key != 0)
                || (e.flags & 4 != 0 && e.frame_id == 0)
            {
                return Err(format!("entry {i} invalid time/spatial flags"));
            }
            let size = e
                .record_count
                .checked_mul(u64::from(e.stride))
                .ok_or_else(|| "entry size overflow".to_string())?;
            if size != e.payload.len() as u64 {
                return Err(format!(
                    "entry {i} payload length {} != record_count * stride ({size})",
                    e.payload.len()
                ));
            }
            let key = (e.stream_id, e.start_tick, e.spatial_key, e.mode, e.descriptor_id);
            if prior_key.is_some_and(|p| p >= key) {
                return Err(format!("entry {i} violates ascending directory key order"));
            }
            prior_key = Some(key);
            next = align_up(
                next.checked_add(size).ok_or_else(|| "container size overflow".to_string())?,
                alignment,
            )?;
        }

        let total_len = usize::try_from(next).map_err(|_| "total container size overflow".to_string())?;
        if total_len > MAX_CONTAINER_BYTES {
            return Err("built container exceeds MAX_CONTAINER_BYTES".to_string());
        }
        let mut out = vec![0u8; total_len];
        let meta_start_usize = meta_start as usize;
        out[meta_start_usize..meta_start_usize + metadata.len()].copy_from_slice(metadata);
        let mut cursor = align_up(meta_end, alignment)? as usize;

        for (index, e) in entries.iter().enumerate() {
            let end = cursor + e.payload.len();
            out[cursor..end].copy_from_slice(e.payload);
            let row = &mut out[HEADER_BYTES + index * DIRECTORY_BYTES..HEADER_BYTES + (index + 1) * DIRECTORY_BYTES];
            put16(row, 0, e.mode);
            put16(row, 2, e.revision);
            put32(row, 4, e.flags);
            put32(row, 8, e.stream_id);
            put32(row, 12, e.frame_id);
            put64(row, 16, e.start_tick as u64);
            put64(row, 24, e.time_count);
            put64(row, 32, e.spatial_key);
            put64(row, 40, cursor as u64);
            put64(row, 48, e.payload.len() as u64);
            put64(row, 56, e.payload.len() as u64);
            put64(row, 64, e.record_count);
            put32(row, 72, e.stride);
            row[80..112].copy_from_slice(&blake3_hash(e.payload));
            put32(row, 112, e.descriptor_id);
            let row_crc = crc32c(row);
            put32(row, 124, row_crc);
            cursor = align_up(end as u64, alignment)? as usize;
        }

        {
            let h = &mut out[..HEADER_BYTES];
            h[..8].copy_from_slice(family.magic());
            put16(h, 8, family.version());
            put16(h, 10, HEADER_BYTES as u16);
            put32(h, 12, flags);
            put64(h, 16, total_len as u64);
            put64(h, 24, HEADER_BYTES as u64);
            put32(h, 32, entries.len() as u32);
            put16(h, 36, DIRECTORY_BYTES as u16);
            put16(h, 38, 1);
            put64(h, 40, meta_start);
            put64(h, 48, metadata.len() as u64);
        }
        let idx_digest = compute_index_digest(
            &out[..HEADER_BYTES],
            &out[HEADER_BYTES..meta_start_usize],
            &out[meta_start_usize..meta_start_usize + metadata.len()],
        );
        out[56..88].copy_from_slice(&idx_digest);
        let hdr_crc = crc32c(&out[..HEADER_BYTES]);
        put32(&mut out[..HEADER_BYTES], 124, hdr_crc);

        admit_envelope(&out, family)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up02_catalog_knows_all_16_new_ids_without_pulse_or_net_streaming() {
        let up02_ids = [
            "Emf.openContainer",
            "Emf.buildContainer",
            "Emf.verifyContainer",
            "Emf.sonifyField",
            "Emf.evaluateDipole",
            "Qac.openContainer",
            "Qac.buildContainer",
            "Qac.renderBinaural",
            "Qac.evaluateDirectivity",
            "Qac.sonifyEmf",
            "Dicom.parseDataset",
            "Dicom.verifyDataset",
            "Dicom.toEmfField",
            "Dicom.reconstructVolume",
            "Sensing.admitEvidence",
            "Sensing.verifyCalibration",
        ];
        for id in up02_ids {
            assert!(is_known(id), "catalog missing `{id}`");
            assert_eq!(canonical_id(id), Some(id));
            let family = family_of(id).expect("family_of");
            assert!(granted_covers(&[id], id));
            assert!(granted_covers(&[family], id));
            assert!(looks_like_catalog_path(id));
        }
        assert_eq!(methods_for_family("Emf").len(), 5);
        assert_eq!(methods_for_family("Qac").len(), 5);
        assert_eq!(methods_for_family("Dicom").len(), 4);
        assert!(methods_for_family("Sensing").contains(&"Sensing.admitEvidence"));
        assert!(methods_for_family("Sensing").contains(&"Sensing.verifyCalibration"));
        assert!(!is_known("Pulse.stream"));
        assert!(!is_known("Net.stream"));
        assert!(!is_known("Bundle.streamHmc"));
    }

    #[test]
    fn up02_crc32c_and_blake3_vectors_and_multi_entry_admission() {
        use envelope_admission::{
            admit_envelope, blake3_hash, build_envelope, crc32c, BuildEntry, EnvelopeFamily,
        };
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(
            blake3_hash(b""),
            [
                0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36,
                0xdc, 0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a,
                0x93, 0xca, 0xe4, 0x1f, 0x32, 0x62,
            ]
        );
        // Multi-block input (> 1024 bytes) exercises BLAKE3 Merkle parent nodes cleanly.
        let big = vec![0x42u8; 2050];
        let big_hash = blake3_hash(&big);
        assert_ne!(big_hash, blake3_hash(b""));

        let p1 = vec![1u8; 64];
        let p2 = vec![2u8; 64];
        let emf_bytes = build_envelope(
            EnvelopeFamily::EmfV4,
            false,
            &[],
            &[
                BuildEntry {
                    mode: 0x0100,
                    revision: 1,
                    flags: 0,
                    stream_id: 0,
                    frame_id: 0,
                    start_tick: 0,
                    time_count: 0,
                    spatial_key: 0,
                    record_count: 1,
                    stride: 64,
                    descriptor_id: 101,
                    payload: &p1,
                },
                BuildEntry {
                    mode: 0x0101,
                    revision: 1,
                    flags: 0,
                    stream_id: 0,
                    frame_id: 0,
                    start_tick: 0,
                    time_count: 0,
                    spatial_key: 0,
                    record_count: 1,
                    stride: 64,
                    descriptor_id: 102,
                    payload: &p2,
                },
            ],
        )
        .expect("build multi-entry EMF v4 container");
        let view = admit_envelope(&emf_bytes, EnvelopeFamily::EmfV4).expect("admits EMF v4");
        assert_eq!(view.entries.len(), 2);
        assert_eq!(view.entries[0].descriptor_id, 101);
        assert_eq!(view.entries[1].descriptor_id, 102);
        assert!(admit_envelope(&emf_bytes, EnvelopeFamily::QacV1).is_err());

        // Flip one payload byte -> fail-closed BLAKE3 rejection.
        let mut corrupt = emf_bytes.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xFF;
        assert!(admit_envelope(&corrupt, EnvelopeFamily::EmfV4).is_err());
    }

    #[test]
    fn catalog_contains_animation_and_hid() {
        assert!(is_known("Animation.evaluate_preset"));
        assert!(is_known("HID.poll"));
        assert!(is_known("GraphDatabase.sparql"));
        assert!(is_known("GraphDatabase.volume_open"));
        assert!(is_known("GraphDatabase.volume_commit"));
        assert!(is_known("GraphDatabase.lexicon_manifest"));
        assert!(is_known("DeonticLogic.evaluate"));
        assert!(is_known("N3Logic.evaluate"));
        assert!(is_known("GraphAuthoring.process"));
        assert!(is_known("LegalLogic.compute"));
        assert!(describe("GraphAuthoring.process").contains("owl:Thing"));
    }

    #[test]
    fn preset_alias_resolves() {
        assert_eq!(
            canonical_id("Animation.orbit_spin"),
            Some("Animation.evaluate_preset")
        );
        assert!(animation_preset("Animation.orbit_spin").is_some());
    }

    #[test]
    fn preset_alias_fills_family_and_preset() {
        let mut payload = payload_from_args(&[Value::F64(0.2)], &[]);
        apply_preset_alias("Animation.glass_reveal", &mut payload);
        let Value::Record(map) = payload else {
            panic!("expected record");
        };
        assert_eq!(
            map.get("family"),
            Some(&Value::String("hud_glass_ui".into()))
        );
        assert_eq!(
            map.get("preset"),
            Some(&Value::String("glass_reveal".into()))
        );
        assert_eq!(map.get("t"), Some(&Value::F64(0.2)));
    }

    #[test]
    fn using_family_covers_methods() {
        let g = ["Animation"];
        assert!(granted_covers(&g, "Animation.evaluate_preset"));
        assert!(granted_covers(&g, "Animation.orbit_spin"));
        assert!(!granted_covers(&g, "HID.poll"));
    }

    #[test]
    fn unknown_family_method_is_catalog_shaped() {
        assert!(looks_like_catalog_path("Animation.not_a_real_preset"));
        assert!(!is_known("Animation.not_a_real_preset"));
        assert!(!looks_like_catalog_path("math.sin"));
    }

    #[test]
    fn families_include_render_and_deontic() {
        let fams = families();
        assert!(fams.contains(&"Render"));
        assert!(fams.contains(&"DeonticLogic"));
        assert!(fams.contains(&"Animation"));
    }

    #[test]
    fn describe_mentions_lease() {
        let d = describe("DeonticLogic.evaluate");
        assert!(d.contains("DeonticLogic.evaluate"));
        assert!(d.contains("using DeonticLogic"));
    }

    #[test]
    fn suggest_nearby_id() {
        let s = did_you_mean("HID.pol");
        assert!(s.is_some());
        assert!(s.unwrap().contains("HID.poll"));
    }
}
