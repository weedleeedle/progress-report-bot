//! This module handles and defines ranks. A rank consists of a Discord role and a word count.
//! The word count is the minimum needed to have that rank. So the first and lowest rank should
//! have a word count of 0, so on and so forth.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use getset::Getters;
use poise::serenity_prelude as serenity;
use sqlx::PgPool;

use crate::mock::GuildLike;
use crate::mock::RoleLike;

/// A rank, which is a pair of a role and a word count.
#[derive(Debug)]
pub struct Rank<'a, T: RoleLike>
{
    role: &'a T,
    minimum_word_count: u32,
}

impl<T: RoleLike> PartialEq for Rank<'_, T>
{
    fn eq(&self, other: &Self) -> bool {
        self.minimum_word_count == other.minimum_word_count
    }
}

impl<T: RoleLike> Eq for Rank<'_, T> {}

impl<'a, T: RoleLike> Rank<'a, T>
{
    pub fn new(role: &'a T, minimum_word_count: u32) -> Self
    {
        Self {
            role,
            minimum_word_count
        }
    }
}

impl<T: RoleLike> PartialOrd for Rank<'_, T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
    }
}

impl<T: RoleLike> Ord for Rank<'_, T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.minimum_word_count.cmp(&other.minimum_word_count)
    }
}

/// A minimal version of [Rank] which uses [serenity::RoleId] instead of [serenity::Role]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct RankId
{
	guild_id: serenity::GuildId,
    role_id: serenity::RoleId,
    minimum_word_count: u32,
}

impl PartialOrd for RankId
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
    }
}

impl Ord for RankId
{
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
	    self.minimum_word_count.cmp(&other.minimum_word_count)
	}
}

impl Hash for RankId
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Two RankIds are identical if their guild and role ids are the same, even if the
        // minimum_word_count is different.
        self.guild_id.hash(state);
        self.role_id.hash(state);
    }
}
impl RankId
{
    /// Attempts to convert a [RankId] into a [Rank].
    /// This requires additional context which is given via the [get_role_object] param
    /// which is an object with the [RoleLike] trait. This is normally a [serenity::Guild]
    /// but can also be some sort of mocking object for testing.
    pub fn to_rank<'a, G: GuildLike<R>, R: RoleLike>(&self, get_role_object: &'a G) -> Option<Rank<'a, R>>
    {
		get_role_object.role(self.role_id)
			.map(|role| Rank::new(role, self.minimum_word_count))
    }
}

impl<R: RoleLike> From<Rank<'_, R>> for RankId
{
    fn from(value: Rank<'_, R>) -> Self {
		Self {
			guild_id: value.role.guild_id(),
			role_id: value.role.id(),
			minimum_word_count: value.minimum_word_count,
		}
    }
}

/// Internal representation of the database record
struct DbRankId
{
    guild_id: i64,
    role_id: i64,
    minimum_word_count: i32,
}

impl From<&DbRankId> for RankId
{
    fn from(value: &DbRankId) -> Self {
        Self {
            guild_id: serenity::GuildId::new(value.guild_id as u64),
            role_id: serenity::RoleId::new(value.role_id as u64),
            minimum_word_count: value.minimum_word_count as u32,
        }
    }
}


/// A set of ranks, ordered from lowest to highest threshold.
/// 
/// Ideally a rank list should start with one rank at 0, but I don't think I will actually enforce that.
/// What *will* be enforced is that you can only have a RankList with at LEAST one rank.
/// This makes ops like [get_rank_for_word_count] infallible.
#[derive(Getters)]
pub struct RankList
{
    guild_id: serenity::GuildId,
    #[getset(get)]
    ranks: BTreeSet<RankId>,
    // When we remove a rank, we add it to this list so that the next time we save we remove these
    // records.
    // We use a cell since we're using this as a cache.
    // We can modify this in immutable operations such as saving, and it doesn't actually change
    // the state of the object
    pending_removals: RefCell<HashSet<RankId>>,
}

impl RankList
{
    /// Adds a rank to the rank list. If a rank already exists with the same minimum_word_count, it is replaced with the new one.
    pub fn add_rank(&mut self, rank_id: &RankId)
    {
        self.ranks.replace(*rank_id);
        // Check to see if we were going to remove this rank. If so, we want to put it back.
        if self.pending_removals.borrow_mut().contains(rank_id)
        {
            self.pending_removals.borrow_mut().remove(rank_id);
        }
    }

    pub fn add_ranks(&mut self, rank_ids: &[RankId])
    {
        for rank_id in rank_ids
        {
            self.add_rank(rank_id);
        }
    }

    /// Interestingly, we don't care about the minimum_word_count here.
    /// We just use the guild_id and role_id.
    pub fn remove_rank(&mut self, rank_id: &RankId)
    {
        let rank = self.ranks.take(rank_id);
        // If we found the rank, add it to pending removals.
        if let Some(rid) = rank
        {
            self.pending_removals.borrow_mut().insert(rid);
        }
    }

    /// Gets the highest rank that has a lower minimum_word_count than the provided word_count.
    pub fn get_rank_for_word_count(&self, word_count: u32) -> RankId
    {
        let mut highest_rank = self.ranks.first().expect("Expected there to be at least one rank!");
        for rank in self.ranks.iter()
        {
            // We can stop iterating as soon as we find a rank that is higher than our word count,
            // since the set is ordered.
            if rank.minimum_word_count > word_count
            {
                break
            }

            highest_rank = rank;
        }

        return *highest_rank;
    }

    /// Loads a RankList from a database.
    pub async fn load(db: &PgPool, guild_id: serenity::GuildId) -> anyhow::Result<Self>
    {
        let guild_id: i64 = guild_id.into();

        let ranks: Vec<DbRankId>  = sqlx::query_as!(DbRankId, "SELECT * FROM rank_table WHERE guild_id = $1;", guild_id)
            .fetch_all(db)
            .await?;

        let ranks: Vec<RankId> = ranks.iter().map(|rank| rank.into()).collect();
        // Convert our vec of rank ids into a RankList
        ranks.as_slice().try_into()
    }

    pub async fn save(&self, db: &PgPool) -> anyhow::Result<()>
    {
        for rank in self.ranks.iter()
        {
            let guild_id: i64 = rank.guild_id.into();
            let role_id: i64 = rank.role_id.into();
            let minimum_word_count: i32 = rank.minimum_word_count as i32;
            sqlx::query!("INSERT INTO rank_table (guild_id, role_id, minimum_word_count) VALUES ($1, $2, $3) ON CONFLICT (guild_id, role_id) DO UPDATE SET minimum_word_count = excluded.minimum_word_count;", guild_id, role_id, minimum_word_count)
                // Okay we don't actually need PgPool to be mutable. I... guess that makes sense?
                // Idk.
                .execute(db)
                .await?;
        } 

        // Take the list of pending removals and clear out the cache 
        let pending_removals = self.pending_removals.replace(HashSet::new());

        for rank in pending_removals.iter()
        {
            let guild_id: i64 = rank.guild_id.into();
            let role_id: i64 = rank.role_id.into();
            sqlx::query!("DELETE FROM rank_table WHERE guild_id = $1 AND role_id = $2;", guild_id, role_id)
                .execute(db)
                .await?;
        }

        Ok(())
    }
}

impl From<RankId> for RankList
{
    fn from(value: RankId) -> Self {
        let mut ranks = BTreeSet::<RankId>::new();
        ranks.insert(value);
        Self {
            guild_id: value.guild_id,
            ranks,
            pending_removals: RefCell::new(HashSet::new())
        }
    }
}

impl TryFrom<&[RankId]> for RankList
{
    type Error = anyhow::Error;

    fn try_from(value: &[RankId]) -> anyhow::Result<Self> {
        let guild_id = value[0].guild_id;
        let mut ranks = BTreeSet::<RankId>::new();
        for rank in value.iter()
        {
            if rank.guild_id != guild_id
            {
                anyhow::bail!("Passed a rank to a rank_list constructor with a different guild_id than previous ranks.");
            }
            ranks.replace(*rank);
        }
        Ok(Self {
            guild_id,
            ranks,
            pending_removals: RefCell::new(HashSet::new())
        })
    }
}

#[cfg(test)]
mod tests
{
    use std::collections::HashMap;
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct MockRole {
        role_id: serenity::RoleId,
    }

    impl RoleLike for MockRole
    {
        fn id(&self) -> serenity::RoleId
        {
            self.role_id
        }

        fn guild_id(&self) -> serenity::GuildId
        {
            const GUILD_ID: u64 = 1;
            GUILD_ID.into()
        }
    }

    type MockGuild = HashMap<serenity::RoleId, MockRole>;

    impl GuildLike<MockRole> for MockGuild
    {
        fn role<'a>(&'a self, role_id: serenity::RoleId) -> Option<&'a MockRole> {
            self.get(&role_id) 
        }

        fn id(&self) -> serenity::GuildId
        {
            const GUILD_ID: u64 = 1;
            serenity::GuildId::new(GUILD_ID)
        }
    }

    fn create_role_in_guild(mock_guild: &mut MockGuild, role_id: serenity::RoleId)
    {
        let mock_role = MockRole
        {
            role_id,
        };

        mock_guild.insert(role_id, mock_role);
    }

    #[test]
    pub fn rank_id_to_rank()
    {
        const GUILD_ID: u64 = 1;
        const ROLE_ID: u64 = 1;
        let mut mock_guild = MockGuild::new();
        create_role_in_guild(&mut mock_guild, ROLE_ID.into());

        let rank_id = RankId
        {
            guild_id: GUILD_ID.into(),
            role_id: ROLE_ID.into(),
            minimum_word_count: 0,
        };

        let rank = rank_id.to_rank(&mock_guild).expect("Expected to get rank");
        assert_eq!(rank.role, mock_guild.get(&ROLE_ID.into()).unwrap());
    }

    #[test]
    pub fn rank_to_rank_id()
    {
        const GUILD_ID: u64 = 1;
        const ROLE_ID: u64 = 1;

        let mock_role = MockRole {
            role_id: ROLE_ID.into(),
        };

        let mock_rank = Rank
        {
            role: &mock_role,
            minimum_word_count: 0
        };

        let rank_id: RankId = mock_rank.into();
        assert_eq!(Into::<u64>::into(rank_id.guild_id), GUILD_ID);
        assert_eq!(Into::<u64>::into(rank_id.role_id), ROLE_ID);
        assert_eq!(rank_id.minimum_word_count, 0);
    }

    #[test]
    pub fn get_highest_rank() 
    {
        const GUILD_ID: u64 = 1;
        const ROLE_ID: u64 = 1;
        let first_rank = RankId {
            guild_id: GUILD_ID.into(),
            role_id: ROLE_ID.into(),
            minimum_word_count: 0,
        };

        let rank_list: RankList = first_rank.into();

        assert_eq!(rank_list.get_rank_for_word_count(0), first_rank);
    }

    #[test]
    pub fn get_highest_rank_within_threshold()
    {
        const GUILD_ID: u64 = 1;
        let first_rank = RankId {
            guild_id: GUILD_ID.into(),
            role_id: 1.into(),
            minimum_word_count: 0,
        };
        let second_rank = RankId {
            guild_id: GUILD_ID.into(),
            role_id: 2.into(),
            minimum_word_count: 100,
        };
        let rank_list: RankList = vec![first_rank, second_rank].as_slice().try_into().unwrap();

        assert_eq!(rank_list.get_rank_for_word_count(10), first_rank);
        assert_eq!(rank_list.get_rank_for_word_count(100), second_rank);
    }

    #[test]
    pub fn create_rank_list_fails_with_ranks_from_different_guilds()
    {
        let first_rank = RankId {
            guild_id: 1.into(),
            role_id: 1.into(),
            minimum_word_count: 0,
        };
        let second_rank = RankId {
            guild_id: 2.into(),
            role_id: 2.into(),
            minimum_word_count: 0,
        };
        let vec = vec![first_rank, second_rank];
        let rank_list: Result<RankList, anyhow::Error> = vec.as_slice().try_into();
        assert!(rank_list.is_err());
    }

    #[test]
    pub fn remove_role_caches_rank()
    {
        let first_rank = RankId {
            guild_id: 1.into(),
            role_id: 1.into(),
            minimum_word_count: 0,
        };
        let mut rank_list: RankList = first_rank.into();
        assert!(rank_list.pending_removals.borrow().is_empty());
        rank_list.remove_rank(&first_rank);
        assert_eq!(rank_list.pending_removals.borrow().len(), 1);
    }

    #[test]
    pub fn remove_role_then_add_back_removes_cached_rank()
    {
        let rank = RankId {
            guild_id: 1.into(),
            role_id: 1.into(),
            minimum_word_count: 0,
        };
        let mut rank_list: RankList = rank.into();
        rank_list.remove_rank(&rank);
        rank_list.add_rank(&rank);
        assert_eq!(rank_list.ranks.len(), 1);
        assert!(rank_list.pending_removals.borrow().is_empty())
    }
    // Fuuuck we can't actually test saving for now... we really should mock PgPool or something...
}
