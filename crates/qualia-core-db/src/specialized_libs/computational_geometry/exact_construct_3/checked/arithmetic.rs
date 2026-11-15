//! Canonical accumulation through existing Core error-free primitives.
use super::super::super::{grow_expansion, sign_of_expansion, two_product, Sign};
use super::Error;
pub const CAPACITY: usize = 4096;

/// Borrowed workspace; construction does not allocate or initialize a heap.
pub struct Scratch<'a> {
    acc: &'a mut [f64],
    next: &'a mut [f64],
    n: usize,
    cancel: &'a dyn Fn() -> bool,
}
impl<'a> Scratch<'a> {
    pub fn new(
        acc: &'a mut [f64],
        next: &'a mut [f64],
        cancel: &'a dyn Fn() -> bool,
    ) -> Result<Self, Error> {
        if acc.len() < CAPACITY || next.len() < CAPACITY {
            return Err(Error::Capacity);
        }
        Ok(Self {
            acc: &mut acc[..CAPACITY],
            next: &mut next[..CAPACITY],
            n: 0,
            cancel,
        })
    }
    pub(super) fn charge(&self, work: &mut usize, n: usize) -> Result<(), Error> {
        if (self.cancel)() {
            return Err(Error::Cancelled);
        }
        *work = work.checked_sub(n).ok_or(Error::Work)?;
        Ok(())
    }
    pub(super) fn reset(&mut self) {
        self.n = 0;
    }
    pub(super) fn scalar(&mut self, value: f64, work: &mut usize) -> Result<(), Error> {
        let n = self.n.checked_add(1).ok_or(Error::Capacity)?;
        if n > CAPACITY {
            return Err(Error::Capacity);
        }
        self.charge(work, n.checked_mul(2).ok_or(Error::Capacity)?)?;
        if !value.is_finite() {
            return Err(Error::NonFinite);
        }
        let written =
            grow_expansion(&self.acc[..self.n], value, self.next).map_err(|_| Error::Capacity)?;
        let mut kept = 0;
        for i in 0..written {
            let v = self.next[i];
            if !v.is_finite() {
                return Err(Error::NonFinite);
            }
            if v != 0. {
                self.next[kept] = v;
                kept += 1;
            }
        }
        std::mem::swap(&mut self.acc, &mut self.next);
        self.n = kept;
        Ok(())
    }
    pub(super) fn product(
        &mut self,
        a: f64,
        b: f64,
        sign: f64,
        work: &mut usize,
    ) -> Result<(), Error> {
        self.charge(work, 1)?;
        if !exact_product_is_supported(a, b) {
            return Err(Error::ProductRange);
        }
        let (p, e) = two_product(a, b);
        self.scalar(p * sign, work)?;
        self.scalar(e * sign, work)
    }
    pub(super) fn triple(
        &mut self,
        a: f64,
        b: f64,
        c: f64,
        sign: f64,
        work: &mut usize,
    ) -> Result<(), Error> {
        self.charge(work, 1)?;
        if !exact_product_is_supported(a, b) {
            return Err(Error::ProductRange);
        }
        let (p, e) = two_product(a, b);
        self.product(p, c, sign, work)?;
        self.product(e, c, sign, work)
    }
    pub(super) fn sign(&self) -> Sign {
        sign_of_expansion(&self.acc[..self.n])
    }
    pub(super) fn copy(&self, out: &mut [f64], work: &mut usize) -> Result<usize, Error> {
        let n = self.n.max(1);
        if n > out.len() {
            return Err(Error::Capacity);
        }
        self.charge(work, n)?;
        if self.n == 0 {
            out[0] = 0.;
        } else {
            out[..n].copy_from_slice(&self.acc[..n]);
        }
        Ok(n)
    }
}

/// Every exact product bit must be representable in an f64 expansion.
pub fn exact_product_is_supported(a: f64, b: f64) -> bool {
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
