//! The executing context of an event.

use sqlx::{pool::Pool, postgres::Postgres};
use twilight_http::Client;
use twilight_model::id::ApplicationId;
use twilight_standby::Standby;

use std::ops::Deref;

/// The executing context of an event.
///
/// This is just a collection of super-common types like [`Client`] and
/// [`Standby`] that are easier to specify concretely, and so we can provide
/// some helper methods for them.
///
/// This type is cheap to clone.
#[derive(Clone)]
pub struct Context {
    http: Client,
    db: Pool<Postgres>,
    standby: Standby,
}

impl Context {
    /// Create a new context.
    pub fn new(http: Client, db: Pool<Postgres>) -> Context {
        Context {
            http,
            db,
            standby: Standby::new(),
        }
    }

    /// The application id of the service.
    pub fn application_id(&self) -> ApplicationId {
        self.http.application_id().unwrap()
    }

    /// Gets a reference to the inner HTTP [`Client`].
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// Gets the database.
    pub fn db(&self) -> &Pool<Postgres> {
        &self.db
    }
}

impl Deref for Context {
    type Target = Standby;

    fn deref(&self) -> &Self::Target {
        &self.standby
    }
}
