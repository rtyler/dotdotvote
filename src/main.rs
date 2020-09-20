#[macro_use]
extern crate diesel;

use async_std::task;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use dotenv::dotenv;
use log::*;
use sqlx::postgres::PgPool;
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
 * Struct for carrying application state into tide request handlers
 */
#[derive(Clone, Debug)]
pub struct AppState {
    pub db: sqlx::Pool<sqlx::PgConnection>,
}

/**
 * Create the sqlx connection pool for postgresql
 */
async fn create_pool() -> Result<sqlx::Pool<sqlx::PgConnection>, sqlx::Error> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    PgPool::builder()
        .max_size(5)
        .build(&database_url).await
}

mod dao {
    use chrono::{DateTime, Utc};
    use serde::Serialize;
    use uuid::Uuid;

    #[derive(Clone, Debug, Serialize)]
    pub struct Poll {
        pub id: i32,
        pub uuid: Uuid,
        pub title: String,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Clone, Debug, Serialize)]
    pub struct Choice{
        pub id: i32,
        pub poll_id: i32,
        pub details: String,
        pub created_at: DateTime<Utc>,
    }
}

/**
 * The json module contains all the JSON API stubs for requests and responses
 *
 * Each are named (hopefully) appropriately
 */
mod json {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    pub struct PollResponse {
        pub poll: crate::dao::Poll,
        pub choices: Vec<crate::dao::Choice>,
    }

    #[derive(Deserialize)]
    pub struct PollCreateRequest {
        pub title: String,
        pub choices: Vec<String>,
    }
}

/**
 * The routes module contains all the tide routes and the logic to fulfill the responses for each
 * route.
 *
 * Modules are nested for cleaner organization here
 */
mod routes {
    use tide::{Body, Request, StatusCode};

    use crate::AppState;

    /**
    *  GET /
    */
    pub async fn index(req: Request<AppState>) -> Result<String, tide::Error> {
        Ok("Wilkommen".to_string())
    }

    pub mod polls {
        use tide::{Body, Request, Response, StatusCode};
        use log::*;
        use sqlx::prelude::*;
        use uuid::Uuid;

        use crate::AppState;
        /**
        *  PUT /api/v1/polls
        */
        pub async fn create(mut req: Request<AppState>) -> Result<Response, tide::Error> {
            let poll = req.body_json::<crate::json::PollCreateRequest>().await?;
            let mut tx = req.state().db.begin().await?;
            if let Ok(res) = sqlx::query!("INSERT INTO polls (title, uuid) VALUES ($1, $2) RETURNING id", poll.title, Uuid::new_v4())
                .fetch_one(&mut tx)
                .await {

                    let mut commit = true;
                    /*
                     * There doesn't seem to be a cleaner way to do a multiple insert with sqlx
                     * that doesn't involve some string manipulation
                     */
                    for choice in poll.choices.iter() {
                        let cin = sqlx::query!("INSERT INTO choices (poll_id, details) VALUES ($1, $2)", res.id, choice).execute(&mut tx).await;
                        if cin.is_err() {
                            commit = false;
                            break;
                        }
                    }

                    if commit {
                        tx.commit().await?;
                    }

                    let response = Response::builder(StatusCode::Created)
                        .body(Body::from_string("success".to_string()))
                        .build();
                    Ok(response)
            }
            else {
                Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to create"))
            }
        }

        /**
        * GET /api/v1/polls/:uuid
        */
        pub async fn get(req: Request<AppState>) -> Result<Body, tide::Error> {
            let uuid = req.param::<String>("uuid");

            if uuid.is_err() {
                return Err(tide::Error::from_str(StatusCode::BadRequest, "No uuid specified"));
            }

            debug!("Fetching poll: {:?}", uuid);

            match Uuid::parse_str(&uuid.unwrap()) {
                Err(err) => {
                    Err(tide::Error::from_str(StatusCode::BadRequest, "Invalid uuid specified"))
                },
                Ok(uuid) => {
                    let mut tx = req.state().db.begin().await?;
                    let poll = sqlx::query_as!(crate::dao::Poll, "SELECT * FROM polls WHERE uuid = $1", uuid)
                        .fetch_one(&mut tx)
                        .await;

                    if let Ok(poll) = poll {
                        info!("Found poll: {:?}", poll);
                        let mut choices = sqlx::query_as!(crate::dao::Choice,
                            "SELECT * FROM choices WHERE poll_id = $1 ORDER by ID ASC", poll.id)
                            .fetch_all(&mut tx)
                            .await?;

                        let response = crate::json::PollResponse { poll, choices };
                        Body::from_json(&response)
                    }
                    else {
                        Err(tide::Error::from_str(StatusCode::NotFound, "Could not find uuid"))
                    }
                },
            }
        }

        /**
        *  POST /api/v1/polls/:uuid/vote
        */
        pub async fn vote(mut req: Request<AppState>) -> Result<Body, tide::Error> {
            Ok(Body::from_string("Hello".to_string()))
        }
        /**
        *  GET /api/v1/polls/:uuid/results
        */
        pub async fn results(req: Request<AppState>) -> Result<Body, tide::Error> {
            Ok(Body::from_string("Hello".to_string()))
        }
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

    task::spawn_blocking(move || {
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
    }).await
}

/**
 * Look up the poll based on the `uuid` parameter in the request
 */
fn requested_poll(req: &Request<Pool>) -> Option<crate::models::Poll> {
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
 *  POST /api/v1/polls/:uuid/vote
 */
async fn vote_in_poll(mut req: Request<Pool>) -> Result<Body, tide::Error> {
    use crate::models::*;
    use crate::schema::votes::dsl::votes;

    let ballot: crate::api_models::Ballot = req.body_json().await?;

    task::spawn_blocking(move || {
        if let Ok(pgconn) = req.state().get() {
            if let Some(poll) = requested_poll(&req) {
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
    }).await
}

/**
 *  GET /api/v1/polls/:uuid/results
 */
async fn poll_results(req: Request<Pool>) -> Result<Body, tide::Error> {
    use crate::api_models::Tally;
    use crate::models::{Vote, Choice};

    task::spawn_blocking(move || {
        if let Ok(pgconn) = req.state().get() {
            if let Some(poll) = requested_poll(&req) {
                let choices: Vec<Choice> = Choice::belonging_to(&poll).get_results(&pgconn).expect("Failed to look up choices");
                let votes: Vec<Vote> = Vote::belonging_to(&poll).get_results(&pgconn).expect("Failed to look up votes");

                let tally = Tally {
                    poll,
                    choices,
                    votes,
                };

                Ok(Body::from_json(&tally)?)
            }
            else {
                Err(tide::Error::from_str(StatusCode::NotFound, "Failed to find poll"))
            }
        }
        else {
            Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to look up poll"))
        }
    }).await
}



#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();

    match create_pool().await {
        Ok(db) => {
            let state = AppState { db };
            let mut app = tide::with_state(state);
            app.at("/").get(routes::index);
            app.at("/api/v1/polls").put(routes::polls::create);
            app.at("/api/v1/polls/:uuid").get(routes::polls::get);
            app.at("/api/v1/polls/:uuid/vote").post(routes::polls::vote);
            app.at("/api/v1/polls/:uuid/results").get(routes::polls::results);
            app.listen("127.0.0.1:8000").await?;
            Ok(())
        },
        Err(err) => {
            error!("Could not initialize pool! {:?}", err);
            Err(std::io::Error::new(std::io::ErrorKind::Other, err))
        },
    }
}
