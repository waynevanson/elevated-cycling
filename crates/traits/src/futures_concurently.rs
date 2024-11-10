use futures::stream::{FuturesUnordered, StreamExt};
use futures::Future;

pub trait IntoJoinConcurrently<T>
where
    Self: Iterator + Sized,
    T: Send + Sync,
{
    async fn join_concurrently_result<C, E>(mut self, max_concurrency: usize) -> Result<C, E>
    where
        Self::Item: Future<Output = Result<T, E>>,
        E: Send + Sync,
        C: Default + Extend<T>,
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

        let mut collection = C::default();

        while let Some(result) = futures_unordered.next().await {
            match result {
                Ok(value) => collection.extend(Some(value)),
                Err(error) => return Err(error),
            };

            while futures_unordered.len() < max_concurrency
                && let Some(fut) = self.next()
            {
                futures_unordered.push(fut);
            }
        }

        Ok(collection)
    }
}

impl<I, T> IntoJoinConcurrently<T> for I
where
    I: Iterator + Sized,
    T: Send + Sync,
{
}
