//! This module handles the types and methods for creating and working with reports,
//! as well as user's saved stats.

use getset::Getters;
use log::debug;
use poise::serenity_prelude as serenity;
use sqlx::PgPool;
use anyhow::Result;
use sqlx::types::chrono::{self, NaiveDateTime, Utc};

use crate::rank::RankList;
use crate::word_count::{TotalWordCount, WordCountArgument};

struct DbReport
{
    #[allow(unused)]
    id: i64,
    guild_id: i64,
    user_id: i64,
    time: chrono::NaiveDateTime,
    total_word_count: i32,
    submission_note: Option<String>,
}

impl From<DbReport> for Report 
{
    fn from(value: DbReport) -> Self {
        Self 
        {
            guild_id: (value.guild_id as u64).into(),
            user_id: (value.user_id as u64).into(),
            timestamp: chrono::DateTime::from_naive_utc_and_offset(value.time, Utc),
            total_word_count: value.total_word_count as u32,
            submission_note: value.submission_note,
        }
    }
}

/// A progress report.
#[derive(Getters)]
pub struct Report 
{
    #[getset(get = "pub")]
    guild_id: serenity::GuildId,
    #[getset(get = "pub")]
    user_id: serenity::UserId,
    #[getset(get = "pub")]
    timestamp: chrono::DateTime<Utc>,
    #[getset(get = "pub")]
    total_word_count: u32,
    #[getset(get = "pub")]
    submission_note: Option<String>,
}

impl Report
{
    pub fn new(
        user: &UserStats,
        when: chrono::DateTime<Utc>,
        word_count_arg: WordCountArgument,
        submission_note: Option<String>
    ) -> Self
    {
        // Convert new project total word count.
        let total_word_count = word_count_arg.convert_to_total(user.current_word_count);
        Report {
            guild_id: user.guild_id,
            user_id: user.user_id,
            timestamp: when,
            total_word_count: total_word_count.into(),
            submission_note,
        }
    }

    pub async fn save(self, db: &PgPool) -> Result<()>
    {
        debug!("Saving a report to the database");
        let guild_id: i64 = self.guild_id.into();
        let user_id: i64 = self.user_id.into();
        let time: NaiveDateTime = self.timestamp.naive_utc();
        sqlx::query!("INSERT INTO reports (guild_id, user_id, time, total_word_count, submission_note) VALUES ($1, $2, $3, $4, $5)", guild_id, user_id, time, self.total_word_count as i32, self.submission_note)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn load(db: &PgPool, id: u64) -> Result<Self>
    {
        let db_report = sqlx::query_as!(DbReport, "SELECT * FROM reports WHERE id = $1", id as i64)
            .fetch_one(db)
            .await?;

        let report: Report = db_report.into();
        Ok(report)
    }

    pub async fn load_reports_for_user(db: &PgPool, guild: serenity::GuildId, user: serenity::UserId) -> Result<Vec<Self>>
    {
        let guild_id: i64 = guild.into();
        let user_id: i64 = user.into();
        let reports: Vec<DbReport> = sqlx::query_as!(DbReport,"SELECT * FROM reports WHERE guild_id = $1 AND user_id = $2 ORDER BY time DESC;", guild_id, user_id)
            .fetch_all(db)
            .await?;

        let reports: Vec<Self> = reports.into_iter().map(|report| report.into()).collect();
        Ok(reports)
    }

    pub async fn delete_user_reports(db: &PgPool, guild: serenity::GuildId, user: serenity::UserId) -> Result<()>
    {
        let guild_id: i64 = guild.into();
        let user_id: i64 = user.into();
        sqlx::query!("DELETE FROM reports WHERE guild_id = $1 AND user_id = $2", guild_id, user_id)
            .execute(db)
            .await?;
        Ok(())
    }
}

/// User's stored overall stats
#[derive(Default, Getters)]
pub struct UserStats
{
    user_id: serenity::UserId,
    guild_id: serenity::GuildId,
    #[getset(get = "pub")]
    role_id: serenity::RoleId,
    /// The highest word count the user has ever attained.
    #[getset(get = "pub")]
    max_word_count: u32,
    /// The user's current project word count.
    #[getset(get = "pub")]
    current_word_count: u32,
}

impl TryFrom<DbUserStats> for UserStats
{
    type Error = anyhow::Error;

    fn try_from(value: DbUserStats) -> Result<Self> {
        let guild_id: u64 = value.guild_id.try_into()?;
        let user_id: u64 = value.user_id.try_into()?;
        let role_id: u64 = value.role_id.try_into()?;
        Ok(Self {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            role_id: role_id.into(),
            max_word_count: value.max_word_count.try_into()?,
            current_word_count: value.current_word_count.try_into()?,
        })
    }
}

/// Internal implementation of the database record.
struct DbUserStats
{
    guild_id: i64,
    user_id: i64,
    // foreign key?
    role_id: i64,
    max_word_count: i64,
    current_word_count: i64,
}


impl UserStats
{
    /// Updates a user's stored word count in place.
    ///
    /// Returns [Some] with the user's old rank  if the user was promoted or demoted.
    /// Returns [None] if the user is the same rank.
    pub fn update_word_count(&mut self, rank_list: &RankList, word_count: WordCountArgument) -> Option<serenity::RoleId>
    {
        let total_word_count: TotalWordCount = word_count.convert_to_total(self.current_word_count);
        self.current_word_count = total_word_count.word_count();
        // Maximize total word count
        self.max_word_count = self.max_word_count.max(self.current_word_count);
        self.update_rank(rank_list)
    }

    /// Updates a user's rank.
    ///
    /// Returns [Some] with the user's old role id if the user was promoted or demoted
    /// Returns [None] if the user is the same rank.
    fn update_rank(&mut self, rank_list: &RankList) -> Option<serenity::RoleId>
    {
        // Originally we updated a user's rank based on max word count, but we actually want to
        // update it based on current word count. 
        // We'll keep tracking max_word_count for posterity though.
        let rank = rank_list.get_rank_for_word_count(self.current_word_count);
        let new_role_id = *rank.rank_id.role_id();
        if new_role_id != self.role_id
        {
            let old_role_id = self.role_id;
            self.role_id = new_role_id;
            Some(old_role_id)
        }
        else
        {
            None
        }
    }

    /// Constructs a new user stat with a word count of 0.
    pub fn new(guild_id: serenity::GuildId, user_id: serenity::UserId, rank_list: &RankList) -> Self
    {
        const NEW_USER_WORD_COUNT: u32 = 0;
        let rank = rank_list.get_rank_for_word_count(NEW_USER_WORD_COUNT);
        Self {
            guild_id,
            user_id,
            role_id: *rank.rank_id.role_id(),
            max_word_count: NEW_USER_WORD_COUNT,
            current_word_count: NEW_USER_WORD_COUNT,
        }
    }

    /// Returns a UserStats if one exists. Otherwise it returns [None].
    pub async fn load(db: &PgPool, guild_id: serenity::GuildId, user_id: serenity::UserId) -> Result<Option<Self>>
    {
        let guild_id: i64 = guild_id.into();
        let user_id: i64 = user_id.into();
        let user_stat: Option<DbUserStats> = sqlx::query_as!(DbUserStats, "SELECT * FROM user_table WHERE guild_id = $1 AND user_id = $2;", guild_id, user_id)
            .fetch_optional(db)
            .await?;

        let user_stat: Option<Result<UserStats>> = user_stat.map(|x| x.try_into());
        user_stat.transpose()
    }

    pub async fn save(self, db: &PgPool) -> Result<()>
    {
        let guild_id: i64 = self.guild_id.into();
        let user_id: i64 = self.user_id.into();
        let role_id: i64 = self.role_id.into();
        let max_word_count: i32 = self.max_word_count as i32;
        let current_word_count: i32 = self.current_word_count as i32;

        sqlx::query!("INSERT INTO user_table (guild_id, user_id, role_id, max_word_count, current_word_count) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (guild_id, user_id) DO UPDATE SET max_word_count = excluded.max_word_count, current_word_count = excluded.current_word_count, role_id = excluded.role_id;", guild_id, user_id, role_id, max_word_count, current_word_count)
            .execute(db)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use crate::rank::Rank;

    use super::*;

    #[test]
    pub fn test_new_user_has_rank()
    {
        let guild_id = serenity::GuildId::new(1);
        let user_id = serenity::UserId::new(1);
        let role_id = serenity::RoleId::new(1);
        let rank = Rank::new(guild_id, role_id, 0.into());
        let rank_list: RankList = rank.into();
        let new_user = UserStats::new(guild_id, user_id, &rank_list);
        assert_eq!(new_user.role_id, *rank.rank_id.role_id());
        assert_eq!(new_user.current_word_count, 0);
        assert_eq!(new_user.max_word_count, 0);
    }

    #[test]
    pub fn test_update_rank_changes_role_id()
    {
        let guild_id = serenity::GuildId::new(1);
        let user_id = serenity::UserId::new(1);
        // Word count 0
        let rank = Rank::new(guild_id, 1.into(), 0.into());
        // Word count 100
        let rank_2 = Rank::new(guild_id, 2.into(), 100.into());
        let rank_list: RankList = 
        {
            let mut rank_list: RankList = rank.into();
            let result = rank_list.add_rank(rank_2);
            assert!(result.is_ok());
            rank_list
        };
        let mut new_user = UserStats::new(guild_id, user_id, &rank_list);
        let updated = new_user.update_word_count(&rank_list, WordCountArgument::Total(100));
        assert!(updated);
        assert_eq!(new_user.role_id, *rank_2.rank_id.role_id());
        assert_eq!(new_user.max_word_count, 100);
        assert_eq!(new_user.current_word_count, 100);
    }

    #[test]
    pub fn test_update_rank_subtract_word_count_doesnt_revert_role_id()
    {
        let guild_id = serenity::GuildId::new(1);
        let user_id = serenity::UserId::new(1);
        // Word count 0
        let rank = Rank::new(guild_id, 1.into(), 0.into());
        // Word count 100
        let rank_2 = Rank::new(guild_id, 2.into(), 100.into());
        let rank_list: RankList = 
        {
            let mut rank_list: RankList = rank.into();
            let result = rank_list.add_rank(rank_2);
            assert!(result.is_ok());
            rank_list
        };
        let mut new_user = UserStats::new(guild_id, user_id, &rank_list);
        new_user.update_word_count(&rank_list, WordCountArgument::Total(100));
        let updated = new_user.update_word_count(&rank_list, WordCountArgument::Relative(-30));
        assert!(!updated);
        assert_eq!(new_user.role_id, *rank_2.rank_id.role_id());
        assert_eq!(new_user.max_word_count, 100);
        assert_eq!(new_user.current_word_count, 70);
    }
}
