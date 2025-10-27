# About

This Discord bot is intended to be a replacement for FrankieBot's progress reports module.
Currently I have no plans of incorporating other FrankieBot modules/features. While there 
is a chance this bot will become a full refactor of FrankieBot, I'm setting out to rebuild 
only the progress reports module. Other replacements might take the form of other Discord bots,
or just extensions of this bot.

I currently don't have a name in mind for the new bot. Something that incorporates "PR" like Prancer, idk.

I'll just call it PRBot for now.

# Command Format

We'll use the newer slash command format for PRBot. As we've discussed, slash commands have some limitations
but none of those should affect PRBot.

Commands include:
- Submitting new progress reports (/report, allowed for anyone)
- Listing previous reports (/listprogress or /listreports)


**Admin Commands**
- Updating user ranks (/updateranks, only allowed for mods/admins)
    - In theory we can limit this command to only be called by people who already have the permission to update ranks,
      but we could limit this further)


**Setup Commands**
- Initial rank setup (/setrank)
    - Used for adding OR changing ranks.
- Remove a rank (/removerank)
- Remove all ranks (/clearranks)
- Set PR channel (for announcements and also submission reports) (/setprchannel)
- Set PR role ping (/setprremindrole)

## Syntax Guide

When I'm demonstrating example command usage, I use `[]` to mean an optional argument that can be excluded
and `<>` to mean a required argument.

## Report Command
```
/report <wordcount> [message]
```

Wordcount can be specified as "total word count" (default)
or "relative word count" by prepending a "+" or "-" to the number.

So `1234` means "A total word count of 1,234 words" and `+1234` means "My previous total word count plus 1,234 words."

```
/report <wordcount_arg>
wordcount_arg := [relative]integer
wordcount := integer
relative := '+' | '-'
```

If a user submits a relative word count but they've never submitted a report before, total and relative word count are the same.
PRBot will treat it as if they had 0 words before.

Subtracting a relative wordcount is supported (though I am not sure if that is something you want). 
This will reduce the user's total word count but will NOT reduce their rank. A user CANNOT be demoted after they've reached a certain rank.

A message or note can be included with the report and will be stored alongside the word count.

## List Progress 
```
/listprogress [user] [from] [to]
```

`/listprogress` or `/listreports` returns a list of the user's previous reports.

This will use [pagination embedded menus](https://docs.rs/poise/latest/poise/builtins/fn.paginate.html) which slash commands support.

When run with no arguments it will get the calling user's reports, listed in reverse chronological order.

### Optional Arguments
```
user - The user to get the progress reports of
from - Return reports starting from this date/time
to - Return reports ending at this date/time
```

I'm a little hesitant to implement complex filtering for this command.
It doesn't seem like most people use this command with arguments? But it can be done.

## Update Ranks
```
/updateranks
```

`/updateranks` behaves exactly as it did before, returning which users have increased their wordcount to the next rank.

It is possible the bot could handle setting ranks itself. See the follow-up section for details on that.

This command can only be called by admins or those with permission to modify a user's roles.

## Setup Commands

These commands all require administrator privledges or similar.

### Set Rank 

```
/setrank <role> <threshold>
```

`/setrank` creates a new rank or edits an existing rank and associates it with the minimum word count needed for that role.
So the first and lowest role would be `/setrank Role 0`. The upper bound for this role would be determined by the next role's wordcount threshold.

`<role>` is *probably* expected to be an actual Discord role, but I think it will also support just any string. I'm not 100% on if Discord and/or poise will let you submit a role as an argument.

This command is used for both adding and editing a rank. If `<role>` exists, it will be updated with the new specified threshold. If it doesn't exist, it will be created.

### Remove Rank

```
/removerank <role>
```

`/removerank` removes an existing rank. As with `/setrank`, `<role>` can be a Discord role or a generic string identifier. In either case, the same name must be used when both adding the role and removing it.

### Remove All Ranks

```
/clearranks
```

`/clearranks` removes all ranks.

### Set PR Channel
```
/setprchannel <channel>
```

`/setprchannel` sets the channel to use for submissions and announcements. 
FrankieBot allows for two different channels for submissions and announcements. 
I currently only have the one for both of them, but this could be split up into two commands, with
`/setprchannel` for submission reports and `/setannouncechannel` for PR window announcements or whatever

### Set PR Role
```
/setprremindrole <role>
```

`/setprremindrole` sets the role to ping when a PR window opens.

# Possible Workflow Changes

In the past, we've discussed a few possible changes to how FrankieBot does progress reports, and this seems like a good opportunity to consider some of them again. 

## Removing PR Windows

We've discussed removing PR windows and instead allowing users to submit reports whenever they want. I don't have any strong arguments for or against PR report windows, but it does make my design a lot easier and requires fewer commands to be registered if we just don't even bother with windows, and there's not a whole lot of reasons to keep them I think? If users just submit reports whenever they want, you can just trigger an update whenever you're looking at updating ranks and it will return whatever the user's rank is at the time.

## Automating Updating Ranks

It might be possible to give the bot permissions to assign ranks automatically. Personally, I've been giving the thought of automating the PBW system but I'm thinking that I won't, since it's an opportunity for me to contribute and I think it's a nice human touch. It would also insulate the cartel from any bugs if PRBot implodes, it won't like... I don't know, delete everyone's roles lol. But I really don't think that would happen, and if PRBot can calculate people's ranks there isn't too much reason why it couldn't also assign them if you wanted.

# Excluded Commands

FrankieBot has a few other commands that I haven't included in this writeup. I excluded them for two reasons: Primarily I have never seen them used, and secondly because I am worried about the statefulness of them.

For example, there is a command for an admin to remove a user's report. What should this do to a user's stored word count? Should it revert the change? This is easy enough to do if a relative word count has been submitted, but what if the user submits a total word count? How does that affect other submissions after it?

The excluded commands are:
- Submitting reports for users (the `.report <user>` version of `.report`)
- Querying windows (the `.query` or `.q` command)
- Editing submisions 
- Deleting submissions
- Window info (the `.info` command)
- Displaying rank info (the `.ranks` command)
    - This one I would actually like to include, but I see it as a "should" not a "need"
      Since the Cartel's ranks are pretty well defined.

# Database Design

This is a more low-level technical discussion, so feel free to skip this if you want. With the previous FrankieBot replacement I switched from using SQLite to PostgreSQL. SQLite works with more or less "normal" files, each of which is treated as a database. This reduces the overhead of SQLite (hence the name), and allows for using multiple databases with them being files. They're also easy to port and move around since they're just files that can be copied.

A database system like PostgreSQL is a lot beefier, which offers some obvious advantages in terms of speed, but it also makes it a lot more... opaque I guess?

One thing I changed with a PostgreSQL system is that multiple databases is discouraged, so instead of every guild/server having its own database, the PostgreSQL database had one database for all guilds it was in, and instead included the guild ID as a filter/key to identify which reports belong to which servers. This changes nothing about the front-end, it works the same either way. And of course, it's unlikely that PRBot will end up in multiple servers anyways, but if it does, those are two ways of working with it.

Here's the tables I'm thinking of using.

## User Table

This table tracks the user's total word count. When updating ranks, this is the table it will refer to.

|guild_id|user_id|max_word_count|current_word_count|current_rank|
|--------|-------|--------------|------------------|------------|
|integer |integer|integer       |integer           |Discord role or string|

The `max_word_count` is used when considering updating ranks. Even if the `current_word_count` is less than the `max_word_count`, the user won't be demoted. But they won't be promoted either until their `current_word_count` exceeds their `max_word_count`. 

A promotion happens when a user's `max_word_count` puts them into a rank that is higher than their `current_rank`. After which the `current_rank` is updated.

## Progress Reports Table

|progress_report_id|guild_id|user_id|time|total_word_count|submission_note|
|------------------|--------|-------|----|----------------|---------------|
|serial            |integer |integer|timestamp|integer    |string         |

Tracks individual progress reports.
`progress_report_id` is an autoincrementing serial which goes from 1 to 2147483647.
If a user submits a relative progress report, it is converted to their project's total word count first
and is stored as such.

## Ranks Table

|guild_id|rank_name|threshold|
|--------|---------|---------|
|integer |string or role id|integer|

Tracks the thresholds between ranks. When `/updateranks` is called, the bot will check this table and the user table to see who has reached a next rank.




