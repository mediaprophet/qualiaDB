use super::header::{self, header, DatasetError, ParseLimits, TransferSyntax};
use std::ops::Range;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementId(pub(super) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemId(pub(super) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Value,
    Sequence,
    EncapsulatedPixels,
}
#[derive(Debug)]
pub struct Element {
    pub tag: u32,
    pub vr: Option<[u8; 2]>,
    pub scope: Option<ItemId>,
    pub kind: ElementKind,
    pub(super) value: Range<usize>,
}
pub(super) struct Item {
    pub sequence: ElementId,
    pub value: Range<usize>,
}
/// Source-bound cold index. No source value is copied, decoded or reserialized.
pub struct Part10Dataset<'a> {
    pub(super) source: &'a [u8],
    pub(super) elements: Vec<Element>,
    pub(super) items: Vec<Item>,
    pub(super) syntax: TransferSyntax,
    pub(super) transfer_uid: &'a str,
    pub(super) digest: [u8; 32],
    pub(super) media_class: Option<&'a str>,
    pub(super) media_instance: Option<&'a str>,
}
#[derive(Clone, Copy)]
enum Kind {
    Root,
    Sequence(ElementId, bool),
    Item(ItemId),
}
#[derive(Clone, Copy)]
struct Scope {
    kind: Kind,
    end: Option<usize>,
    bound: usize,
    last: Option<u32>,
}
const EMPTY: Scope = Scope {
    kind: Kind::Root,
    end: None,
    bound: 0,
    last: None,
};
fn spend(work: &mut usize) -> Result<(), DatasetError> {
    *work = work.checked_sub(1).ok_or(DatasetError::WorkLimit)?;
    Ok(())
}
impl<'a> Part10Dataset<'a> {
    pub fn parse(source: &'a [u8], limits: ParseLimits) -> Result<Self, DatasetError> {
        if source.len() > limits.max_bytes {
            return Err(DatasetError::ByteLimit);
        }
        if !crate::dicom::is_dicom_part10(source) {
            return Err(DatasetError::NotPart10);
        }
        if limits.max_depth > 64 {
            return Err(DatasetError::DepthLimit);
        }
        let need = limits
            .max_elements
            .checked_mul(std::mem::size_of::<Element>())
            .and_then(|n| {
                limits
                    .max_items
                    .checked_mul(std::mem::size_of::<Item>())
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or(DatasetError::Overflow)?;
        if need > limits.max_index_bytes {
            return Err(DatasetError::IndexLimit);
        }
        let mut work = limits.max_work;
        let mut cursor = 132;
        let mut transfer = None;
        let mut media_class = None;
        let mut media_instance = None;
        let mut last = None;
        while cursor < source.len() && header::u16le(source, cursor)? == 2 {
            spend(&mut work)?;
            let h = header(source, cursor, true, source.len())?;
            if h.len == u32::MAX || h.len % 2 != 0 {
                return Err(DatasetError::MalformedHeader);
            }
            if last.is_some_and(|v| h.tag <= v) {
                return Err(DatasetError::Duplicate(h.tag));
            }
            last = Some(h.tag);
            let end = h
                .value
                .checked_add(h.len as usize)
                .ok_or(DatasetError::Overflow)?;
            let value = source.get(h.value..end).ok_or(DatasetError::Truncated)?;
            if matches!(h.tag, 0x00020002 | 0x00020003 | 0x00020010) {
                if h.vr != Some(*b"UI") {
                    return Err(DatasetError::Invalid(h.tag));
                }
                let text = super::query::uid(value, h.tag)?;
                match h.tag {
                    0x00020002 => media_class = Some(text),
                    0x00020003 => media_instance = Some(text),
                    _ => transfer = Some(text),
                }
            }
            cursor = end;
        }
        let transfer_uid = transfer.ok_or(DatasetError::Missing(0x00020010))?;
        let syntax = match transfer_uid {
            "1.2.840.10008.1.2" => TransferSyntax::ImplicitLittleEndian,
            "1.2.840.10008.1.2.1" => TransferSyntax::ExplicitLittleEndian,
            "1.2.840.10008.1.2.4.50"
            | "1.2.840.10008.1.2.4.51"
            | "1.2.840.10008.1.2.4.80"
            | "1.2.840.10008.1.2.4.81"
            | "1.2.840.10008.1.2.4.90"
            | "1.2.840.10008.1.2.4.91"
            | "1.2.840.10008.1.2.4.110"
            | "1.2.840.10008.1.2.4.111"
            | "1.2.840.10008.1.2.4.112"
            | "1.2.840.10008.1.2.4.201"
            | "1.2.840.10008.1.2.4.202"
            | "1.2.840.10008.1.2.4.203"
            | "1.2.840.10008.1.2.5" => TransferSyntax::EncapsulatedExplicitLittleEndian,
            _ => return Err(DatasetError::UnsupportedSyntax),
        };
        let mut result = Self {
            source,
            elements: Vec::new(),
            items: Vec::new(),
            syntax,
            transfer_uid,
            digest: *blake3::hash(source).as_bytes(),
            media_class,
            media_instance,
        };
        result
            .elements
            .try_reserve_exact(limits.max_elements)
            .map_err(|_| DatasetError::IndexLimit)?;
        result
            .items
            .try_reserve_exact(limits.max_items)
            .map_err(|_| DatasetError::IndexLimit)?;
        let mut stack = [EMPTY; 65];
        stack[0] = Scope {
            kind: Kind::Root,
            end: Some(source.len()),
            bound: source.len(),
            last: None,
        };
        let mut depth = 0;
        loop {
            spend(&mut work)?;
            let scope = stack[depth];
            if scope.end == Some(cursor) {
                if depth == 0 {
                    break;
                }
                match scope.kind {
                    Kind::Sequence(id, _) => result.elements[id.0].value.end = cursor,
                    Kind::Item(id) => result.items[id.0].value.end = cursor,
                    _ => {}
                }
                depth -= 1;
                continue;
            }
            if cursor >= scope.bound {
                return Err(DatasetError::Boundary);
            }
            let h = header(
                source,
                cursor,
                syntax != TransferSyntax::ImplicitLittleEndian,
                scope.bound,
            )?;
            if h.tag >> 16 == 0xfffe {
                match (h.tag, scope.kind) {
                    (0xfffee000, Kind::Sequence(seq, opaque)) => {
                        if result.items.len() >= limits.max_items {
                            return Err(DatasetError::ItemLimit);
                        }
                        let end = if h.len == u32::MAX {
                            None
                        } else {
                            Some(
                                h.value
                                    .checked_add(h.len as usize)
                                    .ok_or(DatasetError::Overflow)?,
                            )
                        };
                        if h.len != u32::MAX && h.len % 2 != 0 {
                            return Err(DatasetError::MalformedHeader);
                        }
                        if end.is_some_and(|e| e > scope.bound) {
                            return Err(DatasetError::Boundary);
                        }
                        let id = ItemId(result.items.len());
                        result.items.push(Item {
                            sequence: seq,
                            value: h.value..end.unwrap_or(h.value),
                        });
                        if opaque {
                            let end = end.ok_or(DatasetError::MalformedDelimiter)?;
                            cursor = end;
                            continue;
                        }
                        if depth + 1 > limits.max_depth {
                            return Err(DatasetError::DepthLimit);
                        }
                        depth += 1;
                        stack[depth] = Scope {
                            kind: Kind::Item(id),
                            end,
                            bound: end.unwrap_or(scope.bound),
                            last: None,
                        };
                        cursor = h.value;
                    }
                    (0xfffee00d, Kind::Item(id)) if scope.end.is_none() && h.len == 0 => {
                        result.items[id.0].value.end = cursor;
                        depth -= 1;
                        cursor = h.value;
                    }
                    (0xfffee0dd, Kind::Sequence(id, _)) if scope.end.is_none() && h.len == 0 => {
                        result.elements[id.0].value.end = cursor;
                        depth -= 1;
                        cursor = h.value;
                    }
                    _ => return Err(DatasetError::MalformedDelimiter),
                }
                continue;
            }
            if matches!(scope.kind, Kind::Sequence(..)) {
                return Err(DatasetError::MalformedDelimiter);
            }
            if scope.last == Some(h.tag) {
                return Err(DatasetError::Duplicate(h.tag));
            }
            if scope.last.is_some_and(|v| h.tag < v) {
                return Err(DatasetError::Order);
            }
            stack[depth].last = Some(h.tag);
            if depth > 0 && matches!(h.tag >> 16, 0 | 2 | 6) {
                return Err(DatasetError::MalformedHeader);
            }
            if result.elements.len() >= limits.max_elements {
                return Err(DatasetError::ElementLimit);
            }
            let undefined = h.len == u32::MAX;
            let opaque = h.tag == 0x7fe00010 && undefined;
            if opaque && syntax != TransferSyntax::EncapsulatedExplicitLittleEndian {
                return Err(DatasetError::Invalid(h.tag));
            }
            let sequence =
                h.vr == Some(*b"SQ") || (h.vr.is_none() && header::implicit_sequence(h.tag));
            if undefined && !sequence && !opaque {
                return Err(DatasetError::UnsupportedImplicitSequence);
            }
            if !undefined && h.len % 2 != 0 {
                return Err(DatasetError::MalformedHeader);
            }
            let end = if undefined {
                None
            } else {
                Some(
                    h.value
                        .checked_add(h.len as usize)
                        .ok_or(DatasetError::Overflow)?,
                )
            };
            if end.is_some_and(|e| e > scope.bound) {
                return Err(DatasetError::Boundary);
            }
            let id = ElementId(result.elements.len());
            let parent = match scope.kind {
                Kind::Item(id) => Some(id),
                _ => None,
            };
            result.elements.push(Element {
                tag: h.tag,
                vr: h.vr,
                scope: parent,
                kind: if opaque {
                    ElementKind::EncapsulatedPixels
                } else if sequence {
                    ElementKind::Sequence
                } else {
                    ElementKind::Value
                },
                value: h.value..end.unwrap_or(h.value),
            });
            if sequence || opaque {
                if depth + 1 > limits.max_depth {
                    return Err(DatasetError::DepthLimit);
                }
                depth += 1;
                stack[depth] = Scope {
                    kind: Kind::Sequence(id, opaque),
                    end,
                    bound: end.unwrap_or(scope.bound),
                    last: None,
                };
                cursor = h.value;
            } else {
                cursor = end.ok_or(DatasetError::Boundary)?;
            }
        }
        Ok(result)
    }
}
