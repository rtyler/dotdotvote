use chrono;
use diesel::prelude::*;
use crate::schema::*;

#[derive(Queryable, Associations)]
pub struct Poll {
    id: i32,
    uuid: String,
    title: String,
    created_at: chrono::NaiveDate,
}

#[derive(Queryable, Associations)]
#[belongs_to(Poll)]
pub struct Choice {
    id: i32,
    details: String,
    poll_id: i32,
    created_at: chrono::NaiveDate,
}

#[derive(Queryable)]
pub struct Vote {
    id: i32,
    voter: String,
    choice: Choice,
    created_at: chrono::NaiveDate,
}
