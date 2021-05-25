SELECT id, IFNULL(name, long_alias) AS name, size
FROM files
WHERE short_alias = ? OR long_alias = ?;