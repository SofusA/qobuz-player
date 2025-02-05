use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};
use std::{path::PathBuf, sync::OnceLock};
use tracing::debug;

macro_rules! acquire {
    () => {
        POOL.get().unwrap().acquire().await
    };
}

macro_rules! query {
    ($query:expr, $conn:ident, $value:ident) => {
        sqlx::query!($query, $value)
            .execute(&mut *$conn)
            .await
            .expect("database failure")
    };
}

macro_rules! get_one {
    ($query:expr, $return_type:ident, $conn:ident) => {
        sqlx::query_as!($return_type, $query)
            .fetch_one(&mut *$conn)
            .await
    };
}

static POOL: OnceLock<Pool<Sqlite>> = OnceLock::new();

#[derive(Default, Debug)]
pub struct ApiConfig {
    pub username: Option<String>,
    pub password: Option<String>,
}

pub async fn init() {
    let database_url = if let Ok(url) = std::env::var("DATABASE_URL") {
        PathBuf::from(url.replace("sqlite://", ""))
    } else {
        let mut url = dirs::data_local_dir().unwrap();
        url.push("qobuz-player");

        if !url.exists() {
            std::fs::create_dir_all(&url).expect("failed to create database directory");
        }

        url.push("data.db");

        url
    };

    debug!("DATABASE_URL: {}", database_url.to_string_lossy());

    let options = SqliteConnectOptions::new()
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .filename(database_url)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .expect("failed to open database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migration failed");

    POOL.set(pool).expect("error setting static pool");

    create_config().await;
}

pub async fn set_username(username: String) {
    let mut conn = acquire!().unwrap();
    query!(
        r#"
            UPDATE config
            SET username=?1
            WHERE ROWID = 1
            "#,
        conn,
        username
    );
}

pub async fn set_password(password: String) {
    let mut conn = acquire!().unwrap();
    query!(
        r#"
            UPDATE config
            SET password=?1
            WHERE ROWID = 1
            "#,
        conn,
        password
    );
}

pub async fn create_config() {
    let mut conn = acquire!().unwrap();
    let rowid = 1;
    query!(
        r#"
            INSERT OR IGNORE INTO config (ROWID) VALUES (?1);
            "#,
        conn,
        rowid
    );
}
pub async fn get_config() -> ApiConfig {
    let mut conn = acquire!().unwrap();

    get_one!(
        r#"
            SELECT * FROM config
            WHERE ROWID = 1;
            "#,
        ApiConfig,
        conn
    )
    .unwrap()
}
