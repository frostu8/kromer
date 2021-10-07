//! Bot storage models supported by [`sqlx`].

pub mod xp;

pub use sqlx::Error;

use sqlx::{Acquire, migrate::{Migrate, MigrateError}};

use std::ops::Deref;

/// Runs migrations.
pub async fn migrate<'a, E>(ex: E) -> Result<(), MigrateError> 
where
    E: Acquire<'a>,
    <<E as Acquire<'a>>::Connection as Deref>::Target: Migrate,
{
    sqlx::migrate!()
        .run(ex)
        .await
}
