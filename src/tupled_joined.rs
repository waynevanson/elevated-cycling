use std::iter::Peekable;

pub struct TupleJoined<Iter>
where
    Iter: Iterator,
{
    iter: Peekable<Iter>,
}

impl<Iter> Iterator for TupleJoined<Iter>
where
    Iter: Iterator,
    Iter::Item: Clone,
{
    type Item = (Iter::Item, Iter::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.iter.next()?;
        let second = self.iter.peek()?.clone();
        Some((first, second))
    }
}

pub trait IntoTupleJoinedIter<Iter>
where
    Iter: Iterator,
{
    fn tuple_joined(self) -> TupleJoined<Iter>;
}

impl<T> IntoTupleJoinedIter<T::IntoIter> for T
where
    T: Sized + IntoIterator,
{
    fn tuple_joined(self) -> TupleJoined<T::IntoIter> {
        TupleJoined {
            iter: self.into_iter().peekable(),
        }
    }
}
