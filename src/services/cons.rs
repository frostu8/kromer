use super::{Event, Service};

use pin_project::pin_project;

use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context};

/// Two services types executed one after the other.
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
    U: Service<'f> + Send + Sync + 'f,
{
    type Future = ConsFuture<'f, T::Future, U>;

    fn handle(&'f self, ev: &'f Event) -> Self::Future {
        ConsFuture(State::First(self.0.handle(ev), &self.1, ev))
    }
}

/// A future returned by [`Cons`].
#[pin_project]
pub struct ConsFuture<'f, TF, U>(#[pin] State<'f, TF, U>)
where
    TF: Future<Output = ()>,
    U: Service<'f>;

impl<'f, TF, U> Future for ConsFuture<'f, TF, U>
where
    TF: Future<Output = ()>,
    U: Service<'f>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.project().0.poll(cx)
    }
}

#[pin_project(project = StateProj)]
enum State<'f, TF, U>
where
    TF: Future<Output = ()>,
    U: Service<'f>,
{
    First(#[pin] TF, &'f U, &'f Event),
    Second(#[pin] U::Future),
}

impl<'f, TF, U> Future for State<'f, TF, U>
where
    TF: Future<Output = ()>,
    U: Service<'f>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                StateProj::First(fut, next, event) => {
                    match fut.poll(cx) {
                        // execute next future
                        Poll::Ready(()) => {
                            let fut = next.handle(event);

                            self.set(State::Second(fut))
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                StateProj::Second(fut) => return fut.poll(cx)
            }
        }
    }
}

