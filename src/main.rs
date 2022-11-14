use std::path::PathBuf;
use std::sync::{Arc};
use std::time::Duration;
use actix_web::{App, HttpServer, web};
use log::{error, info, LevelFilter, warn};
use rosc::{OscPacket, OscType};
use simple_logger::SimpleLogger;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::config::config::Config;
use crate::entities::config::ConfigFile;
use crate::entities::spotify::SpotifyInfoArtist;
use crate::managers::spotify::{Spotify, SpotifyAuthError};
use crate::routes::spotify::{spotify_callback, spotify_setup};
use crate::routes::WebData;
use crate::utils::osc::{encode_packet, send_to_delay};

mod utils;
mod entities;
mod routes;
mod http;
mod config;
mod managers;

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

fn task_set_spotify_playback_play(spotify: Arc<Mutex<Spotify>>) -> JoinHandle<()> {
    tokio::task::spawn({

        async move {
            let mut spotify = spotify.lock().await;

            match spotify.get_playback_state().await {
                Ok(res) => {
                    match res {
                        Some(res) => {
                            if !res.is_playing {
                                match spotify.set_playback_play(&res.device.id).await {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                        }
                        None => {
                            match spotify.get_devices().await {
                                Ok(devices) => {
                                    match devices.get_active() {
                                        Some(device) => {
                                            match spotify.set_playback_active(&device.id, true).await {
                                                Ok(_) => {

                                                }
                                                Err(_) => {}
                                            }
                                        }
                                        None => {}
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
    })
}

fn task_set_spotify_playback_pause(spotify: Arc<Mutex<Spotify>>) -> JoinHandle<()> {
    tokio::task::spawn({

        async move {
            let mut spotify = spotify.lock().await;

            match spotify.get_playback_state().await {
                Ok(res) => {
                    match res {
                        Some(res) => {
                            if res.is_playing {
                                match spotify.set_playback_pause(&res.device.id).await {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                        }
                        None => {
                            match spotify.get_devices().await {
                                Ok(devices) => {
                                    match devices.get_active() {
                                        Some(device) => {
                                            match spotify.set_playback_active(&device.id, false).await {
                                                Ok(_) => {

                                                }
                                                Err(_) => {}
                                            }
                                        }
                                        None => {}
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
    })
}

fn task_set_spotify_playback_next(spotify: Arc<Mutex<Spotify>>) -> JoinHandle<()> {
    tokio::task::spawn({

        async move {
            let mut spotify = spotify.lock().await;

            match spotify.get_devices().await {
                Ok(devices) => {
                    match devices.get_active() {
                        Some(device) => {
                            match spotify.set_playback_next(&device.id).await {
                                Ok(_) => {

                                }
                                Err(_) => {}
                            }
                        }
                        None => {}
                    }
                }
                Err(_) => {}
            }
        }
    })
}

fn task_set_spotify_playback_previous(spotify: Arc<Mutex<Spotify>>) -> JoinHandle<()> {
    tokio::task::spawn({

        async move {
            let mut spotify = spotify.lock().await;

            match spotify.get_devices().await {
                Ok(devices) => {
                    match devices.get_active() {
                        Some(device) => {
                            match spotify.set_playback_previous(&device.id).await {
                                Ok(_) => {

                                }
                                Err(_) => {}
                            }
                        }
                        None => {}
                    }
                }
                Err(_) => {}
            }
        }
    })
}

fn task_set_spotify_volume(spotify: Arc<Mutex<Spotify>>,
                           spotify_volume: Arc<Mutex<(f32, f32)>>, spotify_volume_task_active: Arc<Mutex<bool>>
) -> JoinHandle<()> {
    tokio::task::spawn({
        async move {
            loop {
                {
                    let active = spotify_volume_task_active.lock().await;

                    if !*active {
                        continue;
                    }
                }

                let mut spotify_volume = spotify_volume.lock().await;

                if spotify_volume.0 == spotify_volume.1 {
                    continue;
                }

                spotify_volume.0 = spotify_volume.1;

                let volume = spotify_volume.clone();

                drop(spotify_volume);

                let mut spotify = spotify.lock().await;

                match spotify.get_devices().await {
                    Ok(devices) => {
                        let device = devices.get_active();

                        match device {
                            Some(device) => {
                                let volume = (volume.1 * 100_f32) as u16;

                                match spotify.set_volume(&device.id, volume).await {
                                    Ok(_) => {}
                                    Err(_) => {
                                        error!("Something went wrong while setting the volume");
                                    }
                                }

                                let mut active = spotify_volume_task_active.lock().await;
                                *active = false;

                            }
                            None => {}
                        }

                    }
                    Err(_) => {

                    }
                };
            }
        }
    })
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).env().with_colors(true).init().unwrap();

    info!("Spotify OSC");

    let client = Arc::new(reqwest::Client::new());

    let cfg: Config<ConfigFile> = Config::new(PathBuf::from("config.toml"));

    let sock = Arc::new(UdpSocket::bind(&cfg.cfg.general.osc.host_address).await.unwrap());

    let config = Arc::new(Mutex::new(cfg));

    let spotify = Arc::new(Mutex::new(Spotify::new(client.clone(), config.clone())));

    {
        let spotify = spotify.clone();

        let mut spotify = spotify.lock().await;

        match spotify.authenticate().await {
            Ok(_) => {
                info!("Spotify authenticated successfully!");
            }
            Err(err) => {
                match err {
                    SpotifyAuthError::FAILED => {
                        error!("Something went wrong while authenticating...");
                    }
                    SpotifyAuthError::ConfigNotInitialized => {
                        warn!("It appears that you haven't initialized spotify before, don't panic, just make sure to follow the initial setup instructions.");
                    }
                    _ => {}
                }
            }
        }
    }

    tokio::task::spawn({
        let sock = sock.clone();
        let spotify = spotify.clone();
        let config = config.clone();

        let spotify_volume = Arc::new(Mutex::new((0_f32, 0_f32)));
        let spotify_volume_task_active = Arc::new(Mutex::new(false));

        let _ = task_set_spotify_volume(spotify.clone(), spotify_volume.clone(), spotify_volume_task_active.clone());

        async move {
            let mut buf = [0u8; rosc::decoder::MTU];

            loop {
                match sock.recv_from(&mut buf).await {
                    Ok((size, _)) => {
                        let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();

                        match packet {
                            OscPacket::Message(msg) => {
                                let config = config.lock().await;
                                let address = msg.addr.to_string();

                                if address.eq(&config.cfg.parameters.spotify_play) {
                                    let msg = msg.args[0].to_owned();

                                    match msg.bool() {
                                        None => {}
                                        Some(res) => {
                                            if res {
                                                let _ = task_set_spotify_playback_play(spotify.clone());
                                            }
                                        }
                                    }
                                }
                                if address.eq(&config.cfg.parameters.spotify_stop) {
                                    let msg = msg.args[0].to_owned();

                                    match msg.bool() {
                                        None => {}
                                        Some(res) => {
                                            if res {
                                                let _ = task_set_spotify_playback_pause(spotify.clone());
                                            }
                                        }
                                    }
                                }
                                if address.eq(&config.cfg.parameters.spotify_next) {
                                    let msg = msg.args[0].to_owned();

                                    match msg.bool() {
                                        None => {}
                                        Some(res) => {
                                            if res {
                                                let _ = task_set_spotify_playback_next(spotify.clone());
                                            }
                                        }
                                    }
                                }
                                if address.eq(&config.cfg.parameters.spotify_previous) {
                                    let msg = msg.args[0].to_owned();

                                    match msg.bool() {
                                        None => {}
                                        Some(res) => {
                                            if res {
                                                let _ = task_set_spotify_playback_previous(spotify.clone());
                                            }
                                        }
                                    }
                                }
                                if address.eq(&config.cfg.parameters.spotify_volume) {
                                    let msg = msg.args[0].to_owned();

                                    match msg.float() {
                                        None => {}
                                        Some(val) => {
                                            {
                                                let mut spotify_volume = spotify_volume.lock().await;
                                                spotify_volume.1 = val;
                                            }

                                            {
                                                let mut spotify_volume_task_active = spotify_volume_task_active.lock().await;
                                                *spotify_volume_task_active = true;
                                            }

                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {

                    }
                }
            }
        }
    });

    tokio::task::spawn({
        let sock = sock.clone();
        let config = config.clone();
        let spotify = spotify.clone();

        async move {
            let mut chatbox = Chatbox::new();

            loop {
                tokio::time::sleep(Duration::from_secs(4)).await;
                {
                    let mut spotify = spotify.lock().await;

                    match spotify.now_playing().await {
                        Ok(res) => {
                            let config = config.lock().await;

                            match res {
                                Some(res) => {

                                    let spotify_playing_buff = encode_packet(String::from(&config.cfg.parameters.spotify_playing), vec![OscType::Bool(res.is_playing)]).unwrap();

                                    let seek = res.progress_ms as f32 / res.item.duration_ms as f32;
                                    let spotify_seek_buff = encode_packet(String::from(&config.cfg.parameters.spotify_seek), vec![OscType::Float(seek)]).unwrap();

                                    send_to_delay(&sock, &spotify_playing_buff, &config.cfg.general.osc.client_address, Duration::from_millis(20)).await;
                                    send_to_delay(&sock, &spotify_seek_buff, &config.cfg.general.osc.client_address, Duration::from_millis(20)).await;

                                    if chatbox.changed(&res.item.id) {
                                        chatbox.update(&res.item.artists, &res.item.name, &res.item.id);

                                        let spotify_chatbox_buff = encode_packet(String::from(&config.cfg.parameters.spotify_chatbox),
                                                                                 vec![OscType::String(format!("[Spotify] Playing: {} - {}", chatbox.artist, chatbox.song)), OscType::Bool(true)]).unwrap();
                                        send_to_delay(&sock, &spotify_chatbox_buff, &config.cfg.general.osc.client_address, Duration::from_millis(20)).await;
                                    }
                                }
                                None => {
                                    let spotify_playing_buff = encode_packet(String::from(&config.cfg.parameters.spotify_playing), vec![OscType::Bool(false)]).unwrap();
                                    let spotify_seek_buff = encode_packet(String::from(&config.cfg.parameters.spotify_seek), vec![OscType::Float(0_f32)]).unwrap();

                                    send_to_delay(&sock, &spotify_playing_buff, &config.cfg.general.osc.client_address, Duration::from_millis(20)).await;
                                    send_to_delay(&sock, &spotify_seek_buff, &config.cfg.general.osc.client_address, Duration::from_millis(20)).await;
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    });

    let cfg = config.clone();
    let cfg = cfg.lock().await;

    let http_server = HttpServer::new({
        let client = client.clone();
        let config = config.clone();
        let spotify = spotify.clone();

        let web_data = WebData {
            client: client.clone(),
            config,
            spotify
        };

        move || {
            App::new()
                .app_data(web::Data::new(web_data.clone()))
                .service(spotify_callback)
                .service(spotify_setup)
        }
    })
        .bind(cfg.cfg.get_webserver_address())
        .unwrap();

    drop(cfg);

    http_server.run().await.unwrap();
}