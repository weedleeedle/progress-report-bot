-- Add migration script here
CREATE TABLE user_table (
    guild_id bigint NOT NULL,
    user_id bigint NOT NULL,
    role_id bigint NOT NULL,
    max_word_count integer,
    current_word_count integer,
    PRIMARY KEY (guild_id, user_id),
    CONSTRAINT guild_id_is_positive CHECK (guild_id > 0),
    CONSTRAINT user_id_is_positive CHECK (user_id > 0),
    CONSTRAINT role_id_is_positive CHECK (role_id > 0),
    CONSTRAINT max_word_count_is_positive CHECK (max_word_count >= 0),
    CONSTRAINT current_word_count_is_positive CHECK (current_word_count >= 0),
    CONSTRAINT max_word_count_geq_current_word_count CHECK (max_word_count >= current_word_count)
)


