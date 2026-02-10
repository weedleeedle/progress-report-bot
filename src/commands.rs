//! Defines the bot's Discord slash commands

use std::format;
use std::str::FromStr;

use anyhow::anyhow;
use poise::CreateReply;
use poise::serenity_prelude::Role;
use poise::serenity_prelude as serenity;
use poise::Command;
use anyhow::Error;
use anyhow::Result;
use sqlx::types::chrono::Utc;

use crate::display::create_reply_for_reports;
use crate::mock::GuildLike;
use crate::rank::DiscordRank;
use crate::rank::Rank;
use crate::rank::RankList;
use crate::report::Report;
use crate::report::UserStats;
use crate::user_registration::UpdateOrAssignUserRankReturnStatus;
use crate::user_registration::UserReportArgs;
use crate::user_registration::update_or_assign_user_rank;
use crate::word_count::TotalWordCount;
use crate::word_count::WordCountArgument;
use crate::Context;


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
#[cfg(not(debug_assertions))]
pub fn get_commands() -> Vec<Command<crate::core::GlobalCommandData, Error>>
{
    log::info!("Running in release mode. Returning release commands");
    return get_commands_inner();
}

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
#[cfg(debug_assertions)]
pub fn get_commands() -> Vec<Command<crate::core::GlobalCommandData, Error>>
{
    let mut commands = get_commands_inner();
    // Add debug commands if in debug mode
    if cfg!(debug_assertions)
    {
        log::info!("Running in debug mode. Adding debug commands");
        commands.append(&mut debug::get_debug_commands());
    }
    commands
}

fn get_commands_inner() -> Vec<Command<crate::core::GlobalCommandData, Error>>
{
    // Release commands go here
    let commands = vec![set_rank(), list_ranks(), clear_ranks(), report(), reporpt(), list_reports(), clear_reports(), list_stats()];
    log::debug!("get_commands_inner commands: {:?}", commands);
    commands
}

/// Adds or updates a rank.
#[poise::command(slash_command, guild_only, default_member_permissions = "ADMINISTRATOR")]
async fn set_rank(ctx: Context<'_>,
        #[description = "The role to grant when a user reaches the specified word count"]
        role: serenity::Role,
        #[description = "The minimum word count threshold needed for a rank"]
        minimum_word_count: u32
    ) -> Result<()>
{
    log::info!("Running set_rank command");
    log::debug!("role: {}", role);
    log::debug!("minimum_word_count: {}", minimum_word_count);
    let minimum_word_count = TotalWordCount::from(minimum_word_count);
    log::trace!("Converted minimum_word_count to TotalWordCount: {:?}", minimum_word_count);
    let pool = ctx.data().get_pool();
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;
    log::trace!("Guild ID: {}", guild_id);

    let mut ranks = RankList::load(pool, guild_id).await?;
    log::debug!("Existing ranks: {:?}", ranks); 

    let new_rank = Rank::new(guild_id, role.id, minimum_word_count);
    log::debug!("New rank: {:?}", new_rank);
    let result = ranks.add_rank(new_rank);
    if let Err(err) = result
    {
        let guild = ctx.partial_guild().await.unwrap();
        let discord_error = err.to_discord_error(&guild).expect("Unable to get the role from the guild");
        log::warn!("Unable to add rank: {}", discord_error);
        return Err(discord_error.into())
    }

    log::info!("Added new rank {:?} successfully", new_rank);
    ranks.save(pool).await?;

    ctx.say(format!("Added rank {}!", role)).await?;
    Ok(())
}

/// Lists all the ranks
#[poise::command(slash_command, guild_only)]
async fn list_ranks(ctx: Context<'_>) -> Result<()>
{
    log::info!("Running list_ranks command");
    let pool = ctx.data().get_pool();
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;

    let ranks = RankList::load(pool, guild_id).await?;
    log::debug!("Ranks: {:?}", ranks);
    let guild = ctx.partial_guild().await.unwrap();

    let mut response = String::new();
    let ranks: Vec<DiscordRank<Role>> = ranks.iter().map(|x| x.to_rank(&guild).unwrap()).collect();

    log::trace!("Iterating over ranks");
    for rank in ranks
    {
        log::trace!("Rank: {}", rank);
        response.push_str(&format!("{}", rank));
    }

    if response.is_empty()
    {
        log::warn!("No ranks exist yet!");
        response.push_str("No ranks. Make some with /set_rank!");
    }

    log::debug!("Formatted response: {}", response);

    ctx.say(response).await?;
    Ok(())
}

/// CAUTION! This command will irrevocably remove all your ranks!
#[poise::command(slash_command, guild_only, default_member_permissions = "ADMINISTRATOR")]
async fn clear_ranks(ctx: Context<'_>) -> Result<()>
{
    log::info!("Running clear_ranks");
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;
    let pool = ctx.data().get_pool();
    RankList::clear_all_ranks(pool, guild_id).await?;
    ctx.say("Cleared ranks!").await?;
    Ok(())
}

/// Submits a progress report.
///
/// A report can be submitted as a total word count (without a prefix before the number) 
/// or a relative word count (with a + or - before the word count, e.g +300, -150).
///
/// Reducing your project word count will never demote you.
#[poise::command(slash_command, guild_only)]
async fn report(ctx: Context<'_>, 
    #[description = "A word count. Can be total word count (default) or relative by starting your word count with + or -"]
    word_count: String, 
    #[description = "An optional comment to include with your report"]
    comment: Option<String>) -> Result<()>
{
    log::trace!("Run report command");
    report_inner(ctx, word_count, comment).await
}

/// Submits a progress reporpt
#[poise::command(slash_command, guild_only)]
async fn reporpt(ctx: Context<'_>,
    #[description = "A word count. Can be total word count (default) or relative by starting your word count with + or -"]
    word_count: String,
    #[description = "An optional comment to include with your reporpt"]
    comment: Option<String>) -> Result<()>
{
    log::trace!("Run reporpt [sic] command");
    report_inner(ctx, word_count, comment).await
}

async fn report_inner(ctx: Context<'_>, word_count: String, comment: Option<String>) -> Result<()>
{
    log::debug!("Running report_inner command");
    log::trace!("word_count: {}", word_count);
    log::trace!("comment: {:?}", comment);
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;
    let user_id = ctx.author().id;
    log::debug!("User ID: {}", user_id);
    let db = ctx.data().get_pool();
    let word_count = WordCountArgument::from_str(&word_count)?;
    log::debug!("Word count: {:?}", word_count);

    let rank_list = RankList::load(db, guild_id).await?;
    log::debug!("Rank list: {:?}", rank_list);
    let user_stats = UserStats::load(db, guild_id, user_id).await?;
    log::debug!("User stats: {:?}", user_stats);
    log::info!("Checking if user exists already:");
    if user_stats.is_none()
    {
        log::info!("First-time user reporting.");
    }
    else
    {
        log::info!("Pre-existing user reporting.")
    }

    let user = ctx.author_member().await.expect("We know this is being run in a guild, so we don't have to handle this.");
    // Update and save the user's overall stats
    let report_args = UserReportArgs
    {
        ctx,
        guild_id,
        user: &user,
        rank_list: &rank_list,
        report_word_count: word_count
    };
    let (user_stats, result) = update_or_assign_user_rank(&report_args, user_stats).await?;
    // Generate and save the report
    let timestamp = ctx.created_at().with_timezone(&Utc);
    let report = Report::new(&user_stats, timestamp, word_count, comment);
    report.save(db).await?;
    user_stats.save(db).await?;
    let guild = ctx.partial_guild().await.unwrap();
    let response = match result
    {
        UpdateOrAssignUserRankReturnStatus::RegisterNewUserRank(role_id) => 
        {
            let role = guild.role(role_id).ok_or(anyhow!("Whoops! That role doesn't exist!"))?;
            format!("Welcome {}! Congratulations on your first report! You've reached {}", &user, role)
        }
        UpdateOrAssignUserRankReturnStatus::UpdateExistingUserRank(role_id) =>
        {
            let role = guild.role(role_id).ok_or(anyhow!("Whoops! That role doesn't exist!"))?;
            format!("You've reached {}! Congratulations, {}!", role, &user)
        }
        UpdateOrAssignUserRankReturnStatus::ReportNotUpdateUserRank =>
        {
            format!("Progress report submitted. Good work, {}!", &user)
        }
    };

    ctx.say(response).await?;
    Ok(())
}

/// Lists a user's progress reports from latest to earliest.
#[poise::command(slash_command, guild_only)]
async fn list_reports(ctx: Context<'_>) -> Result<()>
{
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;
    let user_id = ctx.author().id;
    let db = ctx.data().get_pool();

    let reports = Report::load_reports_for_user(db, guild_id, user_id).await?;
    let (reply, handler) = create_reply_for_reports(CreateReply::default(), ctx, reports, 5);
    ctx.send(reply).await?;
    if let Some(handler) = handler
    {
        handler.listen(ctx).await?;
    }
    Ok(())
}

/// CAUTION: Clears a user's history of progress reports.
///
/// This command will irrevocably delete a user's entire history of progress reports.
/// Note that this will NOT delete a user's overall stats, so if you want to remove
/// that too, use the `/clear_user` command.
#[poise::command(slash_command, guild_only, default_member_permissions = "ADMINISTRATOR")]
async fn clear_reports(ctx: Context<'_>, user: serenity::User) -> Result<()>
{
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;
    let user_id = user.id;
    let db = ctx.data().get_pool();
    Report::delete_user_reports(db, guild_id, user_id).await?;

    ctx.say(format!("Deleted {}'s reports!", user)).await?;
    Ok(())
}

/// Lists a user's overall stats.
#[poise::command(slash_command, guild_only)]
async fn list_stats(ctx: Context<'_>, user: Option<serenity::User>) -> Result<()>
{
    let guild_id = ctx.guild_id().ok_or(anyhow!("This command can only be run in a server!"))?;
    let user = user.as_ref();
    let user_id = user.unwrap_or_else(|| ctx.author()).id;
    let db = ctx.data().get_pool();
    let user_stats = UserStats::load(db, guild_id, user_id).await?;
    match user_stats
    {
        None => {
            ctx.say("No stats yet! Try submitting a report with `/report`!").await?;
        }
        Some(user_stats) => {
            let guild = ctx.partial_guild().await.unwrap();
            let role = guild.role(*user_stats.role_id()).unwrap();
            ctx.say(format!("Current rank: {}\nHighest word count: {}\nCurrent word count: {}", role, user_stats.max_word_count(), user_stats.current_word_count())).await?;
        }
    }
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
    #[poise::command(slash_command, prefix_command, guild_only, default_member_permissions="ADMINISTRATOR")]
    async fn register_commands(ctx: Context<'_>) -> Result<()>
    {
        poise::builtins::register_in_guild(ctx, &ctx.framework().options().commands, ctx.guild_id().unwrap()).await?;
        ctx.say("Registered commands").await?;
        Ok(())
    }

    #[poise::command(slash_command, prefix_command, guild_only, default_member_permissions="ADMINISTRATOR")]
    async fn unregister_commands(ctx: Context<'_>) -> Result<()>
    {
        let guild_id = ctx.guild_id().ok_or_else(|| anyhow::anyhow!("Can't run this command in DMs!"))?;
        guild_id.set_commands(ctx, vec![]).await?;
        ctx.say("Unregistered commands").await?;
        Ok(())
    }
}

