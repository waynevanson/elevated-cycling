use futures::future::{join_all, JoinAll};
use std::future::Future;

pub trait IntoJoinAll: IntoIterator + Sized {
    fn join_all(self) -> JoinAll<Self::Item>
    where
        Self: IntoIterator,
        Self::Item: Future,
    {
        join_all(self)
    }
}

impl<T> IntoJoinAll for T where T: IntoIterator + Sized {}
