-- Add migration script here
ALTER TABLE rank_table ALTER COLUMN minimum_word_count SET NOT NULL;
