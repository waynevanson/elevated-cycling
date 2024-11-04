use osmpbf::{Element, ElementReader};
use std::io::Read;

pub trait ParMapCollect<Item> {
    fn par_map_collect<Collection>(
        self,
        collector: impl Fn(Element<'_>) -> Collection + Sync + Send,
    ) -> Collection
    where
        Collection: IntoIterator<Item = Item> + Extend<Item> + Default + Sync + Send;
}

impl<Item, R> ParMapCollect<Item> for ElementReader<R>
where
    R: Read + Send,
{
    fn par_map_collect<Collection>(
        self,
        collector: impl Fn(Element<'_>) -> Collection + Sync + Send,
    ) -> Collection
    where
        Collection: IntoIterator<Item = Item> + Extend<Item> + Default + Send + Sync,
    {
        self.par_map_reduce(
            collector,
            || Collection::default(),
            |mut accu, curr| {
                accu.extend(curr);
                accu
            },
        )
        .unwrap()
    }
}
