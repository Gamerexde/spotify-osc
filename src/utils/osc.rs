use std::time::Duration;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use tokio::net::UdpSocket;

pub fn encode_packet(address: String, data: Vec<OscType>) -> rosc::Result<Vec<u8>> {
    encoder::encode(&OscPacket::Message(OscMessage {
        addr: address,
        args: data,
    }))
}

pub async fn send_to_delay(sock: &UdpSocket, buf: &[u8], address: &String, delay: Duration) {
    sock.send_to(&buf, &address).await.unwrap();
    tokio::time::sleep(delay).await;
}

pub struct OscPacketBuilder {
    data: Vec<OscType>,
    address: String,
}

impl OscPacketBuilder {
    pub fn new(address: String) -> Self {
        Self {
            address,
            data: vec![]
        }
    }

    pub fn add_string(mut self, value: String) -> OscPacketBuilder {
        self.data.push(OscType::String(value));
        self
    }

    pub fn add_float(mut self, value: f32) -> OscPacketBuilder {
        self.data.push(OscType::Float(value));
        self
    }

    pub fn add_bool(mut self, value: bool) -> OscPacketBuilder {
        self.data.push(OscType::Bool(value));
        self
    }

    pub fn build(self) -> rosc::Result<Vec<u8>> {
        encoder::encode(&OscPacket::Message(OscMessage {
            addr: self.address,
            args: self.data,
        }))
    }


}