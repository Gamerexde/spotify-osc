use reqwest::{Client, Error, StatusCode};
use crate::entities::spotify::{SpotifyAuthRefreshTokenPayload, SpotifyAuthRefreshTokenResponse, SpotifyAuthTokenPayload, SpotifyAuthTokenResponse, SpotifyInfo};
use crate::http::{RequestError, SpotifyValue};

pub async fn fetch_spotify_info(http: &Client, auth: String) -> Result<SpotifyValue, RequestError> {
    let res = http.get("https://api.spotify.com/v1/me/player/currently-playing?market=ES")
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .send()
        .await;

    match res {
        Ok(res) => {
            let code = res.status();

            if code == StatusCode::NO_CONTENT {
                return Ok(SpotifyValue::EMPTY)
            }

            if code == StatusCode::UNAUTHORIZED {
                return Err(RequestError::UNAUTHORIZED)
            }

            let response = res.json::<SpotifyInfo>().await;

            if response.is_err() {
                return Ok(SpotifyValue::EMPTY)
            }

            let response = response.unwrap();

            Ok(SpotifyValue::INFO(response))
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn authenticate_spotify(http: &Client, code: String, redirect_uri: String, auth: String) -> Result<SpotifyAuthRefreshTokenResponse, Error> {
    let payload = SpotifyAuthTokenPayload {
        code,
        redirect_uri,
        grant_type: "authorization_code".to_string()
    };

    let payload_data = serde_urlencoded::to_string(payload).unwrap();

    let res = http.post("https://accounts.spotify.com/api/token")
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Basic", auth))
        .body(payload_data)
        .send()
        .await;

    return match res {
        Ok(res) => {

            let response_data: SpotifyAuthRefreshTokenResponse = res.json().await.unwrap();

            Ok(response_data)
        }
        Err(err) => {
            Err(err)
        }
    }
}

pub async fn refresh_authenticate_spotify(http: &Client, code: String, auth: String) -> Result<SpotifyAuthTokenResponse, Error> {
    let payload = SpotifyAuthRefreshTokenPayload {
        refresh_token: code,
        grant_type: "refresh_token".to_string()
    };

    let payload_data = serde_urlencoded::to_string(payload).unwrap();

    let res = http.post("https://accounts.spotify.com/api/token")
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Basic", auth))
        .body(payload_data)
        .send()
        .await;

    return match res {
        Ok(res) => {
            let response_data: SpotifyAuthTokenResponse = res.json().await.unwrap();

            Ok(response_data)
        }
        Err(err) => {
            Err(err)
        }
    }
}