CREATE TABLE IF NOT EXISTS sessions (
    token CHAR(64) -- storing 32 bytes in hexadecimal
        PRIMARY KEY,

    user_id BIGINT UNSIGNED,

    created_at DATETIME
        NOT NULL,

    updated_at DATETIME
        NOT NULL,

    INDEX idx_sessions_user_id (user_id),

    CONSTRAINT fk_sessions_user_id
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE CASCADE
);

-- Remove the default (prefer to use UTC times)
ALTER TABLE users
MODIFY COLUMN created_at DATETIME NOT NULL;