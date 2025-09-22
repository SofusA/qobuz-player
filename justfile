init-database:
    cargo sqlx db create && cargo sqlx migrate run --source qobuz-player-controls/migrations

git-reset:
    git fetch && git reset --hard origin

build-gpio:
    cargo build --release --features gpio

build-styles:
    tailwindcss -i qobuz-player-web/assets/tailwind.css -o qobuz-player-web/assets/styles.css --minify
