use num_traits::{One, Zero};
use std::ops::{Add, Div};

pub trait Average {
    fn average<T>(self) -> T
    where
        Self: Iterator + Sized,
        T: PartialEq + Zero + One + Add<Self::Item, Output = T> + Div<Output = T>,
    {
        let (total, count) = self.fold((T::zero(), T::zero()), |(sum, count), item| {
            (sum.add(item), count + T::one())
        });

        if count == T::zero() {
            T::zero()
        } else {
            total / count
        }
    }
}

impl<T> Average for T {}
