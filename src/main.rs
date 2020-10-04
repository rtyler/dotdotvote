/**
 * This is the main Dot dot vote application.
 *
 * Currently everything is packed into this one file, but there are a couple modules
 * to encapsulate some functionality
 */
use async_std::sync::{Arc, RwLock};
use dotenv::dotenv;
use handlebars::Handlebars;
use log::*;
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct AppState<'a> {
    pub db: PgPool,
    hb: Arc<RwLock<Handlebars<'a>>>,
}

impl AppState<'_> {
    fn new(db: PgPool) -> Self {
        Self {
            hb: Arc::new(RwLock::new(Handlebars::new())),
            db: db,
        }
    }

    pub async fn register_templates(&self) -> Result<(), handlebars::TemplateFileError> {
        let mut hb = self.hb.write().await;
        hb.clear_templates();
        hb.register_templates_directory(".hbs", "views")
    }

    pub async fn render(
        &self,
        name: &str,
        data: &serde_json::Value,
    ) -> Result<tide::Body, tide::Error> {
        /*
         * In debug mode, reload the templates on ever render to avoid
         * needing a restart
         */
        #[cfg(debug_assertions)]
        {
            self.register_templates().await;
        }
        let hb = self.hb.read().await;
        let view = hb.render(name, data)?;
        Ok(tide::Body::from_string(view))
    }
}

/**
 * Create the sqlx connection pool for postgresql
 */
async fn create_pool() -> Result<sqlx::PgPool, sqlx::Error> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgPool::connect(&database_url).await
}

mod dao {
    use chrono::{DateTime, Utc};
    use serde::Serialize;
    use sqlx::PgPool;
    use uuid::Uuid;

    #[derive(Clone, Debug, Serialize)]
    pub struct Poll {
        pub id: i32,
        pub uuid: Uuid,
        pub title: String,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Clone, Debug, Serialize)]
    pub struct Choice {
        pub id: i32,
        pub poll_id: i32,
        pub details: String,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Clone, Debug, Serialize)]
    pub struct Vote {
        pub id: i32,
        pub poll_id: i32,
        pub choice_id: i32,
        pub voter: String,
        pub dots: i32,
        pub created_at: DateTime<Utc>,
    }

    impl Poll {
        /*
        pub async fn create(title: &str, tx: &mut (impl sqlx::Connection + Copy + sqlx::executor::RefExecutor<'_>)) -> Result<Poll, sqlx::Error> {
            sqlx::query_as!(Poll,
                "INSERT INTO polls (title, uuid) VALUES ($1, $2) RETURNING *",
                title,
                Uuid::new_v4()
            )
            .fetch_one(tx)
            .await
        }
        */

        pub async fn from_uuid(uuid: uuid::Uuid, db: &PgPool) -> Result<Poll, sqlx::Error> {
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

    #[derive(Serialize)]
    pub struct PollResults {
        pub poll: crate::dao::Poll,
        pub choices: Vec<crate::dao::Choice>,
        pub votes: Vec<crate::dao::Vote>,
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
    pub async fn index(_req: Request<AppState<'_>>) -> Result<String, tide::Error> {
        Ok("Wilkommen".to_string())
    }

    pub mod polls {
        use log::*;
        use tide::{Body, Request, Response, StatusCode};
        use uuid::Uuid;

        use crate::AppState;
        /**
         *  PUT /api/v1/polls
         */
        pub async fn create(mut req: Request<AppState<'_>>) -> Result<Response, tide::Error> {
            let poll = req.body_json::<crate::json::PollCreateRequest>().await?;
            let mut tx = req.state().db.begin().await?;
            if let Ok(res) = sqlx::query!(
                "INSERT INTO polls (title, uuid) VALUES ($1, $2) RETURNING id, uuid",
                poll.title,
                Uuid::new_v4()
            )
            .fetch_one(&mut tx)
            .await
            {
                let mut commit = true;
                /*
                 * There doesn't seem to be a cleaner way to do a multiple insert with sqlx
                 * that doesn't involve some string manipulation
                 */
                for choice in poll.choices.iter() {
                    let cin = sqlx::query!(
                        "INSERT INTO choices (poll_id, details) VALUES ($1, $2)",
                        res.id,
                        choice
                    )
                    .execute(&mut tx)
                    .await;
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
            } else {
                Err(tide::Error::from_str(
                    StatusCode::InternalServerError,
                    "Failed to create",
                ))
            }
        }

        /**
         * GET /api/v1/polls/:uuid
         */
        pub async fn get(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
            let uuid = req.param::<String>("uuid");

            if uuid.is_err() {
                return Err(tide::Error::from_str(
                    StatusCode::BadRequest,
                    "No uuid specified",
                ));
            }

            debug!("Fetching poll: {:?}", uuid);

            match Uuid::parse_str(&uuid.unwrap()) {
                Err(_) => Err(tide::Error::from_str(
                    StatusCode::BadRequest,
                    "Invalid uuid specified",
                )),
                Ok(uuid) => {
                    let db = &req.state().db;

                    if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                        info!("Found poll: {:?}", poll);
                        let choices = sqlx::query_as!(
                            crate::dao::Choice,
                            "SELECT * FROM choices WHERE poll_id = $1 ORDER by ID ASC",
                            poll.id
                        )
                        .fetch_all(db)
                        .await?;

                        let response = crate::json::PollResponse { poll, choices };
                        Body::from_json(&response)
                    } else {
                        Err(tide::Error::from_str(
                            StatusCode::NotFound,
                            "Could not find uuid",
                        ))
                    }
                }
            }
        }

        /**
         *  POST /api/v1/polls/:uuid/vote
         */
        pub async fn vote(mut req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
            let uuid = req.param::<String>("uuid");

            if uuid.is_err() {
                return Err(tide::Error::from_str(
                    StatusCode::BadRequest,
                    "No uuid specified",
                ));
            }

            let vote: crate::json::Vote = req.body_json().await?;

            debug!("Fetching poll: {:?}", uuid);

            match Uuid::parse_str(&uuid.unwrap()) {
                Err(_) => Err(tide::Error::from_str(
                    StatusCode::BadRequest,
                    "Invalid uuid specified",
                )),
                Ok(uuid) => {
                    let db = &req.state().db;

                    if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                        info!("Found poll: {:?}", poll);

                        let mut tx = db.begin().await?;

                        for (choice, dots) in vote.choices.iter() {
                            sqlx::query!(
                                "
                                INSERT INTO votes (voter, choice_id, poll_id, dots)
                                    VALUES ($1, $2, $3, $4)
                            ",
                                vote.voter,
                                *choice,
                                poll.id,
                                *dots
                            )
                            .execute(&mut tx)
                            .await?;
                        }

                        tx.commit().await?;
                        Ok(Body::from_string("success".to_string()))
                    } else {
                        Err(tide::Error::from_str(
                            StatusCode::NotFound,
                            "Could not find uuid",
                        ))
                    }
                }
            }
        }

        /**
         *  GET /api/v1/polls/:uuid/results
         */
        pub async fn results(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
            let uuid = req.param::<String>("uuid");

            if uuid.is_err() {
                return Err(tide::Error::from_str(
                    StatusCode::BadRequest,
                    "No uuid specified",
                ));
            }

            debug!("Fetching poll: {:?}", uuid);

            match Uuid::parse_str(&uuid.unwrap()) {
                Err(_) => Err(tide::Error::from_str(
                    StatusCode::BadRequest,
                    "Invalid uuid specified",
                )),
                Ok(uuid) => {
                    let db = &req.state().db;

                    if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                        let choices = sqlx::query_as!(
                            crate::dao::Choice,
                            "SELECT * FROM choices WHERE poll_id = $1",
                            poll.id
                        )
                        .fetch_all(db)
                        .await?;
                        let votes = sqlx::query_as!(
                            crate::dao::Vote,
                            "SELECT * FROM votes WHERE poll_id = $1",
                            poll.id
                        )
                        .fetch_all(db)
                        .await?;

                        let results = crate::json::PollResults {
                            poll,
                            choices,
                            votes,
                        };
                        Ok(Body::from_json(&results)?)
                    } else {
                        Err(tide::Error::from_str(
                            StatusCode::NotFound,
                            "Could not find uuid",
                        ))
                    }
                }
            }
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();

    match create_pool().await {
        Ok(db) => {
            let state = AppState::new(db);
            let mut app = tide::with_state(state);

            #[cfg(debug_assertions)]
            {
                info!("Enabling a very liberal CORS policy for debug purposes");
                use tide::security::{CorsMiddleware, Origin};
                let cors = CorsMiddleware::new()
                    .allow_methods(
                        "GET, POST, PUT, OPTIONS"
                            .parse::<tide::http::headers::HeaderValue>()
                            .unwrap(),
                    )
                    .allow_origin(Origin::from("*"))
                    .allow_credentials(false);

                app.with(cors);
                app.at("/apidocs").serve_dir("apidocs/");
            }

            debug!("Configuring routes");
            app.at("/").get(routes::index);
            app.at("/api/v1/polls").put(routes::polls::create);
            app.at("/api/v1/polls/:uuid").get(routes::polls::get);
            app.at("/api/v1/polls/:uuid/vote").post(routes::polls::vote);
            app.at("/api/v1/polls/:uuid/results")
                .get(routes::polls::results);
            app.listen("127.0.0.1:8000").await?;
            Ok(())
        }
        Err(err) => {
            error!("Could not initialize pool! {:?}", err);
            Err(std::io::Error::new(std::io::ErrorKind::Other, err))
        }
    }
}
