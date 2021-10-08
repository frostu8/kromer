//! Diminishing "experience" tracking services.

use sqlx::postgres::PgPool;

use crate::model::xp::Guild;

use std::time::{Instant, Duration};
use std::sync::Arc;

use dashmap::DashMap;

use super::{Error, Service, ServiceFuture};

use twilight_model::channel::message::{Message, allowed_mentions::AllowedMentions};
use twilight_model::gateway::event::Event;
use twilight_model::id::{GuildId, UserId};
use twilight_model::application::{
    callback::{InteractionResponse, CallbackData},
    interaction::{
        application_command::{ApplicationCommand, CommandDataOption},
        Interaction, 
    },
};
use twilight_http::Client;

use anyhow::anyhow;

/// Experience awarding service.
#[derive(Clone)]
pub struct Xp {
    db: PgPool,
    cooldowns: Cooldowns,
}

impl Xp {
    pub fn new(db: PgPool) -> Xp {
        Xp {
            db,
            cooldowns: Cooldowns::new(),
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

        // figure out how much exp to award to the user
        let exp = self.cooldowns.update(guild_id, user_id);

        // add experience to the user
        Guild::new(guild_id).add(&self.db, user_id, exp).await?;

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

/// Maximum experience a user can be awarded at once.
pub const MAX_EXP: i32 = 15;

/// A table of cooldowns.
#[derive(Clone, Default)]
pub struct Cooldowns(Arc<DashMap<CooldownIndex, Instant>>);

impl Cooldowns {
    /// Create a new `Cooldowns`.
    pub fn new() -> Cooldowns {
        Cooldowns::default()
    }

    /// Updates a [`Cooldown`] in the cooldowns table, returning a good amount
    /// of exp to reward.
    pub fn update(&self, guild_id: GuildId, user_id: UserId) -> i32 {
        let idx = CooldownIndex(guild_id, user_id);
        let now = Instant::now();

        // swap instants
        match self.0.insert(idx, now) {
            Some(old) => match now.checked_duration_since(old) {
                Some(duration) => exp(duration),
                // this should not happen, but just in case.
                None => 0,
            }
            None => MAX_EXP
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct CooldownIndex(GuildId, UserId);

fn exp(duration: Duration) -> i32 {
    // get exp from duration
    let exp = duration.as_secs() as i32;

    // clamp exp
    exp.min(MAX_EXP)
}

/// Service that enables the `/rank` command.
///
/// ```txt
/// /rank - Gets the level and amount of experience a user has accumulated.
///     [user] - The user to check. If omitted, defaults to the user.
/// ```
#[derive(Clone)]
pub struct RankCommand {
    db: PgPool,
    client: Client,
}

impl RankCommand {
    pub fn new(db: PgPool, client: Client) -> RankCommand {
        RankCommand { db, client }
    }

    async fn handle_interaction(&self, int: &Interaction) -> Result<(), Error> {
        // only handle commands
        let command = match int {
            Interaction::ApplicationCommand(command) => command,
            // ignore any other interactions
            _ => return Ok(()),
        };

        // figure out what command this is
        match command.data.name.as_str() {
            "rank" => self.rank_command(command).await,
            _ => Ok(()),
        }
    }

    async fn rank_command(&self, command: &ApplicationCommand) -> Result<(), Error> {
        // get guild id and role id
        let guild_id = command.guild_id
            .ok_or(anyhow!("guild_id is missing for /rank"))?;

        // get the user_id
        let user_id = command.data.options
            .iter()
            .find(|option| option.name() == "user")
            .map(|user| match user {
                CommandDataOption::String { value, .. } => {
                    value.parse::<u64>()
                        .map(UserId)
                        .map_err(From::from)
                }
                _ => Err(anyhow!("user option is not valid type")),
            })
            .unwrap_or_else(|| {
                command.member.as_ref()
                    .and_then(|member| member.user.as_ref())
                    .map(|user| user.id)
                    .ok_or(anyhow!("member missing for /rank"))
            })?;

        // finally.... finally... find the exp for the specified user
        let user = Guild::new(guild_id).get(&self.db, user_id).await?;

        // create a response
        let content = format!("user <@{}> is level {} with {} exp", user_id, user.level(), user.score());

        let response = CallbackData {
            content: Some(content),
            allowed_mentions: Some(AllowedMentions::default()),
            components: None,
            embeds: Vec::new(),
            flags: None,
            tts: None,
        };

        let response = InteractionResponse::ChannelMessageWithSource(response);

        self.client
            .interaction_callback(command.id, &command.token, &response)
            .exec()
            .await?;

        Ok(())
    }
}

impl Service for RankCommand {
    /// Handles an event.
    fn handle<'f>(&'f self, ev: &'f Event) -> ServiceFuture<'f> {
        Box::pin(async move {
            match ev {
                Event::InteractionCreate(int) => {
                    if let Err(e) = self.handle_interaction(int).await {
                        error!("{}", e);
                    }
                }
                _ => ()
            }
        })
    }
}

