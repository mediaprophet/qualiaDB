//! Cryptographic material binding (Schema 105 `MaterialBinding`), coordinate transform admission
//! (Schema 106 `CoordinateTransform`), and exact signed-`i128` rational clock & half-open
//! `[from, until)` validity verification (C02-C03, C02-C04, C02-C06).

use crate::q42::m64_matter::{
    BankedConstituentLut, M64Error, M64ReadError, M64ReadLimits, M64Reader, M64PayloadInterval,
    Q42ConstituentLutEntry,
};

/// Maximum matrix orthogonality and determinant residual for Schema 106 proper rigid transforms (C02-C04).
pub const TRANSFORM_ORTHOGONALITY_TOLERANCE: f64 = 1e-12;

/// Errors arising during cryptographic material binding, frame transform, or rational clock verification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialBindingError {
    NonFiniteTransformComponent,
    InvalidClockParameters,
    InvalidInstantSubtick,
    ClockOriginMismatch {
        expected_clock_id: u32,
        actual_clock_id: u32,
    },
    RationalArithmeticOverflow,
    InvalidValidityIntervalOrder,
    ValidityIntervalExpiredOrNotYetActive,
    TransformFrameMismatch {
        expected_source_frame: u32,
        actual_source_frame: u32,
        expected_target_frame: u32,
        actual_target_frame: u32,
    },
    TransformProjectiveRowInvalid,
    ImproperReflectionRejected {
        determinant: f64,
    },
    NonOrthogonalOrShearedTransformRejected {
        orthogonality_residual: f64,
        determinant: f64,
    },
    UnsupportedGeneralAffineInInitialSolver,
    UnsupportedMovingFrameInInitialSolver,
    ContainerDigestMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    MaterialTableDigestMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    TableSchemaRevisionMismatch {
        expected: u16,
        actual: u16,
    },
    TableEpochMismatch {
        expected_epoch: u16,
        actual_epoch: u16,
    },
    MaterialIndexOutOfRange {
        index: usize,
        table_len: usize,
    },
    LutEntryCrcInvalid(M64Error),
    M64ContainerStructuralError(M64ReadError),
    CanonicalMaterialHashMismatch {
        expected: u64,
        actual: u64,
    },
    ProvenanceDidHashMismatch {
        expected: u64,
        actual: u64,
    },
    SpatialKeyAloneInsufficient,
}

#[inline]
fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[inline]
fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Shared Schema 1 (`Clock`) rational clock definition per C02-C03.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalClock {
    pub clock_id: u32,
    pub scale: u32,
    pub epoch_seconds: i64,
    pub epoch_nanoseconds: u32,
    pub ticks_per_second_num: u64,
    pub ticks_per_second_den: u64,
}

impl RationalClock {
    pub fn validate(&self) -> Result<(), MaterialBindingError> {
        if self.clock_id == 0
            || self.scale > 2
            || self.epoch_nanoseconds >= 1_000_000_000
            || self.ticks_per_second_num == 0
            || self.ticks_per_second_den == 0
            || self.ticks_per_second_num > 1_000_000_000_000_000
            || self.ticks_per_second_den > 1_000_000_000_000
            || gcd_u64(self.ticks_per_second_num, self.ticks_per_second_den) != 1
        {
            return Err(MaterialBindingError::InvalidClockParameters);
        }
        Ok(())
    }

    /// Verify that two clock descriptors share identical origin, scale, and rate.
    /// Equal rates and ticks alone do not establish simultaneity if `clock_id` or `epoch` differ (C02-C03).
    pub fn verify_same_origin(&self, other: &Self) -> Result<(), MaterialBindingError> {
        self.validate()?;
        other.validate()?;
        if self != other {
            return Err(MaterialBindingError::ClockOriginMismatch {
                expected_clock_id: self.clock_id,
                actual_clock_id: other.clock_id,
            });
        }
        Ok(())
    }
}

/// Shared Schema 5 (`Instant`) rational tick + subtick timestamp per C02-C03.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalInstant {
    pub clock_id: u32,
    pub tick: i64,
    pub subtick_num: u64,
    pub subtick_den: u64,
}

impl RationalInstant {
    pub fn new(
        clock_id: u32,
        tick: i64,
        subtick_num: u64,
        subtick_den: u64,
    ) -> Result<Self, MaterialBindingError> {
        let s = Self {
            clock_id,
            tick,
            subtick_num,
            subtick_den,
        };
        s.validate()?;
        Ok(s)
    }

    pub fn from_integral_tick(clock_id: u32, tick: i64) -> Result<Self, MaterialBindingError> {
        Self::new(clock_id, tick, 0, 1)
    }

    pub fn validate(&self) -> Result<(), MaterialBindingError> {
        if self.clock_id == 0
            || self.subtick_den == 0
            || self.subtick_den > 1_000_000_000_000
            || self.subtick_num >= self.subtick_den
            || gcd_u64(self.subtick_num, self.subtick_den) != 1
        {
            return Err(MaterialBindingError::InvalidInstantSubtick);
        }
        Ok(())
    }

    /// Exact checked comparison of two `RationalInstant` values on the same clock.
    pub fn checked_cmp(&self, other: &Self) -> Result<std::cmp::Ordering, MaterialBindingError> {
        self.validate()?;
        other.validate()?;
        if self.clock_id != other.clock_id {
            return Err(MaterialBindingError::ClockOriginMismatch {
                expected_clock_id: self.clock_id,
                actual_clock_id: other.clock_id,
            });
        }
        match self.tick.cmp(&other.tick) {
            std::cmp::Ordering::Equal => {
                let lhs = (self.subtick_num as u128)
                    .checked_mul(other.subtick_den as u128)
                    .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;
                let rhs = (other.subtick_num as u128)
                    .checked_mul(self.subtick_den as u128)
                    .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;
                Ok(lhs.cmp(&rhs))
            }
            ord => Ok(ord),
        }
    }

    /// Compute exact elapsed seconds `(numerator, denominator)` from `clock` epoch for sample `sample_index`
    /// at sample rate `(rate_num, rate_den)` Hz starting at `self`:
    /// `t_i = start_tick * (tick_den / tick_num) + (subtick_num / subtick_den) * (tick_den / tick_num) + i * (rate_den / rate_num)`.
    pub fn sample_offset_from_clock_epoch_seconds(
        &self,
        clock: &RationalClock,
        sample_index: u64,
        rate_num: u64,
        rate_den: u64,
    ) -> Result<(i128, u128), MaterialBindingError> {
        clock.validate()?;
        self.validate()?;
        if self.clock_id != clock.clock_id {
            return Err(MaterialBindingError::ClockOriginMismatch {
                expected_clock_id: clock.clock_id,
                actual_clock_id: self.clock_id,
            });
        }
        if rate_num == 0 || rate_den == 0 || gcd_u64(rate_num, rate_den) != 1 {
            return Err(MaterialBindingError::InvalidClockParameters);
        }
        // Total ticks in subtick units: (tick * subtick_den + subtick_num) / subtick_den
        let sub_den = self.subtick_den as i128;
        let total_subticks = (self.tick as i128)
            .checked_mul(sub_den)
            .and_then(|v| v.checked_add(self.subtick_num as i128))
            .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;

        // Term 1 (seconds): total_subticks * tick_den / (subtick_den * tick_num)
        let t1_num = total_subticks
            .checked_mul(clock.ticks_per_second_den as i128)
            .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;
        let t1_den = (self.subtick_den as u128)
            .checked_mul(clock.ticks_per_second_num as u128)
            .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;

        // Term 2 (seconds): sample_index * rate_den / rate_num
        let t2_num = (sample_index as i128)
            .checked_mul(rate_den as i128)
            .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;
        let t2_den = rate_num as u128;

        let g_den = gcd_u128(t1_den, t2_den);
        let common_den = (t1_den / g_den)
            .checked_mul(t2_den)
            .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;
        let num = t1_num
            .checked_mul((t2_den / g_den) as i128)
            .and_then(|a| {
                t2_num
                    .checked_mul((t1_den / g_den) as i128)
                    .and_then(|b| a.checked_add(b))
            })
            .ok_or(MaterialBindingError::RationalArithmeticOverflow)?;

        let g = gcd_u128(num.unsigned_abs(), common_den);
        Ok((num / (g as i128), common_den / g))
    }
}

/// Half-open validity interval `[from, until)` per C02-C03 (used by Schema 106, Schema 107, Schema 202).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HalfOpenValidityInterval {
    pub clock: RationalClock,
    pub from: RationalInstant,
    pub until: RationalInstant,
}

impl HalfOpenValidityInterval {
    pub fn new(
        clock: RationalClock,
        from: RationalInstant,
        until: RationalInstant,
    ) -> Result<Self, MaterialBindingError> {
        clock.validate()?;
        if from.clock_id != clock.clock_id {
            return Err(MaterialBindingError::ClockOriginMismatch {
                expected_clock_id: clock.clock_id,
                actual_clock_id: from.clock_id,
            });
        }
        if until.clock_id != clock.clock_id {
            return Err(MaterialBindingError::ClockOriginMismatch {
                expected_clock_id: clock.clock_id,
                actual_clock_id: until.clock_id,
            });
        }
        if from.checked_cmp(&until)? != std::cmp::Ordering::Less {
            return Err(MaterialBindingError::InvalidValidityIntervalOrder);
        }
        Ok(Self { clock, from, until })
    }

    /// Check whether `instant` lies strictly within `[from, until)` (`from <= instant < until`).
    /// Rejects at the exact `until` endpoint with zero tolerance per C02-C03.
    pub fn verify_contains(
        &self,
        instant_clock: &RationalClock,
        instant: &RationalInstant,
    ) -> Result<(), MaterialBindingError> {
        self.clock.verify_same_origin(instant_clock)?;
        let cmp_from = instant.checked_cmp(&self.from)?;
        let cmp_until = instant.checked_cmp(&self.until)?;
        if cmp_from == std::cmp::Ordering::Less || cmp_until != std::cmp::Ordering::Less {
            return Err(MaterialBindingError::ValidityIntervalExpiredOrNotYetActive);
        }
        Ok(())
    }
}

/// Classification of Schema 106 (`CoordinateTransform`) per C02-C04 / EMF-A07.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateTransformKind {
    /// Stationary proper rigid Euclidean isometry ($R R^T = I, \det(R) = +1$).
    ProperRigid,
    /// General invertible affine transformation (includes scaling, shear, or improper reflection).
    GeneralAffine,
    /// Time-dependent moving coordinate frame.
    TimeDependentMoving,
}

/// Schema 106 (`CoordinateTransform`) $4\times 4$ homogeneous matrix mapping local coordinates
/// in `source_frame_id` to `target_frame_id` over `validity`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoordinateTransform4x4 {
    pub source_frame_id: u32,
    pub target_frame_id: u32,
    pub kind: CoordinateTransformKind,
    /// Row-major $4\times 4$ homogeneous transform matrix.
    pub matrix_row_major: [[f64; 4]; 4],
    pub validity: HalfOpenValidityInterval,
}

impl CoordinateTransform4x4 {
    /// Admit a `CoordinateTransform4x4` for an initial stationary physical solver (C02-C04 / EMF-A07).
    /// Rejects `GeneralAffine`, `TimeDependentMoving`, improper reflections ($\det(R) < 0$),
    /// sheared/scaled upper-left $3\times 3$ blocks, and non-`[0,0,0,1]` bottom rows.
    pub fn admit_for_initial_physical_solver(
        &self,
        expected_source_frame: u32,
        expected_target_frame: u32,
        eval_clock: &RationalClock,
        eval_instant: &RationalInstant,
    ) -> Result<([[f64; 3]; 3], [f64; 3]), MaterialBindingError> {
        if self.source_frame_id == 0
            || self.target_frame_id == 0
            || self.source_frame_id != expected_source_frame
            || self.target_frame_id != expected_target_frame
        {
            return Err(MaterialBindingError::TransformFrameMismatch {
                expected_source_frame,
                actual_source_frame: self.source_frame_id,
                expected_target_frame,
                actual_target_frame: self.target_frame_id,
            });
        }
        self.validity.verify_contains(eval_clock, eval_instant)?;

        for row in &self.matrix_row_major {
            if row.iter().any(|v| !v.is_finite()) {
                return Err(MaterialBindingError::NonFiniteTransformComponent);
            }
        }
        match self.kind {
            CoordinateTransformKind::ProperRigid => {}
            CoordinateTransformKind::GeneralAffine => {
                return Err(MaterialBindingError::UnsupportedGeneralAffineInInitialSolver);
            }
            CoordinateTransformKind::TimeDependentMoving => {
                return Err(MaterialBindingError::UnsupportedMovingFrameInInitialSolver);
            }
        }

        let b = &self.matrix_row_major[3];
        if b[0].abs() > TRANSFORM_ORTHOGONALITY_TOLERANCE
            || b[1].abs() > TRANSFORM_ORTHOGONALITY_TOLERANCE
            || b[2].abs() > TRANSFORM_ORTHOGONALITY_TOLERANCE
            || (b[3] - 1.0).abs() > TRANSFORM_ORTHOGONALITY_TOLERANCE
        {
            return Err(MaterialBindingError::TransformProjectiveRowInvalid);
        }

        let r = [
            [
                self.matrix_row_major[0][0],
                self.matrix_row_major[0][1],
                self.matrix_row_major[0][2],
            ],
            [
                self.matrix_row_major[1][0],
                self.matrix_row_major[1][1],
                self.matrix_row_major[1][2],
            ],
            [
                self.matrix_row_major[2][0],
                self.matrix_row_major[2][1],
                self.matrix_row_major[2][2],
            ],
        ];
        let translation = [
            self.matrix_row_major[0][3],
            self.matrix_row_major[1][3],
            self.matrix_row_major[2][3],
        ];

        let mut max_orth_err = 0.0f64;
        for i in 0..3 {
            for j in 0..3 {
                let dot = r[i][0] * r[j][0] + r[i][1] * r[j][1] + r[i][2] * r[j][2];
                let target = if i == j { 1.0 } else { 0.0 };
                max_orth_err = max_orth_err.max((dot - target).abs());
            }
        }
        let det = r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
            - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0]);

        if det < 0.0 && (det + 1.0).abs() <= 1e-6 && max_orth_err <= TRANSFORM_ORTHOGONALITY_TOLERANCE {
            return Err(MaterialBindingError::ImproperReflectionRejected { determinant: det });
        }
        if max_orth_err > TRANSFORM_ORTHOGONALITY_TOLERANCE
            || (det - 1.0).abs() > TRANSFORM_ORTHOGONALITY_TOLERANCE
        {
            return Err(
                MaterialBindingError::NonOrthogonalOrShearedTransformRejected {
                    orthogonality_residual: max_orth_err,
                    determinant: det,
                },
            );
        }

        Ok((r, translation))
    }
}

/// Complete material entry identity descriptor inside Schema 105 (`MaterialBinding`) per C02-C06.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialEntryIdentity {
    /// Zero-based slot index into the 256-entry `BankedConstituentLut` (or bounded LUT slice).
    pub lut_slot_index: u8,
    /// Full 64-bit canonical FNV-1a material IRI hash (`Q42ConstituentLutEntry::canonical_q42_material_hash`).
    pub canonical_q42_material_hash: u64,
    /// Full 64-bit custodian/provenance DID hash (`Q42ConstituentLutEntry::provenance_did_hash`).
    pub provenance_did_hash: u64,
    /// Optional spatial Morton/Hilbert key associated with the chunk; never sufficient alone!
    pub spatial_hilbert_key: u64,
}

/// Schema 105 (`MaterialBinding`) cryptographic binding between an EMF/QAC field stream and an
/// `.m64` container + `BankedConstituentLut` material table.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialBinding {
    /// Full 256-bit BLAKE3 digest of the `.m64` container bytes.
    pub container_blake3: [u8; 32],
    /// Full 256-bit BLAKE3 digest of the 16-KB `BankedConstituentLut` (`as_gpu_bytes()`).
    pub material_table_blake3: [u8; 32],
    /// Schema revision of the material table (must be 1).
    pub table_schema_revision: u16,
    /// Active Bank 1 recipe epoch ID (`active_bank_1_epoch_id`).
    pub table_epoch_id: u16,
    /// Coordinate frame ID of the `.m64` matter container.
    pub material_frame_id: u32,
    /// Coordinate frame ID of the consuming EMF/QAC field stream.
    pub field_frame_id: u32,
    /// Explicit coordinate frame transform from `material_frame_id` to `field_frame_id`.
    pub transform: CoordinateTransform4x4,
    /// Declared material entry identities (indexed by `1..=entries.len()` for 1-based `material_entry`).
    pub entries: Vec<MaterialEntryIdentity>,
}

/// Verified material entry resulting from a full cryptographic and spatio-temporal binding check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerifiedBoundMaterial {
    pub identity: MaterialEntryIdentity,
    pub lut_entry: Q42ConstituentLutEntry,
    pub rotation_to_field_frame: [[f64; 3]; 3],
    pub translation_to_field_frame_m: [f64; 3],
}

/// Compute the unkeyed BLAKE3-256 digest of a byte slice.
#[inline]
pub fn compute_blake3_256(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

impl MaterialBinding {
    /// Verify the complete cryptographic, epoch, frame, and clock binding against raw `.m64`
    /// container bytes and a `BankedConstituentLut`, returning the verified `Q42ConstituentLutEntry`
    /// for the 1-based `one_based_entry_index`.
    pub fn verify_and_resolve_entry(
        &self,
        m64_container_bytes: &[u8],
        m64_limits: M64ReadLimits,
        m64_scratch: &mut [M64PayloadInterval],
        lut: &BankedConstituentLut,
        actual_table_schema_revision: u16,
        one_based_entry_index: u32,
        eval_clock: &RationalClock,
        eval_instant: &RationalInstant,
    ) -> Result<VerifiedBoundMaterial, MaterialBindingError> {
        // 1. Structural and CRC check on the .m64 container
        let _reader = M64Reader::parse(m64_container_bytes, m64_limits, m64_scratch)
            .map_err(MaterialBindingError::M64ContainerStructuralError)?;

        // 2. Full 256-bit BLAKE3 check on the .m64 container bytes
        let actual_container_digest = compute_blake3_256(m64_container_bytes);
        if actual_container_digest != self.container_blake3 {
            return Err(MaterialBindingError::ContainerDigestMismatch {
                expected: self.container_blake3,
                actual: actual_container_digest,
            });
        }

        // 3. Full 256-bit BLAKE3 check on the 16-KB BankedConstituentLut
        let actual_table_digest = compute_blake3_256(lut.as_gpu_bytes());
        if actual_table_digest != self.material_table_blake3 {
            return Err(MaterialBindingError::MaterialTableDigestMismatch {
                expected: self.material_table_blake3,
                actual: actual_table_digest,
            });
        }

        // 4. Schema revision and Bank 1 epoch check
        if actual_table_schema_revision != 1 || self.table_schema_revision != 1 {
            return Err(MaterialBindingError::TableSchemaRevisionMismatch {
                expected: self.table_schema_revision,
                actual: actual_table_schema_revision,
            });
        }
        if lut.active_bank_1_epoch_id != self.table_epoch_id {
            return Err(MaterialBindingError::TableEpochMismatch {
                expected_epoch: self.table_epoch_id,
                actual_epoch: lut.active_bank_1_epoch_id,
            });
        }

        // 5. Coordinate frame & rational clock validity interval check
        let (rot, trans) = self.transform.admit_for_initial_physical_solver(
            self.material_frame_id,
            self.field_frame_id,
            eval_clock,
            eval_instant,
        )?;

        // 6. Entry bounds & complete identity verification
        if one_based_entry_index == 0 || (one_based_entry_index as usize) > self.entries.len() {
            return Err(MaterialBindingError::MaterialIndexOutOfRange {
                index: one_based_entry_index as usize,
                table_len: self.entries.len(),
            });
        }
        let identity = self.entries[(one_based_entry_index as usize) - 1];
        let lut_entry = *lut.entry(identity.lut_slot_index);
        lut_entry
            .validate()
            .map_err(MaterialBindingError::LutEntryCrcInvalid)?;

        if lut_entry.canonical_q42_material_hash != identity.canonical_q42_material_hash {
            return Err(MaterialBindingError::CanonicalMaterialHashMismatch {
                expected: identity.canonical_q42_material_hash,
                actual: lut_entry.canonical_q42_material_hash,
            });
        }
        if lut_entry.provenance_did_hash != identity.provenance_did_hash {
            return Err(MaterialBindingError::ProvenanceDidHashMismatch {
                expected: identity.provenance_did_hash,
                actual: lut_entry.provenance_did_hash,
            });
        }

        Ok(VerifiedBoundMaterial {
            identity,
            lut_entry,
            rotation_to_field_frame: rot,
            translation_to_field_frame_m: trans,
        })
    }
}

/// Explicitly reject attempts to join a field sample and an `.m64` record using only a matching
/// Morton/Hilbert `spatial_key` without a verified `MaterialBinding` (C02-C06).
pub fn reject_spatial_key_only_join(_spatial_key: u64) -> Result<(), MaterialBindingError> {
    Err(MaterialBindingError::SpatialKeyAloneInsufficient)
}
