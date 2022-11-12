use actix_web::{HttpResponse, Responder, web, get};
use reqwest::header;
use crate::entities::spotify::{SpotifyCallbackQuery};
use crate::http::spotify::authenticate_spotify;
use crate::routes::WebData;

#[get("/callback")]
pub async fn spotify_callback(query: web::Query<SpotifyCallbackQuery>, data: web::Data<WebData>) -> impl Responder {
    {
        let config = data.config.clone();
        let config = config.lock().await;

        if config.cfg.spotify.client_id.is_empty() || config.cfg.spotify.client_secret.is_empty() {
            return HttpResponse::Ok().body("You're missing the client id or the client secret on the config file. Set them in the config file, restart the app and try again!")
        }
    }

    let client = data.client.clone();

    let config = data.config.clone();
    let mut config = config.lock().await;

    return match authenticate_spotify(&client, String::from(&query.code), String::from(&config.cfg.spotify.callback_url), config.cfg.get_auth_base64()).await {
        Ok(res) => {

            config.cfg.spotify.token = res.access_token;
            config.cfg.spotify.refresh_token = res.refresh_token;

            config.write();

            HttpResponse::Ok().body("You're now authenticated, token has been saved into the configuration file.")
        }
        Err(_) => {
            HttpResponse::Ok().body("Something went wrong, try again UnU")
        }
    }
}

#[get("/setup")]
pub async fn spotify_setup(data: web::Data<WebData>) -> impl Responder {

    let config = data.config.clone();

    let config = config.lock().await;

    let url = format!("https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope=user-read-currently-playing&redirect_uri={}",
                      &config.cfg.spotify.client_id,
                      &config.cfg.spotify.callback_url);


    HttpResponse::PermanentRedirect().insert_header((header::LOCATION, url)).finish()
}