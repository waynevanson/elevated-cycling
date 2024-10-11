use itertools::{FoldWhile, Itertools};
use std::iter::Peekable;

pub struct SplitWhile<Iter, Folder, Initializer>
where
    Iter: Iterator,
{
    iterator: Peekable<Iter>,
    initializer: Initializer,
    folder: Folder,
}

impl<Iter, Folder, Initializer, Chunk> Iterator for SplitWhile<Iter, Folder, Initializer>
where
    Iter: Iterator,
    Folder: Fn(Chunk, Iter::Item) -> FoldWhile<Chunk>,
    Initializer: Fn() -> Chunk,
{
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        // If there's no elements to consume then there's no chunks to create
        self.iterator.peek()?;
        let chunk = (self.initializer)();
        self.iterator
            .fold_while(chunk, &self.folder)
            .into_inner()
            .into()
    }
}

pub trait IntoSplitWhile<Chunk>: IntoIterator + Sized {
    fn split_while<Initializer, Folder>(
        self,
        initializer: Initializer,
        folder: Folder,
    ) -> SplitWhile<Self::IntoIter, Folder, Initializer>
    where
        Folder: Fn(Chunk, Self::Item) -> FoldWhile<Chunk>,
        Initializer: Fn() -> Chunk,
    {
        SplitWhile {
            folder,
            initializer,
            iterator: self.into_iter().peekable(),
        }
    }
}

impl<T, Chunk> IntoSplitWhile<Chunk> for T where T: IntoIterator + Sized {}
