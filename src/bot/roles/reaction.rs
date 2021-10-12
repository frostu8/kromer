//! Reaction role services.

use crate::command::chat::Arguments;
use crate::impl_service;
use crate::model::roles::reaction::{Message, ReactionRole};
use crate::model::Emoji;
use crate::service::{Context, Error, Service};

use twilight_http::api_error::{ApiError, ErrorCode};
use twilight_http::request::AuditLogReason;

use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Reaction;
use twilight_model::gateway::event::Event;
use twilight_model::id::RoleId;

use twilight_mention::Mention;

use tokio::select;
use tokio::time::sleep;

use std::time::Duration;

use anyhow::anyhow;

/// Reaction role service.
#[derive(Default, Clone)]
pub struct ReactionRoles;

impl ReactionRoles {
    async fn reaction_add(&self, cx: &Context, reaction: &Reaction) -> Result<(), Error> {
        let guild_id = match reaction.guild_id {
            Some(id) => id,
            // if we are not in a guild, silently discard the reaction event
            None => return Ok(()),
        };

        match self.get_reaction_role(cx, reaction).await? {
            // this is a reaction for a role!
            Some(rr) => {
                let res = cx
                    .http()
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

    async fn reaction_remove(&self, cx: &Context, reaction: &Reaction) -> Result<(), Error> {
        let guild_id = match reaction.guild_id {
            Some(id) => id,
            // if we are not in a guild, silently discard the reaction event
            None => return Ok(()),
        };

        match self.get_reaction_role(cx, reaction).await? {
            // this is a reaction for a role!
            Some(rr) => {
                let res = cx
                    .http()
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
        cx: &Context,
        reaction: &Reaction,
    ) -> Result<Option<ReactionRole>, sqlx::Error> {
        let message_id = reaction.message_id;
        let emoji: Emoji = reaction.emoji.clone().into();

        // find the related reaction role
        ReactionRole::get(cx.db(), message_id, emoji).await
    }
}

impl_service! {
    impl Service for ReactionRoles {
        async fn handle(&self, cx: &Context, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::ReactionAdd(reaction) => self.reaction_add(cx, reaction).await,
                Event::ReactionRemove(reaction) => self.reaction_remove(cx, reaction).await,
                _ => Ok(()),
            }
        }
    }
}

/// Allows easy creation of reaction roles.
#[derive(Default, Clone)]
pub struct CreateReactionRole;

impl CreateReactionRole {
    async fn command(&self, cx: &Context, command: Arguments<'_>) -> Result<(), Error> {
        let guild_id = match command.guild_id() {
            Some(guild_id) => guild_id,
            None => return Err(anyhow!("guild_id is missing")),
        };

        let user_id = command.user_id();

        let role_id = command
            .get_string("role")?
            .ok_or(anyhow!("role is missing for /reactionroles add!"))?
            .parse::<u64>()
            .map(RoleId)?;

        // create a response
        command
            .respond()
            .content(
                "react with the emoji of your choice to the message of your \
                 choice to set up the reaction role!. ⚠️ this will expire in \
                 a minute!",
            )
            .ephemeral()
            .exec(cx.http())
            .await?;

        // wait for a reaction...
        let reaction = cx.wait_for(guild_id, move |event: &Event| match event {
            Event::ReactionAdd(reaction) => reaction.0.user_id == user_id,
            _ => false,
        });

        // ...or the timeout
        select! {
            biased;
            _ = sleep(Duration::from_secs(60)) => {
                // send expiration message
                command
                    .followup()
                    .content("request has expired! try `/reactionroles add` again to continue")
                    .ephemeral()
                    .exec(cx.http())
                    .await?;
            }
            event = reaction => {
                let reaction = match event? {
                    Event::ReactionAdd(reaction) => reaction,
                    _ => unreachable!(),
                };

                let emoji = reaction.emoji.clone().into();

                // cool! we now have everything needed to create a rr!
                let message = Message::new(
                    guild_id,
                    reaction.message_id,
                    reaction.channel_id,
                );

                let res = message.create(cx.db(), role_id, emoji).await;

                match res {
                    Ok(_) => {
                        let content = format!(
                            "reaction role set up!\n\
                             i will now give the {} role to anyone who reacts \
                             with {} to that message!",
                            role_id.mention(),
                            emoji,
                        );

                        command
                            .followup()
                            .content(content)
                            .ephemeral()
                            .exec(cx.http())
                            .await?;
                    }
                    Err(err) if err.exists() => {
                        // get the existing reaction role
                        let rr = ReactionRole::get(cx.db(), reaction.message_id, emoji)
                            .await?
                            .expect("db told us a RR already exists, but we can't find it!");

                        let content = format!(
                            "a reaction role that gives {} has already been \
                             set up for the emoji {}! try removing it first!",
                            rr.role_id().mention(),
                            emoji,
                        );

                        command
                            .followup()
                            .content(content)
                            .ephemeral()
                            .exec(cx.http())
                            .await?;
                    }
                    Err(err) => return Err(err.into())
                }
            }
        }

        Ok(())
    }
}

impl_service! {
    impl Service for CreateReactionRole {
        async fn handle(&self, cx: &Context, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        let args = Arguments::new(&*cmd);

                        if args.name() == "reactionroles" {
                            match args.get_subcommand("add")? {
                                Some(args) => return self.command(cx, args).await,
                                None => (),
                            }
                        }
                    }
                    _ => (),
                },
                _ => (),
            }

            Ok(())
        }
    }
}
