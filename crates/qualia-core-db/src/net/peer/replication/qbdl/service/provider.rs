//! Plain whole-source provider. Mixed/private/protected wraps are Unsupported.
use super::session::{
    associate_until, check_identity, encode_grant_body, parse_selection, ServiceIo,
};
use super::{from_outboard, from_proof, map_policy, Error};
use crate::bundle::stream::policy::LocalEffectControl;
use crate::bundle::stream::proof::{
    decode_proof, verify_leaf, Representation, LEAF_BYTES, MAX_SIBLINGS,
};
use crate::bundle::stream::publish::PublicationClass;
use crate::bundle::stream::selection::SelectedPlainEntry;
use crate::net::peer::replication::qbdl::provider::OutboardCatalog;
use crate::net::peer::replication::qbdl::wire::{Kind, MAX_LEAF_RECORD_BYTES, PREFIX_BYTES};
use crate::net::qdnf::types::StrongDigest;

pub fn refuse_non_plain(
    class: PublicationClass,
    representation: Representation,
) -> Result<(), Error> {
    match (class, representation) {
        (PublicationClass::PlainCandidate, Representation::Plaintext) => Ok(()),
        _ => Err(Error::Unsupported),
    }
}

pub struct PlainProvider<'src, 'sel, 'nodes> {
    source: &'src [u8],
    catalog: OutboardCatalog<'sel, 'nodes>,
    policy_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenedTransfer {
    pub transfer_id: [u8; 16],
    pub entry: u32,
    pub ordinal: u32,
    pub page: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServedLeaf {
    pub entry: u32,
    pub leaf: u64,
    pub source_offset: u64,
    pub length: usize,
}

impl<'src, 'sel, 'nodes> PlainProvider<'src, 'sel, 'nodes> {
    pub fn bind(
        class: PublicationClass,
        source: &'src [u8],
        catalog: OutboardCatalog<'sel, 'nodes>,
        policy_generation: u64,
    ) -> Result<Self, Error> {
        refuse_non_plain(class, catalog.tree_spec().representation())?;
        if catalog.tree_spec().key_epoch() != 0 {
            return Err(Error::Unsupported);
        }
        let selected = catalog.selected();
        let start = usize::try_from(selected.offset()).map_err(|_| Error::Limit)?;
        let len = usize::try_from(selected.logical_length()).map_err(|_| Error::Limit)?;
        let end = start.checked_add(len).ok_or(Error::Overflow)?;
        if end > source.len() {
            return Err(Error::Identity);
        }
        Ok(Self {
            source,
            catalog,
            policy_generation,
        })
    }

    pub fn selected(&self) -> SelectedPlainEntry<'sel> {
        self.catalog.selected()
    }

    pub fn answer_describe<S: ServiceIo>(
        &self,
        session: &mut S,
        authority: &mut LocalEffectControl<'_, '_>,
    ) -> Result<super::describe::ServedDescriptor, Error> {
        super::describe::answer_describe(
            session,
            authority,
            self.catalog.selected().candidate_identity(),
            self.policy_generation,
            self.source,
        )
    }

    pub fn answer_describe_pages<S: ServiceIo>(
        &self,
        session: &mut S,
        authority: &mut LocalEffectControl<'_, '_>,
        body: &[u8],
        entry_page: &[u8],
    ) -> Result<super::describe::ServedDescriptor, Error> {
        super::describe::answer_describe_pages(
            session,
            authority,
            self.catalog.selected().candidate_identity(),
            self.policy_generation,
            self.source,
            body,
            entry_page,
        )
    }

    pub fn answer_open<S: ServiceIo>(
        &self,
        session: &mut S,
        authority: &mut LocalEffectControl<'_, '_>,
    ) -> Result<OpenedTransfer, Error> {
        self.authorize(authority, session)?;
        let (header, body, n) = session.recv_control()?;
        if header.kind != Kind::Open {
            return Err(session.fail_close(Error::State));
        }
        if let Err(e) = self.associate(&header) {
            return Err(session.fail_close(e));
        }
        if header.leaf != 0 || header.count != 0 || header.credit_serial != 0 {
            return Err(session.fail_close(Error::Malformed));
        }
        let selected = self.catalog.selected();
        if header.entry != selected.entry_id() {
            return Err(session.fail_close(Error::Identity));
        }
        if n != 72 {
            return Err(session.fail_close(Error::Malformed));
        }
        let (ordinal, page) = match parse_selection(&body, 64) {
            Ok(v) => v,
            Err(e) => return Err(session.fail_close(e)),
        };
        if ordinal != selected.ordinal() {
            return Err(session.fail_close(Error::Identity));
        }
        let mut grant_body = [0u8; 80];
        let g = match encode_grant_body(ordinal, page, &mut grant_body) {
            Ok(g) => g,
            Err(e) => return Err(session.fail_close(e)),
        };
        let reply = associate_until(
            Kind::Grant,
            selected.candidate_identity(),
            self.policy_generation,
            header.transfer_id,
            header.entry,
            0,
            0,
            0,
            0,
            header.deadline,
        );
        session.send_message(&reply, &grant_body[..g])?;
        Ok(OpenedTransfer {
            transfer_id: header.transfer_id,
            entry: header.entry,
            ordinal,
            page,
        })
    }

    pub fn answer_leaf<S: ServiceIo>(
        &self,
        session: &mut S,
        authority: &mut LocalEffectControl<'_, '_>,
        opened: &OpenedTransfer,
    ) -> Result<ServedLeaf, Error> {
        self.answer_leaf_after_credit(session, authority, opened, || Ok(()))
    }

    /// Recheck, then spend one-shot runtime credit, then disclose. `issue` must
    /// hold `RuntimeAccess` only for `issue_credit` — never across WouldBlock.
    pub fn answer_leaf_after_credit<S: ServiceIo>(
        &self,
        session: &mut S,
        authority: &mut LocalEffectControl<'_, '_>,
        opened: &OpenedTransfer,
        issue: impl FnOnce() -> Result<(), Error>,
    ) -> Result<ServedLeaf, Error> {
        self.authorize(authority, session)?;
        let (credit, _, cn) = session.recv_control()?;
        if credit.kind != Kind::Credit || cn != 0 {
            return Err(session.fail_close(Error::State));
        }
        if let Err(e) = self.associate(&credit) {
            return Err(session.fail_close(e));
        }
        if credit.transfer_id != opened.transfer_id
            || credit.entry != opened.entry
            || credit.count != 1
            || credit.credit_serial == 0
        {
            return Err(session.fail_close(Error::Credit));
        }
        let (request, _, rn) = session.recv_control()?;
        if request.kind != Kind::RequestLeaves || rn != 0 {
            return Err(session.fail_close(Error::State));
        }
        if let Err(e) = self.associate(&request) {
            return Err(session.fail_close(e));
        }
        if request.transfer_id != credit.transfer_id
            || request.entry != credit.entry
            || request.leaf != credit.leaf
            || request.count != 1
            || request.credit_serial != credit.credit_serial
            || request.credit_bytes != credit.credit_bytes
        {
            return Err(session.fail_close(Error::Credit));
        }
        let spec = *self.catalog.tree_spec();
        let leaf = request.leaf;
        let payload_len = match spec.logical_leaf_length(leaf) {
            Ok(n) => n,
            Err(e) => return Err(session.fail_close(from_proof(e))),
        };
        let selected = self.catalog.selected();
        let leaf_off = match leaf.checked_mul(LEAF_BYTES) {
            Some(v) => v,
            None => return Err(session.fail_close(Error::Overflow)),
        };
        let start = match selected.offset().checked_add(leaf_off) {
            Some(v) => v,
            None => return Err(session.fail_close(Error::Overflow)),
        };
        let start_usize = match usize::try_from(start) {
            Ok(v) => v,
            Err(_) => return Err(session.fail_close(Error::Limit)),
        };
        let end = match start_usize.checked_add(payload_len) {
            Some(v) => v,
            None => return Err(session.fail_close(Error::Overflow)),
        };
        let entry_end =
            match (selected.offset() as usize).checked_add(selected.logical_length() as usize) {
                Some(v) => v,
                None => return Err(session.fail_close(Error::Overflow)),
            };
        if end > entry_end || end > self.source.len() {
            return Err(session.fail_close(Error::Identity));
        }
        let payload = &self.source[start_usize..end];
        let mut proof = [0u8; 1 + MAX_SIBLINGS * 48];
        let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
        let proof_len = match self.catalog.proof_into(leaf, &mut proof, &mut siblings) {
            Ok(n) => n,
            Err(e) => return Err(session.fail_close(from_outboard(e))),
        };
        let mut decoded = [StrongDigest::ZERO; MAX_SIBLINGS];
        let path = match decode_proof(&proof[..proof_len], &mut decoded) {
            Ok(p) => p,
            Err(e) => return Err(session.fail_close(from_proof(e))),
        };
        if let Err(e) = verify_leaf(&spec, selected.root(), leaf, payload, path) {
            return Err(session.fail_close(from_proof(e)));
        }
        let body_len = 8 + payload.len() + proof_len;
        let mut body = [0u8; 8 + 16384 + 1 + MAX_SIBLINGS * 48];
        if body_len > body.len() {
            return Err(session.fail_close(Error::Capacity));
        }
        body[..4].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        body[4..6].copy_from_slice(&(proof_len as u16).to_be_bytes());
        body[8..8 + payload.len()].copy_from_slice(payload);
        body[8 + payload.len()..body_len].copy_from_slice(&proof[..proof_len]);
        let record_bytes = PREFIX_BYTES.saturating_add(body_len) as u64;
        if request.credit_bytes < record_bytes
            || request.credit_bytes > MAX_LEAF_RECORD_BYTES as u64
        {
            return Err(session.fail_close(Error::Credit));
        }
        self.authorize(authority, session)?;
        if let Err(e) = issue() {
            return Err(session.fail_close(e));
        }
        let header = associate_until(
            Kind::LeafData,
            selected.candidate_identity(),
            self.policy_generation,
            request.transfer_id,
            request.entry,
            leaf,
            1,
            request.credit_serial,
            request.credit_bytes,
            request.deadline,
        );
        session.send_message(&header, &body[..body_len])?;
        Ok(ServedLeaf {
            entry: request.entry,
            leaf,
            source_offset: start,
            length: payload.len(),
        })
    }

    fn associate(
        &self,
        header: &crate::net::peer::replication::qbdl::wire::Header,
    ) -> Result<(), Error> {
        check_identity(
            header,
            self.catalog.selected().candidate_identity(),
            self.policy_generation,
        )
    }

    fn authorize<S: ServiceIo>(
        &self,
        authority: &mut LocalEffectControl<'_, '_>,
        session: &mut S,
    ) -> Result<(), Error> {
        if authority.closure().identity() != self.catalog.selected().candidate_identity() {
            return Err(session.fail_close(Error::Identity));
        }
        authority
            .recheck()
            .map_err(|e| session.fail_close(map_policy(e)))
    }
}
