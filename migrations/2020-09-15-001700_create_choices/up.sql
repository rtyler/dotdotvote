CREATE TABLE choices (
    id SERIAL PRIMARY KEY,
    details TEXT NOT NULL,
    poll_id INT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_poll FOREIGN KEY(poll_id) REFERENCES polls(id)
);

