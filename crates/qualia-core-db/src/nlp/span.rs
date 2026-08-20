//! UTF-8 byte span. Same fields as core-db `TextSpan` minus the hash (computed at emit).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocSpan {
    pub start_utf8: u32,
    pub end_utf8: u32,
}

impl DocSpan {
    pub fn new(start_utf8: u32, end_utf8: u32) -> Self {
        Self {
            start_utf8,
            end_utf8,
        }
    }

    pub fn as_range(self) -> std::ops::Range<usize> {
        self.start_utf8 as usize..self.end_utf8 as usize
    }

    pub fn slice<'a>(self, source: &'a str) -> Option<&'a str> {
        let start = self.start_utf8 as usize;
        let end = self.end_utf8 as usize;
        if start <= end
            && end <= source.len()
            && source.is_char_boundary(start)
            && source.is_char_boundary(end)
        {
            Some(&source[start..end])
        } else {
            None
        }
    }
}
