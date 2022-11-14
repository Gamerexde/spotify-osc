use reqwest::{Client, Error, StatusCode};
use crate::entities::spotify::{SpotifyAuthRefreshTokenPayload, SpotifyAuthRefreshTokenResponse, SpotifyAuthTokenPayload, SpotifyAuthTokenResponse, SpotifyDevices, SpotifyInfo, SpotifyPlayback, SpotifySetActivePayload};
use crate::http::{RequestError, SpotifyValue};

pub async fn fetch_spotify_info(http: &Client, auth: &String) -> Result<SpotifyValue, RequestError> {
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

pub async fn fetch_spotify_devices(http: &Client, auth: &String) -> Result<SpotifyDevices, RequestError> {
    let res = http.get("https://api.spotify.com/v1/me/player/devices")
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .send()
        .await;

    match res {
        Ok(res) => {
            let response = res.json::<SpotifyDevices>().await;
            let response = response.unwrap();

            Ok(response)
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn set_spotify_volume(http: &Client, auth: &String, device_id: &String, volume_percent: u16) -> Result<(), RequestError> {
    let res = http.put("https://api.spotify.com/v1/me/player/volume")
        .query(&[("device_id", device_id)])
        .query(&[("volume_percent", volume_percent.to_string())])
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await;

    match res {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn set_spotify_playback_play(http: &Client, auth: &String, device_id: &String) -> Result<(), RequestError> {
    let res = http.put("https://api.spotify.com/v1/me/player/play")
        .query(&[("device_id", device_id)])
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await;

    match res {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn set_spotify_playback_stop(http: &Client, auth: &String, device_id: &String) -> Result<(), RequestError> {
    let res = http.put("https://api.spotify.com/v1/me/player/pause")
        .query(&[("device_id", device_id)])
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await;

    match res {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn set_spotify_playback_next(http: &Client, auth: &String, device_id: &String) -> Result<(), RequestError> {
    let res = http.post("https://api.spotify.com/v1/me/player/next")
        .query(&[("device_id", device_id)])
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await;

    match res {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn set_spotify_playback_previous(http: &Client, auth: &String, device_id: &String) -> Result<(), RequestError> {
    let res = http.post("https://api.spotify.com/v1/me/player/previous")
        .query(&[("device_id", device_id)])
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await;

    match res {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn set_spotify_active(http: &Client, auth: &String, device_id: &String, keep_state: bool) -> Result<(), RequestError> {

    let payload = SpotifySetActivePayload {
        device_ids: vec![String::from(device_id)],
        play: keep_state
    };

    let payload_data = serde_json::to_string(&payload).unwrap();

    let res = http.put("https://api.spotify.com/v1/me/player")
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload_data)
        .send()
        .await;

    match res {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn get_spotify_playback_state(http: &Client, auth: &String) -> Result<Option<SpotifyPlayback>, RequestError> {
    let res = http.get("https://api.spotify.com/v1/me/player")
        .header(reqwest::header::AUTHORIZATION, format!("{} {}", "Bearer ", auth))
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await;

    match res {
        Ok(res) => {
            let status = res.status();

            if status == StatusCode::NO_CONTENT {
                return Ok(None);
            }

            if !status.is_success() {
                return Err(RequestError::OTHER);
            }

            let response = res.json::<SpotifyPlayback>().await.unwrap();

            Ok(Some(response))
        }
        Err(_) => {
            Err(RequestError::OTHER)
        }
    }
}

pub async fn authenticate_spotify(http: &Client, code: &String, redirect_uri: String, auth: String) -> Result<SpotifyAuthRefreshTokenResponse, Error> {
    let payload = SpotifyAuthTokenPayload {
        code: String::from(code),
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