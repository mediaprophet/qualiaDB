//! Tiny categorical composition helpers.
//!
//! These are intentionally simple: they let economics modules name reusable
//! transforms such as `MarketData -> ReturnSeries -> RiskReport` without each
//! submodule inventing a bespoke adapter trait.

/// A deterministic morphism from `A` to `Self::Codomain`.
pub trait Morphism<A> {
    type Codomain;

    fn apply(&self, input: A) -> Self::Codomain;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl<A> Morphism<A> for Identity {
    type Codomain = A;

    #[inline]
    fn apply(&self, input: A) -> A {
        input
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Compose<F, G> {
    pub first: F,
    pub second: G,
}

impl<F, G> Compose<F, G> {
    pub const fn new(first: F, second: G) -> Self {
        Self { first, second }
    }
}

impl<A, F, G> Morphism<A> for Compose<F, G>
where
    F: Morphism<A>,
    G: Morphism<F::Codomain>,
{
    type Codomain = G::Codomain;

    #[inline]
    fn apply(&self, input: A) -> Self::Codomain {
        self.second.apply(self.first.apply(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct AddOne;
    impl Morphism<i32> for AddOne {
        type Codomain = i32;
        fn apply(&self, input: i32) -> i32 {
            input + 1
        }
    }

    #[derive(Clone, Copy)]
    struct Double;
    impl Morphism<i32> for Double {
        type Codomain = i32;
        fn apply(&self, input: i32) -> i32 {
            input * 2
        }
    }

    #[test]
    fn identity_is_neutral() {
        assert_eq!(Identity.apply(7), 7);
    }

    #[test]
    fn compose_applies_left_then_right() {
        let f = Compose::new(AddOne, Double);
        assert_eq!(f.apply(3), 8);
    }
}
