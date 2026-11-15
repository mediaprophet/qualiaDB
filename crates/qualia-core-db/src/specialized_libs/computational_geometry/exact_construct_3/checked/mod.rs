//! Exact original-coordinate plane operations. Legacy construction APIs and
//! ExactPoint3 storage remain unchanged. Unsupported arithmetic refuses.
mod arithmetic;
mod triangle;
use super::super::{two_diff, Point3, Sign};
use super::{ExactPoint3, ParameterSpan};
pub use arithmetic::{exact_product_is_supported, Scratch, CAPACITY};
pub use triangle::Triangle3;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Capacity,
    Work,
    Cancelled,
    NonFinite,
    ProductRange,
    Degenerate,
    Parallel,
    InvalidPoint,
    InvalidIndex,
}

fn coords(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}
fn difference(a: f64, b: f64) -> Result<[f64; 2], Error> {
    if !a.is_finite() || !b.is_finite() {
        return Err(Error::NonFinite);
    }
    let (hi, lo) = two_diff(a, b);
    if !hi.is_finite() || !lo.is_finite() {
        return Err(Error::NonFinite);
    }
    Ok([lo, hi])
}

fn validate_point(p: &ExactPoint3, scratch: &mut Scratch<'_>, work: &mut usize) -> Result<(), Error> {
    scratch.charge(work, 4)?;
    if p.den_len == 0 || p.den_len > p.den.len()
        || p.x_num_len == 0 || p.x_num_len > p.x_num.len()
        || p.y_num_len == 0 || p.y_num_len > p.y_num.len()
        || p.z_num_len == 0 || p.z_num_len > p.z_num.len()
    { return Err(Error::InvalidPoint); }
    scratch.reset();
    for &d in &p.den[..p.den_len] { scratch.scalar(d, work)?; }
    if scratch.sign() != Sign::Positive { return Err(Error::InvalidPoint); }
    Ok(())
}

/// Original-coordinate normal: each component is the difference of two
/// products of length-two exact differences, hence at most16 raw terms.
pub struct Plane3 {
    origin: Point3,
    normal: [[f64; 16]; 3],
    lengths: [usize; 3],
}
impl Plane3 {
    pub fn new(t: [Point3; 3], scratch: &mut Scratch<'_>, work: &mut usize) -> Result<Self, Error> {
        scratch.charge(work, 6 + 48 + 3)?;
        let a = coords(t[0]);
        let b = coords(t[1]);
        let c = coords(t[2]);
        let u = [
            difference(b[0], a[0])?,
            difference(b[1], a[1])?,
            difference(b[2], a[2])?,
        ];
        let v = [
            difference(c[0], a[0])?,
            difference(c[1], a[1])?,
            difference(c[2], a[2])?,
        ];
        let mut plane = Self {
            origin: t[0],
            normal: [[0.; 16]; 3],
            lengths: [0; 3],
        };
        let mut nonzero = false;
        for axis in 0..3 {
            let j = (axis + 1) % 3;
            let k = (axis + 2) % 3;
            scratch.reset();
            for x in u[j] {
                for y in v[k] {
                    scratch.product(x, y, 1., work)?;
                }
            }
            for x in u[k] {
                for y in v[j] {
                    scratch.product(x, y, -1., work)?;
                }
            }
            nonzero |= scratch.sign() != Sign::Zero;
            plane.lengths[axis] = scratch.copy(&mut plane.normal[axis], work)?;
        }
        if !nonzero {
            return Err(Error::Degenerate);
        }
        Ok(plane)
    }
    fn evaluate_plain(
        &self,
        p: Point3,
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<(), Error> {
        let p = coords(p);
        let a = coords(self.origin);
        scratch.charge(work, 3)?;
        scratch.reset();
        for axis in 0..3 {
            let delta = difference(p[axis], a[axis])?;
            for &n in &self.normal[axis][..self.lengths[axis]] {
                for d in delta {
                    scratch.product(n, d, 1., work)?;
                }
            }
        }
        Ok(())
    }
    /// Sign at an original input point without constructing a rational copy.
    pub fn orient_plain(
        &self,
        p: Point3,
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<Sign, Error> {
        self.evaluate_plain(p, scratch, work)?;
        Ok(scratch.sign())
    }
    pub fn orient(
        &self,
        p: &ExactPoint3,
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<Sign, Error> {
        validate_point(p, scratch, work)?;
        let numerators = [
            &p.x_num[..p.x_num_len],
            &p.y_num[..p.y_num_len],
            &p.z_num[..p.z_num_len],
        ];
        let origin = coords(self.origin);
        scratch.reset();
        for axis in 0..3 {
            for &n in &self.normal[axis][..self.lengths[axis]] {
                for &x in numerators[axis] {
                    scratch.product(n, x, 1., work)?;
                }
                for &d in &p.den[..p.den_len] {
                    scratch.triple(n, origin[axis], d, -1., work)?;
                }
            }
        }
        Ok(scratch.sign())
    }
    /// Computes F(p), F(q) from exact original-coordinate differences; point
    /// numerator q*F(p)-p*F(q), denominator F(p)-F(q). No rounded q-p is used.
    pub fn intersect(
        &self,
        p: Point3,
        q: Point3,
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<(ExactPoint3, ParameterSpan), Error> {
        scratch.charge(work, 512)?;
        let mut fp = [0.; 256];
        let mut fq = [0.; 256];
        self.evaluate_plain(p, scratch, work)?;
        let sp = scratch.sign();
        let np = scratch.copy(&mut fp, work)?;
        self.evaluate_plain(q, scratch, work)?;
        let sq = scratch.sign();
        let nq = scratch.copy(&mut fq, work)?;
        scratch.reset();
        for &v in &fp[..np] {
            scratch.scalar(v, work)?;
        }
        for &v in &fq[..nq] {
            scratch.scalar(-v, work)?;
        }
        let sd = scratch.sign();
        if sd == Sign::Zero {
            return Err(Error::Parallel);
        }
        let span = if sp == Sign::Zero {
            ParameterSpan::AtStart
        } else if sq == Sign::Zero {
            ParameterSpan::AtEnd
        } else if sp != sd {
            ParameterSpan::BeforeStart
        } else if sq == sd {
            ParameterSpan::BeyondEnd
        } else {
            ParameterSpan::Interior
        };
        scratch.charge(work, std::mem::size_of::<ExactPoint3>())?;
        // Exact endpoint incidence needs no rational reconstruction. This is
        // an algebraic zero test, never equality of rounded projections.
        if sp == Sign::Zero || sq == Sign::Zero {
            return Ok((ExactPoint3::from_point3(if sp == Sign::Zero { p } else { q }), span));
        }
        let mut point = ExactPoint3::from_point3(Point3::new(0., 0., 0.));
        point.den_len = scratch.copy(&mut point.den, work)?;
        let p = coords(p);
        let q = coords(q);
        for axis in 0..3 {
            scratch.reset();
            for &v in &fp[..np] {
                scratch.product(q[axis], v, 1., work)?;
            }
            for &v in &fq[..nq] {
                scratch.product(p[axis], v, -1., work)?;
            }
            let (out, len) = match axis {
                0 => (&mut point.x_num[..], &mut point.x_num_len),
                1 => (&mut point.y_num[..], &mut point.y_num_len),
                _ => (&mut point.z_num[..], &mut point.z_num_len),
            };
            *len = scratch.copy(out, work)?;
            if sd == Sign::Negative {
                scratch.charge(work, *len)?;
                for x in &mut out[..*len] {
                    *x = -*x;
                }
            }
        }
        if sd == Sign::Negative {
            scratch.charge(work, point.den_len)?;
            for d in &mut point.den[..point.den_len] {
                *d = -*d;
            }
        }
        Ok((point, span))
    }
}
