//! Defines Discord slash commands

use anyhow::anyhow;
use poise::serenity_prelude::model::guild;
use poise::serenity_prelude::Role;
use poise::serenity_prelude as serenity;
use poise::Command;
use anyhow::Error;
use anyhow::Result;

use crate::rank::DiscordRank;
use crate::rank::Rank;
use crate::rank::RankId;
use crate::rank::RankList;
use crate::word_count::TotalWordCount;

type Context<'a> = poise::Context<'a, crate::core::GlobalCommandData, anyhow::Error>;

/// get_commands() returns a static list of all functions to be registered 
/// with the poise framework.
/// if you add a command, it needs to be added in here.
///
/// If being ran in debug, the debug commands will be added 
/// automatically.
///
/// Note that all commands have two generic types;
/// The first is the external data/state that is included with all commands
/// (see [GlobalCommandData]). The second is an error type,
/// we use [anyhow::Error] as our generic error type across all commands.
pub fn get_commands() -> Vec<Command<crate::core::GlobalCommandData, Error>>
{
    // Release commands go here
    let mut commands = vec![set_rank(), list_ranks()];
    // Add debug commands if in debug mode
    if cfg!(debug_assertions)
    {
        commands.append(&mut debug::get_debug_commands());
    }
    return commands;
}

#[poise::command(slash_command, guild_only, default_member_permissions = "ADMINISTRATOR")]
async fn set_rank(ctx: Context<'_>, role: serenity::Role, minimum_word_count: u32) -> Result<()>
{
    let pool = ctx.data().get_pool();
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;

    let mut ranks = RankList::load(pool, guild_id).await?;

    let new_rank = Rank::new(guild_id, role.id, minimum_word_count);
    let result = ranks.add_rank(new_rank);
    if let Err(err) = result
    {
        let guild = ctx.partial_guild().await.unwrap();
        let discord_error = err.to_discord_error(&guild).expect("Unable to get the role from the guild");
        return Err(discord_error.into())
    }
    ranks.save(pool).await?;

    ctx.say(format!("Added rank {}!", role)).await?;
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn list_ranks(ctx: Context<'_>) -> Result<()>
{
    let pool = ctx.data().get_pool();
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;

    let ranks = RankList::load(pool, guild_id).await?;
    let guild = ctx.partial_guild().await.unwrap();

    let mut response = String::new();
    let ranks: Vec<DiscordRank<Role>> = ranks.iter().map(|x| x.to_rank(&guild).unwrap()).collect();

    for rank in ranks
   {
        response.push_str(&format!("{}", rank));
    }

    ctx.say(response).await?;
    Ok(())
}

#[cfg(debug_assertions)]
pub mod debug {
    //! Special debug commands that will not be compiled and included in release mode.
    //! These commands are used for checking bot connection, sanity checks, 
    //! or registering slash commands per guild (see [register_commands]).
    use anyhow::{Error, Result};
    use poise::Command;

    type Context<'a> = poise::Context<'a, crate::core::GlobalCommandData, anyhow::Error>;

    /// get_debug_commands() is a static list of debug-only commands.
    /// Any commands that are added in this module should be added 
    /// to the vec![] macro in this function.
    ///
    /// See [get_commands()] for more information.
    ///
    pub fn get_debug_commands() -> Vec<Command<crate::core::GlobalCommandData, Error>>
    {
        vec![ping(), register_commands(), unregister_commands()]
    }
    
    /// says "Pong!"
    #[poise::command(slash_command, prefix_command)]
    async fn ping(ctx: Context<'_>) -> Result<()>
    {
        ctx.say("Pong!").await?;
        Ok(())
    }

    /// Registers all available commands as slash commands in your server
    /// 
    /// There are two approaches to registering slash commands in Discord.
    /// One is to register the commands globally. There is a long (hour-ish)
    /// delay before the global commands are available in Discord.
    /// Because of this, global commands are recommended for production use only.
    ///
    /// The alternative is to register commands per-guild (server). This is recommended
    /// for development/debugging. This is what this command does, and why it is here.
    /// It is marked as a prefix command so it works without using Discord's slash
    /// command functionality (though it does also work as a slash command if the commands have
    /// previously been registered to the server.
    #[poise::command(slash_command, prefix_command, guild_only)]
    async fn register_commands(ctx: Context<'_>) -> Result<()>
    {
        poise::builtins::register_in_guild(ctx, &ctx.framework().options().commands, ctx.guild_id().unwrap()).await?;
        ctx.say("Registered commands").await?;
        Ok(())
    }

    #[poise::command(slash_command, prefix_command, guild_only)]
    async fn unregister_commands(ctx: Context<'_>) -> Result<()>
    {
        let guild_id = ctx.guild_id().ok_or_else(|| anyhow::anyhow!("Can't run this command in DMs!"))?;
        guild_id.set_commands(ctx, vec![]).await?;
        ctx.say("Unregistered commands").await?;
        Ok(())
    }
}

