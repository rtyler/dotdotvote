#[macro_use]
extern crate diesel;

use async_std::task;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use dotenv::dotenv;
use log::*;
use tide::{Body, Request, StatusCode};
use uuid::Uuid;

use std::env;

mod api_models;
mod models;
mod schema;

type Pool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;

/**
 * Construct the PostgreSQL connection pool
 *
 * Note: this is used in the tide app state
 */
fn init_db_pool() -> Pool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::new(manager).expect("db pool")
}

/**
 *  GET /
 */
async fn index(req: Request<Pool>) -> Result<String, tide::Error> {
    use crate::schema::polls::dsl::*;

    if let Ok(pgconn) = req.state().get() {
        let total: i64 = polls.count().get_result(&pgconn).expect("Failed to count polls");

        Ok(format!("Found {:?} total polls in system", total))
    }
    else {
        Ok("Failed to get connection".to_string())
    }
}

/**
 *  PUT /api/v1/polls
 */
async fn create_poll(mut req: Request<Pool>) -> Result<tide::Body, tide::Error> {
    use crate::schema::polls::dsl::*;
    use crate::schema::choices::dsl::choices;
    use crate::models::*;

    let poll: crate::api_models::InsertablePoll = req.body_json().await?;
    debug!("Poll received: {:?}", poll);

    if let Ok(pgconn) = req.state().get() {
        pgconn.transaction::<_, tide::Error, _>(|| {
            match diesel::insert_into(polls).values(&poll.poll).get_result::<Poll>(&pgconn) {
                Ok(success) => {
                    poll.choices.iter().map(|details| {
                        let choice = InsertableChoice {
                            poll_id: *success.id(),
                            details: details.to_string(),
                        };
                        // TODO: Handle error
                        let result = diesel::insert_into(choices).values(&choice).execute(&pgconn);
                        debug!("choices insert: {:?}", result);
                    }).collect::<()>();
                    // Once the poll has been creatd, insert the choices
                    debug!("inserted: {:?}", success);
                    Ok(Body::from_json(&success).expect("Failed to serialize"))
                },
                Err(err) => {
                    error!("Failed to insert: {:?}", err);
                    Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to insert!"))
                },
            }
        })
    }
    else {
        Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to get connection!"))
    }
}

/**
 * Look up the poll based on the `uuid` parameter in the request
 */
fn requested_poll(req: &Request<Pool>) -> Option<crate::models::Poll> {
    use crate::models::*;
    use crate::schema::polls::dsl::*;

    let poll_uuid = req.param("uuid");

    if poll_uuid.is_err() {
        warn!("No `uuid` parameter given");
        return None;
    }

    // TODO: error handling on the uuid parse
    let poll_uuid: String = poll_uuid.unwrap();
    let poll_uuid: Uuid = Uuid::parse_str(&poll_uuid).unwrap();

    if let Ok(pgconn) = req.state().get() {
        if let Ok(poll) = polls.filter(uuid.eq(poll_uuid)).first(&pgconn) {
            return Some(poll);
        }
    }
    None
}

/**
 * GET /api/v1/polls/:uuid
 */
async fn get_poll(req: Request<Pool>) -> Result<tide::Body, tide::Error> {
    use crate::models::*;
    use crate::schema::polls::dsl::*;

    // TODO: this is grabbing two connections from the pool, reorder
    if let Ok(pgconn) = req.state().get() {
        if let Some(poll) = requested_poll(&req) {
            let choices: Vec<Choice> = Choice::belonging_to(&poll).get_results(&pgconn).expect("Failed to get relations");

            let response = crate::api_models::Poll {
                poll,
                choices,
            };

            return Ok(Body::from_json(&response)?);
        }
    }
    Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to look up poll!"))
}

/**
 *  POST /api/v1/polls/:uuid/vote
 */
async fn vote_in_poll(mut req: Request<Pool>) -> Result<Body, tide::Error> {
    use crate::models::*;
    use crate::schema::votes::dsl::votes;

    if let Ok(pgconn) = req.state().get() {
        if let Some(poll) = requested_poll(&req) {
            let ballot: crate::api_models::Ballot = req.body_json().await?;
            info!("Ballot received: {:?}", ballot);
            return pgconn.transaction::<_, tide::Error, _>(|| {
                for (choice, dots) in ballot.choices.iter() {
                    let vote = InsertableVote {
                        poll_id: *poll.id(),
                        choice_id: *choice,
                        voter: ballot.voter.clone(),
                        dots: *dots,
                    };

                    match diesel::insert_into(votes).values(&vote).execute(&pgconn) {
                        Ok(success) => debug!("Votes recorded"),
                        Err(err) => {
                            error!("Failed to vote! {:?}", err);
                            return Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to vote"));
                        },
                    }
                }
                Ok(Body::from_string("voted".to_string()))
            });
        }
        else {
            return Err(tide::Error::from_str(StatusCode::NotFound, "Failed to look up poll!"))
        }
    }
    Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to cast ballot"))
}

/**
 *  GET /api/v1/polls/:uuid/results
 */
async fn poll_results(req: Request<Pool>) -> Result<Body, tide::Error> {
    Ok(Body::from_string("Not implemented".to_string()))
}


fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();

    task::block_on(async {
        let mut app = tide::with_state(init_db_pool());
        app.at("/").get(index);
        app.at("/api/v1/polls").put(create_poll);
        app.at("/api/v1/polls/:uuid").get(get_poll);
        app.at("/api/v1/polls/:uuid/vote").post(vote_in_poll);
        app.at("/api/v1/polls/:uuid/results").get(poll_results);
        app.listen("127.0.0.1:8000").await?;
        Ok(())
    })
}
