//! Diminishing "experience" tracking services.

use crate::model::xp::{Guild, Record};
use crate::service::{Error, Service, Context};
use crate::command::chat::Arguments;
use crate::impl_service;

use std::fmt::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::Message;
use twilight_model::gateway::event::Event;
use twilight_model::id::{GuildId, UserId};

use anyhow::anyhow;

/// Experience awarding service.
#[derive(Default, Clone)]
pub struct Xp(Arc<DashMap<(GuildId, UserId), Instant>>);

impl Xp {
    /// Maximum experience a user can be awarded at once.
    pub const MAX_EXP: i32 = 15;

    /// Updates a [`Cooldown`] in the cooldowns table, returning a good amount
    /// of exp to reward.
    pub fn cooldown_update(&self, guild_id: GuildId, user_id: UserId) -> i32 {
        let idx = (guild_id, user_id);
        let now = Instant::now();

        // swap instants
        match self.0.insert(idx, now) {
            Some(old) => match now.checked_duration_since(old) {
                Some(duration) => exp(duration),
                // this should not happen, but just in case.
                None => 0,
            },
            None => Xp::MAX_EXP,
        }
    }

    /// Handles a message.
    pub async fn process(&self, cx: &Context, msg: &Message) -> Result<(), Error> {
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
        let exp = self.cooldown_update(guild_id, user_id);

        // add experience to the user
        Guild::new(guild_id).add(cx.db(), user_id, exp).await?;

        Ok(())
    }
}

impl_service! {
    impl Service for Xp {
        async fn handle(&self, cx: &Context, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::MessageCreate(msg) => self.process(cx, msg).await,
                _ => Ok(()),
            }
        }
    }
}

fn exp(duration: Duration) -> i32 {
    // get exp from duration
    let exp = duration.as_secs() as i32;

    // clamp exp
    exp.min(Xp::MAX_EXP)
}

/// Service that enables the `/rank` command.
///
/// ```txt
/// /rank - Gets the level and amount of experience a user has accumulated.
///     [user] - The user to check. If omitted, defaults to the user.
/// ```
#[derive(Default, Clone)]
pub struct RankCommand;

impl RankCommand {
    async fn command(&self, cx: &Context, command: Arguments<'_>) -> Result<(), Error> {
        // get guild id
        let guild_id = command.guild_id().ok_or(anyhow!("guild_id is missing"))?;

        // get the user_id
        let user_id = match command.get_string("user")? {
            Some(id) => id.parse::<u64>().map(UserId)?,
            None => command.user_id(),
        };

        // finally.... finally... find the exp for the specified user
        let user = Guild::new(guild_id).get(cx.db(), user_id).await?;

        // create a response
        let content = format!(
            "user <@{}> is level {} with {}KR",
            user_id,
            user.level(),
            user.score(),
        );

        command
            .respond()
            .content(content)
            .exec(cx.http())
            .await?;

        Ok(())
    }
}

impl_service! {
    impl Service for RankCommand {
        async fn handle(&self, cx: &Context, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        let args = Arguments::new(&*cmd);

                        if args.name() == "rank" {
                            return self.command(cx, args).await;
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

/// Returns the exp leaders of a guild.
#[derive(Default, Clone)]
pub struct TopCommand;

impl TopCommand {
    async fn command(&self, cx: &Context, command: Arguments<'_>) -> Result<(), Error> {
        // get guild id and role id
        let guild_id = command.guild_id().ok_or(anyhow!("guild_id is missing"))?;

        // get the top listing
        let top = Guild::new(guild_id).top(cx.db(), 10, 0).await?;

        // create a response
        let content = create_top_message(&top);

        command
            .respond()
            .content(content)
            .exec(cx.http())
            .await?;

        Ok(())
    }
}

impl_service! {
    impl Service for TopCommand {
        async fn handle(&self, cx: &Context, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        let args = Arguments::new(&*cmd);

                        if args.name() == "top" {
                            return self.command(cx, args).await;
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

fn create_top_message(top: &[Record]) -> String {
    if top.len() > 0 {
        let mut content = String::new();

        for (i, record) in top.into_iter().enumerate() {
            if i > 0 {
                content.push('\n')
            }

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
