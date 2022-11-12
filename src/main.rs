
use std::path::PathBuf;
use std::sync::{Arc};
use std::time::Duration;
use actix_web::{App, HttpServer, web};
use log::{error, info, LevelFilter, warn};
use rosc::OscType;
use simple_logger::SimpleLogger;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use crate::config::config::Config;
use crate::entities::config::ConfigFile;
use crate::entities::spotify::SpotifyInfoArtist;
use crate::http::spotify::{fetch_spotify_info, refresh_authenticate_spotify};
use crate::http::SpotifyValue;
use crate::routes::spotify::{spotify_callback, spotify_setup};
use crate::routes::WebData;
use crate::utils::osc::{encode_packet, send_to_delay};

mod utils;
mod entities;
mod routes;
mod http;
mod config;

struct Chatbox {
    pub artist: String,
    pub song: String,
    pub id: String
}

impl Chatbox {
    pub fn new() -> Self {
        Self {
            artist: "".to_string(),
            song: "".to_string(),
            id: "".to_string()
        }
    }

    pub fn changed(&self, id: &String) -> bool {
        !self.id.eq(id)
    }

    pub fn update(&mut self, artists: &Vec<SpotifyInfoArtist>, song: &String, id: &String) {
        let mut artists_vec = Vec::new();

        for artist in artists {
            artists_vec.push(String::from(&artist.name));
        }

        self.artist = artists_vec.join(", ");

        self.song = String::from(song);
        self.id = String::from(id)
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).env().with_colors(true).init().unwrap();

    info!("Spotify OSC");

    let client = Arc::new(reqwest::Client::new());

    let cfg: Config<ConfigFile> = Config::new(PathBuf::from("config.toml"));

    let sock = Arc::new(UdpSocket::bind(&cfg.cfg.general.host_address).await.unwrap());

    let config = Arc::new(Mutex::new(cfg));

    tokio::task::spawn({
        let sock = sock.clone();
        let config = config.clone();
        let client = client.clone();

        async move {
            let mut chatbox = Chatbox::new();

            loop {
                tokio::time::sleep(Duration::from_secs(4)).await;
                {
                    let mut config = config.lock().await;

                    if !config.cfg.spotify.token.is_empty() || !config.cfg.spotify.refresh_token.is_empty() {
                        match fetch_spotify_info(&client, String::from(&config.cfg.spotify.token)).await {
                            Ok(res) => {

                                match res {
                                    SpotifyValue::INFO(res) => {
                                        let spotify_playing_buff = encode_packet(String::from(&config.cfg.parameters.spotify_playing), vec![OscType::Bool(res.is_playing)]).unwrap();

                                        let seek = res.progress_ms as f32 / res.item.duration_ms as f32;
                                        let spotify_seek_buff = encode_packet(String::from(&config.cfg.parameters.spotify_seek), vec![OscType::Float(seek)]).unwrap();

                                        send_to_delay(&sock, &spotify_playing_buff, &config.cfg.general.client_address, Duration::from_millis(20)).await;
                                        send_to_delay(&sock, &spotify_seek_buff, &config.cfg.general.client_address, Duration::from_millis(20)).await;

                                        if chatbox.changed(&res.item.id) {
                                            chatbox.update(&res.item.artists, &res.item.name, &res.item.id);

                                            let spotify_chatbox_buff = encode_packet(String::from(&config.cfg.parameters.spotify_chatbox),
                                                                                     vec![OscType::String(format!("[Spotify] Playing: {} - {}", chatbox.artist, chatbox.song))]).unwrap();
                                            send_to_delay(&sock, &spotify_chatbox_buff, &config.cfg.general.client_address, Duration::from_millis(20)).await;
                                        }
                                    }
                                    SpotifyValue::EMPTY => {
                                        let spotify_playing_buff = encode_packet(String::from(&config.cfg.parameters.spotify_playing), vec![OscType::Bool(false)]).unwrap();
                                        let spotify_seek_buff = encode_packet(String::from(&config.cfg.parameters.spotify_seek), vec![OscType::Float(0_f32)]).unwrap();

                                        send_to_delay(&sock, &spotify_playing_buff, &config.cfg.general.client_address, Duration::from_millis(20)).await;
                                        send_to_delay(&sock, &spotify_seek_buff, &config.cfg.general.client_address, Duration::from_millis(20)).await;
                                    }
                                }
                            }
                            Err(_) => {
                                warn!("Token expired, attempting to get new token...");

                                match refresh_authenticate_spotify(&client, String::from(&config.cfg.spotify.refresh_token),
                                                           config.cfg.get_auth_base64()).await
                                {
                                    Ok(res) => {
                                        config.cfg.spotify.token = res.access_token;

                                        config.write();
                                        warn!("New token generated!");
                                        continue;
                                    }
                                    Err(_) => {
                                        error!("Something went wrong while generating the new token.");
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    HttpServer::new({
        let client = client.clone();
        let config = config.clone();

        let web_data = WebData {
            client: client.clone(),
            config
        };

        move || {
            App::new()
                .app_data(web::Data::new(web_data.clone()))
                .service(spotify_callback)
                .service(spotify_setup)
        }
    })
        .bind(("127.0.0.1", 8080))
        .unwrap()
        .run()
        .await
        .unwrap();
}