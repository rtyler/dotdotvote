use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Poll {
    pub poll: crate::models::Poll,
    pub choices: Vec<crate::models::Choice>,
}

#[derive(Debug, Deserialize)]
pub struct InsertablePoll {
    pub poll: crate::models::InsertablePoll,
    /**
     * Just the details of each choice
     */
    pub choices: Vec<String>,
}
