CREATE TABLE IF NOT EXISTS files (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT,
    size INTEGER NOT NULL,
    expiration INTEGER NOT NULL,
    short_alias TEXT NOT NULL,
    long_alias TEXT NOT NULL,
    origin TEXT NOT NULL
);