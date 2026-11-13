use super::{
    u16_at, u32_at, u64_at, zero, Error as E, Header, Kind, Limits, ObjectSelector,
    MAX_RECORD_BYTES,
};
fn exact(b: &[u8], n: usize) -> Result<(), E> {
    if b.len() == n {
        Ok(())
    } else {
        Err(E::Malformed)
    }
}
fn no_leaf(h: &Header) -> Result<(), E> {
    if h.leaf != 0 || h.count != 0 {
        Err(E::Malformed)
    } else {
        Ok(())
    }
}
fn metadata(h: &Header) -> Result<(), E> {
    no_leaf(h)?;
    if h.entry != 0 {
        Err(E::Malformed)
    } else {
        Ok(())
    }
}
fn association(h: &Header) -> Result<(), E> {
    if h.count > 1 || (h.count == 0 && h.leaf != 0) {
        Err(E::Malformed)
    } else {
        Ok(())
    }
}
fn credited(h: &Header, n: usize) -> Result<(), E> {
    if h.credit_serial == 0 || h.credit_bytes < n as u64 || h.credit_bytes > MAX_RECORD_BYTES as u64
    {
        Err(E::Malformed)
    } else {
        Ok(())
    }
}
fn no_credit(h: &Header) -> Result<(), E> {
    if h.credit_serial != 0 || h.credit_bytes != 0 {
        Err(E::Malformed)
    } else {
        Ok(())
    }
}
pub(super) fn validate(h: &Header, b: &[u8]) -> Result<(), E> {
    if h.deadline == 0 {
        return Err(E::Malformed);
    }
    match h.kind {
        Kind::Cancel | Kind::Close => {
            if h.cancellation != 1 {
                return Err(E::Malformed);
            }
        }
        Kind::Outcome => {}
        _ => {
            if h.cancellation != 0 {
                return Err(E::Malformed);
            }
        }
    }
    match h.kind {
        Kind::Describe => {
            metadata(h)?;
            credited(h, 240)?;
            exact(b, 96)?;
            Limits::parse(&b[..64])?;
            ObjectSelector::parse(&b[64..], true)?;
        }
        Kind::DescriptorPage => {
            metadata(h)?;
            credited(h, 208 + b.len())?;
            if b.len() < 32 {
                return Err(E::Malformed);
            }
            let s = ObjectSelector::parse(&b[..32], false)?;
            exact(&b[32..], s.chunk as usize)?;
        }
        Kind::Select => {
            no_leaf(h)?;
            no_credit(h)?;
            exact(b, 8)?;
            selection(b)?;
        }
        Kind::Open => {
            no_leaf(h)?;
            no_credit(h)?;
            exact(b, 72)?;
            Limits::parse(&b[..64])?;
            selection(&b[64..])?;
        }
        Kind::Grant => {
            no_leaf(h)?;
            no_credit(h)?;
            exact(b, 80)?;
            zero(&b[2..8])?;
            Limits::parse(&b[8..72])?;
            selection(&b[72..])?;
            match u16_at(b, 0) {
                1 => {
                    metadata(h)?;
                    zero(&b[72..80])?;
                }
                2 => {}
                _ => return Err(E::Unsupported),
            }
        }
        Kind::Credit => {
            exact(b, 0)?;
            association(h)?;
            if h.count == 0 {
                metadata(h)?;
            }
            credited(h, 208)?;
        }
        Kind::RequestLeaves => {
            exact(b, 0)?;
            if h.count != 1 {
                return Err(E::Malformed);
            }
            credited(h, 217)?;
        }
        Kind::LeafData => {
            if h.count != 1 || b.len() < 8 {
                return Err(E::Malformed);
            }
            credited(h, 208 + b.len())?;
            let p = u32_at(b, 0) as usize;
            let q = u16_at(b, 4) as usize;
            zero(&b[6..8])?;
            if !(1..=16384).contains(&p) || !(1..=1537).contains(&q) {
                return Err(E::Limit);
            }
            exact(b, 8 + p + q)?;
            let siblings = b[8 + p] as usize;
            if siblings > 32 || q != 1 + 48 * siblings {
                return Err(E::Malformed);
            }
        }
        Kind::Cancel => {
            association(h)?;
            no_credit(h)?;
            reason(b, false)?;
        }
        Kind::Close => {
            metadata(h)?;
            no_credit(h)?;
            reason(b, true)?;
        }
        Kind::Resume => {
            no_leaf(h)?;
            no_credit(h)?;
            if b.len() < 8 {
                return Err(E::Malformed);
            }
            let n = u16_at(b, 0) as usize;
            if !(1..=8).contains(&n) {
                return Err(E::Limit);
            }
            zero(&b[2..8])?;
            exact(b, 8 + 8 * n)?;
            let mut previous = None;
            for i in 0..n {
                let v = u64_at(b, 8 + 8 * i);
                if previous.is_some_and(|p| p >= v) {
                    return Err(E::Malformed);
                }
                previous = Some(v);
            }
        }
        Kind::Outcome => {
            association(h)?;
            no_credit(h)?;
            exact(b, 16)?;
            zero(&b[4..8])?;
            zero(&b[12..16])?;
            let status = u16_at(b, 0);
            let stage = u16_at(b, 2);
            let n = u32_at(b, 8);
            if status > 5 || stage > 4 {
                return Err(E::Unsupported);
            }
            if n > 1 || (n != 0 && stage != 3) {
                return Err(E::Malformed);
            }
        }
    }
    Ok(())
}
fn selection(b: &[u8]) -> Result<(), E> {
    if u32_at(b, 0) >= 4096 || u32_at(b, 4) >= 32 {
        Err(E::Limit)
    } else {
        Ok(())
    }
}
fn reason(b: &[u8], close: bool) -> Result<(), E> {
    exact(b, 8)?;
    zero(&b[2..8])?;
    let r = u16_at(b, 0);
    if r > 5 || (!close && r == 0) {
        Err(E::Unsupported)
    } else {
        Ok(())
    }
}
