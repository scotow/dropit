SELECT id, IFNULL(name, long_alias) AS name, size
FROM files
WHERE long_alias = ?;