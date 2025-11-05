-- Add migration script here
CREATE TABLE reports (
    id SERIAL PRIMARY KEY,
    guild_id bigint NOT NULL,
    user_id bigint NOT NULL,
    -- No timezone, stored as UTC.
    time timestamp NOT NULL,
    total_word_count integer NOT NULL,
    submission_note text,
    CONSTRAINT guild_id_is_positive CHECK (guild_id > 0),
    CONSTRAINT user_id_is_positive CHECK (user_id > 0),
    CONSTRAINT word_count_geq_zero CHECK (total_word_count >= 0)
);


