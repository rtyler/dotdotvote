CREATE TABLE polls (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE,
    title TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

