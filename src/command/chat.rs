//! Types to make chat commands less of a pain in the butt.

use twilight_model::application::interaction::application_command::{
    CommandDataOption, CommandData,
};

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;

/// An easy way to index into a chat input interaction's arguments.
pub struct Arguments<'a> {
    top: &'a CommandData,
    options: &'a [CommandDataOption]
}

impl<'a> Arguments<'a> {
    /// Create a new `Arguments`.
    pub fn new(top: &'a CommandData) -> Arguments<'a> {
        Arguments {
            top,
            options: &top.options,
        }
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

