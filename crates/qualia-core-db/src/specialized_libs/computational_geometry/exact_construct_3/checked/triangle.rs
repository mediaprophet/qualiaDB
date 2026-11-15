//! Exact projected halfplanes over the same original-coordinate plane owner.
//! No rounded normal offset or new constructed coordinate defines a boundary.
use super::{coords, difference, Error, ExactPoint3, Plane3, Point3, Scratch, Sign};

pub struct Triangle3 {
    pub plane: Plane3,
    corners: [Point3; 3],
    drop: usize,
    winding: Sign,
}
impl Triangle3 {
    pub fn new(
        corners: [Point3; 3],
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<Self, Error> {
        let plane = Plane3::new(corners, scratch, work)?;
        scratch.charge(work, 3)?;
        // Any nonzero exact normal component yields a nonsingular projection.
        // Prefer the largest leading component for the later rounded drawing.
        let drop = (0..3).max_by(|&a, &b| {
            plane.normal[a][plane.lengths[a] - 1].abs().total_cmp(
                &plane.normal[b][plane.lengths[b] - 1].abs())
        }).ok_or(Error::Degenerate)?;
        let winding = super::super::super::sign_of_expansion(
            &plane.normal[drop][..plane.lengths[drop]]);
        if winding == Sign::Zero { return Err(Error::Degenerate); }
        Ok(Self { plane, corners, drop, winding })
    }
    pub fn drop_axis(&self) -> usize { self.drop }
    pub fn winding(&self) -> Sign { self.winding }

    pub fn side_plain(&self, p: Point3, edge: usize, scratch: &mut Scratch<'_>, work: &mut usize) -> Result<Sign, Error> {
        if edge >= 3 { return Err(Error::InvalidIndex); }
        let a = coords(self.corners[edge]);
        let b = coords(self.corners[(edge + 1) % 3]);
        let p = coords(p);
        let i = (self.drop + 1) % 3;
        let j = (self.drop + 2) % 3;
        scratch.charge(work, 4)?;
        let u = difference(b[i], a[i])?;
        let v = difference(b[j], a[j])?;
        let x = difference(p[i], a[i])?;
        let y = difference(p[j], a[j])?;
        scratch.reset();
        for (left, right, sign) in [(u, y, 1.), (v, x, -1.)] {
            for l in left { for r in right { scratch.product(l, r, sign, work)?; } }
        }
        Ok(scratch.sign())
    }
    pub fn contains_plain(&self, p: Point3, scratch: &mut Scratch<'_>, work: &mut usize) -> Result<bool, Error> {
        if self.plane.orient_plain(p, scratch, work)? != Sign::Zero { return Ok(false); }
        for edge in 0..3 {
            let s = self.side_plain(p, edge, scratch, work)?;
            if s != Sign::Zero && s != self.winding { return Ok(false); }
        }
        Ok(true)
    }

    /// Signed projected edge determinant times the positive rational
    /// denominator. Cyclic axes retain the corresponding normal component.
    pub fn side(
        &self,
        p: &ExactPoint3,
        edge: usize,
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<Sign, Error> {
        if edge >= 3 { return Err(Error::InvalidIndex); }
        // The public point's arrays/lengths/denominator must be admitted even
        // when a caller only asks for an edge predicate.
        super::validate_point(p, scratch, work)?;
        let a = coords(self.corners[edge]);
        let b = coords(self.corners[(edge + 1) % 3]);
        let i = (self.drop + 1) % 3;
        let j = (self.drop + 2) % 3;
        scratch.charge(work, 2)?;
        let u = difference(b[i], a[i])?;
        let v = difference(b[j], a[j])?;
        let numerators = [
            &p.x_num[..p.x_num_len], &p.y_num[..p.y_num_len], &p.z_num[..p.z_num_len],
        ];
        scratch.reset();
        for (delta, axis, sign) in [(u, j, 1.), (v, i, -1.)] {
            for component in delta {
                for &n in numerators[axis] {
                    scratch.product(component, n, sign, work)?;
                }
                for &d in &p.den[..p.den_len] {
                    scratch.triple(component, a[axis], d, -sign, work)?;
                }
            }
        }
        Ok(scratch.sign())
    }
    pub fn contains(
        &self,
        p: &ExactPoint3,
        scratch: &mut Scratch<'_>,
        work: &mut usize,
    ) -> Result<bool, Error> {
        if self.plane.orient(p, scratch, work)? != Sign::Zero { return Ok(false); }
        for edge in 0..3 {
            let s = self.side(p, edge, scratch, work)?;
            if s != Sign::Zero && s != self.winding { return Ok(false); }
        }
        Ok(true)
    }
}
