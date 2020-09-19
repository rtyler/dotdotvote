CREATE TABLE votes (
    id SERIAL PRIMARY KEY,
    voter TEXT NOT NULL,
    choice_id INT NOT NULL,
    poll_id INT NOT NULL,
    dots INT DEFAULT 0 NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    CONSTRAINT fk_poll FOREIGN KEY(poll_id) REFERENCES polls(id),
    CONSTRAINT fk_choice FOREIGN KEY(choice_id) REFERENCES choices(id)
);
