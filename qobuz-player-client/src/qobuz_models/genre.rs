use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenreFeaturedResponse {
    pub albums: GenreFeaturedAlbums,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenreFeaturedAlbums {
    pub items: Vec<super::album_suggestion::AlbumSuggestion>,
}
