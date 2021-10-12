//! Info commands.

use crate::service::{Error, Service};
use crate::command::chat::Arguments;
use crate::impl_service;

use twilight_http::Client;
use twilight_model::application::{
    callback::{CallbackData, InteractionResponse},
    component::{button::{Button, ButtonStyle}, action_row::ActionRow, Component},
    interaction::Interaction,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::event::Event;

/// The `/info` command.
#[derive(Clone)]
pub struct InfoCommand {
    client: Client,
}

impl InfoCommand {
    pub fn new(client: Client) -> InfoCommand {
        InfoCommand { client }
    }
    
    async fn command(&self, command: Arguments<'_>) -> Result<(), Error> {
        // we don't really care about anything about the command besides the
        // id and token so we can respond.

        let response = InteractionResponse::ChannelMessageWithSource(self.make_info_response());

        self.client
            .interaction_callback(command.id(), command.token(), &response)
            .exec()
            .await?;

        Ok(())
    }

    fn make_info_response(&self) -> CallbackData {
        let content = format!("running kromer {} ({})", crate::VERSION, crate::GIT_HASH);

        let mut buttons = Vec::new();

        if let Some(id) = self.client.application_id() {
            buttons.push(Component::Button(Button {
                style: ButtonStyle::Link,
                label: Some(String::from("Invite")),
                url: Some(crate::bot::invite_link(id)),
                disabled: false,
                custom_id: None,
                emoji: None,
            }));
        }

        buttons.push(Component::Button(Button {
            style: ButtonStyle::Link,
            label: Some(String::from("Github")),
            url: Some(String::from(crate::GIT_REPOSITORY)),
            disabled: false,
            custom_id: None,
            emoji: None,
        }));

        CallbackData {
            content: Some(content),
            allowed_mentions: None,
            components: Some(
                std::iter::once(Component::ActionRow(ActionRow {
                    components: buttons,
                }))
                    .collect()
            ),
            embeds: Vec::new(),
            flags: Some(MessageFlags::EPHEMERAL),
            tts: None,
        }
    }
}

impl_service! {
    impl Service for InfoCommand {
        async fn handle(&self, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        let args = Arguments::new(&*cmd);

                        if args.name() == "info" {
                            return self.command(args).await;
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

