//! The different, independent components of the Kromer ecosystem.

mod xp;

pub use xp::Xp;

pub use anyhow::Error;

use std::pin::Pin;
use std::future::Future;

use tokio_stream::{StreamExt, Stream};

use twilight_model::gateway::event::Event;

pub type ServiceFuture<'f> = Pin<Box<dyn Future<Output = ()> + Send + 'f>>;

/// A service type.
///
/// This is a "fire-and-forget" type that executes the service and does nothing
/// else. Services should be treated as first-class and errors must be handled
/// inside of the service (and logged).
pub trait Service {
    /// Handles a gateway event.
    fn handle<'f>(&'f self, ev: &'f Event) -> ServiceFuture<'f>;
}

/// Two services types executed one after the other.
pub struct Cons<T, U>(T, U);

impl<T, U> Service for Cons<T, U> 
where
    T: Service + Send + Sync,
    U: Service + Send + Sync,
{
    fn handle<'f>(&'f self, ev: &'f Event) -> ServiceFuture<'f> {
        Box::pin(async move {
            self.0.handle(ev).await;
            self.1.handle(ev).await;
        })
    }
}

/// A collection of services.
#[derive(Default)]
pub struct Services<T>(T);

impl Services<()> {
    /// Create a new `Services` instance.
    pub fn new() -> Services<()> {
        Services::default()
    }

    /// Add a service to the service collection.
    pub fn add<S>(self, service: S) -> Services<S>
    where
        S: Service + Send + Sync + Clone + 'static
    {
        Services(service)
    }
}

impl<T> Services<T> 
where
    T: Service + Send + Sync + Clone + 'static
{
    /// Add a service to the service collection.
    pub fn add<S>(self, service: S) -> Services<Cons<T, S>>
    where
        S: Service + Send + Sync + Clone + 'static
    {
        Services(Cons(self.0, service))
    }

    /// Runs the services for each event in the stream.
    pub async fn run<E>(&self, mut stream: E)
    where
        E: Stream<Item = (u64, Event)> + Unpin,
    {
        while let Some((shard_id, ev)) = stream.next().await {
            // print status info
            match ev {
                Event::ShardConnected(_) => {
                    info!("SHARD [#{}] CONNECTED.", shard_id);
                }
                _ => ()
            }

            let service = self.0.clone();

            tokio::spawn(async move {
                service.handle(&ev).await;
            });
        }
    }
}

