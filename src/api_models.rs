use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/**
 * Information about a poll
 */
#[derive(Debug, Serialize)]
pub struct Poll {
    pub poll: crate::models::Poll,
    pub choices: Vec<crate::models::Choice>,
}

/**
 * User-provided details to create a Poll
 */
#[derive(Debug, Deserialize)]
pub struct InsertablePoll {
    pub poll: crate::models::InsertablePoll,
    /**
     * Just the details of each choice
     */
    pub choices: Vec<String>,
}


/**
 * User-provided ballot with all their votes
 */
#[derive(Debug, Deserialize)]
pub struct Ballot {
    /**
     * Some self-identifying name for the voter
     */
    pub voter: String,
    /**
     * Just a map of choice_ids and the votes per choice
     */
    pub choices: HashMap<i32, i32>,
}


/**
 * Results from a given poll
 */
#[derive(Debug, Serialize)]
pub struct Tally {
    pub poll: crate::models::Poll,
    pub choices: Vec<(crate::models::Choice, u32)>,
}
