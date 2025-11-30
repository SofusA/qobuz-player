use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
};
use serde_json::json;

use crate::{AppState, ResponseResult, ok_or_broadcast, ok_or_error_component};

pub(crate) fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/album/{id}", get(index))
        .route("/album/{id}/content", get(content))
        .route("/album/{id}/tracks", get(album_tracks_partial))
        .route("/album/{id}/set-favorite", put(set_favorite))
        .route("/album/{id}/unset-favorite", put(unset_favorite))
        .route("/album/{id}/play", put(play))
        .route("/album/{id}/play/{track_position}", put(play_track))
        .route("/album/{id}/link", put(link))
}

async fn play_track(
    State(state): State<Arc<AppState>>,
    Path((id, track_position)): Path<(String, u32)>,
) -> impl IntoResponse {
    state.controls.play_album(&id, track_position);
}

async fn set_favorite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ResponseResult {
    ok_or_broadcast(&state.broadcast, state.client.add_favorite_album(&id).await)?;

    Ok(state.render(
        "toggle-favorite.html",
        &json!({"api": "/album", "id": id, "is_favorite": true}),
    ))
}

async fn unset_favorite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ResponseResult {
    ok_or_broadcast(
        &state.broadcast,
        state.client.remove_favorite_album(&id).await,
    )?;

    Ok(state.render(
        "toggle-favorite.html",
        &json!({"api": "/album", "id": id, "is_favorite": false}),
    ))
}

async fn play(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    state.controls.play_album(&id, 0);
}

async fn link(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    let Some(rfid_state) = state.rfid_state.clone() else {
        return;
    };

    qobuz_player_rfid::link(
        rfid_state,
        qobuz_player_controls::database::LinkRequest::Album(id),
        state.broadcast.clone(),
    )
    .await;
}

async fn index(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    let url = format!("/album/{id}/content");
    state.render("lazy-load-component.html", &json!({"url": url}))
}

async fn content(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> ResponseResult {
    let album_data = ok_or_error_component(&state, state.get_album(&id).await)?;
    let is_favorite = ok_or_error_component(&state, state.is_album_favorite(&id).await)?;

    Ok(state.render(
        "album.html",
        &json!({
            "album": album_data.album,
            "suggested_albums": album_data.suggested_albums,
            "is_favorite": is_favorite,
            "rfid": state.rfid_state.is_some()
        }),
    ))
}

async fn album_tracks_partial(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ResponseResult {
    let album = ok_or_error_component(&state, state.client.album(&id).await)?;

    Ok(state.render(
        "album.html",
        &json!({
            "album": album,
        }),
    ))
}

// #[component]
// fn album(
//     now_playing_id: Option<u32>,
//     album: Album,
//     suggested_albums: Vec<AlbumSimple>,
//     is_favorite: bool,
//     rfid: bool,
// ) -> impl IntoView {
//     let duration = parse_duration(album.duration_seconds);

//     let album_id_clone_1 = album.id.clone();
//     let album_id_clone_2 = album.id.clone();

//     html! {
// <div class="flex flex-wrap gap-4 justify-center items-end w-full p-safe-or-4 *:max-w-sm">
//     <img
//         src=album.image
//         alt=album.title.clone()
//         class="object-contain rounded-lg size-full"
//     />

//     <div class="flex flex-col flex-grow gap-4 items-center w-full">
//         <div class="flex flex-col gap-2 justify-center items-center w-full text-center">
//             <a
//                 href=format!("/artist/{}", album.artist.id)
//                 class="text-gray-400 rounded sm:text-lg"
//             >
//                 {album.artist.name}
//             </a>
//             <span class="text-lg sm:text-xl">{album.title.clone()}</span>
//             <span class="flex gap-2 text-gray-400 sm:text-lg">
//                 <span>{album.release_year}</span>
//                 <span>"•︎"</span>
//                 <span>{format!("{} minutes", duration.minutes)}</span>
//             </span>
//         </div>

//         <ButtonGroup>
//             <button
//                 class=button_class()
//                 hx-swap="none"
//                 hx-put=format!("{}/play", album_id_clone_1)
//             >
//                 <span class="size-6">
//                     <Play />
//                 </span>
//                 <span>Play</span>
//             </button>

//             <ToggleFavorite id=album.id.clone() is_favorite=is_favorite />

//             {rfid
//                 .then_some(
//                     html! {
//                         <button
//                             class=button_class()
//                             hx-swap="none"
//                             hx-put=format!("{}/link", album_id_clone_1)
//                         >
//                             <span class="size-6">
//                                 <Link />
//                             </span>
//                             <span>Link RFID</span>
//                         </button>
//                     },
//                 )}
//         </ButtonGroup>
//     </div>
// </div>
// <div class="flex flex-col gap-4 w-full">
//     <div class="sm:p-4">
//         <AlbumTracks
//             tracks=album.tracks
//             album_id=album_id_clone_2
//             now_playing_id=now_playing_id
//         />
//     </div>

//     {if !suggested_albums.is_empty() {
//         Some(
//             html! {
//                 <div class="flex flex-col gap-2 w-full">
//                     <h3 class="px-4 text-lg">Album suggestions</h3>
//                     <ListAlbumsVertical albums=suggested_albums />
//                 </div>
//             },
//         )
//     } else {
//         None
//     }}
//     <Description description=album.description entity_title=album.title />
// </div>
//     }
// }
