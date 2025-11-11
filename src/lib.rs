#![deny(missing_docs)]

//! Library for Presley, a bot for tracking a Discord server's word counts for their user's
//! projects and assigning and removing ranks based on it.

pub mod word_count;
pub mod rank;
pub mod mock;
pub mod commands;
pub mod core;
pub mod report;
pub mod display;
pub mod user_registration;

type Context<'a> = poise::Context<'a, crate::core::GlobalCommandData, anyhow::Error>;
