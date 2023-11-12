use std::str::FromStr;
use std::sync::Arc;
use ascii::AsciiString;
use log::{debug, info};
use rspotify::clients::OAuthClient;
use tiny_http::{Header, Method};
use tokio::task;
use tokio::task::JoinHandle;
use crate::{AppData};
use crate::utils::http::parse_url;

pub struct HttpEventLoop {
    thread: Option<JoinHandle<()>>,
    app_data: Arc<AppData>,
}

impl HttpEventLoop {
    pub fn new(app_data: Arc<AppData>) -> Self {
        Self {
            thread: None,
            app_data
        }
    }

    pub fn start(&mut self) {
        let thread = task::spawn({
            let app_data = self.app_data.clone();
            let config = app_data.config.clone();
            let tray = app_data.tray.clone();
            let http = app_data.http.clone();
            let spotify = app_data.spotify.clone();

            async move {
                for rq in http.incoming_requests() {

                    debug!("{} {}", rq.method(), rq.url());

                    if rq.method() != &Method::Get {
                        let response = tiny_http::Response::from_string("Method not supported, this http server is just for Spotify authentication :P".to_string());
                        let _ = rq.respond(response);
                        continue;
                    }

                    let http_url = parse_url(rq.url());

                    match http_url.route {
                        "/setup" => {
                            match spotify.get_authorize_url(true) {
                                Ok(url) => {
                                    let response = tiny_http::Response::from_string("")
                                        .with_header(Header {
                                            field: "Location".parse().unwrap(),
                                            value: AsciiString::from_str(url.as_str()).unwrap(),
                                        })
                                        .with_status_code(307);
                                    let _ = rq.respond(response);
                                }
                                Err(_) => {
                                    let response = tiny_http::Response::from_string("Couldn't get URL, check the configuration.");
                                    let _ = rq.respond(response);
                                }
                            }
                        }
                        "/callback" => {
                            match http_url.params.iter().find(|&&x| x.0 == "code") {
                                None => {
                                    let response = tiny_http::Response::from_string("No code found in the queries bruh.");
                                    let _ = rq.respond(response);
                                }
                                Some(code) => {
                                    info!("{}", code.to_owned().1);
                                    match spotify.request_token(code.to_owned().1).await {
                                        Ok(_) => {
                                            let mut config = config.lock().await;
                                            let mut cfg = config.cfg.as_mut().unwrap();

                                            let token = spotify.token.lock().await.unwrap();
                                            let token = token.as_ref().unwrap();

                                            cfg.spotify.token = token.access_token.clone();

                                            if let Some(refresh_token) = &token.refresh_token {
                                                cfg.spotify.refresh_token = refresh_token.clone();
                                            }

                                            // TODO: Send a reload config signal after this. Don't keep this.
                                            config.write().unwrap();

                                            let mut tray = tray.lock().await;
                                            tray.set_authenticated_label("Auth Status: Authenticated");

                                            let response = tiny_http::Response::from_string("Successfully authenticated, you can close this window.");
                                            let _ = rq.respond(response);
                                        }
                                        Err(_) => {
                                            let response = tiny_http::Response::from_string("Couldn't request the token with the code for some reason, try again.");
                                            let _ = rq.respond(response);
                                        }
                                    }
                                }
                            }

                        }
                        _ => {
                            let response = tiny_http::Response::from_string("Spotify OSC");
                            let _ = rq.respond(response);
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