-- Add migration script here
ALTER TABLE rank_table ADD CONSTRAINT unique_word_count_per_guild UNIQUE (guild_id, minimum_word_count);
