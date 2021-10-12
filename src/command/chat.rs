//! Types to make chat commands less of a pain in the butt.

use twilight_model::id::{InteractionId, GuildId, UserId};
use twilight_model::application::interaction::application_command::{
    CommandDataOption, ApplicationCommand,
};

use super::{Response, ResponseType};

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;

/// An easy way to index into a chat input interaction's arguments.
pub struct Arguments<'a> {
    top: &'a ApplicationCommand,
    options: &'a [CommandDataOption]
}

impl<'a> Arguments<'a> {
    /// Create a new `Arguments`.
    pub fn new(top: &'a ApplicationCommand) -> Arguments<'a> {
        Arguments {
            top,
            options: &top.data.options,
        }
    }

    /// The name of the command.
    pub fn name(&self) -> &'a str {
        &self.top.data.name
    }

    /// The interaction's token.
    pub fn token(&self) -> &'a str {
        &self.top.token
    }

    /// The id of the interaction.
    pub fn id(&self) -> InteractionId {
        self.top.id
    }

    /// The guild id of the interaction.
    pub fn guild_id(&self) -> Option<GuildId> {
        self.top.guild_id
    }

    /// The id of the user that executed the interaction.
    ///
    /// # Panics
    /// Panics if both `member` and `user` are missing.
    pub fn user_id(&self) -> UserId {
        self.top.member.as_ref()
            .and_then(|member| member.user.as_ref())
            .or(self.top.user.as_ref())
            .map(|user| user.id)
            .expect("both `member` and `user` are missing!")
    }

    /// Gets a subcommand, if it exists.
    pub fn get_subcommand(&self, name: &str) -> Result<Option<Arguments<'a>>, ArgError> {
        self.get(name)
            .map(|s| match s {
                CommandDataOption::SubCommand { options, .. } => Ok(Arguments {
                    top: self.top,
                    options,
                }),
                opt => Err(ArgError::InvalidType(opt.kind()))
            })
            .transpose()
    }

    /// Gets a string argument.
    pub fn get_string(&self, name: &str) -> Result<Option<&'a str>, ArgError> {
        self.get(name)
            .map(|s| match s {
                CommandDataOption::String { value, .. } => Ok(value.as_ref()),
                opt => Err(ArgError::InvalidType(opt.kind()))
            })
            .transpose()
    }

    /// Starts building a [`Response`].
    pub fn respond(&self) -> Response {
        Response::new(self.top.id, &self.top.token, ResponseType::Initial)
    }

    /// Starts building a [`Response`] for a followup.
    pub fn followup(&self) -> Response {
        Response::new(self.top.id, &self.top.token, ResponseType::Followup)
    }

    fn get(&self, name: &str) -> Option<&'a CommandDataOption> {
        self
            .options
            .iter()
            .find(|option| option.name() == name)
    }
}

/// An error returned by any of the `Arguments::get_*` functions.
#[derive(Debug)]
pub enum ArgError {
    InvalidType(&'static str),
    ParseInt(ParseIntError),
    Unresolved(u64),
}

impl Display for ArgError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ArgError::InvalidType(ty) => write!(f, "unexpected type {}", ty),
            ArgError::ParseInt(err) => Display::fmt(err, f),
            ArgError::Unresolved(id) => write!(f, "unresolved id: {}", id),
        }
    }
}

impl Error for ArgError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ArgError::ParseInt(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ParseIntError> for ArgError {
    fn from(err: ParseIntError) -> ArgError {
        ArgError::ParseInt(err)
    }
}

