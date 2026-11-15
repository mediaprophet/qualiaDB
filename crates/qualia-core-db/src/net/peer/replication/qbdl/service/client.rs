//! In-process client. Wire Grant is protocol state, not local authority.
use super::session::{
    associate, associate_until, check_identity, encode_open_body, parse_selection, reason_body,
    ServiceIo, DEADLINE,
};
use super::{from_proof, Error};
use crate::bundle::stream::descriptor::CandidateIdentity;
use crate::bundle::stream::proof::{
    decode_proof, verify_leaf, Representation, TreeSpec, MAX_SIBLINGS,
};
use crate::bundle::stream::selection::{SelectedPlainEntry, SelectionError};
use crate::net::peer::replication::qbdl::wire::{Kind, MAX_LEAF_RECORD_BYTES};
use crate::net::qdnf::types::StrongDigest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenTicket {
    pub transfer_id: [u8; 16],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafTicket {
    pub transfer_id: [u8; 16],
    pub leaf: u64,
    pub credit_serial: u64,
    pub credit_bytes: u64,
}

pub struct PlainClient<'p> {
    selected: SelectedPlainEntry<'p>,
    policy_generation: u64,
    page: u32,
}

impl<'p> PlainClient<'p> {
    pub fn bind(
        selected: SelectedPlainEntry<'p>,
        policy_generation: u64,
        page: u32,
    ) -> Result<Self, Error> {
        let spec = spec_of(selected)?;
        if spec.representation() != Representation::Plaintext || spec.key_epoch() != 0 {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            selected,
            policy_generation,
            page,
        })
    }

    pub fn identity(&self) -> CandidateIdentity {
        self.selected.candidate_identity()
    }

    pub fn send_open<S: ServiceIo>(&self, session: &mut S) -> Result<OpenTicket, Error> {
        self.send_open_until(session, DEADLINE)
    }

    pub fn send_open_until<S: ServiceIo>(
        &self,
        session: &mut S,
        deadline: u64,
    ) -> Result<OpenTicket, Error> {
        let transfer_id = session.next_transfer_id()?;
        let mut body = [0u8; 72];
        let n = encode_open_body(self.selected.ordinal(), self.page, &mut body)?;
        let header = associate_until(
            Kind::Open,
            self.identity(),
            self.policy_generation,
            transfer_id,
            self.selected.entry_id(),
            0,
            0,
            0,
            0,
            deadline,
        );
        session.send_message(&header, &body[..n])?;
        Ok(OpenTicket { transfer_id })
    }

    pub fn recv_grant_send_credit_request<S: ServiceIo>(
        &self,
        session: &mut S,
        open: &OpenTicket,
        leaf: u64,
    ) -> Result<LeafTicket, Error> {
        self.recv_grant_send_credit_request_until(session, open, leaf, DEADLINE)
    }

    pub fn recv_grant_send_credit_request_until<S: ServiceIo>(
        &self,
        session: &mut S,
        open: &OpenTicket,
        leaf: u64,
        deadline: u64,
    ) -> Result<LeafTicket, Error> {
        let (header, body, n) = session.recv_control()?;
        if header.kind != Kind::Grant {
            return Err(session.fail_close(Error::State));
        }
        if let Err(e) = check_identity(&header, self.identity(), self.policy_generation) {
            return Err(session.fail_close(e));
        }
        if header.transfer_id != open.transfer_id
            || header.entry != self.selected.entry_id()
            || header.leaf != 0
            || header.count != 0
            || n != 80
        {
            return Err(session.fail_close(Error::State));
        }
        let phase = u16::from_be_bytes([body[0], body[1]]);
        let status = u16::from_be_bytes([body[2], body[3]]);
        if phase != 2 || status != 0 {
            return Err(session.fail_close(Error::Unsupported));
        }
        let (ordinal, page) = match parse_selection(&body, 72) {
            Ok(v) => v,
            Err(e) => return Err(session.fail_close(e)),
        };
        if ordinal != self.selected.ordinal() || page != self.page {
            return Err(session.fail_close(Error::Identity));
        }
        let spec = match spec_of(self.selected) {
            Ok(s) => s,
            Err(e) => return Err(session.fail_close(e)),
        };
        if leaf >= spec.leaf_count() {
            return Err(session.fail_close(Error::Limit));
        }
        let transfer_id = open.transfer_id;
        let credit_serial = session.next_credit_serial()?;
        let credit_bytes = MAX_LEAF_RECORD_BYTES as u64;
        let credit = associate_until(
            Kind::Credit,
            self.identity(),
            self.policy_generation,
            transfer_id,
            self.selected.entry_id(),
            leaf,
            1,
            credit_serial,
            credit_bytes,
            deadline,
        );
        session.send_message(&credit, &[])?;
        let request = associate_until(
            Kind::RequestLeaves,
            self.identity(),
            self.policy_generation,
            transfer_id,
            self.selected.entry_id(),
            leaf,
            1,
            credit_serial,
            credit_bytes,
            deadline,
        );
        session.send_message(&request, &[])?;
        Ok(LeafTicket {
            transfer_id,
            leaf,
            credit_serial,
            credit_bytes,
        })
    }

    pub fn recv_leaf<S: ServiceIo>(
        &self,
        session: &mut S,
        ticket: &LeafTicket,
        payload_out: &mut [u8],
    ) -> Result<usize, Error> {
        let mut payload = [0u8; 16384];
        let mut proof = [0u8; 1 + MAX_SIBLINGS * 48];
        let (header, p, q) = session.recv_leaf(&mut payload, &mut proof)?;
        if let Err(e) = check_identity(&header, self.identity(), self.policy_generation) {
            return Err(session.fail_close(e));
        }
        if header.kind != Kind::LeafData
            || header.transfer_id != ticket.transfer_id
            || header.entry != self.selected.entry_id()
            || header.leaf != ticket.leaf
            || header.count != 1
            || header.credit_serial != ticket.credit_serial
            || header.credit_bytes != ticket.credit_bytes
        {
            return Err(session.fail_close(Error::Credit));
        }
        let spec = match spec_of(self.selected) {
            Ok(s) => s,
            Err(e) => return Err(session.fail_close(e)),
        };
        let expect = match spec.logical_leaf_length(ticket.leaf) {
            Ok(n) => n,
            Err(e) => return Err(session.fail_close(from_proof(e))),
        };
        if p != expect {
            return Err(session.fail_close(Error::Identity));
        }
        if payload_out.len() < p {
            return Err(session.fail_close(Error::Capacity));
        }
        let mut decoded = [StrongDigest::ZERO; MAX_SIBLINGS];
        let path = match decode_proof(&proof[..q], &mut decoded) {
            Ok(v) => v,
            Err(e) => return Err(session.fail_close(from_proof(e))),
        };
        if let Err(e) = verify_leaf(
            &spec,
            self.selected.root(),
            ticket.leaf,
            &payload[..p],
            path,
        ) {
            return Err(session.fail_close(from_proof(e)));
        }
        payload_out[..p].copy_from_slice(&payload[..p]);
        Ok(p)
    }

    pub fn send_cancel<S: ServiceIo>(
        &self,
        session: &mut S,
        open: &OpenTicket,
    ) -> Result<(), Error> {
        let header = associate(
            Kind::Cancel,
            self.identity(),
            self.policy_generation,
            open.transfer_id,
            self.selected.entry_id(),
            0,
            0,
            0,
            0,
        );
        session.send_message(&header, &reason_body(1))
    }

    pub fn send_close<S: ServiceIo>(&self, session: &mut S) -> Result<(), Error> {
        let transfer_id = session.next_transfer_id()?;
        let header = associate(
            Kind::Close,
            self.identity(),
            self.policy_generation,
            transfer_id,
            0,
            0,
            0,
            0,
            0,
        );
        session.send_message(&header, &reason_body(0))
    }
}

fn spec_of(selected: SelectedPlainEntry<'_>) -> Result<TreeSpec, Error> {
    selected.tree_spec().map_err(|e| match e {
        SelectionError::Proof(p) => from_proof(p),
        SelectionError::Bounds => Error::Limit,
        _ => Error::Identity,
    })
}
