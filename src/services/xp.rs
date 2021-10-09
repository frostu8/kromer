//! Diminishing "experience" tracking services.

use sqlx::postgres::PgPool;

use crate::model::xp::{Guild, Record};

use std::time::{Instant, Duration};
use std::sync::Arc;
use std::fmt::Write;

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
        // do not track bot messages
        if msg.author.bot {
            return Ok(());
        }

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

    async fn command(&self, command: &ApplicationCommand) -> Result<(), Error> {
        // get guild id
        let guild_id = command.guild_id
            .ok_or(anyhow!("guild_id is missing"))?;

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
        let content = format!(
            "user <@{}> is level {} with {}KR", 
            user_id, 
            user.level(), 
            user.score(),
        );

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
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        if cmd.data.name.as_str() == "rank" {
                            if let Err(err) = self.command(&*cmd).await {
                                error!("error /rank: {}", err);
                            }
                        }
                    }
                    _ => ()
                }
                _ => ()
            }
        })
    }
}

/// Returns the exp leaders of a guild.
#[derive(Clone)]
pub struct TopCommand {
    db: PgPool,
    client: Client,
}

impl TopCommand {
    pub fn new(db: PgPool, client: Client) -> TopCommand {
        TopCommand { db, client }
    }

    async fn command(&self, command: &ApplicationCommand) -> Result<(), Error> {
        // get guild id and role id
        let guild_id = command.guild_id
            .ok_or(anyhow!("guild_id is missing"))?;

        // get the top listing
        let top = Guild::new(guild_id).top(&self.db, 10, 0).await?;

        // create a response
        let content = create_top_message(&top);

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

impl Service for TopCommand {
    /// Handles an event.
    fn handle<'f>(&'f self, ev: &'f Event) -> ServiceFuture<'f> {
        Box::pin(async move {
            match ev {
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        if cmd.data.name.as_str() == "top" {
                            if let Err(err) = self.command(&*cmd).await {
                                error!("error /top: {}", err);
                            }
                        }
                    }
                    _ => ()
                }
                _ => ()
            }
        })
    }
}

fn create_top_message(top: &[Record]) -> String {
    if top.len() > 0 {
        let mut content = String::new();

        for (i, record) in top.into_iter().enumerate() {
            if i > 0 { content.push('\n') }

            write!(
                content, 
                "{} {}KR > <@{}> ", 
                top_emoji(i), 
                record.score(), 
                record.user_id()
            )
                .unwrap();
        }

        content
    } else {
        // fallback in case there is no records
        String::from("NO MEMBERS HAVE SPOKEN SINCE I JOINED!!!")
    }
}

fn top_emoji(idx: usize) -> char {
    match idx {
        0 => '🥇',
        1 => '🥈',
        2 => '🥉',
        _ => '💴',
    }
}

