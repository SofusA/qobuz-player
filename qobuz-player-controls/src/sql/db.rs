use hifirs_qobuz_api::client::ApiConfig;
use once_cell::sync::OnceCell;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};
use std::path::PathBuf;
use tracing::debug;

use crate::{acquire, get_one, query};

static POOL: OnceCell<Pool<Sqlite>> = OnceCell::new();

pub async fn init() {
    let database_url = if let Ok(url) = std::env::var("DATABASE_URL") {
        PathBuf::from(url.replace("sqlite://", ""))
    } else {
        let mut url = dirs::data_local_dir().unwrap();
        url.push("hifi-rs");

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
    if let Ok(mut conn) = acquire!() {
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
}

pub async fn set_password(password: String) {
    if let Ok(mut conn) = acquire!() {
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
}

pub async fn set_user_token(token: &String) {
    if let Ok(mut conn) = acquire!() {
        query!(
            r#"
            UPDATE config
            SET user_token=?1
            WHERE ROWID = 1
            "#,
            conn,
            token
        );
    }
}

pub async fn set_app_id(id: &String) {
    if let Ok(mut conn) = acquire!() {
        query!(
            r#"
            UPDATE config
            SET app_id=?1
            WHERE ROWID = 1
            "#,
            conn,
            id
        );
    }
}

pub async fn set_active_secret(secret: &String) {
    if let Ok(mut conn) = acquire!() {
        query!(
            r#"
            UPDATE config
            SET active_secret=?1
            WHERE ROWID = 1
            "#,
            conn,
            secret
        );
    }
}

pub async fn create_config() {
    if let Ok(mut conn) = acquire!() {
        let rowid = 1;
        query!(
            r#"
            INSERT OR IGNORE INTO config (ROWID) VALUES (?1);
            "#,
            conn,
            rowid
        );
    }
}

pub async fn get_config() -> Option<ApiConfig> {
    if let Ok(mut conn) = acquire!() {
        if let Ok(conf) = get_one!(
            r#"
            SELECT * FROM config
            WHERE ROWID = 1;
            "#,
            ApiConfig,
            conn
        ) {
            Some(conf)
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn close() {
    POOL.get().unwrap().close().await;
}
