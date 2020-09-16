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

fn init_db_pool() -> Pool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::new(manager).expect("db pool")
}

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

async fn create_poll(mut req: Request<Pool>) -> Result<tide::Body, tide::Error> {
    use crate::schema::polls::dsl::*;
    use crate::models::*;

    let poll: InsertablePoll = req.body_json().await?;

    if let Ok(pgconn) = req.state().get() {
        match diesel::insert_into(polls).values(&poll).get_result::<Poll>(&pgconn) {
            Ok(success) => {
                info!("inserted: {:?}", success);
                Ok(Body::from_json(&success)?)
            },
            Err(err) => {
                error!("Failed to insert: {:?}", err);
                Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to insert!"))
            },
        }
    }
    else {
        Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to get connection!"))
    }
}

async fn get_poll(req: Request<Pool>) -> Result<tide::Body, tide::Error> {
    use crate::models::*;
    use crate::schema::polls::dsl::*;

    let poll_uuid = req.param("uuid");

    if poll_uuid.is_err() {
        return Err(tide::Error::from_str(StatusCode::BadRequest, "Missing uuid"));
    }

    // TODO: error handling on the uuid parse
    let poll_uuid: String = poll_uuid.unwrap();
    let poll_uuid: Uuid = Uuid::parse_str(&poll_uuid).unwrap();

    if let Ok(pgconn) = req.state().get() {
        let poll: Poll = polls.filter(uuid.eq(poll_uuid))
            .first(&pgconn)
            .expect("Failed to look up uuid");

        let choices: Vec<Choice> = Choice::belonging_to(&poll).get_results(&pgconn).expect("Failed to get relations");

        let response = crate::api_models::Poll {
            poll,
            choices,
        };

        Ok(Body::from_json(&response)?)
    }
    else {
        Err(tide::Error::from_str(StatusCode::InternalServerError, "Failed to get connection!"))
    }
}

fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();

    task::block_on(async {
        let mut app = tide::with_state(init_db_pool());
        app.at("/").get(index);
        app.at("/api/v1/polls").put(create_poll);
        app.at("/api/v1/polls/:uuid").get(get_poll);
        app.listen("127.0.0.1:8000").await?;
        Ok(())
    })
}
