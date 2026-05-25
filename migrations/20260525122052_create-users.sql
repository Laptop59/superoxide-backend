CREATE TABLE IF NOT EXISTS users (
    id BIGINT UNSIGNED
        AUTO_INCREMENT
        PRIMARY KEY,

    username VARCHAR(32)
        NOT NULL
        UNIQUE,

    password_hash VARCHAR(255)
        NOT NULL,

    created_at DATETIME NOT NULL
        DEFAULT CURRENT_TIMESTAMP,

    roles BIGINT UNSIGNED
        NOT NULL
        DEFAULT 0
);