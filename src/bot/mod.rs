//! The actual services used by the bot.

pub mod roles;
pub mod info;
pub mod xp;

use twilight_model::id::ApplicationId;

/// Generates a bot invite link.
pub fn invite_link(id: ApplicationId) -> String {
    format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&permissions=0&scope=bot%20applications.commands",
        id.0,
    )
}

