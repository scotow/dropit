CREATE TABLE IF NOT EXISTS files (
    id TEXT NOT NULL PRIMARY KEY,
    admin TEXT NOT NULL,
    origin TEXT NOT NULL,
    expiration INTEGER NOT NULL,
    name TEXT,
    size INTEGER NOT NULL,
    short_alias TEXT NOT NULL,
    long_alias TEXT NOT NULL,
    downloads INTEGER
);