//! P10.6 — Independent oracle and fixture licence registry.
//!
//! Every fixture used by the computational-geometry test suite records its
//! origin, licence, SHA-256 checksum, and permitted use. This module is the
//! registry data structure plus a validation harness — it establishes the
//! discipline that P10.7 (benchmark corpus) and future golden-vector work will
//! populate with real payloads.
//!
//! ## Licence discipline (the "copyrighted PDFs contribute no copied vectors" rule)
//!
//! - **CC0 / permissive only.** Accepted licences: CC0, MIT, Apache-2.0,
//!   BSD-3-Clause, public-domain, and project-authored. The registry
//!   **rejects copyleft** (GPL / LGPL / AGPL) — `validate_all` and
//!   `assert_no_copyleft` return a typed error on any copyleft record.
//! - **Textbook = invariant reference only.** A `TextbookInvariant` fixture
//!   records the *name and section* of a stated invariant of a public textbook
//!   description (de Berg, Cheong, van Kreveld & Overmars, *Computational
//!   Geometry: Algorithms and Applications*, 3rd ed. is the project's
//!   reference). It contributes **no copied prose, figures, pseudocode,
//!   tables, or data** — only an independently computed answer that satisfies
//!   the stated invariant.
//! - **Invariant-derived fixtures reproduce independently calculated answers.**
//!   An `IndependentlyComputed` fixture's expected answer was produced by an
//!   independent calculation in this repository, not transcribed from any
//!   external source.
//!
//! This is test/cold infrastructure, not a hot path; `Vec`/`String`/`Box` are
//! acceptable here (the zero-heap rule applies to kernel hot paths, not
//! registries). The public seed table is `&'static` — no heap in the table
//! itself.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ───────────────────────────────────────────────────────────────────────────
//  Origin / licence / use enums
// ───────────────────────────────────────────────────────────────────────────

/// Provenance of a test fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureOrigin {
    /// Derived from a stated invariant of a public textbook description
    /// (de Berg et al. 3rd ed.). Contributes no copied prose, figures,
    /// pseudocode, tables, or data — only an independently computed answer
    /// satisfying the named invariant.
    TextbookInvariant {
        /// Short textbook name (e.g. `"de Berg et al. 3rd ed."`).
        name: &'static str,
        /// Section / invariant reference (e.g. `"§1.1 convex hull definition"`).
        section: &'static str,
    },
    /// The expected answer was computed by an independent calculation in this
    /// repository — not transcribed from any external source.
    IndependentlyComputed,
    /// CC0-licensed input data from an external corpus (e.g. CGAL CC0 test
    /// inputs). The checksum is over the input bytes.
    Cc0Input {
        /// Human-readable source label (e.g. `"CGAL CC0 test inputs"`).
        source: &'static str,
        /// Commit hash or release tag of the sourced corpus.
        commit_or_release: &'static str,
    },
    /// A small hand-traced fixture written from first principles.
    HandAuthored,
}

/// Licence kind attached to a fixture. Copyleft variants are deliberately
/// absent from the accepted set; the registry rejects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenceKind {
    /// Creative Commons Zero (public-domain dedication).
    Cc0,
    /// MIT licence.
    Mit,
    /// Apache 2.0 licence.
    Apache2,
    /// BSD 3-Clause licence.
    Bsd3,
    /// Public domain (non-CC0 dedication).
    PublicDomain,
    /// Authored within the QualiaDB project (project licence applies).
    ProjectAuthored,
    /// GPL — **copyleft, rejected by the registry.** Present only so the
    /// validation harness can detect and refuse it.
    Gpl,
    /// LGPL — **copyleft, rejected by the registry.**
    Lgpl,
    /// AGPL — **copyleft, rejected by the registry.**
    Agpl,
}

impl LicenceKind {
    /// Returns `true` if this licence is copyleft and therefore forbidden
    /// in the registry.
    pub fn is_copyleft(&self) -> bool {
        matches!(
            self,
            LicenceKind::Gpl | LicenceKind::Lgpl | LicenceKind::Agpl
        )
    }
}

/// Permitted use of a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsePermission {
    /// Test suites only — not for benchmark publication.
    TestOnly,
    /// Both test suites and benchmark corpora.
    TestAndBenchmark,
    /// Reference invariant only — used to check an invariant holds, not as
    /// a golden vector for byte comparison.
    ReferenceInvariantOnly,
}

// ───────────────────────────────────────────────────────────────────────────
//  FixtureRecord
// ───────────────────────────────────────────────────────────────────────────

/// A single fixture record: origin, licence, checksum, permitted use.
///
/// This is a POD-ish struct. The seed table is `&'static` (a const table);
/// individual records may also be constructed on the heap in tests (e.g. to
/// exercise validation failure paths).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureRecord {
    /// Stable opaque identifier (e.g. `"convex_hull_2/cd500"`).
    pub id: &'static str,
    /// Provenance of the fixture.
    pub origin: FixtureOrigin,
    /// Licence attached to the fixture payload.
    pub licence: LicenceKind,
    /// Permitted use of the fixture.
    pub permitted_use: UsePermission,
    /// SHA-256 (32 bytes) of the fixture payload. For
    /// `TextbookInvariant` / `IndependentlyComputed` / `HandAuthored`
    /// fixtures this is the hash of the canonical-encoded expected answer;
    /// for `Cc0Input` it is the hash of the input bytes. Must be non-zero.
    pub checksum: [u8; 32],
    /// Size of the fixture payload in bytes. May be `0` for
    /// `TextbookInvariant` / `HandAuthored` records whose expected answer is
    /// computed at test time (see `notes`).
    pub payload_size_bytes: u64,
    /// Short human note (e.g. "506-point scan, empty-circumsphere verified
    /// via exact insphere").
    pub notes: &'static str,
}

impl FixtureRecord {
    /// Computes the SHA-256 checksum of a byte payload. Callers store the
    /// result into `FixtureRecord::checksum` when populating the registry
    /// with real payloads (P10.7 and golden-vector work).
    pub fn compute_checksum(payload: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Typed validation error for the fixture registry. Never panics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureRegistryError {
    /// A record carries a copyleft licence (GPL / LGPL / AGPL).
    CopyleftLicence {
        /// The offending fixture id.
        id: &'static str,
        /// The copyleft licence kind.
        kind: LicenceKind,
    },
    /// A record has an all-zero checksum.
    ZeroChecksum {
        /// The offending fixture id.
        id: &'static str,
    },
    /// A fixture id appears more than once in the registry.
    DuplicateId {
        /// The duplicated fixture id.
        id: &'static str,
    },
    /// A `TextbookInvariant` record carries an empty section.
    MissingSection {
        /// The offending fixture id.
        id: &'static str,
    },
    /// A record carries a licence the registry does not recognise as
    /// accepted (defensive — currently unreachable for the defined
    /// `LicenceKind` variants, but kept for forward safety).
    UnknownLicence {
        /// The offending fixture id.
        id: &'static str,
    },
}

impl std::fmt::Display for FixtureRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureRegistryError::CopyleftLicence { id, kind } => {
                write!(
                    f,
                    "fixture `{id}` carries forbidden copyleft licence ({kind:?})"
                )
            }
            FixtureRegistryError::ZeroChecksum { id } => {
                write!(f, "fixture `{id}` has an all-zero checksum")
            }
            FixtureRegistryError::DuplicateId { id } => {
                write!(f, "duplicate fixture id `{id}`")
            }
            FixtureRegistryError::MissingSection { id } => {
                write!(
                    f,
                    "fixture `{id}` is a TextbookInvariant with an empty section"
                )
            }
            FixtureRegistryError::UnknownLicence { id } => {
                write!(f, "fixture `{id}` carries an unrecognised licence")
            }
        }
    }
}

impl std::error::Error for FixtureRegistryError {}

// ───────────────────────────────────────────────────────────────────────────
//  Registry
// ───────────────────────────────────────────────────────────────────────────

/// The fixture registry. Owns a `&'static [FixtureRecord]` const table.
///
/// All accessors are associated functions operating on the seed table; the
/// struct is never instantiated (it is a namespace).
pub struct FixtureRegistry;

impl FixtureRegistry {
    /// Looks up a fixture by id in the seed table.
    pub fn find(id: &str) -> Option<&'static FixtureRecord> {
        SEED_FIXTURES.iter().find(|r| r.id == id)
    }

    /// Iterates over every fixture in the seed table.
    pub fn iter() -> impl Iterator<Item = &'static FixtureRecord> {
        SEED_FIXTURES.iter()
    }

    /// Validates every record in the seed table:
    /// - licence is not copyleft;
    /// - checksum is non-zero;
    /// - id is unique;
    /// - `TextbookInvariant` records carry a non-empty section.
    ///
    /// Returns the record count on success.
    pub fn validate_all() -> Result<usize, FixtureRegistryError> {
        validate_records(SEED_FIXTURES)
    }

    /// Dedicated licence-discipline check: asserts no record in the seed
    /// table carries a copyleft licence. This is the "copyrighted PDFs
    /// contribute no copied vectors" gate.
    pub fn assert_no_copyleft() -> Result<(), FixtureRegistryError> {
        for r in SEED_FIXTURES {
            if r.licence.is_copyleft() {
                return Err(FixtureRegistryError::CopyleftLicence {
                    id: r.id,
                    kind: r.licence,
                });
            }
        }
        Ok(())
    }
}

/// Validates an arbitrary slice of records against the registry invariants.
/// Used by `FixtureRegistry::validate_all` and by tests that construct bad
/// records in-test.
pub fn validate_records(records: &[FixtureRecord]) -> Result<usize, FixtureRegistryError> {
    let mut seen: Vec<&'static str> = Vec::with_capacity(records.len());
    for r in records {
        // Licence discipline: reject copyleft.
        if r.licence.is_copyleft() {
            return Err(FixtureRegistryError::CopyleftLicence {
                id: r.id,
                kind: r.licence,
            });
        }
        // Checksum discipline: reject all-zero.
        if r.checksum.iter().all(|&b| b == 0) {
            return Err(FixtureRegistryError::ZeroChecksum { id: r.id });
        }
        // Textbook invariant discipline: non-empty section.
        if let FixtureOrigin::TextbookInvariant { section, .. } = r.origin {
            if section.is_empty() {
                return Err(FixtureRegistryError::MissingSection { id: r.id });
            }
        }
        // Uniqueness discipline: no duplicate ids.
        if seen.contains(&r.id) {
            return Err(FixtureRegistryError::DuplicateId { id: r.id });
        }
        seen.push(r.id);
    }
    Ok(records.len())
}

// ───────────────────────────────────────────────────────────────────────────
//  Seed registry
// ───────────────────────────────────────────────────────────────────────────

/// A non-zero placeholder checksum used by seed fixtures that do not yet
/// carry a real payload. It is a fixed, deterministic 32-byte value (the
/// SHA-256 of the ASCII bytes `"qualia-cg-fixture-registry/seed-placeholder"`),
/// computed once and embedded as a literal so the const table stays `&'static`
/// with no runtime hashing. Real payloads (P10.7) replace this with the
/// SHA-256 of the actual fixture bytes.
const SEED_PLACEHOLDER_CHECKSUM: [u8; 32] = [
    0xe0, 0xdf, 0xd1, 0x5c, 0xd3, 0x30, 0x32, 0xbe, 0xfa, 0x6b, 0xfb, 0x9e, 0x76, 0x6a, 0xdf, 0xa5,
    0xcd, 0x6b, 0x1a, 0xc8, 0x46, 0xf1, 0x6d, 0x8e, 0x18, 0x5b, 0xd5, 0xda, 0xee, 0xcb, 0x3c, 0x78,
];

/// The seed fixture registry. Placeholder records asserting the registry
/// structure — they do NOT carry actual fixture payloads. Each record's
/// `notes` field explains its placeholder status. P10.7 (benchmark corpus)
/// and future golden-vector work populate this with real payloads and real
/// checksums.
pub const SEED_FIXTURES: &[FixtureRecord] = &[
    // 1. Convex hull 2-D, CD500-style — textbook invariant (de Berg §1.1).
    FixtureRecord {
        id: "convex_hull_2/cd500",
        origin: FixtureOrigin::TextbookInvariant {
            name: "de Berg et al. 3rd ed.",
            section: "§1.1 convex hull definition",
        },
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::TestAndBenchmark,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "506-point scan placeholder; hull computed at test time, \
                checksum is a structural placeholder until P10.7 attaches the \
                canonical-encoded expected hull.",
    },
    // 2. Delaunay empty-circumcircle — textbook invariant (de Berg §9.1).
    FixtureRecord {
        id: "delaunay_2/empty_circumcircle",
        origin: FixtureOrigin::TextbookInvariant {
            name: "de Berg et al. 3rd ed.",
            section: "§9.1 Delaunay triangulation, empty-circumcircle property",
        },
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::TestAndBenchmark,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "Empty-circumcircle verified via exact incircle predicate; \
                payload computed at test time, checksum is a structural \
                placeholder.",
    },
    // 3. Alpha shape 2-D — independently computed.
    FixtureRecord {
        id: "alpha_shape_2/synthetic",
        origin: FixtureOrigin::IndependentlyComputed,
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::TestAndBenchmark,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "Synthetic point set; alpha shape computed independently in \
                this repo, checksum is a structural placeholder until P10.7.",
    },
    // 4. Isosurface sphere — CC0 input from an external corpus.
    FixtureRecord {
        id: "isosurface/sphere_cc0",
        origin: FixtureOrigin::Cc0Input {
            source: "CGAL CC0 test inputs",
            commit_or_release: "release/5.6",
        },
        licence: LicenceKind::Cc0,
        permitted_use: UsePermission::TestAndBenchmark,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "CC0-licensed scalar-field samples; checksum is a structural \
                placeholder — P10.7 will record the SHA-256 of the actual \
                input bytes.",
    },
    // 5. Persistence H0 hand-traced — hand authored.
    FixtureRecord {
        id: "persistence/h0_hand_traced",
        origin: FixtureOrigin::HandAuthored,
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::TestOnly,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "Hand-traced H0 barcode from first principles; computed at \
                test time, checksum is a structural placeholder.",
    },
    // 6. Persistence H1 hand-traced — hand authored.
    FixtureRecord {
        id: "persistence/h1_hand_traced",
        origin: FixtureOrigin::HandAuthored,
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::TestOnly,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "Hand-traced H1 barcode from first principles; computed at \
                test time, checksum is a structural placeholder.",
    },
    // 7. Natural-neighbour linear precision — textbook invariant (de Berg §7.0).
    FixtureRecord {
        id: "natural_neighbour/linear_precision",
        origin: FixtureOrigin::TextbookInvariant {
            name: "de Berg et al. 3rd ed.",
            section: "§7.0 interpolation, linear-reproduction property",
        },
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::ReferenceInvariantOnly,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "Linear-reproduction invariant: natural-neighbour interpolation \
                of a linear field reproduces the field exactly; computed at \
                test time, checksum is a structural placeholder.",
    },
    // 8. Convex hull 3-D — independently computed.
    FixtureRecord {
        id: "convex_hull_3/independent",
        origin: FixtureOrigin::IndependentlyComputed,
        licence: LicenceKind::ProjectAuthored,
        permitted_use: UsePermission::TestAndBenchmark,
        checksum: SEED_PLACEHOLDER_CHECKSUM,
        payload_size_bytes: 0,
        notes: "3-D convex hull over an independently generated point cloud; \
                checksum is a structural placeholder until P10.7.",
    },
];

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `validate_all` passes on the seed table and returns the count.
    #[test]
    fn seed_registry_validates() {
        let count = FixtureRegistry::validate_all().expect("seed registry must validate");
        assert_eq!(count, SEED_FIXTURES.len());
        assert!(count >= 6, "seed registry should cover each origin variant");
    }

    /// `assert_no_copyleft` passes on the seed table.
    #[test]
    fn seed_registry_has_no_copyleft() {
        FixtureRegistry::assert_no_copyleft().expect("seed registry must have no copyleft");
    }

    /// A GPL record is rejected by `validate_all`.
    #[test]
    fn gpl_record_rejected() {
        let bad = FixtureRecord {
            id: "bad/gpl",
            origin: FixtureOrigin::IndependentlyComputed,
            licence: LicenceKind::Gpl,
            permitted_use: UsePermission::TestOnly,
            checksum: SEED_PLACEHOLDER_CHECKSUM,
            payload_size_bytes: 0,
            notes: "intentionally copyleft for the rejection test",
        };
        let err = validate_records(&[bad.clone()]).expect_err("GPL must be rejected");
        assert_eq!(
            err,
            FixtureRegistryError::CopyleftLicence {
                id: "bad/gpl",
                kind: LicenceKind::Gpl,
            }
        );
    }

    /// An LGPL record is rejected by `validate_all`.
    #[test]
    fn lgpl_record_rejected() {
        let bad = FixtureRecord {
            id: "bad/lgpl",
            origin: FixtureOrigin::IndependentlyComputed,
            licence: LicenceKind::Lgpl,
            permitted_use: UsePermission::TestOnly,
            checksum: SEED_PLACEHOLDER_CHECKSUM,
            payload_size_bytes: 0,
            notes: "intentionally copyleft for the rejection test",
        };
        let err = validate_records(&[bad]).expect_err("LGPL must be rejected");
        assert_eq!(
            err,
            FixtureRegistryError::CopyleftLicence {
                id: "bad/lgpl",
                kind: LicenceKind::Lgpl,
            }
        );
    }

    /// A zero-checksum record is rejected.
    #[test]
    fn zero_checksum_rejected() {
        let bad = FixtureRecord {
            id: "bad/zero_checksum",
            origin: FixtureOrigin::IndependentlyComputed,
            licence: LicenceKind::ProjectAuthored,
            permitted_use: UsePermission::TestOnly,
            checksum: [0u8; 32],
            payload_size_bytes: 0,
            notes: "intentionally zero checksum for the rejection test",
        };
        let err = validate_records(&[bad]).expect_err("zero checksum must be rejected");
        assert_eq!(
            err,
            FixtureRegistryError::ZeroChecksum {
                id: "bad/zero_checksum"
            }
        );
    }

    /// A duplicate-id pair is rejected.
    #[test]
    fn duplicate_id_rejected() {
        let a = FixtureRecord {
            id: "dup/id",
            origin: FixtureOrigin::HandAuthored,
            licence: LicenceKind::ProjectAuthored,
            permitted_use: UsePermission::TestOnly,
            checksum: SEED_PLACEHOLDER_CHECKSUM,
            payload_size_bytes: 0,
            notes: "first copy",
        };
        let b = FixtureRecord {
            id: "dup/id",
            origin: FixtureOrigin::IndependentlyComputed,
            licence: LicenceKind::Mit,
            permitted_use: UsePermission::TestAndBenchmark,
            checksum: SEED_PLACEHOLDER_CHECKSUM,
            payload_size_bytes: 0,
            notes: "second copy with the same id",
        };
        let err = validate_records(&[a, b]).expect_err("duplicate id must be rejected");
        assert_eq!(err, FixtureRegistryError::DuplicateId { id: "dup/id" });
    }

    /// A `TextbookInvariant` record with an empty section is rejected.
    #[test]
    fn missing_section_rejected() {
        let bad = FixtureRecord {
            id: "bad/empty_section",
            origin: FixtureOrigin::TextbookInvariant {
                name: "de Berg et al. 3rd ed.",
                section: "",
            },
            licence: LicenceKind::ProjectAuthored,
            permitted_use: UsePermission::ReferenceInvariantOnly,
            checksum: SEED_PLACEHOLDER_CHECKSUM,
            payload_size_bytes: 0,
            notes: "intentionally empty section for the rejection test",
        };
        let err = validate_records(&[bad]).expect_err("empty section must be rejected");
        assert_eq!(
            err,
            FixtureRegistryError::MissingSection {
                id: "bad/empty_section"
            }
        );
    }

    /// `find` returns the right record for a known id and `None` for unknown.
    #[test]
    fn find_returns_known_and_none_for_unknown() {
        let known = FixtureRegistry::find("convex_hull_2/cd500").expect("known id must resolve");
        assert_eq!(known.id, "convex_hull_2/cd500");
        assert!(known.licence == LicenceKind::ProjectAuthored);

        let also_known = FixtureRegistry::find("isosurface/sphere_cc0");
        assert!(also_known.is_some(), "isosurface seed id must resolve");

        let unknown = FixtureRegistry::find("does/not/exist");
        assert!(unknown.is_none(), "unknown id must return None");
    }

    /// `iter` yields exactly the seed records.
    #[test]
    fn iter_yields_all_seed_records() {
        let collected: Vec<&FixtureRecord> = FixtureRegistry::iter().collect();
        assert_eq!(collected.len(), SEED_FIXTURES.len());
        for (a, b) in collected.iter().zip(SEED_FIXTURES.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    /// Round-trip serde on a `FixtureRecord` via `serde_json`.
    ///
    /// `FixtureRecord` carries `&'static str` fields, so the JSON input must
    /// outlive the deserialised borrow with a `'static` lifetime. We leak the
    /// serialised string (a standard test technique — the memory lives for the
    /// test process's duration) so `serde_json` can borrow `&'static str` from it.
    #[test]
    fn fixture_record_serde_roundtrip() {
        let record = FixtureRegistry::find("delaunay_2/empty_circumcircle").unwrap();
        let json = serde_json::to_string(record).expect("serialize");
        let json_static: &'static str = Box::leak(json.into_boxed_str());
        let back: FixtureRecord = serde_json::from_str(json_static).expect("deserialize");
        assert_eq!(record, &back);
    }

    /// Round-trip serde on a `Cc0Input`-origin record (exercises the
    /// `FixtureOrigin` enum serialisation with payload fields). Same leak
    /// technique as above for the `&'static str` fields.
    #[test]
    fn cc0_input_record_serde_roundtrip() {
        let record = FixtureRegistry::find("isosurface/sphere_cc0").unwrap();
        let json = serde_json::to_string(record).expect("serialize");
        let json_static: &'static str = Box::leak(json.into_boxed_str());
        let back: FixtureRecord = serde_json::from_str(json_static).expect("deserialize");
        assert_eq!(record, &back);
    }

    /// `compute_checksum` produces a non-zero, deterministic 32-byte digest.
    #[test]
    fn compute_checksum_is_deterministic_and_nonzero() {
        let a = FixtureRecord::compute_checksum(b"hello qualia");
        let b = FixtureRecord::compute_checksum(b"hello qualia");
        assert_eq!(a, b, "checksum must be deterministic");
        assert!(a.iter().any(|&x| x != 0), "checksum must be non-zero");
        assert_eq!(a.len(), 32, "checksum must be 32 bytes");

        let c = FixtureRecord::compute_checksum(b"different payload");
        assert_ne!(a, c, "different payloads must produce different checksums");
    }

    /// `is_copyleft` correctly classifies licences.
    #[test]
    fn licence_kind_copyleft_classification() {
        assert!(!LicenceKind::Cc0.is_copyleft());
        assert!(!LicenceKind::Mit.is_copyleft());
        assert!(!LicenceKind::Apache2.is_copyleft());
        assert!(!LicenceKind::Bsd3.is_copyleft());
        assert!(!LicenceKind::PublicDomain.is_copyleft());
        assert!(!LicenceKind::ProjectAuthored.is_copyleft());
        assert!(LicenceKind::Gpl.is_copyleft());
        assert!(LicenceKind::Lgpl.is_copyleft());
        assert!(LicenceKind::Agpl.is_copyleft());
    }

    /// Every `FixtureOrigin` variant is represented in the seed table.
    #[test]
    fn seed_registry_covers_every_origin_variant() {
        let mut has_textbook = false;
        let mut has_independent = false;
        let mut has_cc0 = false;
        let mut has_hand = false;
        for r in SEED_FIXTURES {
            match r.origin {
                FixtureOrigin::TextbookInvariant { .. } => has_textbook = true,
                FixtureOrigin::IndependentlyComputed => has_independent = true,
                FixtureOrigin::Cc0Input { .. } => has_cc0 = true,
                FixtureOrigin::HandAuthored => has_hand = true,
            }
        }
        assert!(has_textbook, "seed must include a TextbookInvariant record");
        assert!(
            has_independent,
            "seed must include an IndependentlyComputed record"
        );
        assert!(has_cc0, "seed must include a Cc0Input record");
        assert!(has_hand, "seed must include a HandAuthored record");
    }
}
