CREATE TABLE IF NOT EXISTS files (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    size INTEGER NOT NULL,
    short_alias TEXT NOT NULL,
    long_alias TEXT NOT NULL
);