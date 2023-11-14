
use std::future::Future;
use std::ops::Deref;
use std::path::{PathBuf};
use std::str::FromStr;
use std::sync::{Arc};
use log::{error, info, LevelFilter, warn};
use rspotify::{AuthCodeSpotify, Credentials, OAuth, scopes, Token};
use rspotify::clients::{BaseClient, OAuthClient};
use simple_logger::SimpleLogger;
use tiny_http::{Server};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use crate::config::config::Config;
use crate::entities::config::ConfigFile;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tray_item::{IconSource, TrayItem};
use crate::event_loops::http::HttpEventLoop;
use crate::event_loops::osc::OscEventLoop;

mod utils;
mod entities;
mod config;
mod event_loops;
mod clients;

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

    /*
    pub fn update(&mut self, artists: &Vec<SpotifyInfoArtist>, song: &String, id: &String) {
        let mut artists_vec = Vec::new();

        for artist in artists {
            artists_vec.push(String::from(&artist.name));
        }

        self.artist = artists_vec.join(", ");

        self.song = String::from(song);
        self.id = String::from(id)
    }

     */
}

pub struct Tray {
    tray: TrayItem,
    authenticated_label_id: u32
}

impl Tray {
    fn new(tx: &Sender<TrayAction>) -> Self {

        let mut tray = TrayItem::new(
            "Spotify OSC",
            IconSource::Resource("aa-exe-icon"),
        ).unwrap();

        tray.add_label("Spotify OSC").unwrap();
        let authenticated_label_id = tray.inner_mut().add_label_with_id("Auth Status: Not Authenticated").unwrap();

        tray.inner_mut().add_separator().unwrap();


        let setup_tx = tx.clone();
        tray.add_menu_item("Setup", move || {
            setup_tx.blocking_send(TrayAction::OpenSetup).unwrap();
        }).unwrap();

        let quit_tx = tx.clone();
        tray.add_menu_item("Quit", move || {
            quit_tx.blocking_send(TrayAction::Quit).unwrap();
        }).unwrap();


        Self {
            tray,
            authenticated_label_id
        }
    }

    fn set_authenticated_label(&mut self, text: &str) {
        self.tray.inner_mut().set_label(text, self.authenticated_label_id).unwrap();
    }


}

enum TrayAction {
    Quit, Owo, OpenSetup
}


pub struct AppData {
    pub tray: Arc<Mutex<Tray>>,
    pub config: Arc<Mutex<Config<ConfigFile>>>,
    pub spotify: Arc<AuthCodeSpotify>,
    pub http: Arc<Server>,
    pub sock: Arc<UdpSocket>
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).env().with_colors(true).init().unwrap();

    let (tx, mut rx) = mpsc::channel::<TrayAction>(1);

    let mut tray = Arc::new(Mutex::new(Tray::new(&tx)));

    info!("Spotify OSC");

    info!("Starting up...");

    info!("Loading configuration...");
    let config = Arc::new(Mutex::new(Config::<ConfigFile>::new(PathBuf::from("config.toml"))));
    {
        let mut config = config.lock().await;

        match config.reload() {
            Ok(_) => {
                info!("Configuration file loaded!");
            }
            Err(error) => {
                error!("Something went wrong loading the configuration file. Error: {}", error.to_string());
                return;
            }
        }
    }

    let spotify_data: (String, String, String);
    {
        let mut config = config.lock().await;
        let cfg = config.cfg.as_ref().unwrap();

        let client_id = cfg.spotify.client_id.clone();
        let client_secret = cfg.spotify.client_secret.clone();
        let redirect_uri = cfg.spotify.callback_url.clone();

        if client_id.is_empty() || client_secret.is_empty() || redirect_uri.is_empty() {
            error!("The client id, the client secret or the redirect url is missing in the configuration file, set them before starting the application.");
            return;
        }

        spotify_data = (client_id, client_secret, redirect_uri);
    }

    let spotify: Arc<AuthCodeSpotify>;
    {
        let mut config = config.lock().await;
        let mut cfg = config.cfg.as_mut().unwrap();

        let scopes = scopes!("user-read-currently-playing", "user-read-playback-state", "user-modify-playback-state");

        let oauth = OAuth {
            scopes,
            redirect_uri: spotify_data.2,
            ..Default::default()
        };

        let creds = Credentials {
            id: spotify_data.0,
            secret: Some(spotify_data.1)
        };

        spotify = Arc::new(AuthCodeSpotify::new(creds, oauth));

        if !cfg.spotify.refresh_token.is_empty() {
            info!("Refresh token found!");
            let mut token = spotify.token.lock().await.unwrap();

            match token.as_mut() {
                None => {
                    // TODO: This may not work.
                    *token = Some(Token {
                        refresh_token: Some(cfg.spotify.refresh_token.clone()),
                        ..Default::default()
                    })
                }
                Some(token_ref) => {
                    token_ref.refresh_token = Some(cfg.spotify.refresh_token.clone());
                }
            }

            info!("Refreshing Spotify session with refresh token...");

            drop(token);

            match spotify.refresh_token().await {
                Ok(_) => {
                    let mut token = spotify.token.lock().await.unwrap();
                    let mut token = token.as_ref().unwrap();

                    cfg.spotify.token = token.access_token.clone();

                    if let Some(refresh_token) = &token.refresh_token {
                        cfg.spotify.refresh_token = refresh_token.clone();
                    }

                    config.write().unwrap();

                    let mut tray = tray.lock().await;
                    tray.set_authenticated_label("Auth Status: Authenticated");
                    info!("Spotify session refreshed successfully!")
                }
                Err(_) => {
                    warn!("Spotify session couldn't be refreshed, please re-authenticate.")
                }
            }
        }
    }

    let osc_host_address: String;
    {
        let mut config = config.lock().await;
        let mut cfg = config.cfg.as_mut().unwrap();

        osc_host_address = cfg.general.osc.host_address.clone();
    }

    info!("Starting OSC udp socket...");
    let sock = match UdpSocket::bind(osc_host_address).await {
        Ok(socket) => {
            Arc::new(socket)
        }
        Err(_) => {
            error!("Can't open OSC udp socket, this may be due to another program using the same port.");
            panic!();
        }
    };

    info!("OSC udp socket initialized!");

    let address: (String, u16);

    {
        let config = config.lock().await;

        let config = config.cfg.as_ref().unwrap();

        if !config.general.web_server.host_address.is_empty() {
            address = (config.general.web_server.host_address.clone(), config.general.web_server.port);
        } else {
            error!("Host address is empty, please set it.");
            return;
        }
    };

    info!("Starting http server...");
    let http = Arc::new(Server::http(format!("{}:{}", address.0, address.1))
        .expect("Couldn't create HTTP server, check the configuration."));

    let app_data = Arc::new(AppData {
        spotify: spotify.clone(),
        tray: tray.clone(),
        config: config.clone(),
        http: http.clone(),
        sock: sock.clone()
    });

    info!("Web server started on {}:{}", address.0, address.1);

    let mut http_event_loop = HttpEventLoop::new(app_data.clone());

    http_event_loop.start();

    let mut osc_event_loop = OscEventLoop::new(app_data.clone());

    task::spawn({
        let spotify = spotify.clone();

        async move {
            loop {
                match rx.recv().await.unwrap() {
                    TrayAction::Quit => {
                        http_event_loop.stop();
                        osc_event_loop.stop();
                        return;
                    }
                    TrayAction::OpenSetup => {
                        open::that(spotify.get_authorize_url(true).unwrap()).unwrap();
                    }
                    _ => {}
                }
            }
        }
    }).await.unwrap();
}