use std::sync::{Arc};
use reqwest::Client;
use tokio::sync::Mutex;
use crate::config::config::Config;
use crate::entities::config::ConfigFile;
use crate::entities::spotify::{SpotifyDevices, SpotifyInfo, SpotifyPlayback};
use crate::http::spotify::{authenticate_spotify, fetch_spotify_devices, fetch_spotify_info, get_spotify_playback_state, refresh_authenticate_spotify, set_spotify_active, set_spotify_playback_next, set_spotify_playback_play, set_spotify_playback_previous, set_spotify_playback_stop, set_spotify_volume};
use crate::http::{SpotifyValue};

const AUTH_ATTEMPTS: usize = 2;

pub struct Spotify {
    http: Arc<Client>,
    config: Arc<Mutex<Config<ConfigFile>>>,
    pub active: bool,
    pub token: String
}

impl Spotify {
    pub fn new (http: Arc<Client>, config: Arc<Mutex<Config<ConfigFile>>>) -> Self {
        Self {
            http,
            config,
            active: false,
            token: "".to_string()
        }
    }

    pub async fn authenticate(&mut self) -> Result<(), SpotifyAuthError> {
        let mut config = self.config.lock().await;

        if config.cfg.spotify.refresh_token.is_empty() || config.cfg.spotify.client_id.is_empty()
            || config.cfg.spotify.client_secret.is_empty() {
            return Err(SpotifyAuthError::ConfigNotInitialized)
        }

        match refresh_authenticate_spotify(&self.http, String::from(&config.cfg.spotify.refresh_token),
                                           config.cfg.get_auth_base64()).await
        {
            Ok(res) => {
                config.cfg.spotify.token = res.access_token;

                config.write();

                self.token = String::from(&config.cfg.spotify.token);
                self.active = true;
                Ok(())
            }
            Err(_) => {
                Err(SpotifyAuthError::FAILED)
            }
        }
    }

    pub async fn init_credentials(&mut self, code: &String) -> Result<(), SpotifyAuthError> {
        let mut config = self.config.lock().await;

        if config.cfg.spotify.client_id.is_empty() || config.cfg.spotify.client_secret.is_empty()
            || config.cfg.spotify.callback_url.is_empty() {
            return Err(SpotifyAuthError::ConfigNotInitialized)
        }

        match authenticate_spotify(&self.http, &code, String::from(&config.cfg.spotify.callback_url), config.cfg.get_auth_base64()).await {
            Ok(res) => {
                config.cfg.spotify.token = res.access_token;
                config.cfg.spotify.refresh_token = res.refresh_token;

                self.token = String::from(&config.cfg.spotify.token);

                config.write();

                self.active = true;
                Ok(())
            }

            Err(_) => {
                Err(SpotifyAuthError::FAILED)
            }
        }
    }

    pub async fn now_playing(&mut self) -> Result<Option<SpotifyInfo>, SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match fetch_spotify_info(&self.http, &self.token).await {
                Ok(val) => {
                    return match val {
                        SpotifyValue::INFO(res) => {
                            Ok(Some(res))
                        }
                        SpotifyValue::EMPTY => {
                            Ok(None)
                        }
                    }
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            }
        }

        return Err(SpotifyAuthError::FAILED)
    }

    pub async fn get_devices(&mut self) -> Result<SpotifyDevices, SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match fetch_spotify_devices(&self.http, &self.token).await {
                Ok(devices) => {
                    return Ok(devices)
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn set_volume(&mut self, device_id: &String, volume: u16) -> Result<(), SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match set_spotify_volume(&self.http, &self.token, &device_id, volume).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn get_playback_state(&mut self) -> Result<Option<SpotifyPlayback>, SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match get_spotify_playback_state(&self.http, &self.token).await {
                Ok(res) => {
                    return Ok(res);
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn set_playback_active(&mut self, device_id: &String, keep_state: bool) -> Result<(), SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match set_spotify_active(&self.http, &self.token, &device_id, keep_state).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn set_playback_play(&mut self, device_id: &String) -> Result<(), SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match set_spotify_playback_play(&self.http, &self.token, &device_id).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn set_playback_next(&mut self, device_id: &String) -> Result<(), SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match set_spotify_playback_next(&self.http, &self.token, &device_id).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn set_playback_previous(&mut self, device_id: &String) -> Result<(), SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match set_spotify_playback_previous(&self.http, &self.token, &device_id).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }

    pub async fn set_playback_pause(&mut self, device_id: &String) -> Result<(), SpotifyAuthError> {
        if !&self.active {
            return Err(SpotifyAuthError::NotInitialized)
        }

        for _ in 0..AUTH_ATTEMPTS {
            match set_spotify_playback_stop(&self.http, &self.token, &device_id).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    match self.authenticate().await {
                        Ok(_) => {
                            continue;
                        }
                        Err(_) => {
                            return Err(SpotifyAuthError::FAILED);
                        }
                    }
                }
            };
        }

        return Err(SpotifyAuthError::FAILED);
    }
}

pub enum SpotifyAuthError {
    FAILED, NotInitialized, ConfigNotInitialized
}