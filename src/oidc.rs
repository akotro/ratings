use actix_web::{cookie, web, HttpRequest, HttpResponse};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest, AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl,
    Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse,
};
use urlencoding::encode;

use crate::{auth::generate_token, auth::validate_token, db_util, models::ApiResponse};
use sqlx::MySqlPool;

#[derive(serde::Deserialize)]
pub struct OidcCallbackQuery {
    code: String,
    state: String,
}

use crate::config::AppConfig;

#[derive(Clone)]
pub struct OidcConfig {
    pub client: openidconnect::core::CoreClient<
        openidconnect::EndpointSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointMaybeSet,
        openidconnect::EndpointMaybeSet,
    >,
    pub provider: String,
    pub issuer_url: String,
    pub redirect_url: String,
    pub frontend_base_url: String,
    pub cookie_domain: Option<String>,
}

pub async fn build_oidc_config(app_config: &AppConfig) -> anyhow::Result<OidcConfig> {
    let client_id = ClientId::new(app_config.oidc_client_id.clone());
    let client_secret = ClientSecret::new(app_config.oidc_client_secret.clone());
    let issuer_url = IssuerUrl::new(app_config.oidc_issuer_url.clone())?;
    let redirect_url = RedirectUrl::new(app_config.oidc_redirect_url.clone())?;

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client).await?;

    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(redirect_url);

    Ok(OidcConfig {
        client,
        provider: app_config.oidc_provider_name.clone(),
        issuer_url: app_config.oidc_issuer_url.clone(),
        redirect_url: app_config.oidc_redirect_url.clone(),
        frontend_base_url: app_config.frontend_base_url.clone(),
        cookie_domain: app_config.cookie_domain.clone(),
    })
}

#[derive(serde::Deserialize)]
pub struct OidcLoginQuery {
    redirect: Option<String>,
}

pub async fn oidc_login(
    query: web::Query<OidcLoginQuery>,
    oidc_config: web::Data<OidcConfig>,
) -> HttpResponse {
    let client = &oidc_config.client;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state_cookie = cookie::Cookie::build("oidc_state", csrf_token.secret().to_string())
        .path("/")
        .http_only(true)
        .secure(true) // NOTE: Set to false if testing locally without HTTPS
        .same_site(cookie::SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(15))
        .finish();

    let nonce_cookie = cookie::Cookie::build("oidc_nonce", nonce.secret().to_string())
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(15))
        .finish();

    let pkce_cookie = cookie::Cookie::build("oidc_pkce_verifier", pkce_verifier.secret())
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(15))
        .finish();

    let redirect_cookie_str = query.redirect.clone().unwrap_or_default();
    let redirect_cookie = cookie::Cookie::build("oidc_redirect", redirect_cookie_str)
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(15))
        .finish();

    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .cookie(state_cookie)
        .cookie(nonce_cookie)
        .cookie(pkce_cookie)
        .cookie(redirect_cookie)
        .finish()
}

pub async fn oidc_callback(
    req: HttpRequest,
    pool: web::Data<MySqlPool>,
    query: web::Query<OidcCallbackQuery>,
    oidc_config: web::Data<OidcConfig>,
) -> HttpResponse {
    let client = &oidc_config.client;

    let state_cookie = req.cookie("oidc_state").map(|c| c.value().to_string());
    if state_cookie.is_none() || state_cookie.is_some_and(|c| c != query.state) {
        return HttpResponse::Found()
            .append_header((
                "Location",
                format!("{}/login?error=csrf_failed", oidc_config.frontend_base_url),
            ))
            .finish();
    }

    let nonce_cookie = req.cookie("oidc_nonce").map(|c| c.value().to_string());
    let nonce = match nonce_cookie {
        Some(n) => Nonce::new(n),
        None => {
            return HttpResponse::Found()
                .append_header((
                    "Location",
                    format!(
                        "{}/login?error=nonce_missing",
                        oidc_config.frontend_base_url
                    ),
                ))
                .finish();
        }
    };

    let pkce_cookie = req
        .cookie("oidc_pkce_verifier")
        .map(|c| c.value().to_string());
    let pkce_verifier = match pkce_cookie {
        Some(v) => PkceCodeVerifier::new(v),
        None => {
            return HttpResponse::Found()
                .append_header((
                    "Location",
                    format!("{}/login?error=pkce_missing", oidc_config.frontend_base_url),
                ))
                .finish();
        }
    };

    let redirect_target = req
        .cookie("oidc_redirect")
        .map(|c| c.value().to_string())
        .filter(|s| !s.is_empty());

    let code = AuthorizationCode::new(query.code.clone());

    let http_client = match reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };

    let token_request = match client.exchange_code(code) {
        Ok(req) => req.set_pkce_verifier(pkce_verifier),
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };

    let token_response = match token_request.request_async(&http_client).await {
        Ok(res) => res,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };

    let id_token = match token_response.id_token() {
        Some(token) => token,
        None => {
            return HttpResponse::BadRequest().json(ApiResponse::<()>::error("No ID token".into()))
        }
    };
    let id_token_verifier = client.id_token_verifier();
    let id_token_signing_alg = match id_token.signing_alg() {
        Ok(sk) => sk,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };
    let id_token_signing_key = match id_token.signing_key(&id_token_verifier) {
        Ok(sk) => sk,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };

    let claims = match id_token.claims(&id_token_verifier, &nonce) {
        Ok(claims) => claims,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };

    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = match AccessTokenHash::from_token(
            token_response.access_token(),
            id_token_signing_alg,
            id_token_signing_key,
        ) {
            Ok(actual) => actual,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(e.to_string()))
            }
        };
        if actual_access_token_hash != *expected_access_token_hash {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Invalid Access Token".into()));
        }
    }

    let subject = claims.subject().as_str().to_string();
    let provider = oidc_config.provider.clone();

    let mut conn = match db_util::get_connection(&pool).await {
        Some(conn) => conn,
        None => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("DB Error".into()))
        }
    };

    let cookie_domain = oidc_config.cookie_domain.clone();

    let create_auth_cookie = |token: String| {
        let mut b = cookie::Cookie::build("token", token)
            .path("/")
            .http_only(false)
            .secure(true)
            .same_site(cookie::SameSite::Lax);
        if let Some(d) = &cookie_domain {
            b = b.domain(d.clone());
        }
        b.finish()
    };

    let create_color_cookie = |color: String| {
        let safe_color = urlencoding::encode(&color).into_owned();
        let mut b = cookie::Cookie::build("color", safe_color)
            .path("/")
            .http_only(false)
            .secure(true)
            .same_site(cookie::SameSite::Lax);
        if let Some(d) = &cookie_domain {
            b = b.domain(d.clone());
        }
        b.finish()
    };

    let clear_cookie = |name: &'static str| {
        cookie::Cookie::build(name, "")
            .path("/")
            .max_age(cookie::time::Duration::ZERO)
            .finish()
    };

    let user_opt = match db_util::get_user_by_oidc(&mut conn, &provider, &subject).await {
        Ok(u) => u,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()))
        }
    };

    if let Some(user) = user_opt {
        let token = generate_token(&req, user.id.clone(), user.username.clone());

        let mut frontend_url = format!("{}/login?oidc_success=true", oidc_config.frontend_base_url);
        if let Some(rt) = &redirect_target {
            frontend_url = format!("{}&redirect={}", frontend_url, encode(rt));
        }

        return HttpResponse::Found()
            .append_header(("Location", frontend_url))
            .cookie(create_auth_cookie(token))
            .cookie(create_color_cookie(user.color))
            .cookie(clear_cookie("oidc_state"))
            .cookie(clear_cookie("oidc_nonce"))
            .cookie(clear_cookie("oidc_pkce_verifier"))
            .cookie(clear_cookie("oidc_redirect"))
            .finish();
    }

    if let Ok(user_claims) = validate_token(&req) {
        match db_util::link_oidc_to_user(&mut conn, &user_claims.id, &provider, &subject).await {
            Ok(_) => {
                let token =
                    generate_token(&req, user_claims.id.clone(), user_claims.username.clone());

                let mut frontend_url =
                    format!("{}/login?oidc_success=true", oidc_config.frontend_base_url);
                if let Some(rt) = &redirect_target {
                    frontend_url = format!("{}&redirect={}", frontend_url, encode(rt));
                }

                return HttpResponse::Found()
                    .append_header(("Location", frontend_url))
                    .cookie(create_auth_cookie(token))
                    .cookie(clear_cookie("oidc_state"))
                    .cookie(clear_cookie("oidc_nonce"))
                    .cookie(clear_cookie("oidc_pkce_verifier"))
                    .cookie(clear_cookie("oidc_redirect"))
                    .finish();
            }
            Err(_e) => {
                let frontend_url =
                    format!("{}/login?error=link_failed", oidc_config.frontend_base_url);

                return HttpResponse::Found()
                    .append_header(("Location", frontend_url))
                    .cookie(clear_cookie("oidc_state"))
                    .cookie(clear_cookie("oidc_nonce"))
                    .cookie(clear_cookie("oidc_pkce_verifier"))
                    .cookie(clear_cookie("oidc_redirect"))
                    .finish();
            }
        }
    }

    let frontend_url = format!(
        "{}/profile?oidc_pending=true&provider={}&subject={}",
        oidc_config.frontend_base_url,
        encode(&provider),
        encode(&subject)
    );

    HttpResponse::Found()
        .append_header(("Location", frontend_url))
        .cookie(clear_cookie("oidc_state"))
        .cookie(clear_cookie("oidc_nonce"))
        .cookie(clear_cookie("oidc_pkce_verifier"))
        .cookie(clear_cookie("oidc_redirect"))
        .finish()
}

#[derive(serde::Deserialize)]
pub struct LinkOidcBody {
    provider: String,
    subject: String,
}

pub async fn link_oidc_account(
    req: HttpRequest,
    pool: web::Data<MySqlPool>,
    body: web::Json<LinkOidcBody>,
) -> HttpResponse {
    let user_claims = match validate_token(&req) {
        Ok(c) => c,
        Err(e) => return e,
    };

    let mut conn = match db_util::get_connection(&pool).await {
        Some(conn) => conn,
        None => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("DB Error".into()));
        }
    };

    let existing = match db_util::get_user_by_oidc(&mut conn, &body.provider, &body.subject).await {
        Ok(u) => u,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(e.to_string()));
        }
    };

    if existing.is_some() {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "OIDC account already linked to another user".into(),
        ));
    }

    match db_util::link_oidc_to_user(&mut conn, &user_claims.id, &body.provider, &body.subject)
        .await
    {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string())),
    }
}
