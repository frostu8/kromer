//! Reaction role services.

use crate::model::roles::reaction::ReactionRole;
use crate::model::Emoji;
use crate::service::{Error, Service};
use crate::impl_service;

use sqlx::postgres::PgPool;

use std::future::Future;

use twilight_http::api_error::{ApiError, ErrorCode};
use twilight_http::request::AuditLogReason;
use twilight_http::Client;
use twilight_model::channel::Reaction;
use twilight_model::gateway::event::Event;

/// Reaction role service.
#[derive(Clone)]
pub struct ReactionRoles {
    db: PgPool,
    client: Client,
}

impl ReactionRoles {
    pub fn new(db: PgPool, client: Client) -> ReactionRoles {
        ReactionRoles { db, client }
    }

    async fn reaction_add(&self, reaction: &Reaction) -> Result<(), Error> {
        let guild_id = match reaction.guild_id {
            Some(id) => id,
            // if we are not in a guild, silently discard the reaction event
            None => return Ok(()),
        };

        match self.get_reaction_role(reaction).await? {
            // this is a reaction for a role!
            Some(rr) => {
                let res = self
                    .client
                    .add_guild_member_role(guild_id, reaction.user_id, rr.role_id())
                    .reason("reaction role add")?
                    .exec()
                    .await;

                match res {
                    Ok(_) => Ok(()),
                    Err(err) => match err.kind() {
                        twilight_http::error::ErrorType::Response {
                            error: ApiError::General(api_err),
                            ..
                        } => match api_err.code {
                            // silently discard permissions lacking errors
                            ErrorCode::PermissionsLacking => Ok(()),
                            _ => Err(err.into()),
                        },
                        _ => Err(err.into()),
                    },
                }
            }
            // this is just a normal reaction
            None => Ok(()),
        }
    }

    async fn reaction_remove(&self, reaction: &Reaction) -> Result<(), Error> {
        let guild_id = match reaction.guild_id {
            Some(id) => id,
            // if we are not in a guild, silently discard the reaction event
            None => return Ok(()),
        };

        match self.get_reaction_role(reaction).await? {
            // this is a reaction for a role!
            Some(rr) => {
                let res = self
                    .client
                    .remove_guild_member_role(guild_id, reaction.user_id, rr.role_id())
                    .reason("reaction role remove")?
                    .exec()
                    .await;

                match res {
                    Ok(_) => Ok(()),
                    Err(err) => match err.kind() {
                        twilight_http::error::ErrorType::Response {
                            error: ApiError::General(api_err),
                            ..
                        } => match api_err.code {
                            // silently discard permissions lacking errors
                            ErrorCode::PermissionsLacking => Ok(()),
                            _ => Err(err.into()),
                        },
                        _ => Err(err.into()),
                    },
                }
            }
            // this is just a normal reaction
            None => Ok(()),
        }
    }

    async fn get_reaction_role(
        &self,
        reaction: &Reaction,
    ) -> Result<Option<ReactionRole>, sqlx::Error> {
        let message_id = reaction.message_id;
        let emoji: Emoji = reaction.emoji.clone().into();

        // find the related reaction role
        ReactionRole::get(&self.db, message_id, emoji).await
    }
}

impl_service! {
    impl Service for ReactionRoles {
        async fn handle(&self, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::ReactionAdd(reaction) => self.reaction_add(reaction).await,
                Event::ReactionRemove(reaction) => self.reaction_remove(reaction).await,
                _ => Ok(()),
            }
        }
    }
}

