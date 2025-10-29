-- Add migration script here
CREATE TABLE rank_table (
    guild_id bigint NOT NULL,
    rank_id bigint NOT NULL,
    minimum_word_count integer,
    PRIMARY KEY (guild_id, rank_id)
);
