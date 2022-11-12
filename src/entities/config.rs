use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ConfigFileSpotify {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
    pub token: String,
    pub refresh_token: String
}

#[derive(Deserialize, Serialize)]
pub struct ConfigFileParameters {
    pub spotify_playing: String,
    pub spotify_seek: String,
    pub spotify_chatbox: String
}

#[derive(Deserialize, Serialize)]
pub struct ConfigFileGeneralOsc {
    pub host_address: String,
    pub client_address: String,
}

#[derive(Deserialize, Serialize)]
pub struct ConfigFileGeneralWebServer {
    pub host_address: String,
    pub port: u16,
}

#[derive(Deserialize, Serialize)]
pub struct ConfigFileGeneral {
    pub osc: ConfigFileGeneralOsc,
    pub web_server: ConfigFileGeneralWebServer,
}

#[derive(Deserialize, Serialize)]
pub struct ConfigFile {
    pub general: ConfigFileGeneral,
    pub spotify: ConfigFileSpotify,
    pub parameters: ConfigFileParameters
}

pub trait Configuration {
    fn default() -> Self;
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            general: ConfigFileGeneral {
                osc: ConfigFileGeneralOsc {
                    host_address: "127.0.0.1:5568".to_string(),
                    client_address: "127.0.0.1:9000".to_string() },
                web_server: ConfigFileGeneralWebServer {
                    host_address: "127.0.0.1".to_string(),
                    port: 8080
                }
            },

            spotify: ConfigFileSpotify {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                callback_url: "http://localhost:8080/callback".to_string(),
                token: "".to_string(),
                refresh_token: "".to_string()
            },
            parameters: ConfigFileParameters {
                spotify_playing: "/avatar/parameters/spotify_playing".to_string(),
                spotify_seek: "/avatar/parameters/spotify_seek".to_string(),
                spotify_chatbox: "/chatbox/input".to_string()
            }
        }
    }
}

impl ConfigFile {
    pub fn get_auth_base64(&self) -> String {
        base64::encode(format!("{}:{}", &self.spotify.client_id, &self.spotify.client_secret))
    }

    pub fn get_webserver_address(&self) -> (String, u16) {
        (String::from(&self.general.web_server.host_address), self.general.web_server.port)
    }
}

