use actix_web::{App, HttpServer, web};
use dotenvy::dotenv;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use sqlx::PgPool;
use std::env;
use tracing_actix_web::TracingLogger;

mod auth_handler;
mod db;
mod jwt;
mod middleware;
mod models;

#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
    oauth_client: BasicClient,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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

    let app_state = AppState {
        db_pool: pool,
        oauth_client: client,
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(TracingLogger::default())
            .service(
                web::scope("/auth")
                    .service(auth_handler::github_login)
                    .service(auth_handler::github_callback),
            )
            .service(
                web::scope("/api")
                    .wrap(middleware::Auth)
                    .route("/me", web::get().to(auth_handler::get_me)),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
