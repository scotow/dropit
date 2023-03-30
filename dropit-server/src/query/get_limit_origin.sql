SELECT SUM(size) AS size, COUNT(*) AS file
FROM files
WHERE origin = ?;