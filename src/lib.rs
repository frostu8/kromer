//! A powerful Discord bot written in Rust and powered by Twilight.
//!
//! Discord is an open, free platform. So why should we continue paying to make
//! our Discord servers the best they can be? A good bot should be open and
//! free, just like the platform, and that's what this aims to be.

#![feature(type_alias_impl_trait)]

pub mod bot;
pub mod command;
pub mod model;
pub mod service;

#[macro_use]
extern crate log;

/// The cargo package version of the bot.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The git commit hash of the bot.
pub const GIT_HASH: &str = env!("GIT_HASH");

/// The current repository mirror.
///
/// May or may not exist.
pub const GIT_REPOSITORY: &str = concat!("https://github.com/frostu8/kromer/tree/", env!("GIT_HASH"));

