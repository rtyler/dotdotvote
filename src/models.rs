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

#[derive(Queryable, Associations)]
#[belongs_to(Poll)]
pub struct Choice {
    id: i32,
    details: String,
    poll_id: i32,
    created_at: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct Vote {
    id: i32,
    voter: String,
    choice_id: i32,
    poll_id: i32,
    created_at: DateTime<Utc>,
}
