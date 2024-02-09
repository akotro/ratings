mod auth;
mod db_models;
mod db_util;
mod routes;

use std::{
    env,
    sync::{Arc, Mutex},
};

use actix_cors::Cors;
use actix_governor::{
    governor::{clock::QuantaInstant, middleware::NoOpMiddleware},
    Governor, GovernorConfig, GovernorConfigBuilder, PeerIpKeyExtractor,
};
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpResponse, HttpServer,
};
use dotenvy::dotenv;
use env_logger::Env;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
use routes::*;

const JWT_SECRET: &str = "JWT_SECRET";

fn _configure_ssl() -> SslAcceptorBuilder {
    let (key_path, cert_path) = if cfg!(debug_assertions) {
        ("resources/key.pem", "resources/certificate.pem")
    } else {
        ("key.pem", "certificate.pem")
    };
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file(key_path, SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file(cert_path).unwrap();
    builder
}

fn configure_governor() -> GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>> {
    GovernorConfigBuilder::default()
        .permissive(true)
        .per_second(60)
        .burst_size(100)
        .finish()
        .unwrap()
}

fn configure_cors() -> Cors {
    let cors = Cors::default()
        // .allowed_origin("http://64.226.108.119:3000")
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header();
    if cfg!(debug_assertions) {
        cors.allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:4173")
    } else {
        cors
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let secret_key = Data::new(env::var(JWT_SECRET).expect("JWT_SECRET must be set"));

    let db_pool = db_util::init_database().await?;

    // let ssl_builder = configure_ssl();
    let governor_conf = configure_governor();

    let ip_blacklist = Arc::new(Mutex::new(Vec::<String>::new()));
    actix_web::rt::spawn(auth::update_blacklist(
        db_pool.clone(),
        ip_blacklist.clone(),
    ));

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    HttpServer::new(move || {
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
                    .service(get_users_route)
                    .service(delete_user_route)
                    .service(create_restaurant_route)
                    .service(get_restaurants_route)
                    .service(get_restaurants_with_avg_rating_route)
                    .service(get_restaurant_ratings_route)
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
    })
    .bind(("0.0.0.0", 5959))?
    // .bind_openssl("0.0.0.0:5959", ssl_builder)?
    .run()
    .await?;

    Ok(())
}
