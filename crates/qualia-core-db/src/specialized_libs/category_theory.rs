//! Category Theory Library
//!
//! Provides the mathematical foundations for modeling operations as strict category-theoretic
//! transformations. This ensures that transformations (e.g., Quantum Gates) are modeled as
//! Morphisms between Objects (e.g., Hilbert Spaces), preventing ad-hoc computational islands.

/// An Object in a Category.
pub trait Object {
    /// Associated properties of this object, if any.
    type Properties;

    fn properties(&self) -> Self::Properties;
}

/// A Morphism from a Domain to a Codomain.
pub trait Morphism<Dom: Object, Cod: Object> {
    /// Applies this morphism to a state representing the Domain,
    /// yielding a state representing the Codomain.
    fn apply(&self, state: &mut Dom) -> Result<(), &'static str>;
}

/// An Endomorphism is a Morphism where the Domain and Codomain are the same Object.
pub trait Endomorphism<T: Object>: Morphism<T, T> {}

/// Blanket implementation for Endomorphism.
impl<T: Object, M: Morphism<T, T>> Endomorphism<T> for M {}

/// A Functor maps Objects to Objects and Morphisms to Morphisms between two Categories.
pub trait Functor<DomCat, CodCat> {
    type MapObject<T: Object>: Object;
    type MapMorphism<Dom: Object, Cod: Object, M: Morphism<Dom, Cod>>: Morphism<
        Self::MapObject<Dom>,
        Self::MapObject<Cod>,
    >;
}

/// The overall Category.
pub trait Category {
    type Obj: Object;
    type Morph<D: Object, C: Object>: Morphism<D, C>;
}
