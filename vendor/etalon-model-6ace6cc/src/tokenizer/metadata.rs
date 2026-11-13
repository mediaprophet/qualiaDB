use super::{budget::Work, TokenizerError, TokenizerProfile, TokenizerRequirements};
use crate::gguf::{KvView, MetadataView};

pub(super) struct Fields<'a> {
    pub tokens: &'a [u8],
    pub merges: &'a [u8],
    pub bos: u32,
    pub eos: u32,
    pub add_bos: bool,
    pub profile: TokenizerProfile,
}

pub(super) fn read<'a>(
    view: &MetadataView<'a>,
    work: &mut Work,
) -> Result<Fields<'a>, TokenizerError> {
    // Every replay is prepaid, including unrelated/nested metadata traversed by KvCursor.
    work.charge(view.metadata_bytes().len())?;
    work.charge(
        usize::try_from(view.header().kv_count).map_err(|_| TokenizerError::ArithmeticOverflow)?,
    )?;
    let keys: [&[u8]; 7] = [
        b"tokenizer.ggml.model",
        b"tokenizer.ggml.pre",
        b"tokenizer.ggml.tokens",
        b"tokenizer.ggml.merges",
        b"tokenizer.ggml.bos_token_id",
        b"tokenizer.ggml.eos_token_id",
        b"tokenizer.ggml.add_bos_token",
    ];
    let mut values: [Option<KvView<'a>>; 7] = [None; 7];
    for kv in view.kvs() {
        let kv = kv.map_err(|_| TokenizerError::Metadata)?;
        for (i, key) in keys.iter().enumerate() {
            if work.equal(kv.key, key)? {
                if values[i].replace(kv).is_some() {
                    return Err(TokenizerError::DuplicateField);
                }
                break;
            }
        }
    }
    let model = string_field(values[0].ok_or(TokenizerError::Metadata)?, work)?;
    if !work.equal(model.as_bytes(), b"gpt2")? {
        return Err(TokenizerError::UnsupportedProfile);
    }
    let pre = string_field(values[1].ok_or(TokenizerError::Metadata)?, work)?;
    let profile = if work.equal(pre.as_bytes(), b"gpt2")? {
        TokenizerProfile::Gpt2
    } else if work.equal(pre.as_bytes(), b"smollm")? {
        TokenizerProfile::Smollm
    } else {
        return Err(TokenizerError::UnsupportedProfile);
    };
    let array = |value: Option<KvView<'a>>| -> Result<&'a [u8], TokenizerError> {
        let value = value.ok_or(TokenizerError::Metadata)?;
        if value.value_type != 9 {
            return Err(TokenizerError::Metadata);
        }
        Ok(value.encoded_value)
    };
    let integer = |value: Option<KvView<'_>>, default| -> Result<u32, TokenizerError> {
        match value {
            None => Ok(default),
            Some(v) if v.value_type == 4 => Ok(u32::from_le_bytes(
                v.encoded_value
                    .try_into()
                    .map_err(|_| TokenizerError::Metadata)?,
            )),
            _ => Err(TokenizerError::Metadata),
        }
    };
    let add_bos = match values[6] {
        None => true,
        Some(v) if v.value_type == 7 && v.encoded_value == [0] => false,
        Some(v) if v.value_type == 7 && v.encoded_value == [1] => true,
        _ => return Err(TokenizerError::Metadata),
    };
    Ok(Fields {
        tokens: array(values[2])?,
        merges: array(values[3])?,
        bos: integer(values[4], 1)?,
        eos: integer(values[5], 2)?,
        add_bos,
        profile,
    })
}

fn take_string<'a>(bytes: &mut &'a [u8], work: &mut Work) -> Result<&'a str, TokenizerError> {
    work.charge(1)?;
    let length = u64::from_le_bytes(
        bytes
            .get(..8)
            .ok_or(TokenizerError::Metadata)?
            .try_into()
            .map_err(|_| TokenizerError::Metadata)?,
    );
    let length = usize::try_from(length).map_err(|_| TokenizerError::ArithmeticOverflow)?;
    let end = 8usize
        .checked_add(length)
        .ok_or(TokenizerError::ArithmeticOverflow)?;
    let data = bytes.get(8..end).ok_or(TokenizerError::Metadata)?;
    work.charge(length)?; // UTF-8 validation.
    let text = core::str::from_utf8(data).map_err(|_| TokenizerError::Metadata)?;
    *bytes = &bytes[end..];
    Ok(text)
}
fn string_field<'a>(value: KvView<'a>, work: &mut Work) -> Result<&'a str, TokenizerError> {
    if value.value_type != 8 {
        return Err(TokenizerError::Metadata);
    }
    let mut bytes = value.encoded_value;
    let text = take_string(&mut bytes, work)?;
    if !bytes.is_empty() {
        return Err(TokenizerError::Metadata);
    }
    Ok(text)
}

pub(super) struct Strings<'a> {
    bytes: &'a [u8],
    remaining: usize,
    pub count: usize,
}
impl<'a> Strings<'a> {
    pub fn new(bytes: &'a [u8], work: &mut Work) -> Result<Self, TokenizerError> {
        work.charge(1)?;
        if bytes.get(..4) != Some(&8u32.to_le_bytes()) {
            return Err(TokenizerError::Metadata);
        }
        let count = u64::from_le_bytes(
            bytes
                .get(4..12)
                .ok_or(TokenizerError::Metadata)?
                .try_into()
                .map_err(|_| TokenizerError::Metadata)?,
        );
        let count = usize::try_from(count).map_err(|_| TokenizerError::ArithmeticOverflow)?;
        Ok(Self {
            bytes: &bytes[12..],
            remaining: count,
            count,
        })
    }
    pub fn next(&mut self, work: &mut Work) -> Result<Option<&'a str>, TokenizerError> {
        if self.remaining == 0 {
            if !self.bytes.is_empty() {
                return Err(TokenizerError::Metadata);
            }
            return Ok(None);
        }
        let text = take_string(&mut self.bytes, work)?;
        self.remaining -= 1;
        Ok(Some(text))
    }
}

pub(super) fn pair<'a>(
    text: &'a str,
    work: &mut Work,
) -> Result<(&'a str, &'a str), TokenizerError> {
    work.charge(
        text.len()
            .checked_mul(2)
            .ok_or(TokenizerError::ArithmeticOverflow)?,
    )?;
    let (a, b) = text.split_once(' ').ok_or(TokenizerError::MalformedMerge)?;
    if a.is_empty() || b.is_empty() || b.contains(' ') {
        return Err(TokenizerError::MalformedMerge);
    }
    Ok((a, b))
}
pub(super) fn special(s: &str) -> bool {
    s.starts_with('<') && s.ends_with('>')
}

impl Fields<'_> {
    pub fn requirements(&self, work: &mut Work) -> Result<TokenizerRequirements, TokenizerError> {
        let mut tokens = Strings::new(self.tokens, work)?;
        let vocab = tokens.count;
        if vocab == 0 || vocab > u32::MAX as usize {
            return Err(TokenizerError::Metadata);
        }
        if self.bos as usize >= vocab {
            return Err(TokenizerError::InvalidTokenId(self.bos));
        }
        if self.eos as usize >= vocab {
            return Err(TokenizerError::InvalidTokenId(self.eos));
        }
        let mut specials = 0;
        while let Some(token) = tokens.next(work)? {
            work.charge(1)?;
            if special(token) {
                specials += 1;
            }
        }
        let mut entries = Strings::new(self.merges, work)?;
        let merges = entries.count;
        while let Some(text) = entries.next(work)? {
            pair(text, work)?;
        }
        Ok(TokenizerRequirements {
            vocab,
            merges,
            specials,
        })
    }
}
