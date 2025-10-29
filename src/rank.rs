//! This module handles and defines ranks. A rank consists of a Discord role and a word count.
//! The word count is the minimum needed to have that rank. So the first and lowest rank should
//! have a word count of 0, so on and so forth.

use std::collections::BTreeSet;

use getset::Getters;
use poise::serenity_prelude as serenity;

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
	ranks: BTreeSet<RankId>
}

impl RankList
{
	/// Adds a rank to the rank list. If a rank already exists with the same minimum_word_count, it is replaced with the new one.
	pub fn add_rank(&mut self, rank_id: &RankId)
	{
		self.ranks.replace(*rank_id);
	}

	pub fn add_ranks(&mut self, rank_ids: &[RankId])
	{
		for rank_id in rank_ids
		{
			self.ranks.replace(*rank_id);
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
}

impl From<RankId> for RankList
{
    fn from(value: RankId) -> Self {
		let mut ranks = BTreeSet::<RankId>::new();
		ranks.insert(value);
		Self {
			guild_id: value.guild_id,
			ranks
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
			ranks
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
}
