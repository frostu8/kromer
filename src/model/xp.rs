//! User experience.

use super::Error;

use sqlx::{Executor, Row, sqlite::Sqlite};

use twilight_model::id::{GuildId, UserId};

/// A user's experience.
#[derive(Debug)]
pub struct User {
    guild_id: u64,
    user_id: u64,
    score: i32,
}

impl User {
    /// The id of the guild this record reflects.
    pub fn guild_id(&self) -> GuildId {
        GuildId(self.guild_id)
    }

    /// The id of the user.
    pub fn user_id(&self) -> UserId {
        UserId(self.user_id)
    }

    /// How much experience the user has.
    pub fn score(&self) -> i32 {
        self.score
    }

    /// What level the user is at.
    pub fn level(&self) -> i32 {
        level(self.score)
    }

    /// Gives (or takes away) some experience to a user.
    pub async fn add_score<'a, E>(
        ex: E, 
        guild_id: u64, 
        user_id: u64, 
        score: i32
    ) -> Result<(), Error>
    where
        E: Executor<'a, Database = Sqlite> + Clone
    {
        let res = sqlx::query(
            r#"
            UPDATE xp 
            SET score = score + $3 
            WHERE guild_id = $1 AND user_id = $2
            "#
        )
            .bind(guild_id as i64)
            .bind(user_id as i64)
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
                "#
            )
                .bind(guild_id as i64)
                .bind(user_id as i64)
                .bind(score)
                .execute(ex)
                .await?;
        }

        Ok(())
    }

    /// Gets a user's experience level.
    ///
    /// If a row doesn't exist, it will return a `User` with zero xp.
    pub async fn get<'a, E>(
        ex: E, 
        guild_id: u64,
        user_id: u64, 
    ) -> Result<User, Error> 
    where
        E: Executor<'a, Database = Sqlite>
    {
        sqlx::query("SELECT * FROM xp WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id as i64)
            .bind(user_id as i64)
            .fetch_optional(ex)
            .await
            .and_then(|row| match row {
                Some(row) => {
                    Ok(User {
                        guild_id: row.try_get::<i64, _>("guild_id")? as u64,
                        user_id: row.try_get::<i64, _>("user_id")? as u64,
                        score: row.try_get("score")?,
                    })
                }
                None => Ok(User {
                    user_id,
                    guild_id,
                    score: 0,
                })
            })
    }
}

/// Calculates the level for a given score.
pub fn level(score: i32) -> i32 {
    (score as f64 / 30.).sqrt() as i32 + 1
}

