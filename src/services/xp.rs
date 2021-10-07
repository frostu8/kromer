//! Diminishing "experience" tracking services.

use sqlx::sqlite::SqlitePool;

use crate::model::xp::User;

use super::{Error, Service, ServiceFuture};

use twilight_model::channel::Message;
use twilight_model::gateway::event::Event;

#[derive(Clone)]
pub struct Xp {
    db: SqlitePool,
}

impl Xp {
    pub fn new(db: SqlitePool) -> Xp {
        Xp {
            db,
        }
    }

    /// Handles a message.
    pub async fn handle_message(&self, msg: &Message) -> Result<(), Error> {
        // check if the message was sent in a guild
        let guild_id = match msg.guild_id {
            Some(guild_id) => guild_id,
            // not in a guild, nothing to worry about.
            None => return Ok(()),
        };

        let user_id = msg.author.id;

        // add experience to the user
        User::add_score(
            &self.db,
            guild_id.0,
            user_id.0,
            1,
        )
            .await?;

        Ok(())
    }
}

impl Service for Xp {
    /// Handles an event.
    fn handle<'f>(&'f self, ev: &'f Event) -> ServiceFuture<'f> {
        Box::pin(async move {
            match ev {
                Event::MessageCreate(msg) => {
                    if let Err(e) = self.handle_message(msg).await {
                        error!("{}", e);
                    }
                }
                _ => ()
            }
        })
    }
}

