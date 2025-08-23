use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub async fn create_pool(database_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
