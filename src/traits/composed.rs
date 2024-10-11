use std::marker::{PhantomData, Tuple};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Copy)]
pub struct Composed<F, G, P> {
    first: F,
    second: G,
    phantom: PhantomData<P>,
}

impl<F, G, P> Composed<F, G, P> {
    pub fn new(first: F, second: G) -> Self {
        Self {
            first,
            phantom: PhantomData,
            second,
        }
    }
}

impl<F, G, P, A, B, C> Fn<A> for Composed<F, G, P>
where
    A: Tuple,
    B: Tuple,
    F: Fn<A, Output = B>,
    G: Fn<B, Output = C>,
{
    extern "rust-call" fn call(&self, args: A) -> Self::Output {
        self.second.call(self.first.call(args))
    }
}

impl<F, G, P, A, B, C> FnMut<A> for Composed<F, G, P>
where
    A: Tuple,
    B: Tuple,
    F: FnMut<A, Output = B>,
    G: FnMut<B, Output = C>,
{
    extern "rust-call" fn call_mut(&mut self, args: A) -> Self::Output {
        self.second.call_mut(self.first.call_mut(args))
    }
}

impl<F, G, P, A, B, C> FnOnce<A> for Composed<F, G, P>
where
    A: Tuple,
    B: Tuple,
    F: FnOnce<A, Output = B>,
    G: FnOnce<B, Output = C>,
{
    type Output = C;

    extern "rust-call" fn call_once(self, args: A) -> Self::Output {
        self.second.call_once(self.first.call_once(args))
    }
}
