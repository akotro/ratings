use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub vapid_public_key: String,
    pub vapid_private_key: String,
    pub public_api_base_url: String,
    pub frontend_base_url: String,
    pub cookie_domain: Option<String>,

    pub oidc_provider_name: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_issuer_url: String,
    pub oidc_redirect_url: String,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            jwt_secret: env::var("JWT_SECRET")?,
            vapid_public_key: env::var("PUBLIC_VAPID_PUBLIC_KEY")?,
            vapid_private_key: env::var("VAPID_PRIVATE_KEY")?,
            public_api_base_url: env::var("PUBLIC_API_BASE_URL")?,
            frontend_base_url: env::var("FRONTEND_BASE_URL")?,
            cookie_domain: env::var("PUBLIC_COOKIE_DOMAIN")
                .ok()
                .filter(|s| !s.is_empty()),

            oidc_provider_name: env::var("PUBLIC_OIDC_PROVIDER_NAME")?,
            oidc_client_id: env::var("OIDC_CLIENT_ID")?,
            oidc_client_secret: env::var("OIDC_CLIENT_SECRET")?,
            oidc_issuer_url: env::var("OIDC_ISSUER_URL")?,
            oidc_redirect_url: env::var("OIDC_REDIRECT_URL")?,
        })
    }
}
