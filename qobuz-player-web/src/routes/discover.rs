use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use serde_json::json;
use tokio::try_join;

use crate::{AppState, Discover, GenreAlbums, GenreData, ResponseResult, ok_or_error_page};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/discover", get(index))
        .route("/discover/genres", get(genres_tab))
        .route("/discover/genres/{id_or_slug}", get(genre_detail))
}

async fn index(State(state): State<Arc<AppState>>) -> ResponseResult {
    let (albums, playlists) = ok_or_error_page(
        &state,
        try_join!(
            state.client.featured_albums(),
            state.client.featured_playlists(),
        ),
    )?;

    let discover = Discover { albums, playlists };

    Ok(state.render(
        "discover.html",
        &json! ({
            "discover": discover,
            "active_tab": "discover",
            "genres": json!(null),
        }),
    ))
}

async fn genres_tab(State(state): State<Arc<AppState>>) -> ResponseResult {
    let genres = ok_or_error_page(&state, state.client.genres().await)?;

    let genre_list: Vec<GenreData> = genres
        .into_iter()
        .map(|g| GenreData {
            id: g.id,
            name: g.name,
            slug: g.slug,
        })
        .collect();
    
    // We still need discover data for the base template structure
    let (albums, playlists) = ok_or_error_page(
        &state,
        try_join!(
            state.client.featured_albums(),
            state.client.featured_playlists(),
        ),
    )?;

    let discover = Discover { albums, playlists };

    Ok(state.render(
        "discover.html",
        &json! ({
            "discover": discover,
            "active_tab": "genres",
            "genres": genre_list,
        }),
    ))
}

async fn genre_detail(
    State(state): State<Arc<AppState>>,
    Path(id_or_slug): Path<String>,
) -> ResponseResult {
    let genres = ok_or_error_page(&state, state.client.genres().await)?;
    
    // Try to parse as ID first, otherwise use as slug
    let genre = if let Ok(genre_id) = id_or_slug.parse::<i64>() {
        // It's a numeric ID
        genres.into_iter().find(|g| g.id == genre_id)
    } else {
        // It's a slug
        genres.into_iter().find(|g| g.slug == id_or_slug)
    };
    
    let genre = genre.ok_or_else(|| {
        state
            .templates
            .borrow()
            .render("error-page.html", &json!({"error": "Genre not found"}))
            .into_response()
    })?;

    let albums = ok_or_error_page(&state, state.client.genre_albums(genre.id).await)?;

    let genre_data = GenreData {
        id: genre.id,
        name: genre.name.clone(),
        slug: genre.slug.clone(),
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
