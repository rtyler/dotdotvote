use dotenv::dotenv;
use log::*;
use sqlx::postgres::PgPool;


type DbPool = sqlx::Pool<sqlx::PgConnection>;

/**
 * Struct for carrying application state into tide request handlers
 */
#[derive(Clone, Debug)]
pub struct AppState {
    pub db: DbPool,
}

/**
 * Create the sqlx connection pool for postgresql
 */
async fn create_pool() -> Result<sqlx::Pool<sqlx::PgConnection>, sqlx::Error> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
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

    impl Poll {
        pub async fn from_uuid(uuid: uuid::Uuid, db: &crate::DbPool) -> Result<Poll, sqlx::Error> {
            sqlx::query_as!(Poll, "SELECT * FROM polls WHERE uuid = $1", uuid)
                .fetch_one(db)
                .await
        }
    }
}

/**
 * The json module contains all the JSON API stubs for requests and responses
 *
 * Each are named (hopefully) appropriately
 */
mod json {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

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

    #[derive(Deserialize)]
    pub struct Vote {
        /**
         * Readable name of the voter
         */
        pub voter: String,
        /**
         * Map of choice IDs and the dots per
         */
        pub choices: HashMap<i32, i32>,
    }
}

/**
 * The routes module contains all the tide routes and the logic to fulfill the responses for each
 * route.
 *
 * Modules are nested for cleaner organization here
 */
mod routes {
    use tide::Request;

    use crate::AppState;

    /**
    *  GET /
    */
    pub async fn index(_req: Request<AppState>) -> Result<String, tide::Error> {
        Ok("Wilkommen".to_string())
    }

    pub mod polls {
        use tide::{Body, Request, Response, StatusCode};
        use log::*;
        use uuid::Uuid;

        use crate::AppState;
        /**
        *  PUT /api/v1/polls
        */
        pub async fn create(mut req: Request<AppState>) -> Result<Response, tide::Error> {
            let poll = req.body_json::<crate::json::PollCreateRequest>().await?;
            let mut tx = req.state().db.begin().await?;
            if let Ok(res) = sqlx::query!("INSERT INTO polls (title, uuid) VALUES ($1, $2) RETURNING id, uuid", poll.title, Uuid::new_v4())
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
                        .body(Body::from_string(format!(r#"{{"poll":"{}"}}"#, res.uuid)))
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
                Err(_) => {
                    Err(tide::Error::from_str(StatusCode::BadRequest, "Invalid uuid specified"))
                },
                Ok(uuid) => {
                    let db = &req.state().db;

                    if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                        info!("Found poll: {:?}", poll);
                        let choices = sqlx::query_as!(crate::dao::Choice,
                            "SELECT * FROM choices WHERE poll_id = $1 ORDER by ID ASC", poll.id)
                            .fetch_all(db)
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
            let uuid = req.param::<String>("uuid");

            if uuid.is_err() {
                return Err(tide::Error::from_str(StatusCode::BadRequest, "No uuid specified"));
            }

            let vote: crate::json::Vote = req.body_json().await?;

            debug!("Fetching poll: {:?}", uuid);

            match Uuid::parse_str(&uuid.unwrap()) {
                Err(_) => {
                    Err(tide::Error::from_str(StatusCode::BadRequest, "Invalid uuid specified"))
                },
                Ok(uuid) => {
                    let db = &req.state().db;

                    if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                        info!("Found poll: {:?}", poll);

                        let mut tx = db.begin().await?;

                        for (choice, dots) in vote.choices.iter() {
                            sqlx::query!("
                                INSERT INTO votes (voter, choice_id, poll_id, dots)
                                    VALUES ($1, $2, $3, $4)
                            ", vote.voter, *choice, poll.id, *dots)
                                .execute(&mut tx)
                                .await?;
                        }

                        tx.commit().await?;
                        Ok(Body::from_string("success".to_string()))
                    }
                    else {
                        Err(tide::Error::from_str(StatusCode::NotFound, "Could not find uuid"))
                    }
                }
            }
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
 *  POST /api/v1/polls/:uuid/vote
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

*/


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
