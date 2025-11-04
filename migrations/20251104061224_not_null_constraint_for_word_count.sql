-- Add migration script here
ALTER TABLE user_table ALTER COLUMN max_word_count SET NOT NULL;
ALTER TABLE user_table ALTER COLUMN current_word_count SET NOT NULL;

