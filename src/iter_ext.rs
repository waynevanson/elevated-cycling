use std::{
    cmp::Ordering,
    ops::{Add, Sub},
};

pub trait IterExt: Iterator + Sized {
    /// Consumes an iterator, finding the item closest to zero.
    /// Returns early when zero is found.
    fn find_first_nearest<T>(
        self,
        filter: impl Fn(&Self::Item) -> T,
        target: T,
    ) -> Option<Self::Item>
    where
        T: Ord + Sub<Output = T> + Add<Output = T> + Copy,
    {
        let mut state = None::<(Self::Item, T)>;

        for curr_item in self {
            let curr_weight = filter(&curr_item);

            let curr_diff: T = match curr_weight.cmp(&target) {
                Ordering::Equal => return Some(curr_item),
                Ordering::Greater => curr_weight - target,
                Ordering::Less => target - curr_weight,
            };

            if !state
                .as_ref()
                .is_some_and(|(_, prev_diff)| prev_diff < &curr_diff)
            {
                state = Some((curr_item, curr_diff))
            }
        }

        state.map(|state| state.0)
    }
}

impl<I> IterExt for I where I: Iterator + Sized {}

#[cfg(test)]
mod test {
    use crate::iter_ext::IterExt;

    #[test]
    fn tester() {
        let last = [2323, 20, 23, 14, 6, 34, -2323, 21, -23245, -2, 2323]
            .into_iter()
            .find_first_nearest(|item| *item, 0);

        assert_eq!(last, Some(-2))
    }

    #[test]
    fn tester1() {
        let last = [2323, 20, 23, 0, 14, 6, 34, -2323, 21, -23245, -2, 2323]
            .into_iter()
            .find_first_nearest(|item| *item, 0);

        assert_eq!(last, Some(0))
    }
}
