-- Add migration script here
CREATE TABLE xp (
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    score INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(user_id, guild_id)
);
