//! This module is a helper that handles registering a new user when they submit a report for the
//! first time.

use poise::serenity_prelude::{GuildId, Member, RoleId};

use crate::{Context, rank::RankList, report::UserStats, word_count::WordCountArgument};

/// Context/data needed for [update_or_assign_user_rank()]
/// to correctly set a user's rank and word count and stuff. 
#[derive(Debug)]
pub struct UserReportArgs<'a>
{
    /// The [crate::Context] passed along with the command.
    pub ctx: Context<'a>,
    /// The [GuildId] the report command was called in
    pub guild_id: GuildId,
    /// A reference to the [Member] who invoked the command.
    pub user: &'a Member,
    /// A reference to the guild's [RankList].
    pub rank_list: &'a RankList,
    /// The submitted word count.
    pub report_word_count: WordCountArgument,
}

/// Returns a variant depending on what [update_or_assign_user_rank] did with the user.
pub enum UpdateOrAssignUserRankReturnStatus
{
    /// This variant is returned when a user first submits a report.
    /// Includes the role_id they were assigned based on their submitted word count.
    RegisterNewUserRank(RoleId),
    /// This variant is returned when a user submits a report that changes their existing rank.
    /// Includes the new role_id.
    UpdateExistingUserRank(RoleId),
    /// This variant is returned when a user submits a report that doesn't update their existing
    /// rank.
    ReportNotUpdateUserRank,
}

/// Updates a user's ranks, assigning any needed roles through discord and returning a copy 
/// of the user's overall stats as well as an [UpdateOrAssignUserRankReturnStatus] enum with
/// the specific status of the user's rank.
///
/// Three things can happen to a user:
/// - This is a user's first report (when [user_stats] is [None]). They are NOT unassigned a rank (since they are a new reporter) but they are assigned a new rank based on their word count.
/// - This is an existing user's report, and their word count is sufficent to change their rank. They are unassigned their old rank and assigned their new rank.
/// - This is an existing user's report. Their word count is not sufficently changed to change their rank. Nothing happens.
///
/// In any of these cases, this function consumes an [Option<UserStats>].
pub async fn update_or_assign_user_rank(args: &UserReportArgs<'_>, user_stats: Option<UserStats>) -> anyhow::Result<(UserStats, UpdateOrAssignUserRankReturnStatus)>
{
    log::trace!("Entering update_or_assign_user_rank");
    log::debug!("Args: {:?}", args);
    match user_stats
    {
        // User is an already registered/existing user
        Some(mut user_stats) => {
            log::debug!("User already exists.");
            let old_role_id = user_stats.update_word_count(args.rank_list, args.report_word_count);
            match old_role_id
            {
                // The user's word count did not bump them into a new rank.
                None => {
                    log::debug!("Updated word count. User kept the same rank.");
                    Ok((user_stats, UpdateOrAssignUserRankReturnStatus::ReportNotUpdateUserRank))
                },
                // The user's word count did push them into a new rank.
                Some(old_role_id) => 
                {
                    log::debug!("Updated word count. User has a new rank.");
                    // user_stats.role_id() actually gives you the NEW role id, not the old one.
                    // This is similar to how insert works in HashMaps and similar.
                    update_existing_user_rank(args.ctx, args.user, old_role_id, *user_stats.role_id()).await?;
                    Ok((user_stats, UpdateOrAssignUserRankReturnStatus::UpdateExistingUserRank(*user_stats.role_id())))
                }
            }
        }
        // This is a new user. We want to assign them a rank no matter what.
        None => {
            log::debug!("User is a new user");
            // Creates a new user with a word count of 0
            let mut new_user_stats = UserStats::new(args.guild_id, args.user.user.id, args.rank_list);
            _ = new_user_stats.update_word_count(args.rank_list, args.report_word_count);
            let new_role_id = *new_user_stats.role_id();
            add_new_user_rank(args.ctx, args.user, *new_user_stats.role_id()).await?;
            Ok((new_user_stats, UpdateOrAssignUserRankReturnStatus::RegisterNewUserRank(new_role_id)))
        }
    }
}

/// Assigns a new [rank]/role to a user.
async fn add_new_user_rank(ctx: Context<'_>, user: &Member, role: RoleId) -> anyhow::Result<()>
{
    log::trace!("Entering add_new_user_rank");
    log::debug!("Adding a new rank to a user");
    user.add_role(ctx, role).await?;
    log::trace!("Exiting add_new_user_rank");
    Ok(())
}

/// Removes a user's old rank, assigns them a new rank
async fn update_existing_user_rank(ctx: Context<'_>, user: &Member, old_role: RoleId, new_role: RoleId) -> anyhow::Result<()>
{
    log::trace!("Entering update_existing_user_rank");
    log::debug!("Updating user rank!");
    log::debug!("Adding new role {:?}", new_role);
    user.add_role(ctx, new_role).await?;
    log::debug!("Removing old role {:?}", old_role);
    user.remove_role(ctx, old_role).await?;
    log::trace!("Exiting update_existing_user_rank");
    Ok(())
}
