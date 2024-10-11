use super::Composed;
use std::marker::Tuple;

pub trait FunctionMap<A, B, C>
where
    A: Tuple,
{
    fn fmap<F>(&self, covariant: F) -> Composed<&Self, F, ()>
    where
        Self: Fn<A, Output = B>,
        F: Fn(B) -> C,
    {
        Composed::new(self, covariant)
    }

    fn fmap_mut<F>(&mut self, covariant: F) -> Composed<&mut Self, F, ()>
    where
        Self: FnMut<A, Output = B>,
        F: FnMut(B) -> C,
    {
        Composed::new(self, covariant)
    }

    fn fmap_once<F>(self, covariant: F) -> Composed<Self, F, ()>
    where
        Self: FnOnce<A, Output = B> + Sized,
        F: FnOnce(B) -> C,
    {
        Composed::new(self, covariant)
    }
}

impl<T, Args, Output1, Output2> FunctionMap<Args, Output1, Output2> for T where Args: Tuple {}
