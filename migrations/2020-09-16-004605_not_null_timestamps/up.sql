ALTER TABLE choices
    ALTER COLUMN created_at SET NOT NULL;

ALTER TABLE votes
    ALTER COLUMN created_at SET NOT NULL;