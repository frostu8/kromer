//! The different, independent components of the Kromer ecosystem.

mod cons;

pub use cons::Cons;
pub use twilight_model::gateway::event::Event;
pub use anyhow::Error;

use twilight_standby::Standby;

use std::future::Future;

use tokio_stream::{Stream, StreamExt};

/// A service type.
///
/// This is a "fire-and-forget" type that executes the service and does nothing
/// else. Services should be treated as first-class and errors must be handled
/// inside of the service (and logged).
pub trait Service<'f> {
    type Future: Future<Output = ()> + Send + 'f;

    /// Handles a gateway event.
    fn handle(&'f self, ev: &'f Event) -> Self::Future;
}

/// A collection of services.
pub struct Services<T> { 
    standby: Standby,
    service: T,
}

impl Services<()> {
    /// Create a new `Services` instance.
    pub fn new(standby: Standby) -> Services<()> {
        Services {
            standby,
            service: (),
        }
    }

    /// Add a service to the service collection.
    pub fn add<S>(self, service: S) -> Services<S>
    where
        S: for<'a> Service<'a> + Send + Sync + Clone + 'static,
    {
        Services {
            service,
            standby: self.standby,
        }
    }
}

impl<T> Services<T>
where
    T: for<'a> Service<'a> + Send + Sync + Clone + 'static,
{
    /// Add a service to the service collection.
    pub fn add<S>(self, service: S) -> Services<Cons<T, S>>
    where
        S: for<'a> Service<'a> + Send + Sync + Clone + 'static,
    {
        Services {
            service: Cons::new(self.service, service),
            standby: self.standby,
        }
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
                    info!("shard #{} connected", shard_id);
                }
                Event::ShardDisconnected(_) => {
                    info!("shard #{} disconnected", shard_id);
                }
                _ => (),
            }

            // handle standby
            self.standby.process(&ev);

            let service = self.service.clone();

            tokio::spawn(async move {
                service.handle(&ev).await;
            });
        }
    }
}

/// Macro for easily implementing a service.
///
/// This requires Nightly rust and `#![feature(type_alias_impl_trait)]` to be
/// enabled.
#[macro_export]
macro_rules! impl_service {
    {
        impl Service for $ty:path {
            async fn handle(&$self_ident:ident, $ev_ident:ident: $ev_ty:ty) -> Result<(), $err_ty:path>
            $body:tt
        }
    } => {
        impl $ty {
            async fn __handle(
                $self_ident: &Self, 
                $ev_ident: $ev_ty,
            ) -> ::std::result::Result<(), $err_ty> {
                $body
            }
        }

        impl<'f> Service<'f> for $ty {
            type Future = impl ::std::future::Future<Output = ()> + 'f;

            fn handle(&'f self, ev: &'f crate::service::Event) -> Self::Future {
                async move {
                    let res = Self::__handle(self, ev).await;

                    if let Err(err) = res {
                        // print error inf
                        ::log::error!("service: {}", err);
                    }
                }
            }
        }
    }
}


