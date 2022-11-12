use std::sync::Arc;
use reqwest::Client;
use tokio::sync::Mutex;
use crate::config::config::Config;
use crate::entities::config::ConfigFile;

pub mod spotify;

#[derive(Clone)]
pub struct WebData {
    pub client: Arc<Client>,
    pub config: Arc<Mutex<Config<ConfigFile>>>
    
}