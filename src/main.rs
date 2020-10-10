/**
 * This is the main Dot dot vote application.
 *
 * Currently everything is packed into this one file, but there are a couple modules
 * to encapsulate some functionality
 */
#[macro_use]
extern crate serde_json;

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

mod dao {
    use chrono::{DateTime, Utc};
    use serde::Serialize;
    use sqlx::PgPool;
    use std::collections::HashMap;
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

    impl Choice {
        pub async fn for_poll(id: i32, db: &PgPool) -> Result<Vec<Choice>, sqlx::Error> {
            sqlx::query_as!(
                Self,
                "SELECT * FROM choices WHERE poll_id = $1 ORDER by ID ASC",
                id
            )
            .fetch_all(db)
            .await
        }
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

    impl Vote {
        pub async fn for_poll(id: i32, db: &PgPool) -> Result<Vec<Vote>, sqlx::Error> {
            sqlx::query_as!(Self, "SELECT * FROM votes WHERE poll_id = $1", id)
                .fetch_all(db)
                .await
        }
    }

    impl Poll {
        pub async fn create(
            req: crate::msg::PollCreateRequest,
            db: &PgPool,
        ) -> Result<Poll, sqlx::Error> {
            let mut tx = db.begin().await?;
            let poll = sqlx::query_as!(
                Poll,
                "INSERT INTO polls (title, uuid) VALUES ($1, $2) RETURNING *",
                req.title,
                Uuid::new_v4()
            )
            .fetch_one(&mut tx)
            .await?;

            let mut commit = true;
            /*
             * There doesn't seem to be a cleaner way to do a multiple insert with sqlx
             * that doesn't involve some string manipulation
             */
            for choice in req.choices.iter() {
                // Skip any empty choice
                if choice.is_empty() {
                    continue;
                }

                let cin = sqlx::query!(
                    "INSERT INTO choices (poll_id, details) VALUES ($1, $2)",
                    poll.id,
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

            return Ok(poll);
        }

        pub async fn from_uuid(uuid: uuid::Uuid, db: &PgPool) -> Result<Poll, sqlx::Error> {
            sqlx::query_as!(Poll, "SELECT * FROM polls WHERE uuid = $1", uuid)
                .fetch_one(db)
                .await
        }

        /**
         * Vote on the given Poll.
         *
         * The Ballot is exicted to be an map of (choice_id, num_dots)
         */
        pub async fn vote(
            &self,
            voter: &str,
            ballot: HashMap<i32, i32>,
            db: &PgPool,
        ) -> Result<(), sqlx::Error> {
            let mut tx = db.begin().await?;

            for (choice, dots) in ballot.iter() {
                // No need to record an empty vote
                if *dots == 0 {
                    continue;
                }

                sqlx::query!(
                    "
                    INSERT INTO votes (voter, choice_id, poll_id, dots)
                        VALUES ($1, $2, $3, $4)
                ",
                    voter,
                    *choice,
                    self.id,
                    *dots
                )
                .execute(&mut tx)
                .await?;
            }

            tx.commit().await?;
            Ok(())
        }
    }
}

/**
 * The msg module contains all the models necessary to define the request/response
 * messages for the JSON API and web forms
 *
 * Each are named (hopefully) appropriately
 */
mod msg {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize)]
    pub struct PollResponse {
        pub poll: crate::dao::Poll,
        pub choices: Vec<crate::dao::Choice>,
    }

    #[derive(Debug, Deserialize)]
    pub struct PollCreateRequest {
        pub title: String,
        pub choices: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
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

    #[derive(Debug, Serialize)]
    pub struct PollResults {
        pub poll: crate::dao::Poll,
        pub choices: Vec<crate::dao::Choice>,
        pub votes: Vec<crate::dao::Vote>,
    }

    #[derive(Debug, Serialize)]
    pub struct PollResult {
        /// Choice details
        pub details: String,
        pub total: i64,
        pub voters: String,
    }
}

/**
 * The routes module contains all the tide routes and the logic to fulfill the responses for each
 * route.
 *
 * Modules are nested for cleaner organization here
 */
mod routes {
    use crate::AppState;
    use log::*;
    use std::collections::HashMap;
    use tide::{Body, Request, StatusCode};
    use uuid::Uuid;

    /**
     * Helper function to pull out a :uuid parameter from the path
     */
    fn get_uuid_param(req: &Request<AppState<'_>>) -> Result<Uuid, tide::Error> {
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
            Ok(uuid) => Ok(uuid),
        }
    }

    /**
     *  GET /
     */
    pub async fn index(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
        let params = json!({
            "page": "home"
        });
        let mut body = req.state().render("index", &params).await?;
        body.set_mime("text/html");
        Ok(body)
    }

    /**
     *  GET /new
     */
    pub async fn new(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
        let mut body = req.state().render("new", &json!({})).await?;
        body.set_mime("text/html");
        Ok(body)
    }

    /**
     *  GET /poll/:uuid
     */
    pub async fn get_poll(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
        let uuid = get_uuid_param(&req)?;
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
            let response = crate::msg::PollResponse { poll, choices };

            let mut body = req
                .state()
                .render(
                    "view_poll",
                    &json!({
                        "poll" : response,
                        "dots" : 3,
                    }),
                )
                .await?;
            body.set_mime("text/html");
            Ok(body)
        } else {
            Err(tide::Error::from_str(
                StatusCode::NotFound,
                "Could not find uuid",
            ))
        }
    }

    /**
     *  GET /poll/:uuid/results
     */
    pub async fn poll_results(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
        use crate::dao::Choice;
        let uuid = get_uuid_param(&req)?;
        let db = &req.state().db;

        if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
            info!("Found poll: {:?}", poll);
            let choices = Choice::for_poll(poll.id, &db).await?;
            let choices: HashMap<i32, Choice> = choices
                .iter()
                .map(|choice| (choice.id, choice.clone()))
                .collect();

            // Really just using this as an ordering function
            let results = sqlx::query!(
                "SELECT choice_id, sum(dots) as dots, string_agg(voter, ', ') as voters FROM votes
                    WHERE poll_id = $1 GROUP BY choice_id ORDER BY dots DESC;",
                poll.id
            )
            .fetch_all(db)
            .await?;

            let results: Vec<crate::msg::PollResult> = results
                .iter()
                .map(|rec| {
                    let total = rec.dots.unwrap_or(0);

                    crate::msg::PollResult {
                        total,
                        details: choices.get(&rec.choice_id).unwrap().details.clone(),
                        voters: rec.voters.as_ref().unwrap_or(&"".to_string()).to_string(),
                    }
                })
                .collect();
            debug!("results: {:?}", results);

            let mut body = req
                .state()
                .render(
                    "poll_results",
                    &json!({
                        "poll" : poll,
                        "results" : results,
                    }),
                )
                .await?;
            body.set_mime("text/html");
            Ok(body)
        } else {
            Err(tide::Error::from_str(
                StatusCode::NotFound,
                "Could not find uuid",
            ))
        }
    }
    /**
     *  POST /poll/:uuid
     */
    pub async fn vote_for_poll(mut req: Request<AppState<'_>>) -> tide::Result {
        let uuid = get_uuid_param(&req)?;
        let mut votes: HashMap<String, String> = req.body_form().await?;
        let db = &req.state().db;

        if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
            info!("Found poll: {:?}", poll);
            // Pop the name off the hash so the rest will be votes
            let name = votes.remove("name").unwrap_or("Unknown".to_string());
            // TODO better error handline
            let choices = votes
                .iter()
                .map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap()))
                .collect();

            poll.vote(&name, choices, &db).await?;

            Ok(tide::Redirect::new(format!("/poll/{}/results", poll.uuid)).into())
        } else {
            Err(tide::Error::from_str(
                StatusCode::NotFound,
                "Could not find uuid",
            ))
        }
    }

    /**
     *  POST /create
     */
    pub async fn create(mut req: Request<AppState<'_>>) -> tide::Result {
        let params = req.body_string().await?;
        if let Ok(create) = serde_qs::Config::new(5, false)
            .deserialize_str::<crate::msg::PollCreateRequest>(&params)
        {
            log::debug!("create: {:?}", create);
            let poll = crate::dao::Poll::create(create, &req.state().db).await?;
            Ok(tide::Redirect::new(format!("/poll/{}", poll.uuid)).into())
        } else {
            Err(tide::Error::from_str(
                StatusCode::InternalServerError,
                "Could not process form",
            ))
        }
    }

    /**
     *  GET /about
     */
    pub async fn about(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
        let mut body = req.state().render("about", &json!({})).await?;
        body.set_mime("text/html");
        Ok(body)
    }

    pub mod api {
        use log::*;
        use tide::{Body, Request, Response, StatusCode};

        use crate::AppState;
        /**
         *  PUT /api/v1/polls
         */
        pub async fn create(mut req: Request<AppState<'_>>) -> Result<Response, tide::Error> {
            let create = req.body_json::<crate::msg::PollCreateRequest>().await?;
            let poll = crate::dao::Poll::create(create, &req.state().db).await?;

            let response = Response::builder(StatusCode::Created)
                .body(Body::from_string(format!(r#"{{"poll":"{}"}}"#, poll.uuid)))
                .build();
            Ok(response)
        }

        /**
         * GET /api/v1/polls/:uuid
         */
        pub async fn get(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
            let uuid = super::get_uuid_param(&req)?;
            let db = &req.state().db;

            if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                info!("Found poll: {:?}", poll);
                let choices = crate::dao::Choice::for_poll(poll.id, &db).await?;
                let response = crate::msg::PollResponse { poll, choices };

                Body::from_json(&response)
            } else {
                Err(tide::Error::from_str(
                    StatusCode::NotFound,
                    "Could not find uuid",
                ))
            }
        }

        /**
         *  POST /api/v1/polls/:uuid/vote
         */
        pub async fn vote(mut req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
            let uuid = super::get_uuid_param(&req)?;
            let vote: crate::msg::Vote = req.body_json().await?;
            let db = &req.state().db;

            if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                info!("Found poll: {:?}", poll);
                poll.vote(&vote.voter, vote.choices, &db).await?;
                Ok(Body::from_string("success".to_string()))
            } else {
                Err(tide::Error::from_str(
                    StatusCode::NotFound,
                    "Could not find uuid",
                ))
            }
        }

        /**
         *  GET /api/v1/polls/:uuid/results
         */
        pub async fn results(req: Request<AppState<'_>>) -> Result<Body, tide::Error> {
            let uuid = super::get_uuid_param(&req)?;
            let db = &req.state().db;

            if let Ok(poll) = crate::dao::Poll::from_uuid(uuid, db).await {
                let choices = crate::dao::Choice::for_poll(poll.id, &db).await?;
                let votes = crate::dao::Vote::for_poll(poll.id, &db).await?;

                let results = crate::msg::PollResults {
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

#[async_std::main]
async fn main() -> Result<(), tide::Error> {
    pretty_env_logger::init();
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = PgPool::connect(&database_url).await?;
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
    }
    /*
     * All builds will have apidocs, since they're handy
     */
    app.at("/apidocs").serve_dir("apidocs/")?;
    app.at("/static").serve_dir("static/")?;

    debug!("Configuring routes");
    app.at("/").get(routes::index);
    app.at("/new").get(routes::new);
    app.at("/create").post(routes::create);
    app.at("/about").get(routes::about);
    app.at("/poll/:uuid").get(routes::get_poll);
    app.at("/poll/:uuid").post(routes::vote_for_poll);
    app.at("/poll/:uuid/results").get(routes::poll_results);

    app.at("/api/v1/polls").put(routes::api::create);
    app.at("/api/v1/polls/:uuid").get(routes::api::get);
    app.at("/api/v1/polls/:uuid/vote").post(routes::api::vote);
    app.at("/api/v1/polls/:uuid/results")
        .get(routes::api::results);
    app.listen("127.0.0.1:8000").await?;
    Ok(())
}
