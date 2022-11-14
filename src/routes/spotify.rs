use actix_web::{HttpResponse, Responder, web, get};
use reqwest::header;
use crate::entities::spotify::{SpotifyCallbackQuery};
use crate::routes::WebData;

#[get("/callback")]
pub async fn spotify_callback(query: web::Query<SpotifyCallbackQuery>, data: web::Data<WebData>) -> impl Responder {
    let spotify = data.spotify.clone();

    let mut spotify = spotify.lock().await;

    match spotify.init_credentials(&query.code).await {
        Ok(_) => {
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