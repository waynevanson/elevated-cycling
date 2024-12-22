pub trait CollectTuples<A, B> {
    fn collect_tuples<Left, Right>(self) -> (Left, Right)
    where
        Left: Default + Extend<A>,
        Right: Default + Extend<B>;
}

impl<T, A, B> CollectTuples<A, B> for T
where
    T: Iterator<Item = (A, B)>,
{
    fn collect_tuples<Left, Right>(self) -> (Left, Right)
    where
        Left: Default + Extend<A>,
        Right: Default + Extend<B>,
    {
        let mut left = Left::default();
        let mut right = Right::default();

        for (left_item, right_item) in self {
            left.extend(Some(left_item));
            right.extend(Some(right_item));
        }

        (left, right)
    }
}
