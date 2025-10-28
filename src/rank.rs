//! This module handles and defines ranks. A rank consists of a Discord role and a word count.
//! The word count is the minimum needed to have that rank. So the first and lowest rank should
//! have a word count of 0, so on and so forth.

use poise::serenity_prelude as serenity;

use crate::mock::GuildLike;
use crate::mock::RoleLike;

/// A rank, which is a pair of a role and a word count.
pub struct Rank<'a, T: RoleLike>
{
    role: &'a T,
    minimum_word_count: u32,
}

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


/// A minimal version of [Rank] which uses [serenity::RoleId] instead of [serenity::Role]
pub struct RankId
{
	guild_id: serenity::GuildId,
    role_id: serenity::RoleId,
    minimum_word_count: u32,
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
}
