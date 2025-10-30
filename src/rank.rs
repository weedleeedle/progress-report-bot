//! This module handles and defines ranks. A rank consists of a Discord role and a word count.
//! The word count is the minimum needed to have that rank. So the first and lowest rank should
//! have a word count of 0, so on and so forth.

use std::collections::BTreeSet;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use derive_more::From;
use derive_more::Into;
use getset::Getters;
use poise::serenity_prelude::GuildId;
use poise::serenity_prelude::RoleId;
use poise::serenity_prelude as serenity;
use sqlx::PgPool;
use thiserror::Error;

use crate::mock::GuildLike;
use crate::mock::RoleLike;

/// A DiscordRank is effectively a reference to a [serenity::Role]
/// with a minimum_word_count attached to it.
#[derive(Debug)]
pub struct DiscordRank<'a, T: RoleLike>
{
    role: &'a T,
    minimum_word_count: u32,
}

impl Display for DiscordRank<'_, serenity::Role>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\t{}\n", self.role, self.minimum_word_count)
    }
}

/// A Rank is a [DiscordRank] but without any references to
/// Discord other than the [GuildId] and [RoleId] contained in the [RankId].
/// This makes it easier to mock and test and pass around without all the additional context
/// of a [serenity::Role] but requires additional work to get that context back
///
/// Two [Rank]s are considered equal if they have the same minimum_word_count.
/// They are sorted and defined based on this word count.
///
/// If you need to id ranks based on their role, use [RankId] instead.
#[derive(Debug, Clone, Copy)]
pub struct Rank
{
    pub rank_id: RankId,
    pub minimum_word_count: u32,
}

impl PartialEq for Rank
{
    fn eq(&self, other: &Self) -> bool {
        self.minimum_word_count == other.minimum_word_count
    }
}

impl Eq for Rank {}

impl PartialOrd for Rank
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl Ord for Rank
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.minimum_word_count.cmp(&other.minimum_word_count)
    }
}

impl Rank
{
    /// Attempts to convert a [Rank] into a [DiscordRank].
    /// This requires additional context which is given via the [get_role_object] param
    /// which is an object with the [RoleLike] trait. This is normally a [serenity::Guild]
    /// but can also be some sort of mocking object for testing.
    pub fn to_rank<'a, G: GuildLike<R>, R: RoleLike>(&self, get_role_object: &'a G) -> Option<DiscordRank<'a, R>>
    {
        get_role_object.role(self.rank_id.role_id)
            .map(|role| DiscordRank {
                role,
                minimum_word_count: self.minimum_word_count,
            })
    }

    pub fn new(guild_id: GuildId, role_id: RoleId, minimum_word_count: u32) -> Self
    {
        Self {
            rank_id: RankId {
                guild_id,
                role_id
            },
            minimum_word_count,
        }
    }
}

impl<R: RoleLike> From<DiscordRank<'_, R>> for Rank
{
    fn from(value: DiscordRank<'_, R>) -> Self {
        Self {
            rank_id: RankId {
                guild_id: value.role.guild_id(),
                role_id: value.role.id(),
            },
            minimum_word_count: value.minimum_word_count
        }
    }
}

impl From<&DbRankId> for Rank
{
    fn from(value: &DbRankId) -> Self {
        Self {
            rank_id: RankId {
                guild_id: serenity::GuildId::new(value.guild_id as u64),
                role_id: serenity::RoleId::new(value.role_id as u64),
            },
            minimum_word_count: value.minimum_word_count as u32,
        }
    }
}

/// A zero-cost wrapper around a Rank to allow us to store a Rank based on its ID instead of by its
/// minimum_word_count like normal.
#[derive(Debug, From, Into, Clone, Copy)]
struct RankHash(Rank);

impl Hash for RankHash
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // We ignore the minimum_word_count when creating the rank hash.
        self.0.rank_id.hash(state);
    }
}

impl PartialEq for RankHash
{
    fn eq(&self, other: &Self) -> bool {
        self.0.rank_id.eq(&other.0.rank_id)
    }
}

impl Eq for RankHash {}

/// A minimal version of [Rank] which uses [serenity::RoleId] instead of [serenity::Role]
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct RankId
{
    guild_id: serenity::GuildId,
    role_id: serenity::RoleId,
}

/// Internal representation of the database record
struct DbRankId
{
    guild_id: i64,
    role_id: i64,
    minimum_word_count: i32,
}


/// A set of ranks, ordered from lowest to highest threshold.
/// 
/// Ideally a rank list should start with one rank at 0, but I don't think I will actually enforce that.
/// What *will* be enforced is that you can only have a RankList with at LEAST one rank.
/// This makes ops like [get_rank_for_word_count] infallible.
#[derive(Getters)]
pub struct RankList
{
    // GuildId can be None only if we haven't added any ranks yet.
    guild_id: Option<serenity::GuildId>,
    // Ensures that we don't get a duplicate key (guild_id + role_id)
    rank_set: HashSet<RankHash>,
    // Sorts ranks in order of their value (minimum_word_count)
    rank_order: BTreeSet<Rank>,
    // When we remove a rank, we add it to this list so that the next time we save we remove these
    // records.
    pending_removals: HashSet<RankHash>,
}

#[derive(Debug, Error)]
pub enum AddRankError
{
    #[error("There already exists a rank {0:?} with that word count")]
    RankExistsWithWordCount(Rank),
}

#[derive(Debug, Error)]
pub enum AddRankDiscordError
{
    #[error("There already exists a role {0} with that word count")]
    RankExistsWithWordCount(serenity::Role)
}

impl AddRankError
{
    pub fn to_discord_error<'a, G: GuildLike<serenity::Role>>(&self, get_role_object: &'a G) -> Option<AddRankDiscordError>
    {
        match self
        {
            AddRankError::RankExistsWithWordCount(rank) => Some(AddRankDiscordError::RankExistsWithWordCount(rank.to_rank(get_role_object)?.role.clone()))
        }
    }
}

impl RankList
{
    /// Adds a rank to the rank list. If a rank already exists with the same minimum_word_count, it is replaced with the new one.
    pub fn add_rank(&mut self, rank: Rank) -> Result<(), AddRankError>
    {
        if let Some(guild_id) = self.guild_id
        {
            if guild_id != rank.rank_id.guild_id
            {
                panic!("Provided guild_id did not match expected guild id!");
            }
        }
        else
        {
            self.guild_id = Some(rank.rank_id.guild_id);
        }

        // Returns true if we have any *equal* element (i.e any rank with the same minimum_word_count)
        // Since we don't know how we want to handle this, we return an error.
        if self.rank_order.contains(&rank)
        {
            let old_rank = self.rank_order.get(&rank).unwrap();
            return Err(AddRankError::RankExistsWithWordCount(*old_rank));
        }

        // Otherwise, we check if we have the *rank* assigned already, and if so, reassign it.
        if self.rank_set.contains(&rank.into())
        {
            let old_rank = self.rank_set.get(&rank.into()).unwrap();
            // Remove the old value for the rank.
            self.rank_order.remove(&(*old_rank).into());
        }

        self.rank_set.replace(rank.into());
        self.rank_order.insert(rank);

        // Check to see if we were going to remove this rank. If so, we want to put it back.
        if self.pending_removals.contains(&rank.into())
        {
            self.pending_removals.remove(&rank.into());
        }

        Ok(())
    }

    pub fn add_ranks(&mut self, ranks: &[Rank])
    {
        for rank in ranks
        {
            self.add_rank(*rank);
        }
    }

    /// Interestingly, we don't care about the minimum_word_count here.
    /// We just use the guild_id and role_id.
    pub fn remove_rank(&mut self, rank: Rank)
    {
        let take_rank = self.rank_set.take(&rank.into());

        // If we found the rank, add it to pending removals.
        if let Some(rank) = take_rank
        {
            self.rank_order.remove(&rank.into());
            self.pending_removals.insert(rank);
        }
    }

    /// Gets the highest rank that has a lower minimum_word_count than the provided word_count.
    pub fn get_rank_for_word_count(&self, word_count: u32) -> Rank
    {
        let mut highest_rank = self.rank_order.first().expect("Expected there to be at least one rank!");
        for rank in self.rank_order.iter()
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

        let ranks: Vec<Rank> = ranks.iter().map(|rank| rank.into()).collect();
        // Convert our vec of ranks into a RankList
        ranks.as_slice().try_into()
    }

    /// Consumes this [RankList] and saves it to the database
    pub async fn save(self, db: &PgPool) -> anyhow::Result<()>
    {
        for rank in self.rank_order.iter()
        {
            let guild_id: i64 = rank.rank_id.guild_id.into();
            let role_id: i64 = rank.rank_id.role_id.into();
            let minimum_word_count: i32 = rank.minimum_word_count as i32;
            println!("{}", minimum_word_count);
            sqlx::query!("INSERT INTO rank_table (guild_id, role_id, minimum_word_count) VALUES ($1, $2, $3) ON CONFLICT (guild_id, role_id) DO UPDATE SET minimum_word_count = excluded.minimum_word_count;", guild_id, role_id, minimum_word_count)
                // Okay we don't actually need PgPool to be mutable. I... guess that makes sense?
                // Idk.
                .execute(db)
                .await?;
            } 

        // Take the list of pending removals and clear out the cache 
        for rank in self.pending_removals.iter()
        {
            let guild_id: i64 = rank.0.rank_id.guild_id.into();
            let role_id: i64 = rank.0.rank_id.role_id.into();
            sqlx::query!("DELETE FROM rank_table WHERE guild_id = $1 AND role_id = $2;", guild_id, role_id)
                .execute(db)
                .await?;
            }

        Ok(())
    }

    pub fn iter(&self) -> std::collections::btree_set::Iter<'_, Rank>
    {
        self.rank_order.iter()
    }
}

impl From<Rank> for RankList
{
    fn from(value: Rank) -> Self {
        // We know this function is infallible since there can't be any duplicates.
        let mut rank_order = BTreeSet::new();
        let mut rank_set = HashSet::new();
        rank_order.insert(value);
        rank_set.insert(value.into());
        Self {
            guild_id: Some(value.rank_id.guild_id),
            rank_order,
            rank_set,
            pending_removals: HashSet::new(),
        }
    }
}

impl TryFrom<&[Rank]> for RankList
{
    type Error = anyhow::Error;

    fn try_from(value: &[Rank]) -> anyhow::Result<Self> {
        let mut iter = value.iter();
        let first = iter.next();

        if let Some(rank_id) = first
        {
            // Use the first element to create the rank list
            let mut rank_list: RankList = (*rank_id).into();
            // Then we iterate over the rest of the list and add them!
            for rank in iter
            {
                rank_list.add_rank(*rank)?;
            }
            Ok(rank_list)
        }
        // If the iterator is empty we just make a new empty RankList
        else
        {
            Ok(RankList {
                guild_id: None,
                rank_set: HashSet::new(),
                rank_order: BTreeSet::new(),
                pending_removals: HashSet::new(),
            })
        }
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

        let rank_id = Rank
        {
            rank_id: RankId{
                guild_id: GUILD_ID.into(),
                role_id: ROLE_ID.into(),
            },
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

        let mock_rank = DiscordRank
        {
            role: &mock_role,
            minimum_word_count: 0
        };

        let rank: Rank = mock_rank.into();
        assert_eq!(Into::<u64>::into(rank.rank_id.guild_id), GUILD_ID);
        assert_eq!(Into::<u64>::into(rank.rank_id.role_id), ROLE_ID);
        assert_eq!(rank.minimum_word_count, 0);
    }

    #[test]
    pub fn get_highest_rank() 
    {
        const GUILD_ID: u64 = 1;
        const ROLE_ID: u64 = 1;
        let first_rank = Rank {
            rank_id: RankId {
                guild_id: GUILD_ID.into(),
                role_id: ROLE_ID.into(),
            },
            minimum_word_count: 0,
        };

        let rank_list: RankList = first_rank.into();

        assert_eq!(rank_list.get_rank_for_word_count(0), first_rank);
    }

    #[test]
    pub fn get_highest_rank_within_threshold()
    {
        const GUILD_ID: u64 = 1;
        let first_rank = Rank {
            rank_id: RankId {
                guild_id: GUILD_ID.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 0,
        };
        let second_rank = Rank {
            rank_id: RankId {
                guild_id: GUILD_ID.into(),
                role_id: 2.into(),
            },
            minimum_word_count: 100,
        };
        let rank_list: RankList = vec![first_rank, second_rank].as_slice().try_into().unwrap();

        assert_eq!(rank_list.get_rank_for_word_count(10), first_rank);
        assert_eq!(rank_list.get_rank_for_word_count(100), second_rank);
    }

    #[test]
    #[should_panic]
    pub fn create_rank_list_fails_with_ranks_from_different_guilds()
    {
        let first_rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 0,
        };
        let second_rank = Rank {
            rank_id: RankId {
                guild_id: 2.into(),
                role_id: 2.into(),
            },
            minimum_word_count: 0,
        };
        let vec = vec![first_rank, second_rank];
        let rank_list: Result<RankList, anyhow::Error> = vec.as_slice().try_into();
    }

    #[test]
    pub fn remove_role_caches_rank()
    {
        let first_rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 0,
        };
        let mut rank_list: RankList = first_rank.into();
        assert!(rank_list.pending_removals.is_empty());
        rank_list.remove_rank(first_rank);
        assert_eq!(rank_list.pending_removals.len(), 1);
    }

    #[test]
    pub fn remove_role_then_add_back_removes_cached_rank()
    {
        let rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 0,
        };
        let mut rank_list: RankList = rank.into();
        rank_list.remove_rank(rank);
        rank_list.add_rank(rank).unwrap();
        assert_eq!(rank_list.rank_set.len(), 1);
        assert_eq!(rank_list.rank_order.len(), 1);
        assert!(rank_list.pending_removals.is_empty())
    }

    #[test]
    pub fn add_rank_then_add_new_rank_with_same_word_count_fails()
    {
        let first_rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 123,
        };

        let second_rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 2.into(),
            },
            minimum_word_count: 123,
        };

        let mut rank_list: RankList = first_rank.into();
        let result = rank_list.add_rank(second_rank);
        assert!(result.is_err());
        assert_eq!(rank_list.rank_set.len(), 1);
        assert_eq!(rank_list.rank_order.len(), 1);
        assert_eq!(rank_list.rank_set.iter().nth(0).unwrap().0.rank_id.role_id, RoleId::new(1));
        assert_eq!(rank_list.rank_order.iter().nth(0).unwrap().rank_id.role_id, RoleId::new(1));
    }

    #[test]
    pub fn add_rank_then_add_rank_again_with_new_wc_updates()
    {
        let first_rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 100,
        };

        let second_rank = Rank {
            rank_id: RankId {
                guild_id: 1.into(),
                role_id: 1.into(),
            },
            minimum_word_count: 200,
        };

        let mut rank_list: RankList = first_rank.into();
        rank_list.add_rank(second_rank).unwrap();
        assert_eq!(rank_list.rank_set.len(), 1);
        assert_eq!(rank_list.rank_order.len(), 1);
        assert_eq!(rank_list.rank_set.iter().nth(0).unwrap().0.minimum_word_count, 200);
        assert_eq!(rank_list.rank_order.iter().nth(0).unwrap().minimum_word_count, 200);
    }
    // Fuuuck we can't actually test saving for now... we really should mock PgPool or something...
}
