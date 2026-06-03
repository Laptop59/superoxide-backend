TRUNCATE TABLE sessions; -- Clear all entries; we're still developing

ALTER TABLE sessions
DROP PRIMARY KEY,
DROP COLUMN token; -- Remove token

ALTER TABLE sessions
ADD COLUMN id BIGINT UNSIGNED
    PRIMARY KEY
    NOT NULL
    AUTO_INCREMENT, -- New column for ID

ADD COLUMN token_hash BINARY(32)
    NOT NULL
    UNIQUE; -- New column for hash