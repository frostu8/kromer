//! A powerful Discord bot written in Rust and powered by Twilight.
//!
//! Discord is an open, free platform. So why should we continue paying to make
//! our Discord servers the best they can be? A good bot should be open and
//! free, just like the platform, and that's what this aims to be.

#![feature(type_alias_impl_trait)]

pub mod bot;
pub mod model;
pub mod service;

#[macro_use]
extern crate log;
