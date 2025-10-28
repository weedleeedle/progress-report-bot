//! This module exposes various traits for interfacing with 
//! [serenity::Guild], [serenity::Role], and [[serenity::User]
//! which may be replaced with mock objects as needed for testing

use poise::serenity_prelude as serenity;

/// This trait is used to mock [serenity::Guild].
/// It exposes functions that act like getters for Guild information.
pub trait GuildLike<R: RoleLike>
{
    /// Gets the role or [RoleLike] associated with a [serenity::RoleId].
    /// Note that an Option is returned, returning None if the role_id
    /// was not associated with any existing role.
    /// You also get a reference to a [RoleLike], not the [RoleLike] itself.
    /// A [GuildLike] is considered to be the owner of any [RoleLike] associated with itself,
    /// so references to [RoleLike] must last as long as the [GuildLike] being referenced.
    fn role<'a>(&'a self, role_id: serenity::RoleId) -> Option<&'a R>;

    fn id(&self) -> serenity::GuildId;
}

impl GuildLike<serenity::Role> for serenity::Guild
{
    fn role(&self, role_id: serenity::RoleId) -> Option<&serenity::Role>
    {
        self.roles.get(&role_id)
    }

    fn id(&self) -> serenity::GuildId
    {
        self.id
    }
}

/// This trait is used to mock [serenity::Role].
/// It exposes functions that act like getters for Role information.
pub trait RoleLike
{
    /// Gets the [serenity::RoleId] associated with this [RoleLike]
    fn id(&self) -> serenity::RoleId;

    /// Gets the [serenity::GuildId] associated with this [RoleLike]
    fn guild_id(&self) -> serenity::GuildId;
}

impl RoleLike for serenity::Role
{
    fn id(&self) -> serenity::RoleId
    {
        self.id
    }

    fn guild_id(&self) -> serenity::GuildId
    {
        self.guild_id
    }
}

