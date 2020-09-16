use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::*;

/**
 * Generate a new UUID to be used in the database
 */
fn generate_uuid() -> Uuid {
    Uuid::new_v4()
}

#[derive(Associations, Debug, Identifiable, Queryable, Serialize)]
#[table_name="polls"]
pub struct Poll {
    id: i32,
    uuid: Uuid,
    title: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name="polls"]
pub struct InsertablePoll {
    pub title: String,
    #[serde(skip_deserializing, default="generate_uuid")]
    pub uuid: Uuid,
}

#[derive(Associations, Debug, Identifiable, Queryable, Serialize)]
#[belongs_to(Poll)]
pub struct Choice {
    id: i32,
    details: String,
    poll_id: i32,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name="choices"]
pub struct InsertableChoice {
    pub details: String,
    pub poll_id: i32,
}

#[derive(Associations, Debug, Queryable, Serialize)]
pub struct Vote {
    id: i32,
    voter: String,
    choice_id: i32,
    poll_id: i32,
    dots: i32,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name="votes"]
pub struct InsertableVote {
    pub poll_id: i32,
    pub choice_id: i32,
    pub voter: String,
    pub dots: i32,
}

