//! `GgufTokenizer` — vocabulary + BOS/EOS + BPE merges parsed from a GGUF KV
//! section (or a compact P64 section), with encode/decode, chat-template handling,
//! and the generation stop-token set.

use super::gguf_skip_value;
use std::collections::HashMap;
use std::sync::OnceLock;

mod pretokenizer;
pub use pretokenizer::{PretokenError, PretokenSpan};

/// GPT-2 `bytes_to_unicode` table — maps raw bytes to BPE merge symbols.
fn gpt2_byte_to_unicode(byte: u8) -> char {
    static TABLE: OnceLock<[char; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut bs: Vec<u32> = (b'!'..=b'~')
            .chain(b'\xA1'..=b'\xAC')
            .chain(b'\xAE'..=b'\xFF')
            .map(|b| b as u32)
            .collect();
        let mut cs = bs.clone();
        let mut n = 0u32;
        for b in 0u32..256 {
            if !bs.contains(&b) {
                bs.push(b);
                cs.push(256 + n);
                n += 1;
            }
        }
        let mut out = ['\0'; 256];
        for (b, c) in bs.into_iter().zip(cs) {
            out[b as usize] = char::from_u32(c).unwrap_or('\u{FFFD}');
        }
        out
    })[byte as usize]
}

fn gpt2_unicode_to_byte(symbol: char) -> Option<u8> {
    (0u16..=255)
        .find(|byte| gpt2_byte_to_unicode(*byte as u8) == symbol)
        .map(|byte| byte as u8)
}

/// Max stop-token ids kept on the tokenizer (eos + chat-end family + extras).
pub const MAX_STOP_TOKEN_IDS: usize = 8;

/// Vocabulary and BOS/EOS metadata extracted from a GGUF KV section.
/// Used by `infer_local_model()` to encode prompts and decode output token IDs.
pub struct GgufTokenizer {
    /// Token ID → string (index = token ID).
    pub vocab: Vec<String>,
    pub bos_token_id: u32,
    pub eos_token_id: u32,
    /// `tokenizer.ggml.add_bos_token` — prepend BOS before prompt tokens when true.
    pub add_bos_token: bool,
    /// `tokenizer.ggml.pre` — e.g. `smollm`, `gpt2`; drives pretokenization.
    pub pre_type: String,
    /// BPE merge ranks: `(left_symbol, right_symbol)` in ascending rank order.
    merge_pairs: Vec<(String, String)>,
    /// Cold-built pair fingerprint -> rank index. A detected fingerprint collision disables the
    /// index and preserves the exact linear oracle.
    merge_rank_index: HashMap<u64, usize>,
    merge_rank_collision: bool,
    /// Fast vocab lookup for BPE tail + legacy greedy path.
    pub(super) token_to_id_map: HashMap<String, u32>,
    /// Special tokens (`<|…|>`, etc.) sorted longest-first for atomic matching.
    special_tokens: Vec<(String, u32)>,
    /// (token_string, token_id) sorted by descending byte length — legacy greedy fallback.
    pub(super) token_to_id: Vec<(String, u32)>,
    /// Decode stop set: always includes `eos_token_id`, plus chat-end specials when
    /// present in vocab (`<|eot_id|>`, `<|im_end|>`, `<end_of_turn>`, …). Fixed array
    /// so the hot path does not allocate.
    stop_token_ids: [u32; MAX_STOP_TOKEN_IDS],
    stop_token_count: u8,
}

/// Chat-template family, detected from the special tokens a model's vocab carries. Instruct models
/// must have their prompt wrapped in this template (with an assistant-turn cue) or they degenerate
/// (emit EOS immediately, or repeat) — a raw prompt gives the model no "your turn to answer" signal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatFamily {
    /// `<|im_start|>role\n…<|im_end|>` — Qwen2 / SmolLM2 / many instruct models.
    ChatMl,
    /// `<|start_header_id|>role<|end_header_id|>\n\n…<|eot_id|>` — Llama-3.x.
    Llama3,
    /// `<start_of_turn>role\n…<end_of_turn>` — Gemma 1/2/3 (no system role).
    Gemma,
    /// `<|turn>role\n…<turn|>` — Gemma 4 instruct (also uses `<|channel>` tool channel).
    Gemma4,
    /// No recognised chat specials — the raw prompt is used unchanged.
    None,
}

impl Default for GgufTokenizer {
    /// 256-entry byte-level fallback tokenizer — used when no GGUF is loaded.
    fn default() -> Self {
        let vocab: Vec<String> = (0u32..256)
            .map(|b| {
                let c = b as u8;
                if c.is_ascii_graphic() || c == b' ' {
                    (c as char).to_string()
                } else {
                    format!("<0x{:02X}>", b)
                }
            })
            .collect();
        let mut t2id: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let token_to_id_map: HashMap<String, u32> =
            t2id.iter().map(|(s, id)| (s.clone(), *id)).collect();
        let mut tok = Self {
            vocab,
            bos_token_id: 1,
            eos_token_id: 2,
            add_bos_token: true,
            pre_type: String::new(),
            merge_pairs: Vec::new(),
            merge_rank_index: HashMap::new(),
            merge_rank_collision: false,
            token_to_id_map,
            special_tokens: Vec::new(),
            token_to_id: t2id,
            stop_token_ids: [0; MAX_STOP_TOKEN_IDS],
            stop_token_count: 0,
        };
        tok.rebuild_stop_token_ids();
        tok
    }
}

impl GgufTokenizer {
    /// Parse vocab + BOS/EOS from a memory-mapped GGUF v2/v3 file.
    /// Falls back to `Default` (byte-level) on any parse error.
    pub fn from_gguf(mmap: &[u8]) -> Self {
        Self::try_parse(mmap).unwrap_or_default()
    }

    fn try_parse(mmap: &[u8]) -> Option<Self> {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
            return None;
        }
        let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 {
            return None;
        } // only v2/v3 have u64 string lengths
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().ok()?);
        let mut pos = 24usize;
        let mut vocab: Option<Vec<String>> = None;
        let mut merges_raw: Option<Vec<String>> = None;
        let mut bos_id: Option<u32> = None;
        let mut eos_id: Option<u32> = None;
        let mut add_bos: Option<bool> = None;
        let mut pre_type: Option<String> = None;

        for _ in 0..kv_count {
            if pos + 8 > mmap.len() {
                break;
            }
            let klen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + klen > mmap.len() {
                break;
            }
            let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
            pos += klen;
            if pos + 4 > mmap.len() {
                break;
            }
            let vtype = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            match key {
                "tokenizer.ggml.tokens" => {
                    vocab = Self::read_string_array(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.merges" => {
                    merges_raw = Self::read_string_array(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.bos_token_id" => {
                    bos_id = Self::read_u32_val(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.eos_token_id" => {
                    eos_id = Self::read_u32_val(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.add_bos_token" => {
                    add_bos = Self::read_bool_val(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.pre" => {
                    pre_type = Self::read_string_val(mmap, &mut pos, vtype);
                }
                _ => {
                    if Self::skip_value(mmap, &mut pos, vtype).is_none() {
                        break;
                    }
                }
            }
        }

        let v = vocab?;
        let bos = bos_id.unwrap_or(1);
        let eos = eos_id.unwrap_or(2);
        let mut t2id: Vec<(String, u32)> = v
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let token_to_id_map: HashMap<String, u32> =
            t2id.iter().map(|(s, id)| (s.clone(), *id)).collect();
        let mut special_tokens: Vec<(String, u32)> = v
            .iter()
            .enumerate()
            .filter(|(_, s)| s.starts_with('<') && s.ends_with('>'))
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        special_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let merge_pairs = Self::parse_merge_pairs(merges_raw.as_deref());
        let (merge_rank_index, merge_rank_collision) = Self::build_merge_rank_index(&merge_pairs);
        let mut tok = Self {
            vocab: v,
            bos_token_id: bos,
            eos_token_id: eos,
            add_bos_token: add_bos.unwrap_or(true),
            pre_type: pre_type.unwrap_or_default(),
            merge_pairs,
            merge_rank_index,
            merge_rank_collision,
            token_to_id_map,
            special_tokens,
            token_to_id: t2id,
            stop_token_ids: [0; MAX_STOP_TOKEN_IDS],
            stop_token_count: 0,
        };
        tok.rebuild_stop_token_ids();
        Some(tok)
    }

    /// Phase 4 v3 / v2: serialize the tokenizer into a compact, contiguous P64 section (no page
    /// alignment needed). Fields: vocab / merges / bos / eos / add_bos / pre, plus (v2) the
    /// stop-token set so decode does not re-guess chat ends. Derived maps are rebuilt by
    /// [`from_p64_section`]. Heap use here is load-time only.
    pub fn to_p64_section(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 << 20);
        out.extend_from_slice(b"Q42T");
        out.extend_from_slice(&2u16.to_le_bytes()); // section version (v2 = stop tokens)
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&self.bos_token_id.to_le_bytes());
        out.extend_from_slice(&self.eos_token_id.to_le_bytes());
        out.push(self.add_bos_token as u8);
        out.extend_from_slice(&[0u8; 3]);
        let put_str = |o: &mut Vec<u8>, s: &str| {
            o.extend_from_slice(&(s.len() as u32).to_le_bytes());
            o.extend_from_slice(s.as_bytes());
        };
        put_str(&mut out, &self.pre_type);
        out.extend_from_slice(&(self.vocab.len() as u32).to_le_bytes());
        for t in &self.vocab {
            put_str(&mut out, t);
        }
        out.extend_from_slice(&(self.merge_pairs.len() as u32).to_le_bytes());
        for (l, r) in &self.merge_pairs {
            put_str(&mut out, l);
            put_str(&mut out, r);
        }
        // v2: stop-token set (eos + chat ends). Fixed 8 slots, count in first byte.
        out.push(self.stop_token_count);
        out.extend_from_slice(&[0u8; 3]);
        for id in &self.stop_token_ids {
            out.extend_from_slice(&id.to_le_bytes());
        }
        out
    }

    /// Compatibility alias for the historical pre-P64 method name.
    #[deprecated(note = "use to_p64_section")]
    pub fn to_q42_section(&self) -> Vec<u8> {
        self.to_p64_section()
    }

    /// Phase 4 v3: rebuild a tokenizer from a P64 tokenizer section — bypasses GGUF KV string-key
    /// parsing entirely. Fully bounds-checked (the section is untrusted input). Returns `None` on any
    /// malformed field.
    pub fn from_p64_section(data: &[u8]) -> Option<Self> {
        let mut p = 0usize;
        let take = |p: &mut usize, n: usize| -> Option<&[u8]> {
            let end = p.checked_add(n)?;
            if end > data.len() {
                return None;
            }
            let s = &data[*p..end];
            *p = end;
            Some(s)
        };
        let take_u32 = |p: &mut usize| -> Option<u32> {
            Some(u32::from_le_bytes(take(p, 4)?.try_into().ok()?))
        };
        let take_str = |p: &mut usize| -> Option<String> {
            let len = take_u32(p)? as usize;
            Some(String::from_utf8_lossy(take(p, len)?).into_owned())
        };
        if take(&mut p, 4)? != b"Q42T" {
            return None;
        }
        let ver = u16::from_le_bytes(take(&mut p, 2)?.try_into().ok()?);
        let _flags = take(&mut p, 2)?;
        let bos = take_u32(&mut p)?;
        let eos = take_u32(&mut p)?;
        let add_bos = take(&mut p, 1)?[0] != 0;
        let _pad = take(&mut p, 3)?;
        let pre_type = take_str(&mut p)?;
        let n_vocab = take_u32(&mut p)? as usize;
        if n_vocab > 1_000_000 {
            return None;
        }
        let mut vocab = Vec::with_capacity(n_vocab);
        for _ in 0..n_vocab {
            vocab.push(take_str(&mut p)?);
        }
        let n_merges = take_u32(&mut p)? as usize;
        if n_merges > 5_000_000 {
            return None;
        }
        let mut merge_pairs = Vec::with_capacity(n_merges);
        for _ in 0..n_merges {
            let l = take_str(&mut p)?;
            let r = take_str(&mut p)?;
            merge_pairs.push((l, r));
        }
        // v2 optional trailer: stop_count + pad3 + 8×u32. v1 rebuilds from vocab specials.
        let mut stored_stops: Option<([u32; MAX_STOP_TOKEN_IDS], u8)> = None;
        if ver >= 2 && p + 4 + MAX_STOP_TOKEN_IDS * 4 <= data.len() {
            let count = take(&mut p, 1)?[0].min(MAX_STOP_TOKEN_IDS as u8);
            let _ = take(&mut p, 3)?;
            let mut ids = [0u32; MAX_STOP_TOKEN_IDS];
            for i in 0..MAX_STOP_TOKEN_IDS {
                ids[i] = take_u32(&mut p)?;
            }
            stored_stops = Some((ids, count));
        }
        // Rebuild the derived maps exactly as `try_parse` does, so encode/decode are identical.
        let mut t2id: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let token_to_id_map: HashMap<String, u32> =
            t2id.iter().map(|(s, id)| (s.clone(), *id)).collect();
        let mut special_tokens: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .filter(|(_, s)| s.starts_with('<') && s.ends_with('>'))
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        special_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let mut tok = Self {
            vocab,
            bos_token_id: bos,
            eos_token_id: eos,
            add_bos_token: add_bos,
            pre_type,
            merge_pairs,
            merge_rank_index: HashMap::new(),
            merge_rank_collision: false,
            token_to_id_map,
            special_tokens,
            token_to_id: t2id,
            stop_token_ids: [0; MAX_STOP_TOKEN_IDS],
            stop_token_count: 0,
        };
        (tok.merge_rank_index, tok.merge_rank_collision) =
            Self::build_merge_rank_index(&tok.merge_pairs);
        if let Some((ids, count)) = stored_stops {
            tok.stop_token_ids = ids;
            tok.stop_token_count = count;
            // Always ensure eos is present even if a stale helper omitted it.
            if !tok.is_stop_token(eos) {
                tok.rebuild_stop_token_ids();
            }
        } else {
            tok.rebuild_stop_token_ids();
        }
        Some(tok)
    }

    /// Rebuild the decode stop set from `eos_token_id` + known chat-end specials in vocab.
    /// Call after any mutation of eos / token_to_id_map (load paths do this automatically).
    pub fn rebuild_stop_token_ids(&mut self) {
        let mut ids = [0u32; MAX_STOP_TOKEN_IDS];
        let mut n = 0usize;
        let mut push = |id: u32| {
            if n >= MAX_STOP_TOKEN_IDS {
                return;
            }
            if ids[..n].contains(&id) {
                return;
            }
            ids[n] = id;
            n += 1;
        };
        push(self.eos_token_id);
        // Chat / instruct end-of-turn tokens. Missing from vocab → no-op.
        // Without these, Llama-3 keeps past <|eot_id|> into pretraining-style continuation.
        const CHAT_ENDS: &[&str] = &[
            "<|eot_id|>",
            "<|im_end|>",
            "<end_of_turn>",
            "<turn|>", // Gemma 4 turn close
            "<|end_of_text|>",
            "</s>",
            "<|end|>",
        ];
        for s in CHAT_ENDS {
            if let Some(&id) = self.token_to_id_map.get(*s) {
                push(id);
            }
        }
        // Also scan special_tokens + full token_to_id_map for end-of-turn markers
        // (SentencePiece / BPE may store them with leading space or ▁ prefixes).
        let looks_like_chat_end = |n: &str| -> bool {
            let l = n.to_ascii_lowercase();
            l == "<|im_end|>"
                || l == "<|eot_id|>"
                || l == "<end_of_turn>"
                || l == "</s>"
                || l.ends_with("im_end|>")
                || l.ends_with("eot_id|>")
                || l.contains("end_of_turn")
                || l.contains("im_end")
                || l.contains("eot_id")
        };
        for (name, id) in &self.special_tokens {
            if looks_like_chat_end(name) {
                push(*id);
            }
        }
        for (name, id) in &self.token_to_id_map {
            if looks_like_chat_end(name) {
                push(*id);
            }
        }
        self.stop_token_ids = ids;
        self.stop_token_count = n as u8;
    }

    /// Whether `id` is a generation stop token (eos and/or chat end-of-turn).
    #[inline]
    pub fn is_stop_token(&self, id: u32) -> bool {
        let n = self.stop_token_count as usize;
        self.stop_token_ids[..n].contains(&id)
    }

    /// Slice of active stop-token ids (for logging / q42 export).
    pub fn stop_tokens(&self) -> &[u32] {
        &self.stop_token_ids[..self.stop_token_count as usize]
    }

    /// Merge extra stop ids (e.g. from a model's canonical `.q42` metadata) into the stop set.
    /// Does not allocate; drops overflow past [`MAX_STOP_TOKEN_IDS`].
    pub fn merge_stop_token_ids(&mut self, extra: &[u32]) {
        let mut n = self.stop_token_count as usize;
        for &id in extra {
            if n >= MAX_STOP_TOKEN_IDS {
                break;
            }
            if self.stop_token_ids[..n].contains(&id) {
                continue;
            }
            self.stop_token_ids[n] = id;
            n += 1;
        }
        // Always keep eos.
        if !self.stop_token_ids[..n].contains(&self.eos_token_id) && n < MAX_STOP_TOKEN_IDS {
            self.stop_token_ids[n] = self.eos_token_id;
            n += 1;
        }
        self.stop_token_count = n as u8;
    }

    /// Tokenize `text`, prepending [`bos_token_id`] when [`add_bos_token`] is set and absent.
    pub fn encode_prompt(&self, text: &str) -> Vec<u32> {
        let mut ids = self.encode(text);
        if self.add_bos_token && ids.first().copied() != Some(self.bos_token_id) {
            let mut with_bos = Vec::with_capacity(ids.len().saturating_add(1));
            with_bos.push(self.bos_token_id);
            with_bos.append(&mut ids);
            with_bos
        } else {
            ids
        }
    }

    /// Detect this model's chat-template family from the special tokens present in its vocab.
    pub fn chat_family(&self) -> ChatFamily {
        if self.token_to_id_map.contains_key("<|im_start|>") {
            ChatFamily::ChatMl
        } else if self.token_to_id_map.contains_key("<|start_header_id|>") {
            ChatFamily::Llama3
        } else if self.token_to_id_map.contains_key("<|turn>")
            || self.token_to_id_map.contains_key("<turn|>")
        {
            // Gemma 4 (before classic Gemma — classic uses <start_of_turn>).
            ChatFamily::Gemma4
        } else if self.token_to_id_map.contains_key("<start_of_turn>") {
            ChatFamily::Gemma
        } else {
            ChatFamily::None
        }
    }

    /// Wrap a user prompt (and optional system message) in the model's chat template, cueing the
    /// assistant turn so an instruct model answers instead of degenerating. The tokenizer BOS is
    /// still prepended by [`encode_prompt`]; it is NOT embedded here (avoids a double BOS). Returns
    /// the raw prompt unchanged when no chat family is recognised.
    pub fn apply_chat_template(&self, system: Option<&str>, user: &str) -> String {
        match self.chat_family() {
            ChatFamily::ChatMl => {
                let mut s = String::new();
                if let Some(sys) = system {
                    s.push_str("<|im_start|>system\n");
                    s.push_str(sys);
                    s.push_str("<|im_end|>\n");
                }
                s.push_str("<|im_start|>user\n");
                s.push_str(user);
                s.push_str("<|im_end|>\n<|im_start|>assistant\n");
                s
            }
            ChatFamily::Llama3 => {
                let mut s = String::new();
                if let Some(sys) = system {
                    s.push_str("<|start_header_id|>system<|end_header_id|>\n\n");
                    s.push_str(sys);
                    s.push_str("<|eot_id|>");
                }
                s.push_str("<|start_header_id|>user<|end_header_id|>\n\n");
                s.push_str(user);
                s.push_str("<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n");
                s
            }
            ChatFamily::Gemma => {
                // Gemma has no system role; fold any system text into the user turn.
                let mut s = String::from("<start_of_turn>user\n");
                if let Some(sys) = system {
                    s.push_str(sys);
                    s.push_str("\n\n");
                }
                s.push_str(user);
                s.push_str("<end_of_turn>\n<start_of_turn>model\n");
                s
            }
            ChatFamily::Gemma4 => {
                // Gemma 4 instruct: paired <|turn>…<turn|> markers (see GGUF chat_template).
                // No dedicated system role — fold system into the user turn. BOS is added by encode.
                let mut s = String::from("<|turn>user\n");
                if let Some(sys) = system {
                    s.push_str(sys);
                    s.push_str("\n\n");
                }
                s.push_str(user);
                s.push_str("<turn|><|turn>model\n");
                s
            }
            ChatFamily::None => user.to_string(),
        }
    }

    /// Apply the model's chat template (if any), then tokenize (+BOS per `add_bos_token`). This is
    /// the path for interactive chat/instruct inference; [`encode_prompt`] stays the raw-completion
    /// path. Chat models without a recognised family fall back to the raw prompt.
    pub fn encode_chat_prompt(&self, user: &str) -> Vec<u32> {
        let templated = self.apply_chat_template(None, user);
        self.encode_prompt(&templated)
    }

    /// Format token IDs for diagnostic logging (MC3f).
    pub fn format_ids_for_log(ids: &[u32]) -> String {
        let mut s = String::from("[");
        for (i, &id) in ids.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            if i >= 64 {
                s.push_str("…");
                break;
            }
            s.push_str(&id.to_string());
        }
        s.push(']');
        s
    }

    /// Greedy longest-match tokenisation; falls back to single-byte encoding.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        if self.uses_bpe() {
            return self.encode_bpe(text);
        }
        self.encode_greedy(text)
    }

    fn uses_bpe(&self) -> bool {
        !self.merge_pairs.is_empty()
            || matches!(
                self.pre_type.as_str(),
                "smollm" | "gpt2" | "mpt" | "olmo" | "jais" | "llama3"
            )
    }

    fn encode_greedy(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut remaining = text;
        while !remaining.is_empty() {
            let mut matched = false;
            for (token, id) in &self.token_to_id {
                if remaining.starts_with(token.as_str()) {
                    ids.push(*id);
                    remaining = &remaining[token.len()..];
                    matched = true;
                    break;
                }
            }
            if !matched {
                let b = remaining.as_bytes()[0];
                ids.push(b as u32);
                let step = remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                remaining = &remaining[step..];
            }
        }
        ids
    }

    /// BPE encode with special-token atomicity + smollm/gpt2 pretokenization.
    fn encode_bpe(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        // One bounded span workspace replaces regex captures and one String per piece.
        let mut spans = vec![PretokenSpan::default(); text.len().max(1)];
        let mut remaining = text;
        while !remaining.is_empty() {
            let mut matched_special = false;
            for (tok, id) in &self.special_tokens {
                if remaining.starts_with(tok.as_str()) {
                    ids.push(*id);
                    remaining = &remaining[tok.len()..];
                    matched_special = true;
                    break;
                }
            }
            if matched_special {
                continue;
            }
            let mut next_special = remaining.len();
            for (tok, _) in &self.special_tokens {
                if let Some(pos) = remaining.find(tok.as_str()) {
                    next_special = next_special.min(pos);
                }
            }
            let segment = &remaining[..next_special];
            if !segment.is_empty() {
                let count = pretokenizer::pretokenize_into(segment, &mut spans)
                    .expect("span workspace covers the segment byte length");
                for span in &spans[..count] {
                    ids.extend(self.bpe_piece(span.get(segment).unwrap()));
                }
            }
            remaining = &remaining[next_special..];
        }
        ids
    }

    /// Regex-free llama.cpp `LLAMA_VOCAB_PRE_TYPE_SMOLLM`-compatible borrowed-span split.
    pub fn pretokenize_into(
        &self,
        text: &str,
        out: &mut [PretokenSpan],
    ) -> Result<usize, PretokenError> {
        pretokenizer::pretokenize_into(text, out)
    }

    fn bpe_piece(&self, piece: &str) -> Vec<u32> {
        if piece.is_empty() {
            return Vec::new();
        }
        if let Some(&id) = self.token_to_id_map.get(piece) {
            return vec![id];
        }
        let word: String = piece.bytes().map(gpt2_byte_to_unicode).collect();
        if let Some(&id) = self.token_to_id_map.get(word.as_str()) {
            return vec![id];
        }
        let mut symbols: Vec<String> = word.chars().map(|c| c.to_string()).collect();
        if symbols.is_empty() {
            return Vec::new();
        }
        loop {
            let mut best_rank: Option<usize> = None;
            let mut best_idx = 0usize;
            for i in 0..symbols.len().saturating_sub(1) {
                if let Some(rank) = self.merge_rank_str(&symbols[i], &symbols[i + 1]) {
                    if best_rank.is_none() || rank < best_rank.unwrap() {
                        best_rank = Some(rank);
                        best_idx = i;
                    }
                }
            }
            let Some(_rank) = best_rank else { break };
            let merged = format!("{}{}", symbols[best_idx], symbols[best_idx + 1]);
            symbols[best_idx] = merged;
            symbols.remove(best_idx + 1);
        }
        let mut ids = Vec::with_capacity(symbols.len());
        for sym in symbols {
            if let Some(&id) = self.token_to_id_map.get(sym.as_str()) {
                ids.push(id);
            } else {
                for ch in sym.chars() {
                    let s = ch.to_string();
                    if let Some(&id) = self.token_to_id_map.get(s.as_str()) {
                        ids.push(id);
                    }
                }
            }
        }
        ids
    }

    fn merge_rank_str(&self, left: &str, right: &str) -> Option<usize> {
        if !self.merge_rank_collision {
            let fingerprint = Self::merge_pair_fingerprint(left, right);
            if let Some(&rank) = self.merge_rank_index.get(&fingerprint) {
                let pair = self.merge_pairs.get(rank)?;
                if pair.0 == left && pair.1 == right {
                    return Some(rank);
                }
                // Defensive exactness if an index built by an older serialized source collides.
                return self
                    .merge_pairs
                    .iter()
                    .position(|(l, r)| l == left && r == right);
            }
            return None;
        }
        self.merge_pairs
            .iter()
            .position(|(l, r)| l == left && r == right)
    }

    fn merge_pair_fingerprint(left: &str, right: &str) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in (left.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(left.bytes())
            .chain((right.len() as u64).to_le_bytes())
            .chain(right.bytes())
        {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    fn build_merge_rank_index(pairs: &[(String, String)]) -> (HashMap<u64, usize>, bool) {
        let mut index: HashMap<u64, usize> = HashMap::with_capacity(pairs.len());
        let mut collision = false;
        for (rank, (left, right)) in pairs.iter().enumerate() {
            let fingerprint = Self::merge_pair_fingerprint(left, right);
            if let Some(&existing) = index.get(&fingerprint) {
                if pairs[existing].0 != *left || pairs[existing].1 != *right {
                    collision = true;
                }
                continue;
            }
            index.insert(fingerprint, rank);
        }
        (index, collision)
    }

    fn parse_merge_pairs(merges: Option<&[String]>) -> Vec<(String, String)> {
        let Some(merges) = merges else {
            return Vec::new();
        };
        let mut pairs = Vec::with_capacity(merges.len());
        for merge in merges {
            if let Some((a, b)) = merge.split_once(' ') {
                pairs.push((a.to_string(), b.to_string()));
            }
        }
        pairs
    }

    fn uses_gpt2_byte_decoder(&self) -> bool {
        matches!(
            self.pre_type.to_ascii_lowercase().as_str(),
            "gpt2"
                | "smollm"
                | "qwen2"
                | "llama-bpe"
                | "deepseek-llm"
                | "deepseek-coder"
                | "falcon"
                | "starcoder"
        )
    }

    fn append_decoded_token_bytes(&self, out: &mut Vec<u8>, s: &str) {
        if s.len() == 6 && s.starts_with("<0x") && s.ends_with('>') {
            if let Ok(byte) = u8::from_str_radix(&s[3..5], 16) {
                out.push(byte);
            }
        } else if self.uses_gpt2_byte_decoder() {
            for symbol in s.chars() {
                if let Some(byte) = gpt2_unicode_to_byte(symbol) {
                    out.push(byte);
                } else {
                    let mut encoded = [0u8; 4];
                    out.extend_from_slice(symbol.encode_utf8(&mut encoded).as_bytes());
                }
            }
        } else if let Some(rest) = s.strip_prefix('\u{2581}') {
            out.push(b' ');
            out.extend_from_slice(rest.as_bytes());
        } else if let Some(rest) = s.strip_prefix('\u{0120}') {
            out.push(b' ');
            out.extend_from_slice(rest.as_bytes());
        } else {
            out.extend_from_slice(s.as_bytes());
        }
    }

    /// Append one vocabulary token to `out` with BPE / SentencePiece space normalization.
    #[allow(dead_code)]
    fn append_decoded_token(out: &mut String, s: &str) {
        if let Some(rest) = s.strip_prefix('\u{2581}') {
            out.push(' ');
            out.push_str(rest);
        } else if let Some(rest) = s.strip_prefix('\u{0120}') {
            // GPT-2 / Llama / SmolLM BPE space marker (Ġ).
            out.push(' ');
            out.push_str(rest);
        } else if s.len() == 6 && s.starts_with("<0x") && s.ends_with('>') {
            if let Ok(b) = u8::from_str_radix(&s[3..5], 16) {
                out.push(b as char);
            }
        } else {
            out.push_str(s);
        }
    }

    /// Map token IDs → strings, joining without separator.
    /// Converts SentencePiece `▁` and GPT-2 BPE `Ġ` → space; `<0x##>` → raw byte.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut out = Vec::new();
        for &id in ids {
            let s = self
                .vocab
                .get(id as usize)
                .map(|s| s.as_str())
                .unwrap_or("");
            self.append_decoded_token_bytes(&mut out, s);
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    /// Decode one token into the exact byte piece used by comparator APIs.
    ///
    /// This allocates and is intended for cold diagnostics, receipts, and corpus comparison,
    /// never for the token-forward hot path.
    pub fn decode_token_bytes_cold(&self, id: u32) -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(token) = self.vocab.get(id as usize) {
            self.append_decoded_token_bytes(&mut out, token);
        }
        out
    }

    pub fn vocab_len(&self) -> u32 {
        self.vocab.len() as u32
    }

    /// Number of BPE merges loaded from GGUF (diagnostic).
    pub fn merge_count(&self) -> usize {
        self.merge_pairs.len()
    }

    // ── internal KV parsers ──────────────────────────────────────────────────

    fn read_string_array(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<Vec<String>> {
        if vtype != 9 {
            Self::skip_value(mmap, pos, vtype)?;
            return None;
        }
        if *pos + 12 > mmap.len() {
            return None;
        }
        let etype = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().ok()?);
        *pos += 4;
        let count = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
        *pos += 8;
        if etype != 8 {
            return None;
        } // must be STRING array
        let mut result = Vec::with_capacity(count.min(256_000));
        for _ in 0..count {
            if *pos + 8 > mmap.len() {
                break;
            }
            let slen = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos + slen > mmap.len() {
                break;
            }
            let s = std::str::from_utf8(&mmap[*pos..*pos + slen])
                .unwrap_or("<?>")
                .to_string();
            *pos += slen;
            result.push(s);
        }
        Some(result)
    }

    fn read_u32_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<u32> {
        if vtype == 4 {
            if *pos + 4 > mmap.len() {
                return None;
            }
            let v = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().ok()?);
            *pos += 4;
            Some(v)
        } else {
            Self::skip_value(mmap, pos, vtype)?;
            None
        }
    }

    fn read_string_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<String> {
        if vtype == 8 {
            if *pos + 8 > mmap.len() {
                return None;
            }
            let slen = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos + slen > mmap.len() {
                return None;
            }
            let s = std::str::from_utf8(&mmap[*pos..*pos + slen])
                .unwrap_or("")
                .to_string();
            *pos += slen;
            Some(s)
        } else {
            Self::skip_value(mmap, pos, vtype)?;
            None
        }
    }

    fn read_bool_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<bool> {
        if vtype == 7 {
            if *pos + 1 > mmap.len() {
                return None;
            }
            let b = mmap[*pos];
            *pos += 1;
            Some(b != 0)
        } else {
            Self::skip_value(mmap, pos, vtype)?;
            None
        }
    }

    fn skip_value(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<()> {
        gguf_skip_value(mmap, pos, vtype)
    }
}
