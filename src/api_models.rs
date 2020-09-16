use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Poll {
    pub poll: crate::models::Poll,
    pub choices: Vec<crate::models::Choice>,
}
