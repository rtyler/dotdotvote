-- Your SQL goes here

CREATE TABLE votes (
    id SERIAL PRIMARY KEY,
    voter TEXT NOT NULL,
    choice_id INT NOT NULL,
    poll_id INT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_poll FOREIGN KEY(poll_id) REFERENCES polls(id),
    CONSTRAINT fk_choice FOREIGN KEY(choice_id) REFERENCES choices(id)
);
