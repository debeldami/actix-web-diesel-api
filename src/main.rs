use self::models::*;
use actix_web::{web, App, Error, HttpResponse, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use dotenv::dotenv;
use serde::Deserialize;
use std::env;
mod models;
mod schema;
use self::schema::cats::dsl::*;
use validator::Validate;
use validator_derive::Validate;
mod errors;
use self::errors::UserError;
use actix_web::middleware::Logger;
use log::{error, info, warn};

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Deserialize, Validate)]
struct CatEndpointPath {
    #[validate(range(min = 1, max = 150))]
    id: i32,
}

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .app_data(
                web::PathConfig::default().error_handler(|_, _| UserError::ValidationError.into()),
            )
            .route("/cats", web::get().to(cats_endpoint))
            .route("/cat/{id}", web::get().to(cat_endpoint)),
    );
}

pub fn data_setup() -> DbPool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let manager = ConnectionManager::<PgConnection>::new(&database_url);

    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create DB connection pool.");

    return pool;
}

async fn cats_endpoint(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let cats_data = web::block(move || {
        let mut conn = pool.get();

        let connection = conn.as_mut().unwrap();
        cats.limit(100).load::<Cat>(connection)
    })
    .await
    .map_err(|_| {
        error!("Failed to get cats");
        UserError::UnexpectedError
    })?
    .map_err(|_| {
        error!("Failed to get DB connection from pool");
        UserError::DBPoolGetError
    })?;

    Ok(HttpResponse::Ok().json(cats_data))
}

async fn cat_endpoint(
    pool: web::Data<DbPool>,
    cat_id: web::Path<CatEndpointPath>,
) -> Result<HttpResponse, Error> {
    let query_id = cat_id.id.clone();

    cat_id.validate().map_err(|_| {
        warn!("Parameter validation failed");
        UserError::ValidationError
    })?;

    let cat_data = web::block(move || {
        let mut conn = pool.get();

        let connection = conn.as_mut().unwrap();

        cats.filter(id.eq(cat_id.id)).first::<Cat>(connection)
    })
    .await
    .map_err(|_| {
        error!("Cat ID: {} not found in DB", &query_id);
        UserError::UnexpectedError
    })?
    .map_err(|_| {
        error!("Failed to get DB connection from pool");
        UserError::NotFoundError
    })?;

    Ok(HttpResponse::Ok().json(cat_data))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");

    env_logger::init();

    let pool = data_setup();

    info!("Listening on port 8080");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .configure(api_config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{test, App};

    #[actix_rt::test]
    async fn test_cats_endpoint_get() {
        let pool = data_setup();
        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(api_config),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/cats").to_request();

        let resp = test::call_service(&mut app, req).await;

        assert!(resp.status().is_success());
    }
}
