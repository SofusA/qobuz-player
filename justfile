init-database:
    cargo sqlx db create && cargo sqlx migrate run --source qobuz-player-database/migrations
