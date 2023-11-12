use std::sync::Arc;
use log::{error, info};
use rosc::{OscError, OscMessage, OscPacket};
use rspotify::{AuthCodeSpotify, ClientError, ClientResult};
use rspotify::clients::OAuthClient;
use rspotify::model::{CurrentPlaybackContext, Device};
use rspotify::model::Type::Track;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::task;
use tokio::task::JoinHandle;
use crate::AppData;

pub struct OscEventLoop {
    thread: Option<JoinHandle<()>>,
    app_data: Arc<AppData>,
}

impl OscEventLoop {
    pub fn new(app_data: Arc<AppData>) -> Self {
        Self {
            thread: None,
            app_data,
        }
    }

    pub fn start(&mut self) {

        let thread = task::spawn({
            let app_data = self.app_data.clone();
            let spotify = app_data.spotify.clone();
            let config = app_data.config.clone();
            let socket = app_data.sock.clone();

            async move {
                let mut buf = [0u8; rosc::decoder::MTU];

                loop {
                    match socket.recv_from(&mut buf).await {
                        Ok((size, _)) => {
                            match rosc::decoder::decode_udp(&buf[..size]) {
                                Ok((_, packet)) => {

                                    if let OscPacket::Message(message) = packet {
                                        let address = message.addr.to_string();

                                        info!("Received message: {}", address);

                                        let authenticated = {
                                            let token = spotify.token.lock().await;
                                            let token = token.as_ref().unwrap();
                                            token.is_some()
                                        };

                                        info!("Spotify Auth status: {}", authenticated);

                                        if !authenticated {
                                            continue;
                                        }

                                        let parameters = {
                                            let config = config.lock().await;
                                            let cfg = config.cfg.as_ref().unwrap();

                                            cfg.parameters.clone()
                                        };


                                        let response = OSCResponse::new(message);

                                        if response.get_bool_parameter(parameters.spotify_play, true) {
                                            spotify_play_toggle(&spotify).await.ok();
                                        }

                                        if response.get_bool_parameter(parameters.spotify_next, true) {
                                            spotify_skip(&spotify).await.ok();
                                        }

                                        if response.get_bool_parameter(parameters.spotify_previous, true) {
                                            spotify_previous(&spotify).await.ok();
                                        }


                                    }
                                }
                                Err(err) => {
                                    error!("Error while decoding OSC packet. {}", err.to_string())
                                }
                            }
                        }
                        Err(_) => {

                        }
                    }
                }
            }
        });

        self.thread = Some(thread);
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.thread.take() {
            handle.abort();
        }
    }
}


async fn spotify_previous(spotify: &Arc<AuthCodeSpotify>) -> Result<(), ClientError> {

    match spotify.current_playback(None, None::<Vec<_>>).await? {
        None => {
            let devices = spotify.device().await?;

            for device in devices {
                if let Some(device_id) = device.id {
                    spotify.previous_track(Some(device_id.as_str())).await?;
                }
                break;
            }
        }
        Some(playback) => {
            if let Some(device) = playback.device.id {
                spotify.previous_track(Some(device.as_str())).await?;
            }

        }
    };

    Ok(())
}

async fn spotify_skip(spotify: &Arc<AuthCodeSpotify>) -> Result<(), ClientError> {

    match spotify.current_playback(None, None::<Vec<_>>).await? {
        None => {
            let devices = spotify.device().await?;

            for device in devices {
                if let Some(device_id) = device.id {
                    spotify.next_track(Some(device_id.as_str())).await?;
                }
                break;
            }
        }
        Some(playback) => {
            if let Some(device) = playback.device.id {
                spotify.next_track(Some(device.as_str())).await?;
            }

        }
    };

    Ok(())
}

async fn spotify_play_toggle(spotify: &Arc<AuthCodeSpotify>) -> Result<(), ClientError> {

    match spotify.current_playback(None, None::<Vec<_>>).await? {
        None => {
            let devices = spotify.device().await?;

            info!("No playback");

            /*
             * TODO: This grabs the first device it finds, not elegant at all. Add a tray option to select the playback device.
             */
            for device in devices {
                if let Some(device_id) = device.id {
                    info!("Starting playback from no playback");
                    spotify.resume_playback(Some(device_id.as_str()), None).await?;
                }
                break;
            }
        }
        Some(playback) => {
            if let Some(device) = playback.device.id {
                if playback.is_playing {
                    spotify.pause_playback(Some(device.as_str())).await?;
                } else {
                    spotify.resume_playback(Some(device.as_str()), None).await?;
                }
            }

        }
    };

    Ok(())
}


struct OSCResponse {
    message: OscMessage
}

impl OSCResponse {

    fn new(message: OscMessage) -> OSCResponse {
        Self {
            message
        }
    }

    fn get_bool_parameter(&self, parameter: String, state: bool) -> bool {
        if self.message.addr.eq(&parameter) {
            let osc_type = self.message.args[0].to_owned();

            if let Some(value) = osc_type.bool() {
                return value == state
            }
        }

        return false
    }

    fn get_float_parameter(&self, parameter: String) -> Option<f32> {
        if self.message.addr.eq(&parameter) {
            let osc_type = self.message.args[0].to_owned();

            return osc_type.float()
        }

        return None
    }
}