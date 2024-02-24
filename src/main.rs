mod auth;
mod db_models;
mod db_util;
mod middleware;
mod models;
mod routes;

use actix_governor::Governor;
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpResponse, HttpServer,
};
use auth::JWT_SECRET;
use dotenvy::dotenv;
use env_logger::Env;
use middleware::{configure_cors, configure_governor, json_error_handler};
use routes::*;
use std::{
    env,
    sync::{Arc, Mutex},
};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    dotenv().ok();
    let secret_key = Data::new(env::var(JWT_SECRET).expect("JWT_SECRET must be set"));

    let db_pool = db_util::init_database().await?;

    let governor_conf = configure_governor();

    let ip_blacklist = Arc::new(Mutex::new(Vec::<String>::new()));
    actix_web::rt::spawn(auth::update_blacklist(
        db_pool.clone(),
        ip_blacklist.clone(),
    ));

    let server_config = HttpServer::new(move || {
        App::new()
            .wrap(Logger::new(
                "%a \"%r\" %s %b %D \"%{Referer}i\" \"%{User-Agent}i\" %U %{r}a",
            ))
            .wrap(Governor::new(&governor_conf))
            .wrap(configure_cors())
            .service(
                web::scope("ratings")
                    .app_data(Data::new(db_pool.clone()))
                    .app_data(Data::new(ip_blacklist.clone()))
                    .app_data(secret_key.clone())
                    .app_data(web::JsonConfig::default().error_handler(json_error_handler))
                    .service(get_users_route)
                    .service(delete_user_route)
                    .service(create_restaurant_route)
                    .service(get_restaurants_route)
                    .service(get_restaurants_with_avg_rating_route)
                    .service(get_restaurant_ratings_route)
                    .service(get_restaurant_ratings_per_period_route)
                    .service(is_restaurant_rating_complete_route)
                    .service(delete_restaurant_route)
                    .service(rate_restaurant_route)
                    .service(get_ratings_route)
                    .service(get_rating_route)
                    .service(update_rating_route)
                    .service(delete_rating_route)
                    .service(
                        web::scope("auth")
                            .service(register_user_route)
                            .service(login_user_route),
                    )
                    .default_service(web::route().to(HttpResponse::NotFound)),
            )
    });

    if cfg!(debug_assertions) {
        server_config.bind(("127.0.0.1", 5959))?.run().await?;
    } else {
        server_config.bind(("0.0.0.0", 5959))?.run().await?;
    }

    Ok(())
}
