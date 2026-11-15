//! Serve descriptor / pages / encode_proof / representation bytes.
//! HTTP status is not a grant. Charge the existing ReservationLedger.

use super::range::{
    write_content_range, write_hex_lower, write_u64_ascii, write_unsatisfiable_content_range,
    InclusiveRange, RangeError, LEGACY_FULL_OBJECT_FALLBACK_BYTES,
};
use super::status::{
    strong_etag, write_strong_etag, FallbackPolicy, StatusError, Validator,
};
use crate::bundle::stream::policy::LocalEffectControl;
use crate::bundle::stream::proof::{
    decode_proof, verify_leaf, ProofError, LEAF_BYTES, MAX_SIBLINGS,
};
use crate::bundle::stream::selection::SelectedPlainEntry;
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardError};
use crate::net::peer::runtime::{ReservationHandle, ReservationLedger, ResourceBudget};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderError {
    Identity,
    Policy,
    Unsupported,
    Unsatisfiable,
    ChangedValidator,
    Encoding,
    Capacity,
    Bounds,
    Proof,
    Budget,
    Ledger(QdnfError),
    Status(StatusError),
    Range(RangeError),
}

impl From<RangeError> for ProviderError {
    fn from(e: RangeError) -> Self {
        match e {
            RangeError::Unsatisfiable => Self::Unsatisfiable,
            RangeError::Capacity => Self::Capacity,
            other => Self::Range(other),
        }
    }
}

impl From<ProofError> for ProviderError {
    fn from(_: ProofError) -> Self {
        Self::Proof
    }
}

impl From<OutboardError> for ProviderError {
    fn from(e: OutboardError) -> Self {
        match e {
            OutboardError::Identity => Self::Identity,
            OutboardError::Capacity => Self::Capacity,
            OutboardError::Proof(_) => Self::Proof,
            _ => Self::Bounds,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpObject {
    Descriptor,
    Page,
    Proof { leaf: u64 },
    Representation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServeOptions {
    pub fallback: FallbackPolicy,
    pub verified_peer: bool,
}

pub struct HttpProvider<'src, 'sel, 'nodes, 'meta> {
    source: &'src [u8],
    catalog: OutboardCatalog<'sel, 'nodes>,
    descriptor: &'meta [u8],
    entry_page: &'meta [u8],
}

pub struct ServedHeaders {
    pub status: u16,
    pub content_range: [u8; 80],
    pub content_range_len: usize,
    pub content_length: u64,
    pub etag: [u8; 98],
    pub etag_len: usize,
}

pub struct Served {
    pub status: u16,
    pub total: u64,
    pub body_len: usize,
    pub headers: ServedHeaders,
    pub charge: Option<ReservationHandle>,
}

impl<'src, 'sel, 'nodes, 'meta> HttpProvider<'src, 'sel, 'nodes, 'meta> {
    pub fn bind(
        source: &'src [u8],
        catalog: OutboardCatalog<'sel, 'nodes>,
        descriptor: &'meta [u8],
        entry_page: &'meta [u8],
    ) -> Result<Self, ProviderError> {
        let selected = catalog.selected();
        let start = usize::try_from(selected.offset()).map_err(|_| ProviderError::Bounds)?;
        let len = usize::try_from(selected.logical_length()).map_err(|_| ProviderError::Bounds)?;
        let end = start.checked_add(len).ok_or(ProviderError::Bounds)?;
        if end > source.len() {
            return Err(ProviderError::Identity);
        }
        if descriptor.is_empty() || entry_page.is_empty() {
            return Err(ProviderError::Identity);
        }
        Ok(Self {
            source,
            catalog,
            descriptor,
            entry_page,
        })
    }

    pub fn selected(&self) -> SelectedPlainEntry<'sel> {
        self.catalog.selected()
    }

    pub fn representation_etag(&self, out: &mut [u8]) -> Result<usize, ProviderError> {
        Ok(write_strong_etag(self.catalog.selected().root(), out)?)
    }

    /// Deployment convention, not a registry allocation.
    pub fn write_path(&self, object: HttpObject, out: &mut [u8]) -> Result<usize, ProviderError> {
        let id = self.catalog.selected().candidate_identity().descriptor_id();
        write_object_path(object, id, self.catalog.selected().entry_id(), out)
    }

    pub fn serve(
        &self,
        object: HttpObject,
        range: Option<InclusiveRange>,
        if_range: Option<&[u8]>,
        expected: Validator<'_>,
        options: ServeOptions,
        authority: &mut LocalEffectControl<'_, '_>,
        ledger: &mut ReservationLedger,
        out: &mut [u8],
    ) -> Result<Served, ProviderError> {
        if let HttpObject::Proof { leaf } = object {
            return serve_proof_into(
                self, leaf, if_range, expected, options, authority, ledger, out,
            );
        }
        if authority.closure().identity() != self.catalog.selected().candidate_identity() {
            return Err(ProviderError::Identity);
        }
        authority.recheck().map_err(|_| ProviderError::Policy)?;

        let (bytes, etag_digest) = self.object_bytes(object)?;
        let total = bytes.len() as u64;
        let mut etag = [0u8; 98];
        let etag_len = write_strong_etag(etag_digest, &mut etag)?;
        check_validators(&etag[..etag_len], if_range, expected)?;

        let decided = decide_range(range, total, options.fallback)?;
        match decided {
            Decision::Unsatisfiable => {
                let mut headers = empty_headers(416);
                headers.etag = etag;
                headers.etag_len = etag_len;
                headers.content_range_len =
                    write_unsatisfiable_content_range(total, &mut headers.content_range)?;
                Ok(Served {
                    status: 416,
                    total,
                    body_len: 0,
                    headers,
                    charge: None,
                })
            }
            Decision::Partial(incl) | Decision::Full(incl) => {
                let start = usize::try_from(incl.start()).map_err(|_| ProviderError::Bounds)?;
                let n = usize::try_from(incl.length()).map_err(|_| ProviderError::Bounds)?;
                let end = start.checked_add(n).ok_or(ProviderError::Bounds)?;
                if end > bytes.len() || n > out.len() {
                    return Err(ProviderError::Capacity);
                }
                let status = if matches!(decided, Decision::Full(_)) {
                    200
                } else {
                    206
                };
                let charge = ledger
                    .reserve(
                        ResourceBudget {
                            bytes: n as u64,
                            work: 1,
                            io: 1,
                        },
                        options.verified_peer,
                    )
                    .map_err(|e| {
                        if e == QdnfError::BudgetExhausted {
                            ProviderError::Budget
                        } else {
                            ProviderError::Ledger(e)
                        }
                    })?;
                out[..n].copy_from_slice(&bytes[start..end]);
                let mut headers = empty_headers(status);
                headers.etag = etag;
                headers.etag_len = etag_len;
                headers.content_length = n as u64;
                headers.content_range_len = write_content_range(incl, total, &mut headers.content_range)?;
                Ok(Served {
                    status,
                    total,
                    body_len: n,
                    headers,
                    charge: Some(charge),
                })
            }
        }
    }

    fn object_bytes(&self, object: HttpObject) -> Result<(&[u8], StrongDigest), ProviderError> {
        let selected = self.catalog.selected();
        match object {
            HttpObject::Descriptor => Ok((
                self.descriptor,
                selected.candidate_identity().descriptor_id(),
            )),
            HttpObject::Page => Ok((self.entry_page, selected.root())),
            HttpObject::Representation => {
                let start = usize::try_from(selected.offset()).map_err(|_| ProviderError::Bounds)?;
                let len =
                    usize::try_from(selected.logical_length()).map_err(|_| ProviderError::Bounds)?;
                Ok((&self.source[start..start + len], selected.root()))
            }
            HttpObject::Proof { .. } => Err(ProviderError::Unsupported),
        }
    }
}

/// Proof bytes cannot borrow a stack buffer. Encode into `out` directly.
pub fn serve_proof_into(
    provider: &HttpProvider<'_, '_, '_, '_>,
    leaf: u64,
    if_range: Option<&[u8]>,
    expected: Validator<'_>,
    options: ServeOptions,
    authority: &mut LocalEffectControl<'_, '_>,
    ledger: &mut ReservationLedger,
    out: &mut [u8],
) -> Result<Served, ProviderError> {
    if authority.closure().identity() != provider.catalog.selected().candidate_identity() {
        return Err(ProviderError::Identity);
    }
    authority.recheck().map_err(|_| ProviderError::Policy)?;
    let selected = provider.catalog.selected();
    let spec = *provider.catalog.tree_spec();
    let payload_len = spec.logical_leaf_length(leaf)?;
    let leaf_off = leaf.checked_mul(LEAF_BYTES).ok_or(ProviderError::Bounds)?;
    let start = selected
        .offset()
        .checked_add(leaf_off)
        .ok_or(ProviderError::Bounds)?;
    let start = usize::try_from(start).map_err(|_| ProviderError::Bounds)?;
    let end = start
        .checked_add(payload_len)
        .ok_or(ProviderError::Bounds)?;
    if end > provider.source.len() {
        return Err(ProviderError::Identity);
    }
    let payload = &provider.source[start..end];
    let mut encoded = [0u8; 1 + MAX_SIBLINGS * 48];
    let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
    // `proof_into` encodes via canonical `encode_proof`; this module has no second codec.
    let n = provider
        .catalog
        .proof_into(leaf, &mut encoded, &mut siblings)?;
    let mut decoded = [StrongDigest::ZERO; MAX_SIBLINGS];
    let path = decode_proof(&encoded[..n], &mut decoded)?;
    verify_leaf(&spec, selected.root(), leaf, payload, path)?;

    let mut etag = [0u8; 98];
    let etag_len = write_strong_etag(selected.root(), &mut etag)?;
    check_validators(&etag[..etag_len], if_range, expected)?;

    let total = n as u64;
    let incl = InclusiveRange::from_start_length(0, total)?;
    if n > out.len() {
        return Err(ProviderError::Capacity);
    }
    let ServeOptions {
        fallback: _,
        verified_peer,
    } = options;
    let charge = ledger
        .reserve(
            ResourceBudget {
                bytes: total,
                work: 1,
                io: 1,
            },
            verified_peer,
        )
        .map_err(|e| {
            if e == QdnfError::BudgetExhausted {
                ProviderError::Budget
            } else {
                ProviderError::Ledger(e)
            }
        })?;
    out[..n].copy_from_slice(&encoded[..n]);
    let mut headers = empty_headers(206);
    headers.etag = etag;
    headers.etag_len = etag_len;
    headers.content_length = total;
    headers.content_range_len = write_content_range(incl, total, &mut headers.content_range)?;
    Ok(Served {
        status: 206,
        total,
        body_len: n,
        headers,
        charge: Some(charge),
    })
}

enum Decision {
    Partial(InclusiveRange),
    Full(InclusiveRange),
    Unsatisfiable,
}

fn decide_range(
    range: Option<InclusiveRange>,
    total: u64,
    fallback: FallbackPolicy,
) -> Result<Decision, ProviderError> {
    match range {
        Some(r) if r.fits(total) => Ok(Decision::Partial(r)),
        Some(_) => Ok(Decision::Unsatisfiable),
        None => {
            if !fallback.allow_full_object
                || total == 0
                || total > fallback.max_full_object_bytes
                || total > LEGACY_FULL_OBJECT_FALLBACK_BYTES
            {
                return Err(ProviderError::Budget);
            }
            Ok(Decision::Full(InclusiveRange::from_start_length(0, total)?))
        }
    }
}

fn check_validators(
    etag: &[u8],
    if_range: Option<&[u8]>,
    expected: Validator<'_>,
) -> Result<(), ProviderError> {
    if let Some(tag) = if_range {
        let got = strong_etag(tag).ok_or(ProviderError::ChangedValidator)?;
        if got != etag {
            return Err(ProviderError::ChangedValidator);
        }
    }
    if let Some(want) = expected.etag {
        if want != etag {
            return Err(ProviderError::ChangedValidator);
        }
    }
    Ok(())
}

fn empty_headers(status: u16) -> ServedHeaders {
    ServedHeaders {
        status,
        content_range: [0; 80],
        content_range_len: 0,
        content_length: 0,
        etag: [0; 98],
        etag_len: 0,
    }
}

pub fn write_object_path(
    object: HttpObject,
    descriptor: StrongDigest,
    entry: u32,
    out: &mut [u8],
) -> Result<usize, ProviderError> {
    let mut n = 0;
    fn put(out: &mut [u8], n: &mut usize, b: &[u8]) -> Result<(), ProviderError> {
        let end = n.checked_add(b.len()).ok_or(ProviderError::Capacity)?;
        if end > out.len() {
            return Err(ProviderError::Capacity);
        }
        out[*n..end].copy_from_slice(b);
        *n = end;
        Ok(())
    }
    put(out, &mut n, b"/objects/")?;
    if n + 96 > out.len() {
        return Err(ProviderError::Capacity);
    }
    n += write_hex_lower(&descriptor.0, &mut out[n..])?;
    match object {
        HttpObject::Descriptor => put(out, &mut n, b"/descriptor")?,
        HttpObject::Page => put(out, &mut n, b"/pages/0")?,
        HttpObject::Representation => {
            put(out, &mut n, b"/representations/")?;
            n += write_u64_ascii(entry as u64, &mut out[n..])?;
        }
        HttpObject::Proof { leaf } => {
            put(out, &mut n, b"/entries/")?;
            n += write_u64_ascii(entry as u64, &mut out[n..])?;
            put(out, &mut n, b"/proof/")?;
            n += write_u64_ascii(leaf, &mut out[n..])?;
        }
    }
    Ok(n)
}

pub fn write_http_head(headers: &ServedHeaders, out: &mut [u8]) -> Result<usize, ProviderError> {
    let mut n = 0;
    fn put(out: &mut [u8], n: &mut usize, b: &[u8]) -> Result<(), ProviderError> {
        let end = n.checked_add(b.len()).ok_or(ProviderError::Capacity)?;
        if end > out.len() {
            return Err(ProviderError::Capacity);
        }
        out[*n..end].copy_from_slice(b);
        *n = end;
        Ok(())
    }
    put(out, &mut n, b"HTTP/1.1 ")?;
    n += write_u64_ascii(headers.status as u64, &mut out[n..])?;
    let reason: &[u8] = match headers.status {
        200 => b" OK\r\n",
        206 => b" Partial Content\r\n",
        416 => b" Range Not Satisfiable\r\n",
        _ => b"\r\n",
    };
    put(out, &mut n, reason)?;
    put(out, &mut n, b"Content-Encoding: identity\r\nAccept-Ranges: bytes\r\n")?;
    if headers.content_range_len > 0 {
        put(out, &mut n, b"Content-Range: ")?;
        put(out, &mut n, &headers.content_range[..headers.content_range_len])?;
        put(out, &mut n, b"\r\n")?;
    }
    put(out, &mut n, b"Content-Length: ")?;
    n += write_u64_ascii(headers.content_length, &mut out[n..])?;
    put(out, &mut n, b"\r\n")?;
    if headers.etag_len > 0 {
        put(out, &mut n, b"ETag: ")?;
        put(out, &mut n, &headers.etag[..headers.etag_len])?;
        put(out, &mut n, b"\r\n")?;
    }
    put(out, &mut n, b"Connection: close\r\n\r\n")?;
    Ok(n)
}
