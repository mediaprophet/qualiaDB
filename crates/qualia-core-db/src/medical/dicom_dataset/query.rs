use super::{DatasetError, Element, ElementId, ElementKind, ItemId, Part10Dataset, TransferSyntax};
#[derive(Clone, Copy, Debug)]
pub struct WorkBudget {
    remaining: usize,
}
impl WorkBudget {
    pub fn new(steps: usize) -> Self {
        Self { remaining: steps }
    }
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    pub fn charge(&mut self, n: usize) -> Result<(), DatasetError> {
        self.remaining = self
            .remaining
            .checked_sub(n)
            .ok_or(DatasetError::WorkLimit)?;
        Ok(())
    }
}
pub(crate) fn uid(value: &[u8], tag: u32) -> Result<&str, DatasetError> {
    let value = value.strip_suffix(&[0]).unwrap_or(value);
    let s = std::str::from_utf8(value).map_err(|_| DatasetError::Invalid(tag))?;
    if s.is_empty()
        || s.len() > 64
        || !s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        || s.split('.')
            .any(|p| p.is_empty() || (p.len() > 1 && p.starts_with('0')))
    {
        return Err(DatasetError::Invalid(tag));
    }
    Ok(s)
}
impl<'a> Part10Dataset<'a> {
    pub fn original_bytes(&self) -> &'a [u8] {
        self.source
    }
    pub fn source_blake3(&self) -> [u8; 32] {
        self.digest
    }
    pub fn transfer_syntax(&self) -> TransferSyntax {
        self.syntax
    }
    pub fn transfer_syntax_uid(&self) -> &'a str {
        self.transfer_uid
    }
    pub fn media_sop_class_uid(&self) -> Option<&'a str> {
        self.media_class
    }
    pub fn media_sop_instance_uid(&self) -> Option<&'a str> {
        self.media_instance
    }
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
    pub fn element(&self, id: ElementId) -> Result<&Element, DatasetError> {
        self.elements.get(id.0).ok_or(DatasetError::InvalidId)
    }
    pub fn value(&self, id: ElementId) -> Result<&'a [u8], DatasetError> {
        let e = self.element(id)?;
        self.source
            .get(e.value.clone())
            .ok_or(DatasetError::Boundary)
    }
    pub fn find(
        &self,
        scope: Option<ItemId>,
        tag: u32,
        work: &mut WorkBudget,
    ) -> Result<Option<ElementId>, DatasetError> {
        if scope.is_some_and(|i| i.0 >= self.items.len()) {
            return Err(DatasetError::InvalidId);
        }
        work.charge(
            self.elements
                .len()
                .checked_add(1)
                .ok_or(DatasetError::Overflow)?,
        )?;
        Ok(self
            .elements
            .iter()
            .position(|e| e.scope == scope && e.tag == tag)
            .map(ElementId))
    }
    pub fn children<'b>(
        &'b self,
        scope: Option<ItemId>,
        work: &mut WorkBudget,
    ) -> Result<impl Iterator<Item = ElementId> + 'b, DatasetError> {
        if scope.is_some_and(|i| i.0 >= self.items.len()) {
            return Err(DatasetError::InvalidId);
        }
        work.charge(self.elements.len())?;
        Ok(self
            .elements
            .iter()
            .enumerate()
            .filter(move |(_, e)| e.scope == scope)
            .map(|(i, _)| ElementId(i)))
    }
    pub fn sequence_items<'b>(
        &'b self,
        id: ElementId,
        work: &mut WorkBudget,
    ) -> Result<impl Iterator<Item = ItemId> + 'b, DatasetError> {
        if self.element(id)?.kind == ElementKind::Value {
            return Err(DatasetError::NotSequence);
        }
        work.charge(self.items.len())?;
        Ok(self
            .items
            .iter()
            .enumerate()
            .filter(move |(_, item)| item.sequence == id)
            .map(|(i, _)| ItemId(i)))
    }
    pub fn item_bytes(&self, id: ItemId) -> Result<&'a [u8], DatasetError> {
        let i = self.items.get(id.0).ok_or(DatasetError::InvalidId)?;
        self.source
            .get(i.value.clone())
            .ok_or(DatasetError::Boundary)
    }
    pub fn expected_value(
        &self,
        scope: Option<ItemId>,
        tag: u32,
        vr: [u8; 2],
        work: &mut WorkBudget,
    ) -> Result<Option<&'a [u8]>, DatasetError> {
        let Some(id) = self.find(scope, tag, work)? else {
            return Ok(None);
        };
        let e = self.element(id)?;
        if e.kind != ElementKind::Value || e.vr.is_some_and(|v| v != vr) {
            return Err(DatasetError::Invalid(tag));
        }
        Ok(Some(self.value(id)?))
    }
    pub fn uid(
        &self,
        scope: Option<ItemId>,
        tag: u32,
        work: &mut WorkBudget,
    ) -> Result<Option<&'a str>, DatasetError> {
        self.expected_value(scope, tag, *b"UI", work)?
            .map(|v| uid(v, tag))
            .transpose()
    }
    pub fn text(
        &self,
        scope: Option<ItemId>,
        tag: u32,
        vr: [u8; 2],
        work: &mut WorkBudget,
    ) -> Result<Option<&'a str>, DatasetError> {
        self.expected_value(scope, tag, vr, work)?
            .map(|v| {
                let s = std::str::from_utf8(v)
                    .map_err(|_| DatasetError::Invalid(tag))?
                    .trim_matches(' ');
                if s.is_empty() || s.bytes().any(|b| b < 32 || b == 127) {
                    Err(DatasetError::Invalid(tag))
                } else {
                    Ok(s)
                }
            })
            .transpose()
    }
    pub fn unsigned_short(
        &self,
        tag: u32,
        work: &mut WorkBudget,
    ) -> Result<Option<u16>, DatasetError> {
        self.expected_value(None, tag, *b"US", work)?
            .map(|v| {
                if v.len() != 2 {
                    return Err(DatasetError::Invalid(tag));
                }
                Ok(u16::from_le_bytes([v[0], v[1]]))
            })
            .transpose()
    }
    pub fn frame_count(&self, work: &mut WorkBudget) -> Result<u32, DatasetError> {
        match self.text(None, 0x00280008, *b"IS", work)? {
            None => Ok(1),
            Some(text) => {
                if text.len() > 12 || !text.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(DatasetError::Invalid(0x00280008));
                }
                let n = text
                    .parse::<u32>()
                    .map_err(|_| DatasetError::Invalid(0x00280008))?;
                if n == 0 {
                    return Err(DatasetError::Invalid(0x00280008));
                }
                Ok(n)
            }
        }
    }
}
