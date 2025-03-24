-- Add migration script here

CREATE TABLE IF NOT EXISTS tokens (
    transfer_id      TEXT PRIMARY KEY,
    token_id         TEXT NOT NULL,
    refresh_token_id TEXT NOT NULL
)
