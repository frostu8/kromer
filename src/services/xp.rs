//! Diminishing "experience" tracking services.

use sqlx::sqlite::SqlitePool;

use crate::model::xp::User;

use super::{Error, Service, ServiceFuture};

use twilight_model::channel::message::{Message, allowed_mentions::AllowedMentions};
use twilight_model::gateway::event::Event;
use twilight_model::id::UserId;
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
            guild_id,
            user_id,
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

/// Service that enables the `/rank` command.
///
/// ```txt
/// /rank - Gets the level and amount of experience a user has accumulated.
///     [user] - The user to check. If omitted, defaults to the user.
/// ```
#[derive(Clone)]
pub struct RankCommand {
    db: SqlitePool,
    client: Client,
}

impl RankCommand {
    pub fn new(db: SqlitePool, client: Client) -> RankCommand {
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
        let user = User::get(&self.db, guild_id, user_id).await?;

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

