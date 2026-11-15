use super::cursor::Reader;
use super::*;

#[derive(Clone, Debug)]
pub struct MetadataView<'a> {
    bytes: &'a [u8],
    limits: ParseLimits,
    header: Header,
    tensor_start: usize,
    kv_work: u64,
    metadata_end: usize,
}

impl<'a> MetadataView<'a> {
    pub fn parse(bytes: &'a [u8], limits: ParseLimits) -> Result<Self, ParseError> {
        let mut cursor = KvCursor::new(bytes, limits)?;
        for value in cursor.by_ref() {
            value?;
        }
        let header = cursor.header();
        let tensor_start = cursor.offset();
        let kv_work = cursor.reader.work;
        let mut reader = cursor.reader;
        for _ in 0..header.tensor_count {
            reader.tensor()?;
        }
        Ok(Self {
            bytes,
            limits,
            header,
            tensor_start,
            kv_work,
            metadata_end: reader.pos,
        })
    }
    pub fn header(&self) -> Header {
        self.header
    }
    pub fn metadata_end(&self) -> usize {
        self.metadata_end
    }
    pub fn metadata_bytes(&self) -> &'a [u8] {
        &self.bytes[..self.metadata_end]
    }
    pub fn kvs(&self) -> KvCursor<'a> {
        KvCursor::new(self.bytes, self.limits).expect("immutable validated header")
    }
    pub fn tensors(&self) -> TensorIter<'a> {
        TensorIter {
            reader: Reader {
                bytes: self.bytes,
                limits: self.limits,
                pos: self.tensor_start,
                work: self.kv_work,
            },
            remaining: self.header.tensor_count,
            failed: false,
        }
    }
    /// Computes an explicit caller-selected alignment. No padding or payload need
    /// be present; general.alignment metadata is not interpreted automatically.
    pub fn tensor_data_start(&self, alignment: u64) -> Result<u64, ParseError> {
        let error = |kind| ParseError {
            offset: self.metadata_end,
            kind,
        };
        if !alignment.is_power_of_two() {
            return Err(error(ParseErrorKind::InvalidAlignment));
        }
        (self.metadata_end as u64)
            .checked_add(alignment - 1)
            .map(|end| end & !(alignment - 1))
            .ok_or_else(|| error(ParseErrorKind::ArithmeticOverflow))
    }
}

#[derive(Clone, Debug)]
pub struct TensorIter<'a> {
    reader: Reader<'a>,
    remaining: u64,
    failed: bool,
}
impl<'a> Iterator for TensorIter<'a> {
    type Item = Result<TensorView<'a>, ParseError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.failed {
            return None;
        }
        match self.reader.tensor() {
            Ok(value) => {
                self.remaining -= 1;
                Some(Ok(value))
            }
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}
impl core::iter::FusedIterator for TensorIter<'_> {}
