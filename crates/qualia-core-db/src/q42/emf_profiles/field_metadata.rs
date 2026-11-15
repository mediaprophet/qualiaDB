//! Fixed deterministic shared/EMF schema construction through the common builder.
use super::super::field_envelope::{BlockInput, FieldInput, FieldValue as V, MetadataInput};
use super::field_writer::*;
use super::*;
fn f(key: u16, value: V<'_>) -> FieldInput<'_> {
    FieldInput {
        key,
        flags: 1,
        value,
    }
}
fn push<'a>(fields: &mut [FieldInput<'a>], count: &mut usize, key: u16, value: V<'a>) {
    fields[*count] = f(key, value);
    *count += 1;
}
pub(super) fn with_metadata<T>(
    document: &FieldDocument<'_>,
    run: impl FnOnce(MetadataInput<'_>) -> Result<T, Error>,
) -> Result<T, Error> {
    let clock = document.clock;
    let scale = [clock.scale];
    let epoch = [clock.epoch_seconds];
    let nanos = [clock.epoch_nanoseconds];
    let numerator = [clock.ticks_per_second_num];
    let denominator = [clock.ticks_per_second_den];
    let zero = [0u32];
    let one = [1u32];
    let two = [2u32];
    let three = [3u32];
    let four = [4u32];
    let clock_fields = [
        f(1, V::U32(&scale)),
        f(2, V::I64(&epoch)),
        f(3, V::U32(&nanos)),
        f(4, V::U64(&numerator)),
        f(5, V::U64(&denominator)),
    ];
    let empty = FieldInput {
        key: 0,
        flags: 0,
        value: V::Bytes(&[]),
    };
    let mut frame_fields = [empty; 7];
    let mut frame_count = 0;
    push(&mut frame_fields, &mut frame_count, 1, V::U32(&zero));
    push(
        &mut frame_fields,
        &mut frame_count,
        3,
        V::F64(&document.frame.origin_m),
    );
    push(
        &mut frame_fields,
        &mut frame_count,
        4,
        V::F64(&document.frame.rotation_xyzw),
    );
    let grid = document.frame.grid;
    let dims = grid.map_or([1; 3], |g| g.dimensions);
    let dims64 = dims.map(u64::from);
    let bricks = grid.map_or([1; 3], |g| g.brick_dimensions);
    let spacing = grid.map_or([1.0; 3], |g| g.spacing_m);
    let indexing = grid.map_or(GridIndexing::Linear, |g| g.indexing);
    let order = [indexing.order()];
    let common_order = [indexing.common_order().unwrap_or(0)];
    if let Some(grid) = grid {
        grid.validate()?;
        push(&mut frame_fields, &mut frame_count, 5, V::F64(&spacing));
        push(&mut frame_fields, &mut frame_count, 6, V::U32(&dims));
        if indexing.common_order().is_some() {
            push(
                &mut frame_fields,
                &mut frame_count,
                7,
                V::U32(&common_order),
            );
        }
    }
    push(
        &mut frame_fields,
        &mut frame_count,
        8,
        V::Utf8("right-handed-X-forward-Y-left-Z-up"),
    );
    let mut provenance_fields = [empty; 4];
    let mut provenance_count = 0;
    let role = [document.provenance.role];
    push(
        &mut provenance_fields,
        &mut provenance_count,
        1,
        V::U32(&role),
    );
    if let Some(s) = document.provenance.algorithm {
        push(&mut provenance_fields, &mut provenance_count, 3, V::Utf8(s));
    }
    if let Some(s) = document.provenance.algorithm_version {
        push(&mut provenance_fields, &mut provenance_count, 4, V::Utf8(s));
    }
    if let Some(s) = document.provenance.lineage {
        push(&mut provenance_fields, &mut provenance_count, 7, V::Utf8(s));
    }
    let step = [document.temporal_step_ticks.unwrap_or(0)];
    if document.temporal_step_ticks.is_some_and(|s| s <= 0) {
        return Err(Error::Time);
    }
    let ext_fields = [
        f(1, V::U32(&zero)),
        f(2, V::U32(&one)),
        f(3, V::MetadataRefs(&one)),
        f(4, V::MetadataRefs(&two)),
        f(5, V::MetadataRefs(&three)),
        f(6, V::Utf8(FIELD_PROFILE)),
        f(7, V::Utf8(indexing.name())),
        f(8, V::U64(&dims64)),
        f(9, V::U32(&bricks)),
        f(10, V::U32(&order)),
        f(11, V::I64(&step)),
    ];
    let mut desc_fields = [empty; 10];
    let mut desc_count = 0;
    let (rate_num, rate_den) = if let Some(step) = document.temporal_step_ticks {
        let step = step as u64;
        let g = gcd(clock.ticks_per_second_num, step);
        let n = clock.ticks_per_second_num / g;
        let d = u128::from(clock.ticks_per_second_den) * u128::from(step / g);
        (n, u64::try_from(d).map_err(|_| Error::Time)?)
    } else {
        (0, 0)
    };
    let rate_num = [rate_num];
    let rate_den = [rate_den];
    if document.temporal_step_ticks.is_some() {
        push(&mut desc_fields, &mut desc_count, 1, V::MetadataRefs(&one));
    }
    push(&mut desc_fields, &mut desc_count, 2, V::MetadataRefs(&two));
    push(
        &mut desc_fields,
        &mut desc_count,
        3,
        V::MetadataRefs(&three),
    );
    push(
        &mut desc_fields,
        &mut desc_count,
        7,
        V::Utf8("electromagnetic_field_snapshot"),
    );
    push(
        &mut desc_fields,
        &mut desc_count,
        8,
        V::Utf8("mixed:mode-registry"),
    );
    push(
        &mut desc_fields,
        &mut desc_count,
        12,
        V::Utf8(FIELD_MODEL_URI),
    );
    if document.temporal_step_ticks.is_some() {
        push(&mut desc_fields, &mut desc_count, 13, V::U64(&rate_num));
        push(&mut desc_fields, &mut desc_count, 14, V::U64(&rate_den));
    }
    push(
        &mut desc_fields,
        &mut desc_count,
        18,
        V::MetadataRefs(&four),
    );
    let blocks = [
        BlockInput {
            id: 1,
            schema_type: 1,
            revision: 1,
            flags: 1,
            fields: &clock_fields,
        },
        BlockInput {
            id: 2,
            schema_type: 2,
            revision: 1,
            flags: 1,
            fields: &frame_fields[..frame_count],
        },
        BlockInput {
            id: 3,
            schema_type: 4,
            revision: 1,
            flags: 1,
            fields: &provenance_fields[..provenance_count],
        },
        BlockInput {
            id: 4,
            schema_type: 103,
            revision: 1,
            flags: 1,
            fields: &ext_fields,
        },
        BlockInput {
            id: 5,
            schema_type: 3,
            revision: 1,
            flags: 1,
            fields: &desc_fields[..desc_count],
        },
    ];
    run(MetadataInput { blocks: &blocks })
}
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
