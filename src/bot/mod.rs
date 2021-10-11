//! The actual services used by the bot.

pub mod roles;
pub mod info;
pub mod xp;

use twilight_model::id::ApplicationId;
use twilight_http::Client;

/// Gets the application id associated with the token.
pub async fn fetch_application_id(
    client: &Client,
) -> Result<ApplicationId, anyhow::Error> {
    client.current_user_application()
        .exec()
        .await?
        .model()
        .await
        .map_err(From::from)
        .map(|app| app.id)
}

/// Generates a bot invite link.
pub fn invite_link(id: ApplicationId) -> String {
    format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&permissions=0&scope=bot%20applications.commands",
        id.0,
    )
}

