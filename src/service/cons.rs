use super::{Event, Service};

use std::pin::Pin;
use std::task::{Context, Poll};

/// Two services types executed at the same time.
#[derive(Clone)]
pub struct Cons<T, U>(T, U);

impl<T, U> Cons<T, U> {
    pub fn new(first: T, second: U) -> Cons<T, U> {
        Cons(first, second)
    }
}

impl<'f, T, U> Service<'f> for Cons<T, U>
where
    T: Service<'f> + Send + Sync,
    U: Service<'f> + Send + Sync,
{
    type Future = Future<T::Future, U::Future>;

    fn handle(&'f self, cx: &'f super::Context, ev: &'f Event) -> Self::Future {
        Future::new(self.0.handle(cx, ev), self.1.handle(cx, ev))
    }
}

pub struct Future<F, G>(Option<F>, Option<G>);

impl<F, G> Future<F, G> {
    pub fn new(fut1: F, fut2: G) -> Future<F, G> {
        Future(Some(fut1), Some(fut2))
    }
}

impl<'f, F, G> std::future::Future for Future<F, G>
where
    F: std::future::Future<Output = ()>,
    G: std::future::Future<Output = ()>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // SAFETY: we have the only pin mut ref, and the following function
        // calls don't move the references
        let Self(fut1, fut2) = unsafe { self.get_unchecked_mut() };

        run_optional_future(fut1, cx);
        run_optional_future(fut2, cx);

        if fut1.is_none() && fut2.is_none() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

fn run_optional_future<F>(fut: &mut Option<F>, cx: &mut Context)
where
    F: std::future::Future<Output = ()>,
{
    if let Some(inner) = fut {
        unsafe {
            match Pin::new_unchecked(inner).poll(cx) {
                // SAFETY: this is okay, because we're running the destructor 
                // in-place
                Poll::Ready(()) => *fut = None,
                Poll::Pending => (),
            }
        }
    }
}

