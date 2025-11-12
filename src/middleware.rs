use actix_cors::Cors;
use actix_governor::{
    governor::{clock::QuantaInstant, middleware::NoOpMiddleware},
    GovernorConfig, GovernorConfigBuilder, PeerIpKeyExtractor,
};
use actix_web::HttpResponse;

use crate::models;

pub fn configure_governor() -> GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>> {
    GovernorConfigBuilder::default()
        .permissive(true)
        .seconds_per_request(60)
        .burst_size(100)
        .finish()
        .unwrap()
}

pub fn configure_cors() -> Cors {
    let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header();
    if cfg!(debug_assertions) {
        cors.allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:5174")
            .allowed_origin("http://localhost:4173")
            .allowed_origin("http://localhost:3000")
    } else {
        cors
    }
}

pub fn json_error_handler(
    err: actix_web::error::JsonPayloadError,
    _req: &actix_web::HttpRequest,
) -> actix_web::Error {
    let message = match &err {
        actix_web::error::JsonPayloadError::ContentType => "Unsupported Media Type".to_string(),
        actix_web::error::JsonPayloadError::Deserialize(json_error) => {
            format!("JSON deserialize error: {}", json_error)
        }
        _ => "Invalid JSON payload".to_string(),
    };

    let response = HttpResponse::BadRequest().json(models::ApiResponse::<()>::error(message));

    actix_web::error::InternalError::from_response("", response).into()
}
