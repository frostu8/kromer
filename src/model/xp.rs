//! User experience.

use super::Error;

use sqlx::{postgres::Postgres, Executor, FromRow};

use twilight_model::id::{GuildId, UserId};

/// A user's experience.
#[derive(Debug, FromRow)]
pub struct Record {
    guild_id: i64,
    user_id: i64,
    score: i32,
}

impl Record {
    /// The id of the guild this record reflects.
    pub fn guild_id(&self) -> GuildId {
        GuildId(self.guild_id as u64)
    }

    /// The id of the user.
    pub fn user_id(&self) -> UserId {
        UserId(self.user_id as u64)
    }

    /// How much experience the user has.
    pub fn score(&self) -> i32 {
        self.score
    }

    /// What level the user is at.
    pub fn level(&self) -> i32 {
        level(self.score)
    }
}

/// A group of records attached to a certain guild.
pub struct Guild(i64);

impl Guild {
    /// Create a new `Guild` reference.
    ///
    /// This does nothing until operations are made to it.
    pub fn new(id: GuildId) -> Guild {
        Guild(id.0 as i64)
    }

    /// Gets the top `count` users in the guild.
    pub async fn top<'a, E>(&self, ex: E, count: u64, page: u64) -> Result<Vec<Record>, Error>
    where
        E: Executor<'a, Database = Postgres>,
    {
        let count = count as i64;
        let offset = count * (page as i64);

        sqlx::query_as(
            r#"
            SELECT * FROM xp WHERE guild_id = $1
            ORDER BY score DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(self.0)
        .bind(count)
        .bind(offset)
        .fetch_all(ex)
        .await
    }

    /// Gets a user's experience level.
    ///
    /// If a row doesn't exist, it will return a `User` with zero xp.
    pub async fn get<'a, E>(&self, ex: E, user_id: UserId) -> Result<Record, Error>
    where
        E: Executor<'a, Database = Postgres>,
    {
        let user_id = user_id.0 as i64;

        sqlx::query_as("SELECT * FROM xp WHERE guild_id = $1 AND user_id = $2")
            .bind(self.0)
            .bind(user_id)
            .fetch_optional(ex)
            .await
            .map(|user| {
                user.unwrap_or(Record {
                    guild_id: self.0,
                    user_id,
                    score: 0,
                })
            })
    }

    /// Gives (or takes away) some experience to a user.
    pub async fn add<'a, E>(&self, ex: E, user_id: UserId, score: i32) -> Result<(), Error>
    where
        E: Executor<'a, Database = Postgres> + Clone,
    {
        let user_id = user_id.0 as i64;

        let res = sqlx::query(
            r#"
            UPDATE xp 
            SET score = score + $3 
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(self.0)
        .bind(user_id)
        .bind(score)
        .execute(ex.clone())
        .await?;

        // if no records were updated, the user record doesn't exist!
        if res.rows_affected() == 0 {
            // insert it instead
            sqlx::query(
                r#"
                INSERT INTO xp (guild_id, user_id, score)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(self.0)
            .bind(user_id)
            .bind(score)
            .execute(ex)
            .await?;
        }

        Ok(())
    }
}

/// Calculates the level for a given score.
pub fn level(score: i32) -> i32 {
    (score as f64 / 30.).sqrt() as i32 + 1
}
