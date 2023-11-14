use std::sync::{Arc};
use std::time::Duration;
use ascii::AsciiChar::P;
use log::{error, info};
use rosc::{OscMessage, OscPacket, OscType};
use rspotify::{AuthCodeSpotify, ClientError};
use rspotify::clients::OAuthClient;
use rspotify::model::{CurrentPlaybackContext, Device, PlayableItem};
use tokio::{task, time};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::AppData;
use crate::utils::osc::{encode_packet, OscPacketBuilder, send_to_delay};
use crate::utils::time::seconds_to_music_time;

struct SpotifyVolumeThread {
    thread: JoinHandle<()>,
    spotify: Arc<AuthCodeSpotify>,
    volume: Arc<Mutex<u8>>,
    active: Arc<Mutex<bool>>
}

impl SpotifyVolumeThread {
    pub fn new(spotify: Arc<AuthCodeSpotify>) -> Self {
        let volume = Arc::new(Mutex::new(0_u8));
        let active = Arc::new(Mutex::new(false));

        let thread = task::spawn({
            let spotify = spotify.clone();
            let volume = volume.clone();
            let active = active.clone();

            async move {
                loop {
                    time::sleep(Duration::from_millis(500)).await;

                    let mut active = active.lock().await;

                    if !*active {
                        continue;
                    }

                    info!("Timer is now 0, setting the volume.");

                    *active = false;

                    drop(active);


                    let volume = volume.lock().await;

                    match get_current_device_playback(&spotify).await {
                        Ok(current_device_opt) => {
                            if let Some(current_device) = current_device_opt {
                                if let Some(device_id) = current_device.device.id {
                                    info!("Setting volume: {}%", *volume);
                                    spotify.volume(*volume, Some(device_id.as_str())).await.ok();
                                }
                            }
                        }
                        Err(_) => {}
                    }

                }
            }
        });

        Self {
            spotify,
            thread,
            volume,
            active
        }
    }

    pub fn stop(&self) {
        self.thread.abort();
    }

    pub async fn set_volume(&self, volume: u8) {
        let mut volume_mut = self.volume.lock().await;
        *volume_mut = volume;
    }

    pub async fn reset_timer(&self) {
        let mut active = self.active.lock().await;
        *active = true;
    }
}

pub struct OscEventLoop {
    osc_recieve_thread: JoinHandle<()>,
    osc_send_thread: JoinHandle<()>,
    app_data: Arc<AppData>,
    spotify_volume_thread: Arc<SpotifyVolumeThread>
}

impl OscEventLoop {
    pub fn new(app_data: Arc<AppData>) -> Self {
        let spotify_volume_thread = Arc::new(SpotifyVolumeThread::new(app_data.spotify.clone()));

        let osc_recieve_thread = task::spawn({
            let app_data = app_data.clone();
            let spotify = app_data.spotify.clone();
            let config = app_data.config.clone();
            let socket = app_data.sock.clone();
            let spotify_volume_thread = spotify_volume_thread.clone();

            async move {
                let mut buf = [0u8; rosc::decoder::MTU];

                loop {
                    match socket.recv_from(&mut buf).await {
                        Ok((size, _)) => {
                            match rosc::decoder::decode_udp(&buf[..size]) {
                                Ok((_, packet)) => {
                                    if let OscPacket::Message(message) = packet {
                                        let address = message.addr.to_string();

                                        let authenticated = {
                                            let token = spotify.token.lock().await;
                                            let token = token.as_ref().unwrap();
                                            token.is_some()
                                        };

                                        if !authenticated {
                                            continue;
                                        }

                                        let parameters = {
                                            let config = config.lock().await;
                                            let cfg = config.cfg.as_ref().unwrap();

                                            cfg.parameters.clone()
                                        };

                                        let response = OSCResponse::new(message);

                                        if let Some(value) = response.get_float_parameter(parameters.spotify_volume) {

                                            spotify_volume_thread.set_volume((value * 100_f32) as u8).await;
                                            spotify_volume_thread.reset_timer().await;
                                        }

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
                        Err(_) => {}
                    }
                }
            }
        });

        let osc_send_thread = task::spawn({
            let app_data = app_data.clone();
            let spotify = app_data.spotify.clone();
            let config = app_data.config.clone();
            let socket = app_data.sock.clone();

            async move {
                loop {
                    time::sleep(Duration::from_secs(3)).await;

                    let authenticated = {
                        let token = spotify.token.lock().await;
                        let token = token.as_ref().unwrap();
                        token.is_some()
                    };

                    if !authenticated {
                        continue;
                    }

                    let (parameters, client_address) = {
                        let config = config.lock().await;
                        let cfg = config.cfg.as_ref().unwrap();

                        (cfg.parameters.clone(), cfg.general.osc.client_address.clone())
                    };


                    if let Ok(Some(current_playback_device)) = get_current_device_playback(&spotify).await {
                        if let Some(current_playback) = current_playback_device.playback {
                            if let Some(item) = current_playback.item {
                                match item {
                                    PlayableItem::Track(track) => {
                                        let artist = track.artists.first().unwrap();
                                        let song = track.name;

                                        let packet = OscPacketBuilder::new(parameters.spotify_chatbox)
                                            .add_string(
                                                format!("[Spotify] [{}] Playing: {} - {} [{}] - [{}]", (if current_playback.is_playing { "▶" } else { "⏸" }),
                                                        artist.name, song, seconds_to_music_time(current_playback.progress.unwrap().num_seconds()),
                                                        seconds_to_music_time(track.duration.num_seconds()))
                                            ).add_bool(true)
                                            .build().unwrap();

                                        send_to_delay(&socket, packet.as_slice(), &client_address, Duration::from_millis(20)).await;
                                    }
                                    PlayableItem::Episode(_) => {}
                                };
                            }
                        }
                    }
                }
            }
        });

        Self {
            spotify_volume_thread,
            osc_recieve_thread,
            osc_send_thread,
            app_data,
        }
    }

    pub fn stop(&mut self) {
        self.osc_recieve_thread.abort();
        self.osc_send_thread.abort();

        self.spotify_volume_thread.stop();
    }
}


struct CurrentDevicePlayback {
    device: Device,
    playback: Option<CurrentPlaybackContext>,
}

async fn get_current_device_playback(spotify: &Arc<AuthCodeSpotify>) -> Result<Option<CurrentDevicePlayback>, ClientError> {
    match spotify.current_playback(None, None::<Vec<_>>).await? {
        None => {
            let devices = spotify.device().await?;

            for device in devices {
                return Ok(Some(CurrentDevicePlayback {
                    device,
                    playback: None,
                }));
            }
        }
        Some(playback) => {
            return Ok(Some(CurrentDevicePlayback {
                device: playback.device.clone(),
                playback: Some(playback),
            }));
        }
    };

    Ok((None))
}

async fn spotify_previous(spotify: &Arc<AuthCodeSpotify>) -> Result<(), ClientError> {
    if let Some(current_playback) = get_current_device_playback(&spotify).await? {
        if let Some(device_id) = current_playback.device.id {
            spotify.previous_track(Some(device_id.as_str())).await?;
        }
    }

    Ok(())
}

async fn spotify_skip(spotify: &Arc<AuthCodeSpotify>) -> Result<(), ClientError> {
    if let Some(current_playback) = get_current_device_playback(&spotify).await? {
        if let Some(device_id) = current_playback.device.id {
            spotify.next_track(Some(device_id.as_str())).await?;
        }
    }

    Ok(())
}

async fn spotify_play_toggle(spotify: &Arc<AuthCodeSpotify>) -> Result<(), ClientError> {
    if let Some(current_playback) = get_current_device_playback(&spotify).await? {
        match current_playback.playback {
            None => {
                if let Some(device_id) = current_playback.device.id {
                    spotify.resume_playback(Some(device_id.as_str()), None).await?;
                }
            }
            Some(playback) => {
                if let Some(device_id) = current_playback.device.id {
                    if playback.is_playing {
                        spotify.pause_playback(Some(device_id.as_str())).await?;
                    } else {
                        spotify.resume_playback(Some(device_id.as_str()), None).await?;
                    }
                }
            }
        }
    }

    Ok(())
}


struct OSCResponse {
    message: OscMessage,
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
                return value == state;
            }
        }

        return false;
    }

    fn get_float_parameter(&self, parameter: String) -> Option<f32> {
        if self.message.addr.eq(&parameter) {
            let osc_type = self.message.args[0].to_owned();

            return osc_type.float();
        }

        return None;
    }
}