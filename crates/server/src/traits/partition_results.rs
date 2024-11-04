use itertools::Itertools;
use len_trait::Empty;

pub trait PartitionResults<T, E> {
    fn partition_results<A, B>(self) -> Result<A, B>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        A: Default + Extend<T> + Empty,
        B: Default + Extend<E> + Empty,
    {
        let (successes, errors) = self.partition_result::<A, B, T, E>();

        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(successes)
        }
    }
}

impl<T, F, G> PartitionResults<F, G> for T {}
