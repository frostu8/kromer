-- Add migration script here
CREATE TABLE reaction_roles (
    -- where to find the message
    guild_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,

    -- the role to give when reacted to
    role_id BIGINT NOT NULL,

    -- the emoji to test for
    emoji BIGINT NOT NULL,

    CONSTRAINT unique_emoji_on_message UNIQUE (message_id, emoji)
);
