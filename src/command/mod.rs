//! Command utilities.

pub mod chat;

use twilight_model::application::callback::{CallbackData, InteractionResponse};
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::id::InteractionId;

use twilight_http::Client;

use anyhow::Error;

/// A response to an interaction.
pub struct Response<'a> {
    id: InteractionId,
    token: &'a str,
    ty: ResponseType,
    data: CallbackData,
}

impl<'a> Response<'a> {
    /// Creates a new response.
    pub fn new(id: InteractionId, token: &'a str, ty: ResponseType) -> Self {
        Response {
            id,
            token,
            ty,
            data: CallbackData {
                allowed_mentions: Some(AllowedMentions::default()),
                components: None,
                content: None,
                embeds: Vec::new(),
                flags: None,
                tts: None,
            },
        }
    }

    /// Sets the response's content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.data.content = Some(content.into());
        self
    }

    /// Marks the response as ephemeral.
    pub fn ephemeral(mut self) -> Self {
        *self.data.flags.get_or_insert(MessageFlags::empty()) |= MessageFlags::EPHEMERAL;
        self
    }

    /// Sends the response.
    pub async fn exec(self, client: &Client) -> Result<(), Error> {
        match self.ty {
            ResponseType::Initial => self.exec_initial(client).await,
            ResponseType::Followup => self.exec_followup(client).await,
        }
    }

    async fn exec_initial(self, client: &Client) -> Result<(), Error> {
        let response = InteractionResponse::ChannelMessageWithSource(self.data);

        client
            .interaction_callback(self.id, self.token, &response)
            .exec()
            .await
            .map(|_| ())
            .map_err(From::from)
    }

    async fn exec_followup(self, client: &Client) -> Result<(), Error> {
        let mut req = client.create_followup_message(self.token)?;

        if let Some(content) = self.data.content.as_ref() {
            req = req.content(content);
        }

        if let Some(flags) = self.data.flags {
            req = req.ephemeral(flags.contains(MessageFlags::EPHEMERAL));
        }

        req.exec().await.map(|_| ()).map_err(From::from)
    }
}

pub enum ResponseType {
    Initial,
    Followup,
}
