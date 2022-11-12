use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SpotifyInfoArtist {
    pub name: String
}

#[derive(Deserialize, Serialize)]
pub struct SpotifyInfoItem {
    pub name: String,
    pub duration_ms: i64,
    pub artists: Vec<SpotifyInfoArtist>,
    pub id: String
}

#[derive(Deserialize, Serialize)]
pub struct SpotifyInfo {
    pub progress_ms: i64,
    pub item: SpotifyInfoItem,
    pub is_playing: bool
}

#[derive(Debug, Deserialize)]
pub struct SpotifyCallbackQuery {
    pub code : String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyAuthTokenPayload {
    pub code: String,
    pub redirect_uri : String,
    pub grant_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyAuthRefreshTokenPayload {
    pub refresh_token: String,
    pub grant_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub scope: String
}

#[derive(Serialize, Deserialize)]
pub struct SpotifyAuthRefreshTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub refresh_token: String,
    pub scope: String
}