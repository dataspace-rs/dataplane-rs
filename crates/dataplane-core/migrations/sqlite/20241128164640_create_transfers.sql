-- Add migration script here

CREATE TABLE IF NOT EXISTS transfers (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    source TEXT NOT NULL,
    token_id TEXT NOT NULL,
    refresh_token_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
)
