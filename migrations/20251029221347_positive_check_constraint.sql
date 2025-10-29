-- Add migration script here
ALTER TABLE rank_table ADD CONSTRAINT guild_id_positive CHECK (guild_id > 0);
ALTER TABLE rank_table ADD CONSTRAINT role_id_positive CHECK (role_id > 0);
ALTER TABLE rank_table ADD CONSTRAINT word_count_non_negative CHECK (minimum_word_count >= 0);
