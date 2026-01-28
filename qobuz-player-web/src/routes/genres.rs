use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use serde_json::json;

use crate::{AppState, GenreAlbums, GenreData, ResponseResult, ok_or_error_page};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/genres", get(index))
        .route("/genres/{id}", get(genre_detail))
}

async fn index(State(state): State<Arc<AppState>>) -> ResponseResult {
    let genres = ok_or_error_page(&state, state.client.genres().await)?;

    let genre_list: Vec<GenreData> = genres
        .into_iter()
        .map(|g| GenreData {
            id: g.id,
            name: g.name,
            slug: g.slug,
        })
        .collect();

    Ok(state.render(
        "genres.html",
        &json!({
            "genres": genre_list,
        }),
    ))
}

async fn genre_detail(
    State(state): State<Arc<AppState>>,
    Path(genre_id): Path<i64>,
) -> ResponseResult {
    let genres = ok_or_error_page(&state, state.client.genres().await)?;
    let genre = genres
        .into_iter()
        .find(|g| g.id == genre_id)
        .ok_or_else(|| {
            state
                .templates
                .borrow()
                .render("error-page.html", &json!({"error": "Genre not found"}))
                .into_response()
        })?;

    let albums = ok_or_error_page(&state, state.client.genre_albums(genre_id).await)?;

    let genre_data = GenreData {
        id: genre.id,
        name: genre.name.clone(),
        slug: genre.slug,
    };

    let genre_albums = GenreAlbums {
        genre: genre_data,
        albums,
    };

    Ok(state.render(
        "genre-detail.html",
        &json!({
            "genre_albums": genre_albums,
        }),
    ))
}
