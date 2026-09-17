//! UTF-8 byte spans. Canonical source positions for diagnostics.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub const fn point(at: u32) -> Self {
        Self { start: at, end: at }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn slice<'a>(&self, src: &'a str) -> &'a str {
        let start = self.start as usize;
        let end = (self.end as usize).min(src.len());
        if start >= src.len() || start > end {
            return "";
        }
        &src[start..end]
    }
}
