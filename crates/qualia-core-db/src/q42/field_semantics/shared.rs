use super::*;
pub(super) fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
pub(super) fn rate(n: u64, d: u64) -> Result<(), Error> {
    if n == 0 || d == 0 || n > 1_000_000_000_000_000 || d > 1_000_000_000_000 || gcd(n, d) != 1 {
        return Err(Error::Time);
    }
    Ok(())
}
pub(super) fn validate(
    block: Block<'_>,
    ctx: &Context<'_, '_>,
    registry: &impl DomainRegistry,
) -> Result<(), Error> {
    match block.schema_type() {
        1 => {
            if scalar32(block, 1)? > 2 || scalar32(block, 3)? >= 1_000_000_000 {
                return Err(Error::Time);
            }
            rate(scalar64(block, 4)?, scalar64(block, 5)?)?;
        }
        2 => frame(block, ctx, registry)?,
        3 => descriptor(block, ctx, registry)?,
        4 => {
            if scalar32(block, 1)? > 4 {
                return Err(Error::Provenance);
            }
            if let Some(f) = field(block, 2) {
                if f.value().len() % 32 != 0 {
                    return Err(Error::Provenance);
                }
            }
            for key in [3, 4, 5] {
                if let Some(f) = field(block, key) {
                    identifier(f)?;
                }
            }
            // Lineage is generic strict UTF-8 already checked structurally.
        }
        5 => {
            ctx.require_schema(scalar32(block, 1)?, 1)?;
            let n = scalar64(block, 3)?;
            let d = scalar64(block, 4)?;
            if d == 0 || d > 1_000_000_000_000 || n >= d || gcd(n, d) != 1 {
                return Err(Error::Time);
            }
        }
        _ => return Err(Error::UnsupportedDomain),
    }
    Ok(())
}
fn frame(
    block: Block<'_>,
    ctx: &Context<'_, '_>,
    registry: &impl DomainRegistry,
) -> Result<(), Error> {
    let kind = scalar32(block, 1)?;
    if kind > 1 {
        return Err(Error::Frame);
    }
    if let Some(parent) = field(block, 2) {
        ctx.require_schema(u32_at(parent.value(), 0), 2)?;
    }
    let q = required(block, 4)?.value();
    let f = |i: usize| f64::from_bits(u64_at(q, i * 8));
    let norm = ((f(0) * f(0) + f(1) * f(1)) + f(2) * f(2)) + f(3) * f(3);
    if (norm - 1.0).abs() > 1e-12 {
        return Err(Error::Frame);
    }
    let grid = [field(block, 5), field(block, 6), field(block, 7)];
    if grid.iter().any(Option::is_some) {
        if grid[0].is_none() || grid[1].is_none() {
            return Err(Error::Frame);
        }
        let spacing = grid[0].unwrap().value();
        let dimensions = grid[1].unwrap().value();
        let mut product = 1u64;
        for i in 0..3 {
            if f64::from_bits(u64_at(spacing, i * 8)) <= 0.0 {
                return Err(Error::Frame);
            }
            let n = u32_at(dimensions, i * 4);
            if n == 0 {
                return Err(Error::Frame);
            }
            product = product.checked_mul(u64::from(n)).ok_or(Error::Overflow)?;
        }
        if let Some(order) = grid[2] {
            let order = u32_at(order.value(), 0);
            if order > 1 {
                return Err(Error::Frame);
            }
            if order == 1 && (0..3).any(|i| u32_at(dimensions, i * 4) > 1 << 21) {
                // Family brick coordinates may have smaller extents than full cells.
                registry.validate_grid_frame(block, ctx)?;
            }
        } else {
            registry.validate_grid_frame(block, ctx)?;
        }
        let _ = product;
    }
    let convention = required(block, 8)?;
    identifier(convention)?;
    if kind == 1 {
        if field(block, 9).is_none_or(|f| f.value().is_empty()) {
            return Err(Error::Frame);
        }
        registry.validate_frame_profile(block, ctx)?;
    } else if text(convention)? != "right-handed-X-forward-Y-left-Z-up" {
        registry.validate_frame_profile(block, ctx)?;
    }
    Ok(())
}
fn descriptor(
    block: Block<'_>,
    ctx: &Context<'_, '_>,
    registry: &impl DomainRegistry,
) -> Result<(), Error> {
    for (key, schema) in [(1, 1), (2, 2), (3, 4)] {
        if let Some(f) = field(block, key) {
            ctx.require_schema(u32_at(f.value(), 0), schema)?;
        }
    }
    for key in [4, 5, 6] {
        if let Some(f) = field(block, key) {
            let schema = registry
                .descriptor_target_schema(key)
                .ok_or(Error::UnsupportedDomain)?;
            let target = ctx.require_schema(u32_at(f.value(), 0), schema)?;
            ctx.require_admitted(target.id(), registry)?;
        }
    }
    let timed = [1, 13, 14].map(|key| field(block, key).is_some());
    if timed.iter().any(|&present| present) {
        if timed.iter().any(|&present| !present) {
            return Err(Error::Time);
        }
        rate(scalar64(block, 13)?, scalar64(block, 14)?)?;
    }
    for key in [7, 8, 10, 12] {
        if let Some(f) = field(block, key) {
            identifier(f)?;
        }
    }
    let units = text(required(block, 8)?)?;
    if ![
        "Pa",
        "m/s",
        "Pa*m/s",
        "J/m3",
        "N",
        "m",
        "Hz",
        "s",
        "1",
        "mixed:mode-registry",
    ]
    .contains(&units)
    {
        registry.validate_units(units, ctx)?;
    }
    if let Some(f) = field(block, 9) {
        if u32_at(f.value(), 0) == 0 {
            return Err(Error::Descriptor);
        }
    }
    if let Some(f) = field(block, 11) {
        if !(1..=7).contains(&u32_at(f.value(), 0)) {
            return Err(Error::Descriptor);
        }
    }
    if let Some(f) = field(block, 18) {
        for target in f.value().chunks_exact(4) {
            ctx.require_admitted(u32_at(target, 0), registry)?;
        }
    }
    Ok(())
}
