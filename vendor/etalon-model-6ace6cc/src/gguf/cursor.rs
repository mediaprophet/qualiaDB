use super::types::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub limits: ParseLimits,
    pub pos: usize,
    pub work: u64,
}

impl<'a> Reader<'a> {
    pub fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            offset: self.pos,
            kind,
        }
    }
    pub fn take(&mut self, count: usize) -> Result<&'a [u8], ParseError> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| self.error(ParseErrorKind::ArithmeticOverflow))?;
        if end > self.limits.max_metadata_bytes {
            return Err(self.error(ParseErrorKind::MetadataLimit));
        }
        let result = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| self.error(ParseErrorKind::Truncated))?;
        self.pos = end;
        Ok(result)
    }
    pub fn charge(&mut self, units: u64) -> Result<(), ParseError> {
        let work = self
            .work
            .checked_add(units)
            .ok_or_else(|| self.error(ParseErrorKind::ArithmeticOverflow))?;
        if work > self.limits.max_work {
            return Err(self.error(ParseErrorKind::WorkLimit));
        }
        self.work = work;
        Ok(())
    }
    pub fn u32(&mut self) -> Result<u32, ParseError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Result<u64, ParseError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn string(&mut self) -> Result<&'a [u8], ParseError> {
        let len = self.u64()?;
        let len =
            usize::try_from(len).map_err(|_| self.error(ParseErrorKind::ArithmeticOverflow))?;
        self.take(len)
    }
    pub fn known_type(&self, kind: u32) -> Result<(), ParseError> {
        if kind <= 12 {
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::UnknownValueType(kind)))
        }
    }
    fn value(&mut self, value_type: u32) -> Result<(), ParseError> {
        #[derive(Clone, Copy)]
        struct Frame {
            kind: u32,
            remaining: u64,
        }
        let mut stack = [Frame {
            kind: 0,
            remaining: 0,
        }; MAX_ARRAY_DEPTH + 1];
        stack[0] = Frame {
            kind: value_type,
            remaining: 1,
        };
        let mut depth = 0;
        loop {
            if stack[depth].remaining == 0 {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                continue;
            }
            let kind = stack[depth].kind;
            stack[depth].remaining -= 1;
            self.charge(1)?;
            self.known_type(kind)?;
            match kind {
                0 | 1 | 7 => {
                    self.take(1)?;
                }
                2 | 3 => {
                    self.take(2)?;
                }
                4..=6 => {
                    self.take(4)?;
                }
                10..=12 => {
                    self.take(8)?;
                }
                8 => {
                    self.string()?;
                }
                9 => {
                    let element_type = self.u32()?;
                    let remaining = self.u64()?;
                    self.known_type(element_type)?;
                    if depth >= self.limits.max_nesting_depth {
                        return Err(self.error(ParseErrorKind::DepthLimit));
                    }
                    depth += 1;
                    stack[depth] = Frame {
                        kind: element_type,
                        remaining,
                    };
                }
                _ => unreachable!("value type was checked"),
            }
        }
        Ok(())
    }
    fn kv(&mut self) -> Result<KvView<'a>, ParseError> {
        self.charge(1)?;
        let key = self.string()?;
        let value_type = self.u32()?;
        let start = self.pos;
        self.value(value_type)?;
        Ok(KvView {
            key,
            value_type,
            encoded_value: &self.bytes[start..self.pos],
        })
    }
    pub fn tensor(&mut self) -> Result<TensorView<'a>, ParseError> {
        self.charge(1)?;
        let name = self.string()?;
        let rank = self.u32()?;
        self.charge(u64::from(rank))?;
        let count = (rank as usize)
            .checked_mul(8)
            .ok_or_else(|| self.error(ParseErrorKind::ArithmeticOverflow))?;
        let dimensions = self.take(count)?;
        let ggml_type = self.u32()?;
        let offset = self.u64()?;
        Ok(TensorView {
            name,
            rank,
            ggml_type,
            offset,
            dimensions,
        })
    }
}

/// Yields one error at the first malformed/over-budget KV, then remains stopped.
/// Successful exhaustion and a partial failed prefix remain distinguishable.
#[derive(Clone, Debug)]
pub struct KvCursor<'a> {
    pub(super) reader: Reader<'a>,
    header: Header,
    remaining: u64,
    failure: Option<ParseError>,
}

impl<'a> KvCursor<'a> {
    pub fn new(bytes: &'a [u8], limits: ParseLimits) -> Result<Self, ParseError> {
        let mut reader = Reader {
            bytes,
            limits,
            pos: 0,
            work: 0,
        };
        if limits.max_nesting_depth > MAX_ARRAY_DEPTH {
            return Err(reader.error(ParseErrorKind::InvalidLimits));
        }
        let raw = reader.take(24)?;
        if &raw[..4] != b"GGUF" {
            return Err(ParseError {
                offset: 0,
                kind: ParseErrorKind::InvalidMagic,
            });
        }
        let version = u32::from_le_bytes(raw[4..8].try_into().unwrap());
        if !match limits.versions {
            VersionPolicy::V2V3 => matches!(version, 2 | 3),
            VersionPolicy::AtLeast2 => version >= 2,
        } {
            return Err(ParseError {
                offset: 4,
                kind: ParseErrorKind::UnsupportedVersion(version),
            });
        }
        let header = Header {
            version,
            tensor_count: u64::from_le_bytes(raw[8..16].try_into().unwrap()),
            kv_count: u64::from_le_bytes(raw[16..24].try_into().unwrap()),
        };
        Ok(Self {
            reader,
            header,
            remaining: header.kv_count,
            failure: None,
        })
    }
    pub fn header(&self) -> Header {
        self.header
    }
    pub fn offset(&self) -> usize {
        self.reader.pos
    }
    pub fn is_complete(&self) -> bool {
        self.remaining == 0 && self.failure.is_none()
    }
    pub fn failure(&self) -> Option<ParseError> {
        self.failure
    }
}

impl<'a> Iterator for KvCursor<'a> {
    type Item = Result<KvView<'a>, ParseError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.failure.is_some() {
            return None;
        }
        match self.reader.kv() {
            Ok(value) => {
                self.remaining -= 1;
                Some(Ok(value))
            }
            Err(error) => {
                self.failure = Some(error);
                Some(Err(error))
            }
        }
    }
}

impl core::iter::FusedIterator for KvCursor<'_> {}
