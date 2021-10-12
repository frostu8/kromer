//! Info commands.

use crate::service::{Error, Service, Context};
use crate::command::chat::Arguments;
use crate::impl_service;

use twilight_model::application::{
    callback::{CallbackData, InteractionResponse},
    component::{button::{Button, ButtonStyle}, action_row::ActionRow, Component},
    interaction::Interaction,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::event::Event;

/// The `/info` command.
#[derive(Default, Clone)]
pub struct InfoCommand;

impl InfoCommand {
    async fn command(&self, cx: &Context, command: Arguments<'_>) -> Result<(), Error> {
        // we don't really care about anything about the command besides the
        // id and token so we can respond.

        let response = InteractionResponse::ChannelMessageWithSource(self.make_info_response(cx));

        cx
            .http()
            .interaction_callback(command.id(), command.token(), &response)
            .exec()
            .await?;

        Ok(())
    }

    fn make_info_response(&self, cx: &Context) -> CallbackData {
        let content = format!("running kromer {} ({})", crate::VERSION, crate::GIT_HASH);

        let mut buttons = Vec::new();

        buttons.push(Component::Button(Button {
            style: ButtonStyle::Link,
            label: Some(String::from("Invite")),
            url: Some(crate::bot::invite_link(cx.application_id())),
            disabled: false,
            custom_id: None,
            emoji: None,
        }));

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
        async fn handle(&self, cx: &Context, ev: &Event) -> Result<(), Error> {
            match ev {
                Event::InteractionCreate(int) => match &int.0 {
                    Interaction::ApplicationCommand(cmd) => {
                        let args = Arguments::new(&*cmd);

                        if args.name() == "info" {
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

