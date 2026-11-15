#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetError {
    NotPart10,
    Truncated,
    Overflow,
    ByteLimit,
    IndexLimit,
    DepthLimit,
    ElementLimit,
    ItemLimit,
    WorkLimit,
    MalformedHeader,
    MalformedDelimiter,
    Boundary,
    Duplicate(u32),
    Order,
    Missing(u32),
    Invalid(u32),
    UnsupportedSyntax,
    UnsupportedImplicitSequence,
    NotSequence,
    InvalidId,
}
impl std::fmt::Display for DatasetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM dataset: {self:?}")
    }
}
impl std::error::Error for DatasetError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferSyntax {
    ImplicitLittleEndian,
    ExplicitLittleEndian,
    EncapsulatedExplicitLittleEndian,
}
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub max_bytes: usize,
    pub max_index_bytes: usize,
    pub max_elements: usize,
    pub max_items: usize,
    pub max_depth: usize,
    pub max_work: usize,
}
impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_index_bytes: 2 * 1024 * 1024,
            max_elements: 8192,
            max_items: 4096,
            max_depth: 32,
            max_work: 32768,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Header {
    pub tag: u32,
    pub vr: Option<[u8; 2]>,
    pub len: u32,
    pub value: usize,
}
pub(super) fn u16le(bytes: &[u8], at: usize) -> Result<u16, DatasetError> {
    let b = bytes
        .get(at..at.checked_add(2).ok_or(DatasetError::Overflow)?)
        .ok_or(DatasetError::Truncated)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
pub(super) fn u32le(bytes: &[u8], at: usize) -> Result<u32, DatasetError> {
    let b = bytes
        .get(at..at.checked_add(4).ok_or(DatasetError::Overflow)?)
        .ok_or(DatasetError::Truncated)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
pub(super) fn long_vr(vr: &[u8; 2]) -> bool {
    matches!(
        vr,
        b"OB"
            | b"OD"
            | b"OF"
            | b"OL"
            | b"OV"
            | b"OW"
            | b"SQ"
            | b"SV"
            | b"UC"
            | b"UN"
            | b"UR"
            | b"UT"
            | b"UV"
    )
}
pub(super) fn header(
    bytes: &[u8],
    at: usize,
    explicit: bool,
    end: usize,
) -> Result<Header, DatasetError> {
    let bounded = bytes.get(..end).ok_or(DatasetError::Boundary)?;
    let tag = ((u16le(bounded, at)? as u32) << 16)
        | u16le(bounded, at.checked_add(2).ok_or(DatasetError::Overflow)?)? as u32;
    let add = |n| at.checked_add(n).ok_or(DatasetError::Overflow);
    if tag >> 16 == 0xfffe || !explicit {
        return Ok(Header {
            tag,
            vr: None,
            len: u32le(bounded, add(4)?)?,
            value: add(8)?,
        });
    }
    let vr = [
        *bounded.get(add(4)?).ok_or(DatasetError::Truncated)?,
        *bounded.get(add(5)?).ok_or(DatasetError::Truncated)?,
    ];
    let short = matches!(
        &vr,
        b"AE"
            | b"AS"
            | b"AT"
            | b"CS"
            | b"DA"
            | b"DS"
            | b"DT"
            | b"FL"
            | b"FD"
            | b"IS"
            | b"LO"
            | b"LT"
            | b"PN"
            | b"SH"
            | b"SL"
            | b"SS"
            | b"ST"
            | b"TM"
            | b"UI"
            | b"UL"
            | b"US"
    );
    if long_vr(&vr) {
        if u16le(bounded, add(6)?)? != 0 {
            return Err(DatasetError::MalformedHeader);
        }
        Ok(Header {
            tag,
            vr: Some(vr),
            len: u32le(bounded, add(8)?)?,
            value: add(12)?,
        })
    } else if short {
        Ok(Header {
            tag,
            vr: Some(vr),
            len: u16le(bounded, add(6)?)? as u32,
            value: add(8)?,
        })
    } else {
        Err(DatasetError::MalformedHeader)
    }
}
pub(super) fn implicit_sequence(tag: u32) -> bool {
    matches!(
        tag,
        0x52009229
            | 0x52009230
            | 0x00209113
            | 0x00209116
            | 0x00289110
            | 0x00289145
            | 0x00209111
            | 0x00409096
            | 0x004008ea
            | 0x00081140
            | 0x00089124
            | 0x00082218
    )
}
