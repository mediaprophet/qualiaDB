//! E21.3 — independent-review PACKET. Prepared in-tree; review is not executed.

/// Independent cryptographic composition / hostile-environment review has not
/// been commissioned or signed.
pub fn independent_review_executed() -> bool {
    false
}

/// The checklist packet exists and is filled with unsigned prepared items.
pub fn review_packet_prepared() -> bool {
    true
}

/// An item is ready for a human reviewer. It is never a signed attestation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewStatus {
    PreparedAwaitingReviewer = 1,
}

/// One composition / hostile-environment checklist row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewItem {
    pub id: &'static str,
    pub topic: &'static str,
    pub status: ReviewStatus,
    /// Always false: this packet does not claim a reviewer signature.
    pub signed_by_reviewer: bool,
}

impl ReviewItem {
    const fn prepared(id: &'static str, topic: &'static str) -> Self {
        Self {
            id,
            topic,
            status: ReviewStatus::PreparedAwaitingReviewer,
            signed_by_reviewer: false,
        }
    }
}

/// Composition review packet. Not a completed independent review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndependentReviewPacket {
    pub handshake_transcript: ReviewItem,
    pub ml_dsa_identity: ReviewItem,
    pub protection_labels: ReviewItem,
    pub session_key_confirmation: ReviewItem,
    pub hostile_environment: ReviewItem,
}

impl IndependentReviewPacket {
    pub const fn prepared() -> Self {
        Self {
            handshake_transcript: ReviewItem::prepared(
                "e21.3/handshake",
                "QPR handshake transcript binding and identity mix",
            ),
            ml_dsa_identity: ReviewItem::prepared(
                "e21.3/ml-dsa",
                "ML-DSA-65 identity signatures; no length-inferred algorithm",
            ),
            protection_labels: ReviewItem::prepared(
                "e21.3/labels",
                "Protection labels, join, declassify, and egress markings",
            ),
            session_key_confirmation: ReviewItem::prepared(
                "e21.3/session-keys",
                "Installed session keys and confirmation; no unkeyed finished",
            ),
            hostile_environment: ReviewItem::prepared(
                "e21.3/hostile-env",
                "Hostile-environment composition (partition, replay, downgrade)",
            ),
        }
    }

    pub const fn items(self) -> [ReviewItem; 5] {
        [
            self.handshake_transcript,
            self.ml_dsa_identity,
            self.protection_labels,
            self.session_key_confirmation,
            self.hostile_environment,
        ]
    }

    /// No item is a signed review finding.
    pub fn any_signed(self) -> bool {
        let items = self.items();
        let mut i = 0usize;
        while i < items.len() {
            if items[i].signed_by_reviewer {
                return true;
            }
            i += 1;
        }
        false
    }
}

/// Construct the prepared packet. Does not execute or attest a review.
pub fn prepared_review_packet() -> IndependentReviewPacket {
    IndependentReviewPacket::prepared()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::harness::claims::independent_review_commissioned;
    use crate::net::qdnf::harness::limitations::INDEPENDENT_CRYPTO_REVIEW;

    #[test]
    fn review_packet_is_prepared_and_unsigned() {
        assert!(review_packet_prepared());
        assert!(!independent_review_executed());
        assert!(!independent_review_commissioned());
        let packet = prepared_review_packet();
        assert!(!packet.any_signed());
        let items = packet.items();
        assert_eq!(items.len(), 5);
        let mut i = 0usize;
        while i < items.len() {
            assert!(!items[i].signed_by_reviewer);
            assert_eq!(items[i].status, ReviewStatus::PreparedAwaitingReviewer);
            assert!(!items[i].id.is_empty());
            i += 1;
        }
        assert!(INDEPENDENT_CRYPTO_REVIEW.contains("not executed"));
        assert!(packet.handshake_transcript.topic.contains("handshake"));
        assert!(packet.ml_dsa_identity.topic.contains("ML-DSA"));
        assert!(packet.protection_labels.topic.contains("label"));
        assert!(packet.session_key_confirmation.topic.contains("session"));
    }
}
