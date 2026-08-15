-- The structure is as follows:
--
-- Tests -> Parts -> Sections -> Questions
--          (timed)  (tabbed)

CREATE TABLE IF NOT EXISTS tests (
    id BIGINT UNSIGNED
        AUTO_INCREMENT
        PRIMARY KEY,

    name VARCHAR(255)
        NOT NULL,

    type ENUM('objective', 'subjective')
        NOT NULL,

    created_by BIGINT UNSIGNED
        NULL,

    created_at DATETIME
        NOT NULL,

    updated_at DATETIME
        NOT NULL,

    CONSTRAINT fk_tests_created_by
        FOREIGN KEY (created_by) REFERENCES users(id)
        ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS test_parts (
    id BIGINT UNSIGNED
        AUTO_INCREMENT
        PRIMARY KEY,

    name VARCHAR(255)
        NOT NULL,

    duration_seconds INT UNSIGNED
        NULL,

    test_id BIGINT UNSIGNED
        NOT NULL,

    position INT UNSIGNED
        NOT NULL,

    created_at DATETIME
        NOT NULL,

    updated_at DATETIME
        NOT NULL,

    CONSTRAINT fk_test_parts_test_id
        FOREIGN KEY (test_id) REFERENCES tests(id)
        ON DELETE CASCADE,

    CONSTRAINT uq_test_id_part_position
        UNIQUE (test_id, position)
);

CREATE TABLE IF NOT EXISTS test_sections (
    id BIGINT UNSIGNED
        AUTO_INCREMENT
        PRIMARY KEY,

    name VARCHAR(255)
        NOT NULL,

    part_id BIGINT UNSIGNED
        NOT NULL,

    position INT UNSIGNED
        NOT NULL,

    created_at DATETIME
        NOT NULL,

    updated_at DATETIME
        NOT NULL,

    CONSTRAINT fk_test_sections_part_id
        FOREIGN KEY (part_id) REFERENCES test_parts(id)
        ON DELETE CASCADE,

    CONSTRAINT uq_part_id_section_position
        UNIQUE (part_id, position)
);

CREATE TABLE IF NOT EXISTS questions (
    id BIGINT UNSIGNED
        AUTO_INCREMENT
        PRIMARY KEY,

    section_id BIGINT UNSIGNED
        NOT NULL,

    parent_id BIGINT UNSIGNED -- If root, this is NULL.
        NULL,

    parent_key BIGINT UNSIGNED
        GENERATED ALWAYS AS (
            -- Parent ID will practically never reach 18446744073709551615.
            -- Converts NULL to this value.
            COALESCE(parent_id, 18446744073709551615)
        ) VIRTUAL,

    position INT UNSIGNED
        NOT NULL,

    question_text TEXT
        NOT NULL,

    created_at DATETIME
        NOT NULL,

    updated_at DATETIME
        NOT NULL,

    CONSTRAINT fk_questions_section_id
        FOREIGN KEY (section_id)
            REFERENCES test_sections(id)
        ON DELETE CASCADE,

    -- Don't allow a parent of a question to be in a different section.
    CONSTRAINT uq_question_id_section_id
        UNIQUE (id, section_id),

    CONSTRAINT fk_questions_parent_id_section_id
        FOREIGN KEY (parent_id, section_id)
            REFERENCES questions(id, section_id)
        ON DELETE CASCADE,

    -- If we used parent_id instead, as the database treats NULLs as distinct
    -- here, it would allow two root questions to have the same position.
    -- So the NULL is converted to a NON-NULL value which is parent_key.
    CONSTRAINT uq_section_id_parent_key_position
        UNIQUE (section_id, parent_key, position)
);