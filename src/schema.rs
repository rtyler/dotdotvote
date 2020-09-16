table! {
    choices (id) {
        id -> Int4,
        details -> Text,
        poll_id -> Int4,
        created_at -> Timestamptz,
    }
}

table! {
    polls (id) {
        id -> Int4,
        uuid -> Uuid,
        title -> Text,
        created_at -> Timestamptz,
    }
}

table! {
    votes (id) {
        id -> Int4,
        voter -> Text,
        choice_id -> Int4,
        poll_id -> Int4,
        created_at -> Timestamptz,
        dots -> Int4,
    }
}

joinable!(votes -> choices (choice_id));

allow_tables_to_appear_in_same_query!(
    choices,
    polls,
    votes,
);
