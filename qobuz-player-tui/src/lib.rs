use app::{App, AppState};

mod app;
mod ui;

pub async fn init() {
    let mut terminal = ratatui::init();

    let favorites = qobuz_player_controls::favorites().await.unwrap();

    let mut app = App {
        favorites,
        ..Default::default()
    };

    let mut app_state = AppState::default();

    let _app_result = app.run(&mut app_state, &mut terminal).await;
    ratatui::restore();
}
