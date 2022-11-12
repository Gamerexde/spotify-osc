use crate::entities::spotify::SpotifyInfo;

pub mod spotify;

pub enum SpotifyValue {
    INFO(SpotifyInfo),
    EMPTY
}

pub enum RequestError {
    UNAUTHORIZED,
    OTHER
}