table! {
    choices (id) {
        id -> Int4,
        details -> Text,
        poll_id -> Int4,
        created_at -> Nullable<Timestamptz>,
    }
}

table! {
    polls (id) {
        id -> Int4,
        uuid -> Varchar,
        title -> Text,
        created_at -> Nullable<Timestamptz>,
    }
}

table! {
    votes (id) {
        id -> Int4,
        voter -> Text,
        choice_id -> Int4,
        poll_id -> Int4,
        created_at -> Nullable<Timestamptz>,
    }
}

joinable!(votes -> choices (choice_id));

allow_tables_to_appear_in_same_query!(
    choices,
    polls,
    votes,
);
