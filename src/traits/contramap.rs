use std::marker::Tuple;

use super::Composed;

pub trait Contramap<A, B, C>
where
    A: Tuple,
    B: Tuple,
{
    fn contramap<F>(&self, contravariant: F) -> Composed<F, &Self, ()>
    where
        Self: Fn<B, Output = C>,
        F: Fn<A, Output = B>,
    {
        Composed::new(contravariant, self)
    }

    fn contramap_mut<F>(&mut self, contravariant: F) -> Composed<F, &mut Self, ()>
    where
        Self: FnMut<B, Output = C>,
        F: FnMut<A, Output = B>,
    {
        Composed::new(contravariant, self)
    }

    fn contramap_once<F>(self, contravariant: F) -> Composed<F, Self, ()>
    where
        Self: FnMut<B, Output = C> + Sized,
        F: FnMut<A, Output = B>,
    {
        Composed::new(contravariant, self)
    }
}

impl<T, A, B, C> Contramap<A, B, C> for T
where
    A: Tuple,
    B: Tuple,
{
}
