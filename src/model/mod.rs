//! Bot storage models supported by [`sqlx`].

pub mod roles;
pub mod xp;

pub use sqlx::Error;

use sqlx::{
    database::{HasArguments, HasValueRef},
    encode::IsNull,
    migrate::{Migrate, MigrateError},
    types::Type,
    Acquire, Database, Decode, Encode,
};

use twilight_model::channel::ReactionType;

use std::fmt::{self, Debug, Formatter};
use std::mem;
use std::ops::Deref;

/// Stores emojis in SQL records.
///
/// Because a Discord emoji may also be custom, it doesn't make sense to store
/// the codepoint as an INTEGER in your SQL database of choice. Instead, we use
/// a BIGINT and switch between the last bit.
pub enum Emoji {
    Unicode(char),
    Custom(u64),
}

impl Emoji {
    const CUSTOM_BIT: usize = mem::size_of::<i64>() - 1;

    fn as_i64(&self) -> i64 {
        match self {
            Emoji::Custom(id) => *id as i64 | (1 << Self::CUSTOM_BIT),
            Emoji::Unicode(ch) => *ch as i64,
        }
    }
}

impl Debug for Emoji {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.as_i64())
        } else {
            match self {
                Emoji::Unicode(ch) => f.debug_tuple("Emoji::Unicode").field(ch).finish(),
                Emoji::Custom(id) => f.debug_tuple("Emoji::Unicode").field(id).finish(),
            }
        }
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Emoji
where
    i64: Decode<'r, DB>,
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<Emoji, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let data = i64::decode(value)? as u64;

        // get the custom bit
        let is_custom = (data >> Self::CUSTOM_BIT) & 1 > 0;

        if is_custom {
            // return the rest of the data as an emoji id
            Ok(Emoji::Custom(data & !(1 << Self::CUSTOM_BIT)))
        } else {
            // decode it as a u32 codepoint
            Ok(Emoji::Unicode(
                char::from_u32(data as u32).ok_or("codepoint invalid")?,
            ))
        }
    }
}

impl<'q, DB: Database> Encode<'q, DB> for Emoji
where
    i64: Encode<'q, DB>,
{
    fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
        self.as_i64().encode(buf)
    }

    fn produces(&self) -> Option<<DB as Database>::TypeInfo> {
        self.as_i64().produces()
    }

    fn size_hint(&self) -> usize {
        self.as_i64().size_hint()
    }
}

impl<DB: Database> Type<DB> for Emoji
where
    i64: Type<DB>,
{
    fn type_info() -> <DB as Database>::TypeInfo {
        i64::type_info()
    }

    fn compatible(ty: &<DB as Database>::TypeInfo) -> bool {
        i64::compatible(ty)
    }
}

/// The [`ReactionType`] provided, if it is a `Unicode` reaction type, should
/// have a name field with at least 1 character in it (the emoji in question).
impl From<ReactionType> for Emoji {
    fn from(r: ReactionType) -> Emoji {
        match r {
            ReactionType::Unicode { name, .. } => {
                // pull the unicode character out
                Emoji::Unicode(name.chars().next().unwrap())
            }
            ReactionType::Custom { id, .. } => Emoji::Custom(id.0),
        }
    }
}

/// Runs migrations.
pub async fn migrate<'a, E>(ex: E) -> Result<(), MigrateError>
where
    E: Acquire<'a>,
    <<E as Acquire<'a>>::Connection as Deref>::Target: Migrate,
{
    sqlx::migrate!().run(ex).await
}
