//! Shared constructed-point identities. Rounded proximity is only a rejection
//! prefilter; equality is proved by cross-multiplied Core expansions.
use super::super::super::{
    grow_expansion, scale_expansion, sign_of_expansion, ExactPoint3, Point3,
    Sign,
};
use super::{charge, Error};
pub(super) use super::super::super::exact_construct_3::checked::{self, Triangle3};
pub(super) const MAX_CONSTRUCTED: usize = 2048;
const EXPANSION_CAP: usize = 4096;
/// Allocated once by the cold union. Comparisons only reuse active prefixes.
pub(super) struct Scratch<'a> {
    acc: Vec<f64>,
    next: Vec<f64>,
    term: Vec<f64>,
    cancel: &'a dyn Fn() -> bool,
}
impl<'a> Scratch<'a> {
    pub fn new(work: &mut usize, cancel: &'a dyn Fn() -> bool) -> Result<Self, Error> {
        if cancel() { return Err(Error::Cancelled); }
        charge(work, 3 * EXPANSION_CAP)?;
        fn buffer() -> Result<Vec<f64>, Error> {
            let mut v = Vec::new();
            v.try_reserve_exact(EXPANSION_CAP)
                .map_err(|_| Error::Bytes)?;
            if v.capacity() > EXPANSION_CAP {
                return Err(Error::Bytes);
            }
            v.resize(EXPANSION_CAP, 0.);
            Ok(v)
        }
        Ok(Self {
            acc: buffer()?,
            next: buffer()?,
            term: buffer()?,
            cancel,
        })
    }
    pub fn checked(&mut self) -> Result<checked::Scratch<'_>, Error> {
        checked::Scratch::new(&mut self.acc, &mut self.next, self.cancel).map_err(construction_error)
    }
    #[cfg(test)]
    pub fn addresses(&self) -> [usize; 3] {
        let mut a = [
            self.acc.as_ptr() as usize,
            self.next.as_ptr() as usize,
            self.term.as_ptr() as usize,
        ];
        a.sort_unstable();
        a
    }
}
pub(super) struct Node {
    pub p: Point3,
    exact: Option<Box<ExactPoint3>>,
}
impl Node {
    pub fn is_original_point(&self, p: Point3) -> bool {
        self.exact.is_none() && self.p == p
    }
    pub fn exact(&self, work: &mut usize) -> Result<ExactPoint3, Error> {
        // Includes full fixed-array copies/initialization, not merely active
        // expansion lengths. This is a logical f64-slot charge, not wall time.
        charge(work, std::mem::size_of::<ExactPoint3>() / std::mem::size_of::<f64>())?;
        Ok(self.exact
            .as_deref()
            .cloned()
            .unwrap_or_else(|| ExactPoint3::from_point3(self.p)))
    }
}
pub(super) fn construction_error(e: checked::Error) -> Error {
    match e {
        checked::Error::Work => Error::Work,
        checked::Error::Cancelled => Error::Cancelled,
        other => Error::Construction(other),
    }
}
pub(super) struct Registry {
    pub nodes: Vec<Node>,
    limit: usize,
    constructed: usize,
}
impl Registry {
    pub fn new(limit: usize) -> Self {
        Self {
            nodes: Vec::new(),
            limit: limit.min(8192),
            constructed: 0,
        }
    }
    pub fn plain(&mut self, p: Point3, work: &mut usize) -> Result<u32, Error> {
        for (i, n) in self.nodes.iter().enumerate() {
            charge(work, 1)?;
            if n.p == p && n.exact.is_none() {
                return Ok(i as u32);
            }
        }
        self.push(Node { p, exact: None })
    }
    fn push(&mut self, n: Node) -> Result<u32, Error> {
        if self.nodes.len() >= self.limit {
            return Err(Error::Points);
        }
        let id = self.nodes.len() as u32;
        self.nodes.push(n);
        Ok(id)
    }
    pub fn constructed(
        &mut self,
        p: ExactPoint3,
        work: &mut usize,
        scratch: &mut Scratch<'_>,
    ) -> Result<u32, Error> {
        if (scratch.cancel)() { return Err(Error::Cancelled); }
        charge(work, p.x_num_len.checked_add(p.y_num_len)
            .and_then(|n| n.checked_add(p.z_num_len))
            .and_then(|n| n.checked_add(p.den_len)).ok_or(Error::Overflow)?)?;
        let rounded = p.to_point3();
        for (i, n) in self.nodes.iter().enumerate() {
            charge(work, 1)?;
            // Different rounded results can represent the same rational. This
            // screen NEVER authorizes a weld; the exact check below does.
            let scale = 1.0 + rounded.x.abs() + rounded.y.abs() + rounded.z.abs();
            if (rounded.x - n.p.x).abs() + (rounded.y - n.p.y).abs() + (rounded.z - n.p.z).abs()
                > scale * 1e-12
            {
                continue;
            }
            if equal(&p, &n.exact(work)?, work, scratch)? {
                return Ok(i as u32);
            }
            if rounded == n.p {
                let previous = n.exact(work)?;
                return Err(Error::RegistryCollision {
                    node: i,
                    existing_plain: n.exact.is_none(),
                    rounded,
                    old_lengths: [
                        previous.x_num_len,
                        previous.y_num_len,
                        previous.z_num_len,
                        previous.den_len,
                    ],
                    new_lengths: [p.x_num_len, p.y_num_len, p.z_num_len, p.den_len],
                    old_components: components(&previous),
                    new_components: components(&p),
                });
            }
        }
        if self.constructed >= MAX_CONSTRUCTED || self.nodes.len() >= self.limit {
            return Err(Error::Points);
        }
        // Stack construction and the retained boxed copy are distinct logical
        // visits. Refuse before allocating or changing insertion counters.
        charge(work, std::mem::size_of::<ExactPoint3>() / std::mem::size_of::<f64>())?;
        self.constructed += 1;
        self.push(Node {
            p: rounded,
            exact: Some(Box::new(p)),
        })
    }
}
fn coordinate(p: &ExactPoint3, a: usize) -> &[f64] {
    match a {
        0 => &p.x_num[..p.x_num_len],
        1 => &p.y_num[..p.y_num_len],
        _ => &p.z_num[..p.z_num_len],
    }
}
fn components(p: &ExactPoint3) -> [[f64; 8]; 4] {
    std::array::from_fn(|a| {
        let s = if a < 3 {
            coordinate(p, a)
        } else {
            &p.den[..p.den_len]
        };
        std::array::from_fn(|i| s.get(i).copied().unwrap_or(0.))
    })
}
fn difference_product(
    a: &[f64],
    b: &[f64],
    c: &[f64],
    d: &[f64],
    work: &mut usize,
    scratch: &mut Scratch<'_>,
) -> Result<Sign, Error> {
    let bound = a
        .len()
        .checked_mul(b.len())
        .and_then(|n| c.len().checked_mul(d.len()).and_then(|m| n.checked_add(m)))
        .and_then(|n| n.checked_mul(2))
        .and_then(|n| n.checked_add(1))
        .ok_or(Error::Overflow)?;
    if bound > EXPANSION_CAP {
        return Err(Error::Arithmetic);
    }
    let Scratch { acc, next, term, cancel } = scratch;
    charge(work, 1)?;
    acc[0] = 0.;
    let mut n: usize = 1;
    for (left, right, sign) in [(a, b, 1.), (c, d, -1.)] {
        for &v in right {
            if cancel() { return Err(Error::Cancelled); }
            charge(work, left.len())?;
            if left.iter().any(|&x| !product_supported(x, v)) {
                return Err(Error::Arithmetic);
            }
            charge(work, left.len().checked_mul(4).ok_or(Error::Overflow)?)?;
            let k = scale_expansion(left, v * sign, term).map_err(|_| Error::Arithmetic)?;
            // Raw products preserve their exact total but need not be ordered
            // or non-overlapping. Fold every term through canonical Core grow,
            // starting with [0], then remove only exact zero components.
            for &component in &term[..k] {
                if cancel() { return Err(Error::Cancelled); }
                let m = n.checked_add(1).ok_or(Error::Overflow)?;
                charge(work, m.checked_mul(2).ok_or(Error::Overflow)?)?;
                if !component.is_finite() {
                    return Err(Error::Arithmetic);
                }
                let written =
                    grow_expansion(&acc[..n], component, next).map_err(|_| Error::Arithmetic)?;
                let mut kept = 0;
                for i in 0..written {
                    let value = next[i];
                    if !value.is_finite() {
                        return Err(Error::Arithmetic);
                    }
                    if value != 0. {
                        next[kept] = value;
                        kept += 1;
                    }
                }
                if kept == 0 {
                    next[0] = 0.;
                    kept = 1;
                }
                std::mem::swap(acc, next);
                n = kept;
            }
        }
    }
    Ok(sign_of_expansion(&acc[..n]))
}
// An error-free product cannot retain bits below the smallest f64 subnormal.
// Check the exact product's lowest bit before Core scale; its FMA residual
// would otherwise silently round an unsupported tail to zero.
fn product_supported(a: f64, b: f64) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    if a == 0. || b == 0. {
        return true;
    }
    if !(a * b).is_finite() {
        return false;
    }
    fn lowest_bit(x: f64) -> i32 {
        let bits = x.to_bits() & 0x7fff_ffff_ffff_ffff;
        let exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & 0x000f_ffff_ffff_ffff;
        let (mantissa, base) = if exponent == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1u64 << 52), exponent - 1023 - 52)
        };
        base + mantissa.trailing_zeros() as i32
    }
    lowest_bit(a) + lowest_bit(b) >= -1074
}
fn equal(
    a: &ExactPoint3,
    b: &ExactPoint3,
    work: &mut usize,
    scratch: &mut Scratch<'_>,
) -> Result<bool, Error> {
    for axis in 0..3 {
        if difference_product(
            coordinate(a, axis),
            &b.den[..b.den_len],
            coordinate(b, axis),
            &a.den[..a.den_len],
            work,
            scratch,
        )? != Sign::Zero
        {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn cmp_axis(
    a: &Node,
    b: &Node,
    axis: usize,
    work: &mut usize,
    scratch: &mut Scratch<'_>,
) -> Result<Sign, Error> {
    let a = a.exact(work)?;
    let b = b.exact(work)?;
    difference_product(
        coordinate(&a, axis),
        &b.den[..b.den_len],
        coordinate(&b, axis),
        &a.den[..a.den_len],
        work,
        scratch,
    )
}
pub(super) fn triangle(t: [Point3; 3], work: &mut usize, arithmetic: &mut Scratch<'_>) -> Result<Triangle3, Error> {
    Triangle3::new(t, &mut arithmetic.checked()?, work).map_err(construction_error)
}
#[cfg(test)]
pub(super) fn plane(p: &Node, t: [Point3; 3], work: &mut usize, arithmetic: &mut Scratch<'_>) -> Result<Sign, Error> {
    let plane = checked::Plane3::new(t, &mut arithmetic.checked()?, work).map_err(construction_error)?;
    plane.orient(&p.exact(work)?, &mut arithmetic.checked()?, work).map_err(construction_error)
}
#[cfg(test)]
pub(super) fn inside(p: &Node, t: [Point3; 3], work: &mut usize, arithmetic: &mut Scratch<'_>) -> Result<bool, Error> {
    let t = triangle(t, work, arithmetic)?;
    inside_on(p, &t, work, arithmetic)
}
pub(super) fn inside_on(p: &Node, t: &Triangle3, work: &mut usize, arithmetic: &mut Scratch<'_>) -> Result<bool, Error> {
    if p.exact.is_none() {
        return t.contains_plain(p.p, &mut arithmetic.checked()?, work).map_err(construction_error);
    }
    t.contains(&p.exact(work)?, &mut arithmetic.checked()?, work).map_err(construction_error)
}
pub(super) fn on_edge(p: &Node, t: &Triangle3, edge: usize, work: &mut usize, arithmetic: &mut Scratch<'_>) -> Result<bool, Error> {
    if p.exact.is_none() {
        return t.side_plain(p.p, edge, &mut arithmetic.checked()?, work)
            .map(|s| s == Sign::Zero).map_err(construction_error);
    }
    t.side(&p.exact(work)?, edge, &mut arithmetic.checked()?, work)
        .map(|s| s == Sign::Zero).map_err(construction_error)
}
