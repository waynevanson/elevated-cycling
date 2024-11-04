use futures::stream::{FuturesUnordered, StreamExt};
use futures::Future;

pub trait IntoJoinConcurrently {
    async fn join_concurrently<Collection>(mut self, max_concurrency: usize) -> Collection
    where
        Self: Iterator + Sized,
        Self::Item: Future,
        <Self::Item as Future>::Output: Send + Sync,
        Collection: Default + Extend<<Self::Item as Future>::Output>,
    {
        if max_concurrency <= 0 {
            panic!(
                "Expected max_concurency to be greater than 0 but received {}",
                max_concurrency
            );
        }

        let mut futures_unordered = FuturesUnordered::new();

        while futures_unordered.len() < max_concurrency
            && let Some(fut) = self.next()
        {
            futures_unordered.push(fut);
        }

        let mut collection = Collection::default();

        while let Some(result) = futures_unordered.next().await {
            collection.extend(Some(result));

            while futures_unordered.len() < max_concurrency
                && let Some(fut) = self.next()
            {
                futures_unordered.push(fut);
            }
        }

        collection
    }
}

impl<T> IntoJoinConcurrently for T where T: Iterator + Sized {}
