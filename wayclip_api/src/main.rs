use crate::settings::Settings;
use crate::storage::{LocalStorage, SftpStorage, Storage};
use actix_extensible_rate_limit::{
    RateLimiter,
    backend::{SimpleInputFunctionBuilder, memory::InMemoryBackend},
};
use actix_web::{App, HttpServer, web};
use dotenvy::dotenv;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use std::fs as std_fs;
use std::sync::Arc;
use std::time::Duration;
use tracing_actix_web::TracingLogger;
use wayclip_core::models::SubscriptionTier;

mod auth_handler;
mod clip_handler;
mod db;
mod jwt;
mod middleware;
mod settings;
mod storage;

#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
    oauth_client: BasicClient,
    storage: Arc<dyn Storage>,
    tier_limits: Arc<HashMap<SubscriptionTier, i64>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Settings::new().expect("Failed to load configuration");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool.");

    let github_client_id =
        ClientId::new(env::var("GITHUB_CLIENT_ID").expect("Missing GITHUB_CLIENT_ID"));
    let github_client_secret =
        ClientSecret::new(env::var("GITHUB_CLIENT_SECRET").expect("Missing GITHUB_CLIENT_SECRET"));
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap();
    let token_url =
        TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap();

    let redirect_url = RedirectUrl::new("http://127.0.0.1:8080/auth/callback".to_string()).unwrap();

    let client = BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url);

    let storage: Arc<dyn Storage> = match config.storage_type.as_str() {
        "LOCAL" => Arc::new(LocalStorage::new(&config)),
        "SFTP" => Arc::new(SftpStorage::new(&config)),
        _ => panic!("Invalid STORAGE_TYPE specified"),
    };

    if config.storage_type == "LOCAL" {
        let local_path = config
            .local_storage_path
            .clone()
            .unwrap_or_else(|| "./uploads".to_string());
        std_fs::create_dir_all(&local_path).expect("Could not create local storage directory");
    }

    let tier_limits = Arc::new(config.get_tier_limits());

    let app_state = AppState {
        db_pool: pool,
        oauth_client: client,
        storage,
        tier_limits,
    };

    let app_settings = web::Data::new(config.clone());
    let backend = InMemoryBackend::builder().build();

    HttpServer::new(move || {
        let input = SimpleInputFunctionBuilder::new(Duration::from_secs(3600), 20)
            .real_ip_key()
            .build();
        let ratelimiter = RateLimiter::builder(backend.clone(), input)
            .add_headers()
            .build();
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .app_data(app_settings.clone())
            .wrap(TracingLogger::default())
            .service(
                web::scope("/auth")
                    .service(auth_handler::github_login)
                    .service(auth_handler::github_callback),
            )
            .service(
                web::scope("/api")
                    .wrap(middleware::Auth)
                    .service(auth_handler::get_me)
                    .service(clip_handler::get_clips_index)
                    .service(clip_handler::delete_clip)
                    .service(
                        web::scope("")
                            .wrap(ratelimiter)
                            .service(clip_handler::share_clip),
                    ),
            )
            .service(clip_handler::serve_clip)
            .service(clip_handler::serve_clip_raw)
            .service(clip_handler::report_clip)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
