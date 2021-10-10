//! Models pertaining to reaction roles.

use super::super::{Emoji, Error};

use sqlx::{postgres::Postgres, Executor, FromRow};

use std::fmt::{self, Display, Formatter};

use twilight_model::id::{ChannelId, GuildId, MessageId, RoleId};

#[derive(FromRow)]
pub struct ReactionRole {
    guild_id: i64,
    message_id: i64,
    channel_id: i64,

    role_id: i64,

    emoji: Emoji,
}

impl ReactionRole {
    /// The role id the reaction role pertains to.
    pub fn role_id(&self) -> RoleId {
        RoleId(self.role_id as u64)
    }

    /// Gets a `ReactionRole` by a message and the emoji.
    pub async fn get<'a, E>(
        ex: E,
        message_id: MessageId,
        emoji: Emoji,
    ) -> Result<Option<ReactionRole>, Error>
    where
        E: Executor<'a, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM reaction_roles WHERE message_id = $1 AND emoji = $2")
            .bind(message_id.0 as i64)
            .bind(emoji)
            .fetch_optional(ex)
            .await
    }
}

pub struct Message {
    guild_id: GuildId,
    message_id: MessageId,
    channel_id: ChannelId,
}

impl Message {
    /// Create a new [`ReactionRole`] on this message.
    pub async fn create<'a, E>(
        &self,
        ex: E,
        role_id: RoleId,
        emoji: Emoji,
    ) -> Result<(), CreateError>
    where
        E: Executor<'a, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO reaction_roles (guild_id, message_id, channel_id, role_id, emoji)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(self.guild_id.0 as i64)
        .bind(self.message_id.0 as i64)
        .bind(self.channel_id.0 as i64)
        .bind(role_id.0 as i64)
        .bind(emoji)
        .execute(ex)
        .await
        .map(|_| ())
        .map_err(From::from)
    }
}

/// Specialized error for [`Message::create`].
#[derive(Debug)]
pub enum CreateError {
    AlreadyExists,
    Other(Error),
}

impl From<Error> for CreateError {
    fn from(err: Error) -> CreateError {
        // check if a unique constraint was violated, and which one
        match &err {
            Error::Database(db_err) => {
                // 23505 is unique_violation error code
                // see https://www.postgresql.org/docs/current/errcodes-appendix.html
                if db_err.code().map(|c| c == "23505").unwrap_or(false) {
                    return CreateError::AlreadyExists;
                }
            }
            _ => (),
        }

        CreateError::Other(err)
    }
}

impl Display for CreateError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CreateError::AlreadyExists => f.write_str("reaction role already exists"),
            CreateError::Other(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CreateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreateError::Other(err) => err.source(),
            _ => None,
        }
    }
}
